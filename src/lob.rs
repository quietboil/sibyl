//! Functions for performing operations on large objects (LOBs).

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use crate::{Result, Session, oci::{self, *}, stmt::{ToSql, ToSqlOut, Params}, session::SvcCtx};
#[cfg(feature="nonblocking")]
use crate::task;

/// A marker trait for internal LOB descriptors - CLOB, NCLOB and BLOB.
pub trait InternalLob {}
impl InternalLob for OCICLobLocator {}
impl InternalLob for OCIBLobLocator {}

pub(crate) fn is_initialized<T>(locator: &Descriptor<T>, env: &OCIEnv, err: &OCIError) -> Result<bool>
where T: DescriptorType<OCIType=OCILobLocator>
{
    let mut flag = 0u8;
    oci::lob_locator_is_init(env, err, locator, &mut flag)?;
    Ok( flag != 0 )
}

pub(crate) const LOB_IS_TEMP       : u32 = 1;
pub(crate) const LOB_IS_OPEN       : u32 = 2;
pub(crate) const LOB_FILE_IS_OPEN  : u32 = 4;

struct LobInner<T> where T: DescriptorType<OCIType=OCILobLocator>  + 'static {
    locator: Descriptor<T>,
    svc: Arc<SvcCtx>,
    status_flags: AtomicU32,
}

impl<T> Drop for LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> + 'static {
    #[cfg(feature="blocking")]
    fn drop(&mut self) {
        let svc: &OCISvcCtx     = self.as_ref();
        let err: &OCIError      = self.as_ref();
        let loc: &OCILobLocator = self.as_ref();
        let status_flags = self.status_flags.load(Ordering::Acquire);

        if status_flags & LOB_IS_OPEN != 0 {
            unsafe {
                OCILobClose(svc, err, loc);
            }
        } else if status_flags & LOB_FILE_IS_OPEN != 0 {
            unsafe {
                OCILobFileClose(svc, err, loc);
            }
        }
        if status_flags & LOB_IS_TEMP != 0 {
            unsafe {
                OCILobFreeTemporary(svc, err, loc);
            }
        } else {
            let mut flag = 0u8;
            if oci::lob_is_temporary(svc, err, loc, &mut flag).is_ok() && flag != 0 {
                unsafe {
                    OCILobFreeTemporary(svc, err, loc);
                }
            }
        }
    }

    #[cfg(feature="nonblocking")]
    fn drop(&mut self) {
        let ctx = self.svc.clone();
        let loc = Descriptor::take_over(&mut self.locator);
        let flags = self.status_flags.load(Ordering::Acquire);
        task::spawn(futures::LobDrop::new(ctx, loc, flags));
    }
}

impl<T> AsRef<OCILobLocator> for LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCILobLocator {
        self.locator.as_ref()
    }
}

impl<T> AsRef<OCIEnv> for LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCIEnv {
        self.svc.as_ref().as_ref()
    }
}

impl<T> AsRef<OCIError> for LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCIError {
        self.svc.as_ref().as_ref()
    }
}

impl<T> AsRef<OCISvcCtx> for LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCISvcCtx {
        self.svc.as_ref().as_ref()
    }
}

impl<T> LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn new(locator: Descriptor<T>, svc: Arc<SvcCtx>) -> Self {
        Self { locator, svc, status_flags: AtomicU32::new(0) }
    }

    fn new_temp(locator: Descriptor<T>, svc: Arc<SvcCtx>) -> Self {
        Self { locator, svc, status_flags: AtomicU32::new(LOB_IS_TEMP) }
    }
}

/// LOB locator.
pub struct LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> + 'static
{
    inner: LobInner<T>,
    chunk_size: AtomicU32,
    session: &'a Session<'a>,
}

impl<'a,T> AsRef<Descriptor<T>> for LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    fn as_ref(&self) -> &Descriptor<T> {
        &self.inner.locator
    }
}

impl<'a,T> AsRef<OCILobLocator> for LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCILobLocator {
        self.inner.as_ref()
    }
}

impl<'a,T> AsRef<OCIEnv> for LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCIEnv {
        self.session.as_ref()
    }
}

impl<'a,T> AsRef<OCIError> for LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCIError {
        self.session.as_ref()
    }
}

