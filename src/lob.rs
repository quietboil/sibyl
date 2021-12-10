//! Functions for performing operations on large objects (LOBs).

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
pub mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub mod nonblocking;

use std::{sync::Arc, marker::PhantomData};

use crate::{Result, Connection, env::Env, oci::{self, *}, stmt::args::{ToSql, ToSqlOut}, conn::Session, ptr::{ScopedPtr, ScopedMutPtr}};
use libc::c_void;

/// A marker trait for internal LOB descriptors - CLOB, NCLOB and BLOB.
pub trait InternalLob {}
impl InternalLob for OCICLobLocator {}
impl InternalLob for OCIBLobLocator {}

pub(crate) fn is_initialized<T>(locator: &Descriptor<T>, env: *mut OCIEnv, err: *mut OCIError) -> Result<bool>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    let mut flag = 0u8;
    oci::lob_locator_is_init(env, err, locator.get(), &mut flag)?;
    Ok( flag != 0 )
}

struct LobInner<T>
    where T: DescriptorType<OCIType=OCILobLocator>  + 'static
{
    locator: Descriptor<T>,
    conn:    Arc<Session>,
}

impl<T> LobInner<T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    fn new(locator: Descriptor<T>, conn: Arc<Session>) -> Self {
        Self { locator, conn }
    }

    fn get(&self) -> *mut OCILobLocator {
        self.locator.get()
    }

    fn as_ptr(&self) -> *const *mut OCILobLocator {
        self.locator.as_ptr()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.conn.err_ptr()
    }

    fn env_ptr(&self) -> *mut OCIEnv {
        self.conn.env_ptr()
    }
}

/// LOB locator.
pub struct LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator> + 'static
{
    inner: LobInner<T>,
    phantom_conn: PhantomData<&'a Connection<'a>>
}

impl<'a,T> AsRef<Descriptor<T>> for LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    fn as_ref(&self) -> &Descriptor<T> {
        &self.inner.locator
    }
}

impl<'a,T> LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    fn get(&self) -> *mut OCILobLocator {
        self.inner.get()
    }

    fn as_ptr(&self) -> *const *mut OCILobLocator {
        self.inner.as_ptr()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.inner.err_ptr()
    }

    fn env_ptr(&self) -> *mut OCIEnv {
        self.inner.env_ptr()
    }

    pub(crate) fn make(locator: Descriptor<T>, conn: &'a Connection) -> Self {
        Self { inner: LobInner::new(locator, conn.clone_session()), phantom_conn: PhantomData }
    }

    /**
        Determines whether the LOB locator belongs to a local database table or a remote database
        table. The value `true` indicates that the LOB locator is from a remote database table.
        The application must fetch the LOB descriptor from the database before querying this attribute.
    */
    pub fn is_remote(&self) -> Result<bool> {
        let is_remote: u8 = self.as_ref().get_attr(OCI_ATTR_LOB_REMOTE, self.err_ptr())?;
        Ok( is_remote != 0 )
    }

    /// Returns the LOB's `SQLT` type, i.e. SQLT_CLOB, SQLT_BLOB or SQLT_BFILE.
    pub fn get_type(&self) -> Result<u16> {
        let lob_type: u16 = self.as_ref().get_attr(OCI_ATTR_LOB_TYPE, self.err_ptr())?;
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
        is_initialized(&self.inner.locator, self.env_ptr(), self.err_ptr())
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
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
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
        # stmt.execute(&[])?;
        let stmt = conn.prepare("
            INSERT INTO test_lobs (text) VALUES (empty_clob())
            RETURN id, text INTO :id, :text
        ")?;
        let mut id : usize = 0;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id, &mut lob ])?;

        let text = "
            To Mercy, Pity, Peace, and Love
            All pray in their distress;
            And to these virtues of delight
            Return their thankfulness.
        ";
        lob.append(text)?;

        // Retrieve this CLOB twice into two different locators
        let stmt = conn.prepare("
            select text from test_lobs where id = :id
        ")?;
        let rows = stmt.query(&[ &id ])?;
        let row = rows.next()?.expect("selected row");
        let lob1 : CLOB = row.get(0)?.expect("CLOB locator");

        let rows = stmt.query(&[ &id ])?;
        let row = rows.next()?.expect("selected row");
        let lob2 : CLOB = row.get(0)?.expect("CLOB locator");

        // Even though locators are different, they point to
        // the same LOB which makes them "equal"
        assert!(lob1.is_equal(&lob2)?, "CLOB1 == CLOB2");
        assert!(lob2.is_equal(&lob1)?, "CLOB2 == CLOB1");
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let stmt = conn.prepare("
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
        # stmt.execute(&[]).await?;
        # let stmt = conn.prepare("
        #     INSERT INTO test_lobs (text) VALUES (empty_clob())
        #     RETURN id, text INTO :id, :text
        # ").await?;
        # let mut id : usize = 0;
        # let mut lob = CLOB::new(&conn)?;
        # stmt.execute_into(&[], &mut [ &mut id, &mut lob ]).await?;
        # let text = "
        #     To Mercy, Pity, Peace, and Love
        #     All pray in their distress;
        #     And to these virtues of delight
        #     Return their thankfulness.
        # ";
        # lob.append(text).await?;
        # let stmt = conn.prepare("
        #     select text from test_lobs where id = :id
        # ").await?;
        # let rows = stmt.query(&[ &id ]).await?;
        # let row = rows.next().await?.expect("selected row");
        # let lob1 : CLOB = row.get(0)?.expect("CLOB locator");
        # let rows = stmt.query(&[ &id ]).await?;
        # let row = rows.next().await?.expect("selected row");
        # let lob2 : CLOB = row.get(0)?.expect("CLOB locator");
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
        oci::lob_is_equal(self.env_ptr(), self.get(), other.get(), &mut flag)?;
        Ok( flag != 0 )
    }

    /**
        Returns the character set form of the input CLOB or NCLOB locator. If the input locator is for a BLOB
        or a BFILE, it returns `CharSetForm::Undefined` because there is no concept of a character set for binary
        LOBs or binary files.
    */
    pub fn charset_form(&self) -> Result<CharSetForm> {
        let mut csform = 0u8;
        oci::lob_char_set_form(self.env_ptr(), self.err_ptr(), self.get(), &mut csform)?;
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
        oci::lob_char_set_id(self.env_ptr(), self.err_ptr(), self.get(), &mut csid)?;
        Ok( csid )
    }
}

