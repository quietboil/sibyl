//! Functions for performing operations on large objects (LOBs).

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use crate::{Result, Session, oci::{self, *}, stmt::{ToSql, Params}, session::SvcCtx};

/// A marker trait for internal LOB descriptors - CLOB, NCLOB and BLOB.
pub trait InternalLob {}
impl InternalLob for OCICLobLocator {}
impl InternalLob for OCIBLobLocator {}

pub(crate) fn is_initialized(locator: &Ptr<OCILobLocator>, env: &OCIEnv, err: &OCIError) -> Result<bool> {
    let mut flag = oci::Aligned::new(0u8);
    oci::lob_locator_is_init(env, err, locator.as_ref(), flag.as_mut_ptr())?;
    Ok( <u8>::from(flag) != 0 )
}

pub(crate) const LOB_IS_TEMP       : u32 = 1;
pub(crate) const LOB_IS_OPEN       : u32 = 2;
pub(crate) const LOB_FILE_IS_OPEN  : u32 = 4;

struct LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> + 'static {
    locator: Ptr<T::OCIType>,
    _descriptor: Option<Descriptor<T>>,
    svc: Arc<SvcCtx>,
    status_flags: AtomicU32,
}

#[cfg(not(docsrs))]
impl<T> Drop for LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> + 'static {
    fn drop(&mut self) {
        let svc: &OCISvcCtx     = self.as_ref();
        let err: &OCIError      = self.as_ref();
        let loc: &OCILobLocator = self.as_ref();
        let status_flags = self.status_flags.load(Ordering::Acquire);

        #[cfg(feature="nonblocking")]
        let _ = self.svc.set_blocking_mode();
        
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
            let env: &OCIEnv = self.as_ref();
            let mut flag = oci::Aligned::new(0u8);
            if oci::lob_is_temporary(env, err, loc, flag.as_mut_ptr()).is_ok() && <u8>::from(flag) != 0 {
                unsafe {
                    OCILobFreeTemporary(svc, err, loc);
                }
            }
        }

        #[cfg(feature="nonblocking")]
        let _ = self.svc.set_nonblocking_mode();
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
    fn new(descriptor: Descriptor<T>, svc: Arc<SvcCtx>) -> Self {
        Self { locator: descriptor.get_ptr(), _descriptor: Some(descriptor), svc, status_flags: AtomicU32::new(0) }
    }

    fn new_temp(descriptor: Descriptor<T>, svc: Arc<SvcCtx>) -> Self {
        Self { locator: descriptor.get_ptr(), _descriptor: Some(descriptor), svc, status_flags: AtomicU32::new(LOB_IS_TEMP) }
    }

    fn at_column(descriptor: &Descriptor<T>, svc: Arc<SvcCtx>) -> Self {
        // Descriptor stays at the column buffer
        Self { locator: descriptor.get_ptr(), _descriptor: None, svc, status_flags: AtomicU32::new(0) }
    }
}

/// LOB locator.
pub struct LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> + 'static
{
    inner: LobInner<T>,
    chunk_size: AtomicU32,
    session: &'a Session<'a>,
}