impl<'a,T> AsRef<OCISvcCtx> for LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCISvcCtx {
        self.session.as_ref()
    }
}

impl<'a,T> LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {

    fn make_new(locator: Descriptor<T>, session: &'a Session) -> Self {
        Self {
            inner: LobInner::new(locator, session.get_svc()),
            chunk_size: AtomicU32::new(0),
            session
        }
    }

    fn make_temp(locator: Descriptor<T>, session: &'a Session) -> Self {
        Self {
            inner: LobInner::new_temp(locator, session.get_svc()),
            chunk_size: AtomicU32::new(0),
            session
        }
    }

    /**
    Determines whether the LOB locator belongs to a local database table or a remote database
    table. The value `true` indicates that the LOB locator is from a remote database table.
    The application must fetch the LOB descriptor from the database before querying this attribute.
    */
    pub fn is_remote(&self) -> Result<bool> {
        let is_remote: u8 = self.inner.locator.get_attr(OCI_ATTR_LOB_REMOTE, self.as_ref())?;
        Ok( is_remote != 0 )
    }

    /// Returns the LOB's `SQLT` type, i.e. SQLT_CLOB, SQLT_BLOB or SQLT_BFILE.
    pub fn get_type(&self) -> Result<u16> {
        let lob_type: u16 = self.inner.locator.get_attr(OCI_ATTR_LOB_TYPE, self.as_ref())?;
        Ok( lob_type )
    }

    /**
    Returns `true` if the given LOB or BFILE locator is initialized.

    InternalLob LOB locators can be initialized by one of these methods:
    - Selecting a non-NULL LOB into the locator
    - Pinning an object that contains a non-NULL LOB attribute
    - Setting the locator to empty by calling `empty`

    BFILE locators can be initialized by one of these methods:
    - Selecting a non-NULL BFILE into the locator
    - Pinning an object that contains a non-NULL BFILE attribute
    - Calling `set_file_name`
    */
    pub fn is_initialized(&self) -> Result<bool> {
        is_initialized(&self.inner.locator, self.as_ref(), self.as_ref())
    }

    /**
    Compares the given LOB or BFILE locators for equality. Two LOB or BFILE locators are equal
    if and only if they both refer to the same LOB or BFILE value.

    Two NULL locators are considered not equal by this function.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    use sibyl::CLOB;

    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    # let stmt = session.prepare("
    #     declare
    #         name_already_used exception; pragma exception_init(name_already_used, -955);
    #     begin
    #         execute immediate '
    #             create table test_lobs (
    #                 id       number generated always as identity,
    #                 text     clob,
    #                 data     blob,
    #                 ext_file bfile
    #             )
    #         ';
    #     exception
    #         when name_already_used then null;
    #     end;
    # ")?;
    # stmt.execute(())?;
    let stmt = session.prepare("
        INSERT INTO test_lobs (text) VALUES (empty_clob()) RETURN id INTO :id
    ")?;
    let mut id : usize = 0;
    stmt.execute_into((), &mut id)?;

    // must lock LOB's row before writing into the LOB
    let stmt = session.prepare("
        SELECT text FROM test_lobs WHERE id = :ID FOR UPDATE
    ")?;
    let rows = stmt.query(&id)?;
    let row = rows.next()?.expect("one row");
    let lob : CLOB = row.get(0)?.expect("CLOB locator for writing");

    let text = "
        To Mercy, Pity, Peace, and Love
        All pray in their distress;
        And to these virtues of delight
        Return their thankfulness.
    ";
    lob.open()?;
    lob.append(text)?;
    lob.close()?;
    session.commit()?;

    // Retrieve this CLOB twice into two different locators
    let stmt = session.prepare("
        SELECT text FROM test_lobs WHERE id = :id
    ")?;
    let rows = stmt.query(&id)?;
    let row = rows.next()?.expect("one row");
    let lob1 : CLOB = row.get(0)?.expect("CLOB locator for reading");

    let rows = stmt.query(&id)?;
    let row = rows.next()?.expect("one row");
    let lob2 : CLOB = row.get(0)?.expect("CLOB locator for reading");

    // Even though locators are different, they point to
    // the same LOB which makes them "equal"
    assert!(lob1.is_equal(&lob2)?, "CLOB1 == CLOB2");
    assert!(lob2.is_equal(&lob1)?, "CLOB2 == CLOB1");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # let stmt = session.prepare("
    #     declare
    #         name_already_used exception; pragma exception_init(name_already_used, -955);
    #     begin
    #         execute immediate '
    #             create table test_lobs (
    #                 id       number generated always as identity,
    #                 text     clob,
    #                 data     blob,
    #                 ext_file bfile
    #             )
    #         ';
    #     exception
    #         when name_already_used then null;
    #     end;
    # ").await?;
    # stmt.execute(()).await?;
    # let stmt = session.prepare("INSERT INTO test_lobs (text) VALUES (empty_clob()) RETURN id INTO :id").await?;
    # let mut id : usize = 0;
    # stmt.execute_into((), &mut id).await?;
    # let stmt = session.prepare("SELECT text FROM test_lobs WHERE id = :ID FOR UPDATE").await?;
    # let rows = stmt.query(&id).await?;
    # let row = rows.next().await?.expect("one row");
    # let lob : CLOB = row.get(0)?.expect("CLOB locator for writing");
    # let text = "
    #     To Mercy, Pity, Peace, and Love
    #     All pray in their distress;
    #     And to these virtues of delight
    #     Return their thankfulness.
    # ";
    # lob.open().await?;
    # lob.append(text).await?;
    # lob.close().await?;
    # session.commit().await?;
    # let stmt = session.prepare("SELECT text FROM test_lobs WHERE id = :id").await?;
    # let rows = stmt.query(&id).await?;
    # let row = rows.next().await?.expect("one row");
    # let lob1 : CLOB = row.get(0)?.expect("CLOB locator for reading");
    # let rows = stmt.query(&id).await?;
    # let row = rows.next().await?.expect("one row");
    # let lob2 : CLOB = row.get(0)?.expect("CLOB locator for reading");
    # assert!(lob1.is_equal(&lob2)?, "CLOB1 == CLOB2");
    # assert!(lob2.is_equal(&lob1)?, "CLOB2 == CLOB1");
    # Ok(()) })
    # }
    ```
    */
    pub fn is_equal<U>(&self, other: &LOB<'a,U>) -> Result<bool>
    where U: DescriptorType<OCIType=OCILobLocator>
    {
        let mut flag = 0u8;
        oci::lob_is_equal(self.as_ref(), self.as_ref(), other.as_ref(), &mut flag)?;
        Ok( flag != 0 )
    }

    /**
    Returns the character set form of the input CLOB or NCLOB locator. If the input locator is for a BLOB
    or a BFILE, it returns `CharSetForm::Undefined` because there is no concept of a character set for binary
    LOBs or binary files.
    */
    pub fn charset_form(&self) -> Result<CharSetForm> {
        let mut csform = 0u8;
        oci::lob_char_set_form(self.as_ref(), self.as_ref(), self.as_ref(), &mut csform)?;
        let csform = match csform {
            SQLCS_IMPLICIT => CharSetForm::Implicit,
            SQLCS_NCHAR    => CharSetForm::NChar,
            _              => CharSetForm::Undefined
        };
        Ok( csform )
    }

    /**
    Returns the LOB locator's database character set ID. If the input locator is for a BLOB or a BFILE,
    it returns 0 because there is no concept of a character set for binary LOBs or binary files.
    */
    pub fn charset_id(&self) -> Result<u16> {
        let mut csid = 0u16;
        oci::lob_char_set_id(self.as_ref(), self.as_ref(), self.as_ref(), &mut csid)?;
        Ok( csid )
    }
}