impl<'a, T> LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator> + InternalLob
{
    /// Creates a new empty LOB. This is an alias for `empty`.
    pub fn new(conn: &'a Connection) -> Result<Self> {
        Self::empty(conn)
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
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
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
        # stmt.execute(&[])?;
        let stmt = conn.prepare("
            insert into test_lobs (text) values (:text) returning id into :id
        ")?;
        let mut id : usize = 0;
        let lob = CLOB::empty(&conn)?;
        stmt.execute_into(&[ &lob ], &mut [ &mut id ])?;
        # assert!(id > 0);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let stmt = conn.prepare("
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
        # stmt.execute(&[]).await?;
        # let stmt = conn.prepare("
        #     insert into test_lobs (text) values (:text) returning id into :id
        # ").await?;
        # let mut id : usize = 0;
        # let lob = CLOB::empty(&conn)?;
        # stmt.execute_into(&[ &lob ], &mut [ &mut id ]).await?;
        # assert!(id > 0);
        # Ok(()) })
        # }
        ```
    */
    pub fn empty(conn: &'a Connection) -> Result<Self> {
        let locator = Descriptor::new(conn.env_ptr())?;
        locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, conn.err_ptr())?;
        Ok(Self::make(locator, conn))
    }

    /**
        Sets the internal LOB locator to empty.

        The locator can then be used as a bind variable for an INSERT or UPDATE statement
        to initialize the LOB to empty. Once the LOB is empty, `write` can be called to
        populate the LOB with data.
    */
    pub fn clear(&self) -> Result<()> {
        self.as_ref().set_attr(OCI_ATTR_LOBEMPTY, 0u32, self.err_ptr())
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
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::NChar, Cache::No)?;

        assert!(lob.is_nclob()?);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let lob = CLOB::temp(&conn, CharSetForm::NChar, Cache::No).await?;
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
    pub fn new(conn: &'a Connection) -> Result<Self> {
        let locator = Descriptor::new(conn.env_ptr())?;
        Ok(Self::make(locator, conn))
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
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        let (dir_name, file_name) = file.file_name()?;

        assert_eq!(dir_name, "MEDIA_DIR");
        assert_eq!(file_name, "hello_world.txt");
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let file = BFile::new(&conn)?;
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
                self.env_ptr(), self.err_ptr(), self.get(),
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
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let mut file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;

        assert!(file.file_exists()?);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let mut file = BFile::new(&conn)?;
        # file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        # assert!(file.file_exists().await?);
        # Ok(()) })
        # }
        ```
    */
    pub fn set_file_name(&self, dir: &str, name: &str) -> Result<()> {
        oci::lob_file_set_name(
            self.env_ptr(), self.err_ptr(),
            self.as_ptr(),
            dir.as_ptr(),  dir.len() as u16,
            name.as_ptr(), name.len() as u16
        )
    }
}

macro_rules! impl_lob_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for LOB<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new(self.inner.as_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCILobLocator>() }
        }
        impl ToSql for &LOB<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new(self.inner.as_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCILobLocator>() }
        }
    };
}

impl_lob_to_sql!{ OCICLobLocator  => SQLT_CLOB  }
impl_lob_to_sql!{ OCIBLobLocator  => SQLT_BLOB  }
impl_lob_to_sql!{ OCIBFileLocator => SQLT_BFILE }

macro_rules! impl_lob_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new(self.as_mut_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCILobLocator>() }
        }
        impl ToSqlOut for LOB<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new(self.inner.locator.as_mut_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCILobLocator>() }
        }
    };
}

impl_lob_to_sql_output!{ OCICLobLocator  => SQLT_CLOB  }
impl_lob_to_sql_output!{ OCIBLobLocator  => SQLT_BLOB  }
impl_lob_to_sql_output!{ OCIBFileLocator => SQLT_BFILE }

