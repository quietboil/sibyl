//! Functions for performing operations on large objects (LOBs).

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
pub mod blocking;

use crate::{
    Result, catch,
    oci::*,
    env::Env,
    conn::Connection,
    stmt::args::{ToSql, ToSqlOut},
};
use libc::c_void;
use std::cell::Cell;

/// A marker trait for internal LOB descriptors - CLOB, NCLOB and BLOB.
pub trait InternalLob {}
impl InternalLob for OCICLobLocator {}
impl InternalLob for OCIBLobLocator {}

pub(crate) fn is_initialized<T>(locator: &Descriptor<T>, env: *mut OCIEnv, err: *mut OCIError) -> Result<bool>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    let mut flag = 0u8;
    catch!{err =>
        OCILobLocatorIsInit(env, err, locator.get(), &mut flag)
    }
    Ok( flag != 0 )
}

/// LOB locator.
pub struct LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    locator: Descriptor<T>,
    conn: &'a Connection<'a>,
    chunk_size: Cell<u32>,
}

impl<'a,T> LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    pub(crate) fn as_ptr(&self) -> *const OCILobLocator {
        self.locator.get()
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut OCILobLocator {
        self.locator.get()
    }

    pub(crate) fn make(locator: Descriptor<T>, conn: &'a Connection) -> Self {
        Self { locator, conn, chunk_size: Cell::new(0) }
    }

    /**
        Determines whether the LOB locator belongs to a local database table or a remote database
        table. The value `true` indicates that the LOB locator is from a remote database table.
        The application must fetch the LOB descriptor from the database before querying this attribute.
    */
    pub fn is_remote(&self) -> Result<bool> {
        let is_remote: u8 = self.locator.get_attr(OCI_ATTR_LOB_REMOTE, self.conn.err_ptr())?;
        Ok( is_remote != 0 )
    }

    /// Returns the LOB's `SQLT` type, i.e. SQLT_CLOB, SQLT_BLOB or SQLT_BFILE.
    pub fn get_type(&self) -> Result<u16> {
        let lob_type: u16 = self.locator.get_attr(OCI_ATTR_LOB_TYPE, self.conn.err_ptr())?;
        Ok( lob_type )
    }

    /**
        Compares the given LOB or BFILE locators for equality. Two LOB or BFILE locators are equal
        if and only if they both refer to the same LOB or BFILE value.

        Two NULL locators are considered not equal by this function.

        # Example
        ```
        use sibyl::CLOB;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
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
        let mut rows = stmt.query(&[ &id ])?;
        let row = rows.next()?.expect("selected row");
        let lob1 : CLOB = row.get(0)?.expect("CLOB locator");

        let mut rows = stmt.query(&[ &id ])?;
        let row = rows.next()?.expect("selected row");
        let lob2 : CLOB = row.get(0)?.expect("CLOB locator");

        // Even though locators are different, they point to
        // the same LOB which makes them "equal"
        assert!(lob1.is_equal(&lob2)?, "CLOB1 == CLOB2");
        assert!(lob2.is_equal(&lob1)?, "CLOB2 == CLOB1");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn is_equal<U>(&self, other: &LOB<'a,U>) -> Result<bool>
        where U: DescriptorType<OCIType=OCILobLocator>
    {
        let mut flag = 0u8;
        catch!{self.conn.err_ptr() =>
            OCILobIsEqual(self.conn.env_ptr(), self.as_ptr(), other.as_ptr(), &mut flag)
        }
        Ok( flag != 0 )
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
        is_initialized(&self.locator, self.conn.env_ptr(), self.conn.err_ptr())
    }

    /**
        Returns the character set form of the input CLOB or NCLOB locator. If the input locator is for a BLOB
        or a BFILE, it returns `CharSetForm::Undefined` because there is no concept of a character set for binary
        LOBs or binary files.
    */
    pub fn charset_form(&self) -> Result<CharSetForm> {
        let mut csform = 0u8;
        catch!{self.conn.err_ptr() =>
            OCILobCharSetForm(self.conn.env_ptr(), self.conn.err_ptr(), self.as_ptr(), &mut csform)
        }
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
        catch!{self.conn.err_ptr() =>
            OCILobCharSetId(self.conn.env_ptr(), self.conn.err_ptr(), self.as_ptr(), &mut csid)
        }
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
        ```
        use sibyl::{ CLOB };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
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
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn empty(conn: &'a Connection) -> Result<Self> {
        let locator = Descriptor::new(conn.env_ptr())?;
        locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, conn.err_ptr())?;
        Ok( Self { locator, conn, chunk_size: Cell::new(0) } )
    }

    /**
        Sets the internal LOB locator to empty.

        The locator can then be used as a bind variable for an INSERT or UPDATE statement
        to initialize the LOB to empty. Once the LOB is empty, `write` can be called to
        populate the LOB with data.
    */
    pub fn clear(&self) -> Result<()> {
        self.locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, self.conn.err_ptr())
    }
}

impl<'a> LOB<'a,OCICLobLocator> {
    /**
        Returns `true` if the LOB locator is for an NCLOB.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::NChar, Cache::No)?;

        assert!(lob.is_nclob()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
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
        Ok( Self { locator, conn, chunk_size: Cell::new(0) } )
    }

    /**
        Returns the directory object and file name associated with this BFILE locator.

        # Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        let (dir_name, file_name) = file.file_name()?;

        assert_eq!(dir_name, "MEDIA_DIR");
        assert_eq!(file_name, "hello_world.txt");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn file_name(&self) -> Result<(String,String)> {
        let mut dir  = String::with_capacity(30);
        let mut name = String::with_capacity(255);
        let mut dir_len  = dir.capacity() as u16;
        let mut name_len = name.capacity() as u16;
        catch!{self.conn.err_ptr() =>
            let dir  = dir.as_mut_vec();
            let name = name.as_mut_vec();
            OCILobFileGetName(
                self.conn.env_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                dir.as_mut_ptr(),  &mut dir_len  as *mut u16,
                name.as_mut_ptr(), &mut name_len as *mut u16
            )
        }
        unsafe {
            dir.as_mut_vec().set_len(dir_len as usize);
            name.as_mut_vec().set_len(name_len as usize);
        }
        Ok( ( dir, name ) )
    }

    /**
        Sets the directory object and file name in the BFILE locator.

        # Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let mut file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;

        assert!(file.file_exists()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_file_name(&self, dir: &str, name: &str) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobFileSetName(
                self.conn.env_ptr(), self.conn.err_ptr(),
                self.locator.as_ptr(),
                dir.as_ptr(),  dir.len() as u16,
                name.as_ptr(), name.len() as u16
            )
        }
        Ok(())
    }
}

macro_rules! impl_lob_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for LOB<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, self.locator.as_ptr() as *const c_void, std::mem::size_of::<*mut OCILobLocator>() )
            }
        }
        impl ToSql for &LOB<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, self.locator.as_ptr() as *const c_void, std::mem::size_of::<*mut OCILobLocator>() )
            }
        }
    };
}

impl_lob_to_sql!{ OCICLobLocator  => SQLT_CLOB  }
impl_lob_to_sql!{ OCIBLobLocator  => SQLT_BLOB  }
impl_lob_to_sql!{ OCIBFileLocator => SQLT_BFILE }

macro_rules! impl_lob_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
                ($sqlt, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCILobLocator>(), std::mem::size_of::<*mut OCILobLocator>())
            }
        }
        impl ToSqlOut for LOB<'_, $ts> {
            fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
                self.locator.to_sql_output()
            }
        }
    };
}

impl_lob_to_sql_output!{ OCICLobLocator  => SQLT_CLOB  }
impl_lob_to_sql_output!{ OCIBLobLocator  => SQLT_BLOB  }
impl_lob_to_sql_output!{ OCIBFileLocator => SQLT_BFILE }