impl<'a, T> LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> + InternalLob {
    /// Creates a new empty LOB. This is an alias for `empty`.
    pub fn new(session: &'a Session) -> Result<Self> {
        Self::empty(session)
    }

    /**
    Creates a new empty LOB.

    The locator can then be used as a bind variable for an INSERT or UPDATE statement
    to initialize the LOB to empty. Once the LOB is empty, `write` can be called to
    populate the LOB with data.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    use sibyl::{ CLOB };

    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    # let stmt = session.prepare("
    #     declare
    #         name_already_used exception; pragma exception_init(name_already_used, -955);
    #     begin
    #         execute immediate '
    #             create table test_lobs (
    #                 id       number generated always as identity,
    #                 text     clob,
    #                 data     blob,
    #                 ext_file bfile
    #             )
    #         ';
    #     exception
    #         when name_already_used then null;
    #     end;
    # ")?;
    # stmt.execute(())?;
    let stmt = session.prepare("
        INSERT INTO test_lobs (text) VALUES (:new_lob) RETURNING id INTO :id
    ")?;
    let mut id : usize = 0;
    let lob = CLOB::empty(&session)?;
    stmt.execute_into(&lob, &mut id)?;
    # assert!(id > 0);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # let stmt = session.prepare("
    #     declare
    #         name_already_used exception; pragma exception_init(name_already_used, -955);
    #     begin
    #         execute immediate '
    #             create table test_lobs (
    #                 id       number generated always as identity,
    #                 text     clob,
    #                 data     blob,
    #                 ext_file bfile
    #             )
    #         ';
    #     exception
    #         when name_already_used then null;
    #     end;
    # ").await?;
    # stmt.execute(()).await?;
    # let stmt = session.prepare("
    #     insert into test_lobs (text) values (:new_lob) returning id into :id
    # ").await?;
    # let mut id : usize = 0;
    # let lob = CLOB::empty(&session)?;
    # stmt.execute_into(&lob, &mut id).await?;
    # assert!(id > 0);
    # Ok(()) })
    # }
    ```
    */
    pub fn empty(session: &'a Session) -> Result<Self> {
        let locator = Descriptor::<T>::new(session)?;
        locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, session.as_ref())?;
        Ok(Self::make_new(locator, session))
    }

    /**
    Sets the internal LOB locator to empty.

    The locator can then be used as a bind variable for an INSERT or UPDATE statement
    to initialize the LOB to empty. Once the LOB is empty, `write` can be called to
    populate the LOB with data.
    */
    pub fn clear(&self) -> Result<()> {
        self.inner.locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, self.as_ref())
    }
}