// impl<'a,T> AsRef<Descriptor<T>> for LOB<'a,T>
//     where T: DescriptorType<OCIType=OCILobLocator>
// {
//     fn as_ref(&self) -> &Descriptor<T> {
//         &self.inner.locator
//     }
// }

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

    fn make_new(lob_descriptor: Descriptor<T>, session: &'a Session) -> Self {
        Self {
            inner: LobInner::new(lob_descriptor, session.get_svc()),
            chunk_size: AtomicU32::new(0),
            session
        }
    }

    fn make_temp(lob_descriptor: Descriptor<T>, session: &'a Session) -> Self {
        Self {
            inner: LobInner::new_temp(lob_descriptor, session.get_svc()),
            chunk_size: AtomicU32::new(0),
            session
        }
    }

    pub(crate) fn at_column(column_lob_descriptor: &Descriptor<T>, session: &'a Session) -> Self {
        Self {
            inner: LobInner::at_column(column_lob_descriptor, session.get_svc()),
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
        let is_remote: u8 = attr::get(OCI_ATTR_LOB_REMOTE, T::get_type(), self.inner.locator.as_ref(), self.as_ref())?;
        Ok( is_remote != 0 )
    }

    /// Returns the LOB's `SQLT` type, i.e. SQLT_CLOB, SQLT_BLOB or SQLT_BFILE.
    pub fn get_type(&self) -> Result<u16> {
        let lob_type: u16 = attr::get(OCI_ATTR_LOB_TYPE, T::get_type(), self.inner.locator.as_ref(), self.as_ref())?;
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

    # Example (blocking)

    ```
    use sibyl::CLOB;
    /*
        CREATE TABLE test_lobs (
            id       INTEGER GENERATED ALWAYS AS IDENTITY,
            text     CLOB,
            data     BLOB,
            ext_file BFILE
        )
     */
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        INSERT INTO test_lobs (text) VALUES (empty_clob()) RETURNING id INTO :ID
    ")?;
    let mut id = 0;
    stmt.execute(&mut id)?;

    let stmt = session.prepare("
        SELECT text FROM test_lobs WHERE id = :ID
    ")?;
    let row = stmt.query_single(&id)?.unwrap();
    let lob : CLOB = row.get(0)?;
    
    let text = "
        To Mercy, Pity, Peace, and Love
        All pray in their distress;
        And to these virtues of delight
        Return their thankfulness.
    ";
    lob.append(text)?;
    session.commit()?;

    // Retrieve this CLOB twice into two different locators
    let row1 = stmt.query_single(&id)?.unwrap();
    let lob1 : CLOB = row1.get(0)?;

    let row2 = stmt.query_single(&id)?.unwrap();
    let lob2 : CLOB = row2.get(0)?;

    // Even though locators are different, they point to
    // the same LOB which makes them "equal"
    assert!(lob1.is_equal(&lob2)?, "CLOB1 == CLOB2");
    assert!(lob2.is_equal(&lob1)?, "CLOB2 == CLOB1");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main()  {}
    ```

    # Example (nonblocking)

    ```
    use sibyl::CLOB;
    /*
        CREATE TABLE test_lobs (
            id       INTEGER GENERATED ALWAYS AS IDENTITY,
            text     CLOB,
            data     BLOB,
            ext_file BFILE
        )
     */
    # use sibyl::Result;
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
        INSERT INTO test_lobs (text) VALUES (Empty_CLOB()) RETURNING id INTO :ID
    ").await?;
    let mut id = 0;
    stmt.execute(&mut id).await?;

    let stmt = session.prepare("
        SELECT text FROM test_lobs WHERE id = :ID
    ").await?;
    let row = stmt.query_single(&id).await?.unwrap();
    let lob : CLOB = row.get(0)?;
    
    let text = "
        To Mercy, Pity, Peace, and Love
        All pray in their distress;
        And to these virtues of delight
        Return their thankfulness.
    ";
    lob.append(text).await?;
    session.commit().await?;

    // Retrieve this CLOB twice into two different locators
    let row1 = stmt.query_single(&id).await?.unwrap();
    let lob1 : CLOB = row1.get(0)?;

    let row2 = stmt.query_single(&id).await?.unwrap();
    let lob2 : CLOB = row2.get(0)?;

    // Even though locators are different, they point to
    // the same LOB which makes them "equal"
    assert!(lob1.is_equal(&lob2)?, "CLOB1 == CLOB2");
    assert!(lob2.is_equal(&lob1)?, "CLOB2 == CLOB1");
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main()  {}
    ```
    */
    pub fn is_equal<U>(&self, other: &LOB<'a,U>) -> Result<bool>
    where U: DescriptorType<OCIType=OCILobLocator>
    {
        let mut flag = oci::Aligned::new(0u8);
        oci::lob_is_equal(self.as_ref(), self.as_ref(), other.as_ref(), flag.as_mut_ptr())?;
        Ok( <u8>::from(flag) != 0 )
    }

    /**
    Returns the character set form of the input CLOB or NCLOB locator. If the input locator is for a BLOB
    or a BFILE, it returns `CharSetForm::Undefined` because there is no concept of a character set for binary
    LOBs or binary files.
    */
    pub fn charset_form(&self) -> Result<CharSetForm> {
        let mut csform = oci::Aligned::new(0u8);
        oci::lob_char_set_form(self.as_ref(), self.as_ref(), self.as_ref(), csform.as_mut_ptr())?;
        let csform = match csform.into() {
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
        let mut csid = oci::Aligned::new(0u16);
        oci::lob_char_set_id(self.as_ref(), self.as_ref(), self.as_ref(), csid.as_mut_ptr())?;
        Ok( csid.into() )
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

    # Example (blocking)

    ```
    use sibyl::{ CLOB };
    /*
        CREATE TABLE test_lobs (
            id       INTEGER GENERATED ALWAYS AS IDENTITY,
            text     CLOB,
            data     BLOB,
            ext_file BFILE
        )
     */
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        INSERT INTO test_lobs (text) VALUES (:NEW_LOB) RETURNING id INTO :ID
    ")?;
    let mut id : usize = 0;
    let lob = CLOB::empty(&session)?;
    stmt.execute((&lob, &mut id, ()))?;
    # assert!(id > 0);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    # Example (nonblocking)

    ```
    use sibyl::{ CLOB };

    # use sibyl::Result;
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
        INSERT INTO test_lobs (text) VALUES (:NEW_LOB) RETURNING id INTO :ID
    ").await?;
    let mut id : usize = 0;
    let lob = CLOB::empty(&session)?;
    stmt.execute((&lob, &mut id, ())).await?;
    # assert!(id > 0);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn empty(session: &'a Session) -> Result<Self> {
        let lob_descriptor = Descriptor::<T>::new(session)?;
        lob_descriptor.set_attr(OCI_ATTR_LOBEMPTY, 0u32, session.as_ref())?;
        Ok(Self::make_new(lob_descriptor, session))
    }

    /**
    Sets the internal LOB locator to empty.

    The locator can then be used as a bind variable for an INSERT or UPDATE statement
    to initialize the LOB to empty. Once the LOB is empty, `write` can be called to
    populate the LOB with data.
    */
    pub fn clear(&self) -> Result<()> {
        attr::set(OCI_ATTR_LOBEMPTY, 0u32, T::get_type(), self.inner.locator.as_ref(), self.as_ref())
        // self.inner.locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, self.as_ref())
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
    # let session = sibyl::test_env::get_session()?;
    let lob = CLOB::temp(&session, CharSetForm::NChar, Cache::No)?;

    assert!(lob.is_nclob()?);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
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
    # let session = sibyl::test_env::get_session()?;
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
    # let session = sibyl::test_env::get_session().await?;
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
        let mut dir_len  = oci::Aligned::new(dir.capacity() as u16);
        let mut name_len = oci::Aligned::new(name.capacity() as u16);
        unsafe {
            let dir  = dir.as_mut_vec();
            let name = name.as_mut_vec();
            oci::lob_file_get_name(
                self.as_ref(), self.as_ref(), self.as_ref(),
                dir.as_mut_ptr(),  dir_len.as_mut_ptr(),
                name.as_mut_ptr(), name_len.as_mut_ptr(),
            )?;
            dir.set_len(<u16>::from(dir_len) as usize);
            name.set_len(<u16>::from(name_len) as usize);
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
    # let session = sibyl::test_env::get_session()?;
    let mut file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;

    assert!(file.file_exists()?);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
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

impl<T> ToSql for &LOB<'_, T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = std::mem::size_of::<*mut T::OCIType>();
        params.bind(pos, T::sql_type(), self.inner.locator.as_ptr() as _, len, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl<T> ToSql for &mut LOB<'_, T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = std::mem::size_of::<*mut T::OCIType>();
        params.bind(pos, T::sql_type(), self.inner.locator.as_mut_ptr() as _, len, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl LOB<'_,OCICLobLocator> {
    /// Debug helper that fetches first 50 (at most) bytes of the CLOB content
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

#[cfg(not(docsrs))]
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

#[cfg(not(docsrs))]
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

#[cfg(not(docsrs))]
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