impl<'a> LOB<'a,OCICLobLocator> {
    /**
    Returns `true` if the LOB locator is for an NCLOB.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let lob = CLOB::temp(&session, CharSetForm::NChar, Cache::No)?;

    assert!(lob.is_nclob()?);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # let lob = CLOB::temp(&session, CharSetForm::NChar, Cache::No).await?;
    # assert!(lob.is_nclob()?);
    # Ok(()) })
    # }
    ```
    */
    pub fn is_nclob(&self) -> Result<bool> {
        let csform = self.charset_form()?;
        Ok( csform as u8 == CharSetForm::NChar as u8 )
    }
}

impl<'a> LOB<'a,OCIBFileLocator> {
    /// Creates a new uninitialized BFILE.
    pub fn new(session: &'a Session) -> Result<Self> {
        let locator = Descriptor::<OCIBFileLocator>::new(session)?;
        Ok(Self::make_new(locator, session))
    }

    /**
    Returns the directory object and file name associated with this BFILE locator.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    use sibyl::BFile;

    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
    let (dir_name, file_name) = file.file_name()?;

    assert_eq!(dir_name, "MEDIA_DIR");
    assert_eq!(file_name, "hello_world.txt");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # let file = BFile::new(&session)?;
    # file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
    # let (dir_name, file_name) = file.file_name()?;
    # assert_eq!(dir_name, "MEDIA_DIR");
    # assert_eq!(file_name, "hello_world.txt");
    # Ok(()) })
    # }
    ```
    */
    pub fn file_name(&self) -> Result<(String,String)> {
        let mut dir  = String::with_capacity(30);
        let mut name = String::with_capacity(255);
        let mut dir_len  = dir.capacity() as u16;
        let mut name_len = name.capacity() as u16;
        unsafe {
            let dir  = dir.as_mut_vec();
            let name = name.as_mut_vec();
            oci::lob_file_get_name(
                self.as_ref(), self.as_ref(), self.as_ref(),
                dir.as_mut_ptr(),  &mut dir_len  as *mut u16,
                name.as_mut_ptr(), &mut name_len as *mut u16
            )?;
            dir.set_len(dir_len as usize);
            name.set_len(name_len as usize);
        }
        Ok( ( dir, name ) )
    }

    /**
    Sets the directory object and file name in the BFILE locator.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    use sibyl::BFile;

    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let mut file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;

    assert!(file.file_exists()?);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # let mut file = BFile::new(&session)?;
    # file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
    # assert!(file.file_exists().await?);
    # Ok(()) })
    # }
    ```
    */
    pub fn set_file_name(&self, dir: &str, name: &str) -> Result<()> {
        oci::lob_file_set_name(
            self.as_ref(), self.as_ref(),
            self.inner.locator.as_ptr() as _,
            dir.as_ptr(),  dir.len() as u16,
            name.as_ptr(), name.len() as u16
        )
    }
}

macro_rules! impl_lob_to_sql {
    ($($ts:ty),+) => {
        $(
            impl ToSql for &LOB<'_, $ts> {
                fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    self.inner.locator.bind_to(pos, params, stmt, err)
                }
            }
            impl ToSqlOut for &mut LOB<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    self.inner.locator.bind_to(pos, params, stmt, err)
                }
            }
        )+
    };
}

impl_lob_to_sql!{ OCICLobLocator, OCIBLobLocator, OCIBFileLocator }

impl LOB<'_,OCICLobLocator> {
    /// Debug helper that fetches first 50 (at most) bytes of CLOB content
    #[cfg(feature="blocking")]
    fn content_head(&self) -> Result<String> {
        const MAX_LEN : usize = 50;
        let len = self.len()?;
        let len = std::cmp::min(len, MAX_LEN);
        let mut buf = String::new();
        let len = self.read(0, len, &mut buf)?;
        if len == MAX_LEN {
            buf.push_str("...");
        }
        Ok(buf)
    }
}

impl std::fmt::Debug for LOB<'_,OCICLobLocator> {
    #[cfg(feature="blocking")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.content_head() {
            Ok(text) => f.write_fmt(format_args!("CLOB {}", text)),
            Err(err) => f.write_fmt(format_args!("CLOB {:?}", err))
        }
    }

    #[cfg(feature="nonblocking")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CLOB")
    }
}

impl LOB<'_,OCIBLobLocator> {
    /// Debug helper that fetches first 50 (at most) bytes of BLOB content
    #[cfg(feature="blocking")]
    fn content_head(&self) -> Result<String> {
        const MAX_LEN : usize = 50;
        let len = self.len()?;
        let len = std::cmp::min(len, MAX_LEN);
        let mut buf = Vec::new();
        let len = self.read(0, len, &mut buf)?;
        let res = &buf[..len];
        let res = if len == MAX_LEN { format!("{:?}...", res) } else { format!("{:?}", res) };
        Ok(res)
    }
}

impl std::fmt::Debug for LOB<'_,OCIBLobLocator> {
    #[cfg(feature="blocking")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.content_head() {
            Ok(text) => f.write_fmt(format_args!("BLOB {}", text)),
            Err(err) => f.write_fmt(format_args!("BLOB {:?}", err))
        }
    }

    #[cfg(feature="nonblocking")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BLOB")
    }
}

impl LOB<'_,OCIBFileLocator> {
    /// Debug helper that fetches first 50 (at most) bytes of BFILE content
    #[cfg(feature="blocking")]
    fn content_head(&self) -> Result<String> {
        const MAX_LEN : usize = 50;
        let len = self.len()?;
        let len = std::cmp::min(len, MAX_LEN);
        let mut buf = Vec::new();
        let len = self.read(0, len, &mut buf)?;
        let res = &buf[..len];
        let res = if len == MAX_LEN { format!("{:?}...", res) } else { format!("{:?}", res) };
        Ok(res)
    }
}

impl std::fmt::Debug for LOB<'_,OCIBFileLocator> {
    #[cfg(feature="blocking")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.content_head() {
            Ok(text) => f.write_fmt(format_args!("BFILE {}", text)),
            Err(err) => f.write_fmt(format_args!("BFILE {:?}", err))
        }
    }

    #[cfg(feature="nonblocking")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BFILE")
    }
}
