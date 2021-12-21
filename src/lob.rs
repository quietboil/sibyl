//! Functions for performing operations on large objects (LOBs).

use std::{ptr, sync::{Arc, atomic::{AtomicU32, Ordering}}, marker::PhantomData};
use crate::{Result, Connection, BFile, oci::{self, *}, stmt::{ToSql, ToSqlOut}, conn::SvcCtx};
use libc::c_void;


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

struct LobInner<T> where T: DescriptorType<OCIType=OCILobLocator>  + 'static {
    locator: Descriptor<T>,
    svc: Arc<SvcCtx>,
}

impl<T> Drop for LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> + 'static {
    fn drop(&mut self) {
        let mut is_open = 0u8;
        let res = unsafe {
            OCILobIsOpen(self.as_ref(), self.as_ref(), self.as_ref(), &mut is_open)
        };
        if res == OCI_SUCCESS && is_open != 0 {
            unsafe {
                OCILobClose(self.as_ref(), self.as_ref(), self.as_ref());
            }
        }
        let mut is_temp = 0u8;
        let res = unsafe {
            OCILobIsTemporary(self.as_ref(), self.as_ref(), self.as_ref(), &mut is_temp)
        };
        if res == OCI_SUCCESS && is_temp != 0 {
            unsafe {
                OCILobFreeTemporary(self.as_ref(), self.as_ref(), self.as_ref());
            }
        }
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
        Self { locator, svc }
    }

    fn as_ptr(&self) -> *const *mut OCILobLocator {
        self.locator.as_ptr()
    }

    fn clone(&self) -> Result<Self> {
        let mut locator = Descriptor::<T>::new(self)?;
        oci::lob_locator_assign(self.as_ref(), self.as_ref(), self.as_ref(), locator.as_mut_ptr())?;
        let svc = self.svc.clone();
        Ok(Self { locator, svc } )
    }
}

/// LOB locator.
pub struct LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> + 'static
{
    inner: LobInner<T>,
    chunk_size: AtomicU32,
    phantom_conn: PhantomData<&'a Connection<'a>>
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
        self.inner.as_ref()
    }
}

impl<'a,T> AsRef<OCIError> for LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCIError {
        self.inner.as_ref()
    }
}

impl<'a,T> AsRef<OCISvcCtx> for LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn as_ref(&self) -> &OCISvcCtx {
        self.inner.as_ref()
    }
}

impl<'a,T> LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {

    // fn as_ptr(&self) -> *const *mut OCILobLocator {
    //     self.inner.as_ptr()
    // }

    // fn get_ptr(&self) -> Ptr<OCILobLocator> {
    //     self.inner.get_ptr()
    // }

    pub(crate) fn make(locator: Descriptor<T>, conn: &'a Connection) -> Self {
        Self { 
            inner: LobInner::new(locator, conn.get_svc()),
            chunk_size: AtomicU32::new(0),
            phantom_conn: PhantomData 
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
        lob.open()?;
        lob.append(text)?;
        lob.close()?;
        conn.commit()?;

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
        # sibyl::current_thread_block_on(async {
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
        # lob.open().await?;
        # lob.append(text).await?;
        # lob.close().await?;
        # conn.commit().await?;
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

    /**
        Creates a new LOB locator that points to the same LOB data as the provided locator.

        For internal LOBs, the source locator's LOB data gets copied to the destination locator's
        LOB data only when the destination locator gets stored in the table. Therefore, issuing a
        flush of the object containing the destination locator copies the LOB data. For BFILEs,
        only the locator that refers to the operating system file is copied to the table; the
        operating system file is not copied.

        If the source locator is for an internal LOB that was enabled for buffering, and the source
        locator has been used to modify the LOB data through the LOB buffering subsystem, and the
        buffers have not been flushed since the write, then the source locator may not be assigned
        to the destination locator. This is because only one locator for each LOB can modify the LOB
        data through the LOB buffering subsystem.

        If the source LOB locator refers to a temporary LOB, the destination is made into a temporary
        LOB too. The source and the destination are conceptually different temporary LOBs. The source
        temporary LOB is deep copied, and a destination locator is created to refer to the new deep
        copy of the temporary LOB. Hence `is_equal` returns `false` when the new locator is created
        from the temporary one. However, as an optimization is made to minimize the number of deep
        copies, so the source and destination locators point to the same LOB until any modification
        is made through either LOB locator. Hence `is_equal` returns `true` right after the clone
        locator is created until the first modification. In both these cases, after new locator is
        constructed any changes to either LOB do not reflect in the other LOB.

        # Failures

        Returns `Err` when a remote locator is passed to it.

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
            insert into test_lobs (text) values (empty_clob()) returning id, text into :id, :text
        ")?;
        let mut id1 : usize = 0;
        let mut lob1 = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id1, &mut lob1 ])?;

        let text = [
            "To see a World in a Grain of Sand\n",
            "And a Heaven in a Wild Flower\n",
            "Hold Infinity in the palm of your hand\n",
            "And Eternity in an hour\n"
        ];

        let written = lob1.append(text[0])?;
        assert_eq!(written, text[0].len());
        assert_eq!(lob1.len()?, text[0].len());

        let lob2 = lob1.clone()?;

        // They point to the same value and at this time they are completely in sync
        assert!(lob2.is_equal(&lob1)?);

        let written = lob2.append(text[1])?;
        assert_eq!(written, text[1].len());

        // Now they are out of sync
        assert!(!lob2.is_equal(&lob1)?);
        // At this time `lob1` is not yet aware that `lob2` added more text the LOB they "share".
        assert_eq!(lob2.len()?, text[0].len() + text[1].len());
        assert_eq!(lob1.len()?, text[0].len());

        let written = lob1.append(text[2])?;
        assert_eq!(written, text[2].len());

        // Now, after writing, `lob1` has caught up with `lob2` prior writing and added more text
        // on its own. But now it's `lob2` turn to lag behind and not be aware of the added text.
        assert_eq!(lob1.len()?, text[0].len() + text[1].len() + text[2].len());
        assert_eq!(lob2.len()?, text[0].len() + text[1].len());

        // Let's save `lob2` now. It is still only knows about `text[0]` and `text[1]` fragments.
        let stmt = conn.prepare("
            insert into test_lobs (text) values (:text) returning id, text into :id, :saved_text
        ")?;
        let mut saved_lob_id : usize = 0;
        let mut saved_lob = CLOB::new(&conn)?;
        stmt.execute_into(&[ &lob2 ], &mut [ &mut saved_lob_id, &mut saved_lob ])?;

        // And thus `saved_lob` locator points to a distinct LOB value ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... which has only the `text[0]` and `text[1]`
        assert_eq!(saved_lob.len()?, text[0].len() + text[1].len());

        let written = lob2.append(text[3])?;
        assert_eq!(written, text[3].len());

        assert_eq!(lob2.len()?, text[0].len() + text[1].len() + text[2].len() + text[3].len());
        assert_eq!(lob1.len()?, text[0].len() + text[1].len() + text[2].len());

        // As `saved_lob` points to the enturely different LOB ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... it is not affected by `lob1` and `lob2` additions.
        assert_eq!(saved_lob.len()?, text[0].len() + text[1].len());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn clone(&self) -> Result<Self> {
        let inner = self.inner.clone()?;
        let chunk_size = self.chunk_size.load(Ordering::Relaxed);
        Ok(Self { inner, chunk_size: AtomicU32::new(chunk_size), phantom_conn: PhantomData })
    }

    /**
        Closes a previously opened internal or external LOB.

        Closing a LOB requires a round-trip to the server for both internal and external LOBs.
        For internal LOBs, `close` triggers other code that relies on the close call and for external
        LOBs (BFILEs), close actually closes the server-side operating system file.

        It is not required to close LOB explicitly as it will be automatically closed when Rust drops
        the locator.

        # Failures
        - An error is returned if the internal LOB is not open.

        No error is returned if the BFILE exists but is not opened.

        When the error is returned, the LOB is no longer marked as open, but the transaction is successfully
        committed. Hence, all the changes made to the LOB and non-LOB data in the transaction are committed,
        but the domain and function-based indexing are not updated. If this happens, rebuild your functional
        and domain indexes on the LOB column.

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
            insert into test_lobs (text) values (empty_clob()) returning id, text into :id, :text
        ")?;
        let mut id : usize = 0;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id, &mut lob ])?;

        let text = [
            "Love seeketh not itself to please,\n",
            "Nor for itself hath any care,\n",
            "But for another gives its ease,\n",
            "And builds a Heaven in Hell's despair.\n",
        ];

        lob.open()?;
        lob.append(text[0])?;
        lob.append(text[1])?;
        lob.append(text[2])?;
        lob.append(text[3])?;
        lob.close()?;

        assert_eq!(lob.len()?, text[0].len() + text[1].len() + text[2].len() + text[3].len());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn close(&self) -> Result<()> {
        oci::lob_close(self.as_ref(), self.as_ref(), self.as_ref())
    }

    /**
        Returns the length of a LOB.

        For character LOBs, it is the number of characters; for binary LOBs and BFILEs, it is the number of
        bytes in the LOB.

        Notes:
        - If the LOB is NULL, the length is undefined.
        - The length of a BFILE includes the EOF, if it exists.
        - The length of an empty internal LOB is zero.
        - Any zero-byte or space fillers in the LOB written by previous calls to `erase` or `write` are also
          included in the length count.
    */
    pub fn len(&self) -> Result<usize> {
        let mut len = 0u64;
        oci::lob_get_length(self.as_ref(), self.as_ref(), self.as_ref(), &mut len)?;
        Ok( len as usize )
    }

    /**
        Returns `true` if the internal LOB is open or if the BFILE was opened using the input locator.

        If the input BFILE locator was never passed to `open` or `open_file`  the BFILE is considered
        not to be opened by this BFILE locator. However, a different BFILE locator may have opened
        the BFILE. Multiple opens can be performed on the same BFILE using different locators. In other
        words, openness is associated with a specific locator for BFILEs.

        For internal LOBs openness is associated with the LOB, not with the locator. If locator1 opened
        the LOB, then locator2 also sees the LOB as open.

        For internal LOBs, this call requires a server round-trip because it checks the state on the
        server to see if the LOB is open. For external LOBs (BFILEs), this call also requires a round-trip
        because the operating system file on the server side must be checked to see if it is open.
    */
    pub fn is_open(&self) -> Result<bool> {
        let mut flag = 0u8;
        oci::lob_is_open(self.as_ref(), self.as_ref(), self.as_ref(), &mut flag)?;
        Ok( flag != 0 )
    }

    /**
        Opens a LOB, internal or external, only for reading.

        Opening a LOB requires a round-trip to the server for both internal and external LOBs. For internal
        LOBs, the open triggers other code that relies on the open call. For external LOBs (BFILEs), open
        requires a round-trip because the actual operating system file on the server side is being opened.

        # Failures
        - It is an error to open the same LOB twice.
        - If a user tries to write to a LOB that was opened in read-only mode, an error is returned.
    */
    pub fn open_readonly(&self) -> Result<()> {
        oci::lob_open(self.as_ref(), self.as_ref(), self.as_ref(), OCI_LOB_READONLY)
    }

    /**
        For CLOBs and NCLOBs, if you do not pass `char_len`, then `char_len` is calculated internally as
        `byte_len/max char width`, so if max char width is 4, `char_len` is calculated as `byte_len/4`.
        OCILobRead2() does not calculate how many bytes are required for each character. Instead, OCILobRead2()
        fetches the number of characters that in the worst case can fit in `byte_len`. To fill the buffer, check
        the returned value to see how much of the buffer is filled, then call OCILobRead2() again to fetch the
        remaining bytes.
    */
    fn read_piece(&self, piece: u8, piece_size: usize, offset: usize, byte_len: &mut usize, char_len: usize, char_form: u8, buf: &mut Vec<u8>) -> Result<bool> {
        buf.reserve(piece_size);
        let mut num_bytes = *byte_len as u64;
        let mut num_chars = char_len as u64;
        let res = unsafe {
            oci::lob_read(
                self.as_ref(), self.as_ref(), self.as_ref(),
                &mut num_bytes, &mut num_chars, (offset + 1) as u64,
                buf.as_mut_ptr().add(buf.len()), piece_size as u64, piece,
                AL32UTF8, char_form
            )
        }?;
        unsafe {
            buf.set_len(buf.len() + num_bytes as usize);
        }
        *byte_len = num_bytes as usize;
        Ok( res == OCI_NEED_DATA )
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
        # sibyl::current_thread_block_on(async {
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
        let locator = Descriptor::<T>::new(conn)?;
        locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, conn.as_ref())?;
        Ok(Self::make(locator, conn))
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

    /**
        Appends another LOB value at the end of this LOB.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let text1 = "
            The sun does arise,
            And make happy the skies.
            The merry bells ring
            To welcome the Spring.
        ";
        let text2 = "
            The sky-lark and thrush,
            The birds of the bush,
            Sing louder around,
            To the bells’ cheerful sound.
            While our sports shall be seen
            On the Ecchoing Green.
        ";
        let lob1 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob1.append(text1)?;
        assert_eq!(lob1.len()?, text1.len());

        let lob2 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob2.append(text2)?;
        // Cannot use `len` shortcut with `text2` because of `RIGHT SINGLE QUOTATION MARK`
        assert_eq!(lob2.len()?, text2.chars().count());

        lob1.append_lob(&lob2)?;
        assert_eq!(lob1.len()?, text1.len() + text2.chars().count());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append_lob(&self, lob: &Self) -> Result<()> {
        oci::lob_append(self.as_ref(), self.as_ref(), self.as_ref(), lob.as_ref())
    }

    /**
        Copies all or a portion of another LOB value.

        If the data exists at the destination's start position, it is overwritten with the source data.

        If the destination's start position is beyond the end of the current data, zero-byte fillers (for BLOBs)
        or spaces (for CLOBs) are written into the destination LOB from the end of the current data to the beginning
        of the newly written data from the source.

        The destination LOB is extended to accommodate the newly written data if it extends beyond the current
        length of the destination LOB.

        LOB buffering must not be enabled for either locator.

        Notes:
        - To copy the entire source LOB specify `amount` as `std::usize::MAX`.
        - `offset` and `src_offset` - the number of characters (character LOB) or bytes (binary LOB) from the
          beginning of the LOB - start at 0.
        - You can call `len` to determine the length of the source LOB.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let text = "
        O Nightingale, that on yon bloomy Spray
        Warbl'st at eeve, when all the Woods are still,
        Thou with fresh hope the Lovers heart dost fill,
        While the jolly hours lead on propitious May,

        Thy liquid notes that close the eye of Day,
        First heard before the shallow Cuccoo's bill
        Portend success in love; O if Jove's will
        Have linkt that amorous power to thy soft lay,

        ...........................................
        ...........................................
        ..........................................

        For my relief; yet hadst no reason why,
        Whether the Muse, or Love call thee his mate,
        Both them I serve, and of their train am I.
        ";
        let lob1 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob1.append(text)?;

        let lost_text = "
        Now timely sing, ere the rude Bird of Hate
        Foretell my hopeles doom, in som Grove ny:
        As thou from yeer to yeer hast sung too late
        ";
        let lob2 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob2.append(lost_text)?;

        // Set source offset to 1 to skip leading \n:
        lob1.copy(&lob2, 1, 131, 364)?;

        // Read back the overwriten fragment.
        // Start with the leading \n, to make comparison easier:
        let mut fragment = String::with_capacity(135*4);
        lob1.read(363, 132, &mut fragment)?;

        assert_eq!(fragment, lost_text);

        // ASCII only, so we can use `len` as a "shortcut".
        let text_len = lob1.len()?;
        // Recall that the buffer needs to be allocated for the worst case
        let mut sonnet = String::with_capacity(text_len*4);
        lob1.read(0, text_len, &mut sonnet)?;

        let orig = "
        O Nightingale, that on yon bloomy Spray
        Warbl'st at eeve, when all the Woods are still,
        Thou with fresh hope the Lovers heart dost fill,
        While the jolly hours lead on propitious May,

        Thy liquid notes that close the eye of Day,
        First heard before the shallow Cuccoo's bill
        Portend success in love; O if Jove's will
        Have linkt that amorous power to thy soft lay,

        Now timely sing, ere the rude Bird of Hate
        Foretell my hopeles doom, in som Grove ny:
        As thou from yeer to yeer hast sung too late

        For my relief; yet hadst no reason why,
        Whether the Muse, or Love call thee his mate,
        Both them I serve, and of their train am I.
        ";

        assert_eq!(sonnet, orig);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn copy(&self, src: &Self, src_offset: usize, amount: usize, offset: usize) -> Result<()> {
        oci::lob_copy(
            self.as_ref(), self.as_ref(),
            self.as_ref(), src.as_ref(),
            amount as u64, (offset + 1) as u64, (src_offset + 1) as u64
        )
    }

    /**
        Loads and copies all or a portion of the file into an internal LOB.

        The data are copied from the source BFILE to the destination internal LOB (BLOB or CLOB). No character set
        conversions are performed when copying the BFILE data to a CLOB or NCLOB. Also, when binary data is loaded
        into a BLOB, no character set conversions are performed. Therefore, the BFILE data must be in the same
        character set as the LOB in the database. No error checking is performed to verify this.

        If the data exists at the destination's start position, it is overwritten with the source data. If the
        destination's start position is beyond the end of the current data, zero-byte fillers (for BLOBs) or spaces
        (for CLOBs) are written into the destination LOB from the end of the data to the beginning of the newly
        written data from the source. The destination LOB is extended to accommodate the newly written data if it
        extends beyond the current length of the destination LOB.

        # Failures
        - This function throws an error when a remote locator is passed to it.
        - It is an error to try to copy from a NULL BFILE.

        # Example
        Note that this example assumes that the demo directories were created (@?/demo/schema/mk_dir) and
        the test user has permissions to read them (see `etc/create_sandbox.sql`)

        ```
        use sibyl::{CLOB, Cache, CharSetForm, BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        assert!(file.file_exists()?, "hello_world.txt");

        // To load this file into a temporary CLOB Oracle expects it to be UTF-16 BE encoded.

        // File must be opened first
        file.open_readonly()?;
        assert!(file.is_open()?);

        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob.load_from_file(&file, 0, 28, 0)?;
        file.close()?;

        assert_eq!(13, lob.len()?);

        let mut text = String::with_capacity(64);
        // Even though we are asking for more characters than the LOB has it should only return what it has
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "Hello, World!");

        // Do it again in Cyrillic. No need to erase it. We'll just load new content over the old one
        file.set_file_name("MEDIA_DIR", "hello_world_cyrillic.txt")?;
        assert!(file.file_exists()?, "hello_world_cyrillic.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        lob.load_from_file(&file, 0, 34, 0)?;
        file.close()?;

        assert_eq!(16, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "Здравствуй, Мир!");

        // And the supplemental plane
        file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
        assert!(file.file_exists()?, "hello_supplemental.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        // As the text in the "cyrillic" test was longer...
        lob.trim(0)?;
        lob.load_from_file(&file, 0, 18, 0)?;
        file.close()?;

        // Note that "char count" for all characters used in this test is defined in Unicode as 2.
        assert_eq!(8, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        // Depending on the font you use, you might or might not see the the test characters
        // They are represented in "Cousine" for example
        assert_eq!(text, "🚲🛠📬🎓");

        // Same set of tests, but now with a CLOB from the test table
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
            insert into test_lobs (text) values (empty_clob()) returning text into :text
        ")?;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut lob ])?;

        lob.open()?;

        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        assert!(file.file_exists()?, "hello_world.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        lob.load_from_file(&file, 0, 28, 0)?;
        file.close()?;

        assert_eq!(13, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "Hello, World!");

        file.set_file_name("MEDIA_DIR", "hello_world_cyrillic.txt")?;
        assert!(file.file_exists()?, "hello_world_cyrillic.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        lob.load_from_file(&file, 0, 34, 0)?;
        file.close()?;

        assert_eq!(16, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "Здравствуй, Мир!");

        file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
        assert!(file.file_exists()?, "hello_supplemental.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        lob.trim(0)?;
        lob.load_from_file(&file, 0, 18, 0)?;
        file.close()?;

        assert_eq!(8, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "🚲🛠📬🎓");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn load_from_file(&self, src: &BFile, src_offset: usize, amount: usize, offset: usize) -> Result<()> {
        oci::lob_load_from_file(
            self.as_ref(), self.as_ref(),
            self.as_ref(), src.as_ref(),
            amount as u64, (offset + 1) as u64, (src_offset + 1) as u64
        )
    }

    /**
        Erases a specified portion of the internal LOB data starting at a specified offset.
        Returns the actual number of characters or bytes erased.

        For BLOBs, erasing means that zero-byte fillers overwrite the existing LOB value.
        For CLOBs, erasing means that spaces overwrite the existing LOB value.
    */
    pub fn erase(&self, offset: usize, amount: usize) -> Result<usize> {
        let mut count: u64 = amount as u64;
        oci::lob_erase(
            self.as_ref(), self.as_ref(), self.as_ref(),
            &mut count as *mut u64, (offset + 1) as u64
        )?;
        Ok( count as usize )
    }

    /**
        Returns the chunk size (in bytes) of a LOB.

        For LOBs with storage parameter BASICFILE, chunk size is amount of a chunk's space that is
        used to store the internal LOB value. This is the amount that users should use when reading
        or writing the LOB value. If possible, users should start their writes at chunk boundaries,
        such as the beginning of a chunk, and write a chunk at a time.

        For LOBs with storage parameter SECUREFILE, chunk size is an advisory size and is provided
        for backward compatibility.

        When creating a table that contains an internal LOB, the user can specify the chunking factor,
        which can be a multiple of Oracle Database blocks. This corresponds to the chunk size used by
        the LOB data layer when accessing and modifying the LOB value. Part of the chunk is used to store
        system-related information, and the rest stores the LOB value. This function returns the amount
        of space used in the LOB chunk to store the LOB value. Performance is improved if the application
        issues read or write requests using a multiple of this chunk size. For writes, there is an added
        benefit because LOB chunks are versioned and, if all writes are done on a chunk basis, no extra
        versioning is done or duplicated. Users could batch up the write until they have enough for a chunk
        instead of issuing several write calls for the same chunk.
    */
    pub fn chunk_size(&self) -> Result<usize> {
        let mut size = self.chunk_size.load(Ordering::Relaxed);
        if size == 0 {
            oci::lob_get_chunk_size(self.as_ref(), self.as_ref(), self.as_ref(), &mut size)?;
            self.chunk_size.store(size, Ordering::Relaxed);
        }
        Ok( size as usize )
    }

    /**
        Returns he user-specified content type string for the data in a SecureFile, if set.

        This function only works on SecureFiles.
    */
    pub fn content_type(&self) -> Result<String> {
        let mut txt = String::with_capacity(OCI_LOB_CONTENTTYPE_MAXSIZE);
        let mut len = txt.capacity() as u32;
        unsafe {
            let txt = txt.as_mut_vec();
            oci::lob_get_content_type(
                self.as_ref(), self.as_ref(), self.as_ref(), self.as_ref(),
                txt.as_mut_ptr(), &mut len, 0
            )?;
            txt.set_len(len as usize);
        }
        Ok( txt )
    }

    /**
        Sets a content type string for the data in the SecureFile to something that can be used by an application.

        This function only works on SecureFiles.
    */
    pub fn set_content_type(&self, content_type: &str) -> Result<()> {
        let len = content_type.len() as u32;
        let ptr = if len > 0 { content_type.as_ptr() } else { ptr::null::<u8>() };
        oci::lob_set_content_type(
            self.as_ref(), self.as_ref(), self.as_ref(), self.as_ref(),
            ptr, len, 0
        )
    }

    /**
        Opens an internal LOB for reading and writing.

        Opening a LOB requires a round-trip to the server for both internal and external LOBs. For internal
        LOBs, the open triggers other code that relies on the open call. For external LOBs (BFILEs), open
        requires a round-trip because the actual operating system file on the server side is being opened.

        It is not mandatory that you wrap all LOB operations inside the open and close calls. However, if you
        open a LOB, then you must close it before you commit your transaction. When an internal LOB is closed,
        it updates the functional and domain indexes on the LOB column. It is an error to commit the transaction
        before closing all opened LOBs that were opened by the transaction.

        When the error is returned, the LOB is no longer marked as open, but the transaction is successfully
        committed. Hence, all the changes made to the LOB and non-LOB data in the transaction are committed,
        but the domain and function-based indexing are not updated. If this happens, rebuild your functional
        and domain indexes on the LOB column.

        It is not necessary to open a LOB to perform operations on it. When using function-based indexes,
        extensible indexes or context, and making multiple calls to update or write to the LOB, you should
        first call `open`, then update the LOB as many times as you want, and finally call `close`. This
        sequence of operations ensures that the indexes are only updated once at the end of all the write
        operations instead of once for each write operation.

        If you do not wrap your LOB operations inside the open or close API, then the functional
        and domain indexes are updated each time you write to the LOB. This can adversely affect
        performance. If you have functional or domain indexes, Oracle recommends that you enclose
        write operations to the LOB within the open or close statements.

        # Failures
        - It is an error to open the same LOB twice.
    */
    pub fn open(&self) -> Result<()> {
        oci::lob_open(self.as_ref(), self.as_ref(), self.as_ref(), OCI_LOB_READWRITE)
    }

    /**
        Truncates the LOB value to a shorter length.

        For character LOBs, `new_len` is the number of characters; for binary LOBs and BFILEs, it is the number
        of bytes in the LOB.
    */
    pub fn trim(&self, new_len: usize) -> Result<()> {
        oci::lob_trim(self.as_ref(), self.as_ref(), self.as_ref(), new_len as u64)
    }

    fn write_piece(&self, piece: u8, offset: usize, data: &[u8]) -> Result<usize> {
        let mut byte_cnt = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        let mut char_cnt = 0u64;
        let charset_form = self.charset_form()? as u8;
        oci::lob_write(
            self.as_ref(), self.as_ref(), self.as_ref(),
            &mut byte_cnt, &mut char_cnt, (offset + 1) as u64,
            data.as_ptr(), data.len() as u64, piece,
            ptr::null_mut::<c_void>(), ptr::null::<c_void>(),
            AL32UTF8, charset_form
        )?;
        Ok( byte_cnt as usize )
    }

    fn append_piece(&self, piece: u8, data: &[u8]) -> Result<usize> {
        let mut byte_cnt = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        let mut char_cnt = 0u64;
        let charset_form = self.charset_form()? as u8;
        oci::lob_write_append(
            self.as_ref(), self.as_ref(), self.as_ref(),
            &mut byte_cnt, &mut char_cnt,
            data.as_ptr(), data.len() as u64, piece,
            ptr::null_mut::<c_void>(), ptr::null::<c_void>(),
            AL32UTF8, charset_form
        )?;
        Ok( byte_cnt as usize )
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
        # sibyl::current_thread_block_on(async {
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

    /**
        Creates an empty temporary CLOB or NCLOB.

        The LOB is feed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub fn temp(conn: &'a Connection, csform: CharSetForm, cache: Cache) -> Result<Self> {
        let locator = Descriptor::<OCICLobLocator>::new(conn)?;
        oci::lob_create_temporary(
            conn.as_ref(), conn.as_ref(), locator.as_ref(),
            OCI_DEFAULT as u16, csform as u8, OCI_TEMP_CLOB, cache as u8, OCI_DURATION_SESSION
        )?;
        Ok(Self::make(locator, conn))
    }

    /**
        Writes a buffer into a LOB.

        Returns the number of bytes written to the database.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        let written = lob.write(3, "Hello, World!")?;
        assert_eq!(13, written);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn write(&self, offset: usize, text: &str) -> Result<usize> {
        self.write_piece(OCI_ONE_PIECE, offset, text.as_bytes())
    }

    /**
        Starts piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::{ CLOB, CharSetForm, Cache };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob.open()?;
        let chunk_size = lob.chunk_size()?;
        let data = vec![42u8;chunk_size];
        let text = std::str::from_utf8(&data)?;

        let written = lob.write_first(0, text)?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.write_next(text)?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.write_last(text)?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len()?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn write_first(&self, offset: usize, text: &str) -> Result<usize> {
        self.write_piece(OCI_FIRST_PIECE, offset, text.as_bytes())
    }

    /**
        Continues piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn write_next(&self, text: &str) -> Result<usize> {
        self.write_piece(OCI_NEXT_PIECE, 0, text.as_bytes())
    }

    /**
        Terminates piece-wise writing into a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn write_last(&self, text: &str) -> Result<usize> {
        self.write_piece(OCI_LAST_PIECE, 0, text.as_bytes())
    }

    /**
        Writes data starting at the end of a LOB.

        Returns the number of bytes written to the database.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        let written = lob.append("Hello, World!")?;
        assert_eq!(13, written);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_ONE_PIECE, text.as_bytes())
    }

    /**
        Starts piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::{ CLOB, CharSetForm, Cache };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob.open()?;
        let chunk_size = lob.chunk_size()?;
        let data = vec![42u8;chunk_size];
        let text = std::str::from_utf8(&data)?;

        let written = lob.append_first(text)?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.append_next(text)?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.append_last(text)?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len()?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append_first(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_FIRST_PIECE, text.as_bytes())
    }

    /**
        Continues piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn append_next(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_NEXT_PIECE, text.as_bytes())
    }

    /**
        Terminates piece-wise writing at the end of a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn append_last(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_LAST_PIECE, text.as_bytes())
    }

    /**
        Reads specified number of characters from this LOB, appending them to `buf`.
        If successful, this function will return the total number of bytes read.

        Note that the specified number of characters to read is only honored if the buffer is large enough to accept that many.
        The upper bound on the number of returned characters is calculated by Oracle as `buf_remaining_capacity/max_char_width`.
        Because of the AL32UTF8 encoding configured in OCI environments created by *sibyl* the `max_char_width` in that calculation
        is 4. The underlying OCI call - OCILobRead2() - does not calculate how many bytes are required for **each** character.
        Instead, it fetches the number of characters that even in **the worst case** - for example when all characters in the
        requested fragment are from the supplementary planes - would fit in the provided buffer.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        let written = lob.write(3, "Hello, World!")?;
        assert_eq!(13, written);

        let mut buf = String::with_capacity(16*4);
        let read = lob.read(0, 16, &mut buf)?;

        assert_eq!(16, read);
        assert_eq!(buf, "   Hello, World!");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read(&self, offset: usize, len: usize, buf: &mut String) -> Result<usize> {
        let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        let mut num_read : usize = 0;
        self.read_piece(OCI_ONE_PIECE, len * 4, offset, &mut num_read, len, form, buf)?;
        Ok( num_read )
    }

    /**
        Starts piece-wise reading of the specified number of characters from this LOB into the provided buffer,
        returning a flag which indicates whether there are more pieces to read until the requested fragment is
        complete. Application should call `read_next` (and **only** `read_next`) repeatedly until `read_next`
        returns `false`.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        let chunk_size = lob.chunk_size()?;
        lob.open()?;
        let fragment = vec![52u8;chunk_size];
        let fragment_as_text = unsafe { String::from_utf8_unchecked(fragment) };
        let mut written = lob.append_first(&fragment_as_text)?;
        for _i in 0..10 {
            written += lob.append_next(&fragment_as_text)?;
        }
        written += lob.append_last(&fragment_as_text)?;

        assert_eq!(chunk_size * 12, written);

        let mut text = String::new();
        let mut text_len : usize = 0;
        let mut read_len : usize = 0;
        let piece_size = chunk_size;
        let offset = chunk_size * 2;
        let length = chunk_size * 5;
        let mut has_next = lob.read_first(piece_size, offset, length, &mut text, &mut text_len)?;
        while has_next {
            has_next = lob.read_next(piece_size, &mut text, &mut read_len)?;
            text_len += read_len;
        }
        assert_eq!(text_len, chunk_size * 5);
        assert_eq!(text_len, text.len());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut String, num_read: &mut usize) -> Result<bool> {
        let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        *num_read = 0;
        self.read_piece(OCI_FIRST_PIECE, piece_size, offset, num_read, len, form, buf)
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a flag which indicates
        whether there are more pieces to read until the requested fragment is complete. Application should keep
        calling `read_next` until it returns `false`.
    */
    pub fn read_next(&self, piece_size: usize, buf: &mut String, num_read: &mut usize) -> Result<bool> {
        // let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, piece_size, 0, num_read, 0, 0, buf)
    }
}

impl<'a> LOB<'a,OCIBLobLocator> {
    /**
        Creates an empty temporary BLOB.

        The LOB is feed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub fn temp(conn: &'a Connection, cache: Cache) -> Result<Self> {
        let locator = Descriptor::<OCIBLobLocator>::new(conn)?;
        oci::lob_create_temporary(
            conn.as_ref(), conn.as_ref(), locator.as_ref(),
            OCI_DEFAULT as u16, 0u8, OCI_TEMP_BLOB, cache as u8, OCI_DURATION_SESSION
        )?;
        Ok(Self::make(locator, conn))
    }

    /**
        Writes a buffer into a LOB.

        Returns the number of bytes written to the database.

        # Example
        ```
        use sibyl::{BLOB, Cache};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = BLOB::temp(&conn, Cache::No)?;
        let written = lob.write(3, "Hello, World!".as_bytes())?;
        assert_eq!(13, written);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_ONE_PIECE, offset, data)
    }

    /**
        Starts piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::BLOB;

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
            insert into test_lobs (data) values (empty_blob()) returning data into :data
        ")?;
        let mut lob = BLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut lob ])?;
        lob.open()?;
        let chunk_size = lob.chunk_size()?;
        let data = vec![42u8;chunk_size];

        let written = lob.write_first(0, &data)?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.write_next(&data)?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.write_last(&data)?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len()?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn write_first(&self, offset: usize, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_FIRST_PIECE, offset, data)
    }

    /**
        Continues piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn write_next(&self, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_NEXT_PIECE, 0, data)
    }

    /**
        Terminates piece-wise writing into a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn write_last(&self, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_LAST_PIECE, 0, data)
    }

    /**
        Writes data starting at the end of a LOB.

        Returns the number of bytes written to the database.

        # Example
        ```
        use sibyl::{BLOB, Cache};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = BLOB::temp(&conn, Cache::No)?;
        let written = lob.append("Hello, World!".as_bytes())?;
        assert_eq!(13, written);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_ONE_PIECE, data)
    }

    /**
        Starts piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::{BLOB, Cache};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let mut lob = BLOB::temp(&conn, Cache::No)?;
        let chunk_size = lob.chunk_size()?;
        let data = vec![165u8;chunk_size];

        let written = lob.append_first(&data)?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.append_next(&data)?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.append_last(&data)?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len()?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append_first(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_FIRST_PIECE, data)
    }

    /**
        Continues piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn append_next(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_NEXT_PIECE, data)
    }

    /**
        Terminates piece-wise writing at the end of a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn append_last(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_LAST_PIECE, data)
    }

    /**
        Reads specified number of bytes from this LOB, appending them to `buf`.
        If successful, this function returns the number of bytes that were read and appended to `buf`.

        # Example
        ```
        use sibyl::{BLOB, Cache, BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "modem.jpg")?;
        assert!(file.file_exists()?);
        let file_len = file.len()?;
        file.open_readonly()?;
        let lob = BLOB::temp(&conn, Cache::No)?;
        lob.load_from_file(&file, 0, file_len, 0)?;
        assert_eq!(lob.len()?, file_len);

        let mut data = Vec::with_capacity(file_len);
        let num_read = lob.read(0, file_len, &mut data)?;

        assert_eq!(num_read, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read(&self, offset: usize, mut len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.read_piece(OCI_ONE_PIECE, len, offset, &mut len, 0, 0, buf)?;
        Ok( len )
    }

    /**
        Starts piece-wise reading of the specified number of bytes from this LOB into the provided buffer, returning a tuple
        with 2 elements - the number of bytes read in the current piece and the flag which indicates whether there are more
        pieces to read until the requested fragment is complete. Application should call `read_next` (and **only** `read_next`)
        repeatedly until "more data" flag becomes `false`.

        # Example
        ```
        use sibyl::{BLOB, Cache, BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "monitor.jpg")?;
        assert!(file.file_exists()?);
        let file_len = file.len()?;
        file.open_readonly()?;
        let lob = BLOB::temp(&conn, Cache::No)?;
        lob.load_from_file(&file, 0, file_len, 0)?;
        assert_eq!(lob.len()?, file_len);

        let chunk_size = lob.chunk_size()?;
        let mut data = Vec::with_capacity(file_len);
        let mut data_len : usize = 0;
        let piece_size = chunk_size;
        let offset = 0;
        let length = file_len;
        let mut has_next = lob.read_first(piece_size, offset, length, &mut data, &mut data_len)?;
        assert_eq!(data_len, chunk_size);
        assert_eq!(data.len(), data_len);
        while has_next {
            let mut bytes_read : usize = 0;
            has_next = lob.read_next(piece_size, &mut data, &mut bytes_read)?;
            data_len += bytes_read;
            assert_eq!(data_len, data.len());
        }

        assert_eq!(data_len, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = len;
        self.read_piece(OCI_FIRST_PIECE, piece_size, offset, num_read, 0, 0, buf)
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a tuple with 2 elements - the number of
        bytes read in the current piece and the flag which indicates whether there are more pieces to read until the requested
        fragment is complete. Application should keep calling `read_next` until "more data" flag becomes `false`.
    */
    pub fn read_next(&self, piece_size: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, piece_size, 0, num_read, 0, 0, buf)
    }
}

impl<'a> LOB<'a,OCIBFileLocator> {
    /// Creates a new uninitialized BFILE.
    pub fn new(conn: &'a Connection) -> Result<Self> {
        let locator = Descriptor::<OCIBFileLocator>::new(conn)?;
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
        # sibyl::current_thread_block_on(async {
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
        # sibyl::current_thread_block_on(async {
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
            self.as_ref(), self.as_ref(),
            self.inner.locator.as_ptr() as _,
            dir.as_ptr(),  dir.len() as u16,
            name.as_ptr(), name.len() as u16
        )
    }
    
    /**
        Closes a previously opened BFILE.

        No error is returned if the BFILE exists but is not opened.

        This function is only meaningful the first time it is called for a particular
        BFILE locator. Subsequent calls to this function using the same BFILE locator
        have no effect.
    */
    pub fn close_file(&self) -> Result<()> {
        oci::lob_file_close(self.as_ref(), self.as_ref(), self.as_ref())
    }

    /**
        Tests to see if the BFILE exists on the server's operating system.

        # Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "formatted_doc.txt")?;

        assert!(file.file_exists()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn file_exists(&self) -> Result<bool> {
        let mut exists = 0u8;
        oci::lob_file_exists(self.as_ref(), self.as_ref(), self.as_ref(), &mut exists)?;
        Ok( exists != 0 )
    }

    /**
        Returns `true` if the BFILE was opened using this particular locator.
        However, a different locator may have the file open. Openness is associated
        with a particular locator.

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
        assert!(!file.is_file_open()?);

        file.open_file()?;
        assert!(file.is_file_open()?);

        file.close_file()?;
        assert!(!file.is_file_open()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn is_file_open(&self) -> Result<bool> {
        let mut is_open = 0u8;
        oci::lob_file_is_open(self.as_ref(), self.as_ref(), self.as_ref(), &mut is_open)?;
        Ok( is_open != 0 )
    }

    /**
        Opens a BFILE on the file system of the server. The BFILE can only be opened
        for read-only access. BFILEs can not be written through Oracle Database.

        This function is only meaningful the first time it is called for a particular
        BFILE locator. Subsequent calls to this function using the same BFILE locator
        have no effect.

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

        file.open_file()?;

        assert!(file.is_file_open()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn open_file(&self) -> Result<()> {
        oci::lob_file_open(self.as_ref(), self.as_ref(), self.as_ref(), OCI_FILE_READONLY)
    }

    /**
        Reads specified number of bytes from this LOB, appending them to `buf`.
        If successful, this function returns the number of bytes that were read and appended to `buf`.

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
        let file_len = file.len()?;
        file.open_file()?;

        let mut data = Vec::with_capacity(file_len);
        let num_read = file.read(0, file_len, &mut data)?;

        assert_eq!(num_read, file_len);
        assert_eq!(data, [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21]);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read(&self, offset: usize, mut len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.read_piece(OCI_ONE_PIECE, len, offset, &mut len, 0, 0, buf)?;
        Ok( len )
    }

    /**
        Starts piece-wise reading of the specified number of bytes from this LOB into the provided buffer, returning a tuple
        with 2 elements - the number of bytes read in the current piece and the flag which indicates whether there are more
        pieces to read until the requested fragment is complete. Application should call `read_next` (and **only** `read_next`)
        repeatedly until "more data" flag becomes `false`.

        # Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "keyboard.jpg")?;
        assert!(file.file_exists()?);
        let file_len = file.len()?;
        file.open_readonly()?;

        let mut data = Vec::with_capacity(file_len);
        let mut data_len : usize = 0;
        let piece_size = 8192;
        let offset = 0;
        let length = file_len;
        let mut has_next = file.read_first(piece_size, offset, length, &mut data, &mut data_len)?;
        assert_eq!(data.len(), data_len);
        while has_next {
            let mut bytes_read : usize = 0;
            has_next = file.read_next(piece_size, &mut data, &mut bytes_read)?;
            data_len += bytes_read;
            assert_eq!(data_len, data.len());
        }

        assert_eq!(data_len, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = len;
        self.read_piece(OCI_FIRST_PIECE, piece_size, offset, num_read, 0, 0, buf)
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a tuple with 2 elements - the number of
        bytes read in the current piece and the flag which indicates whether there are more pieces to read until the requested
        fragment is complete. Application should keep calling `read_next` until "more data" flag becomes `false`.
    */
    pub fn read_next(&self, piece_size: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, piece_size, 0, num_read, 0, 0, buf)
    }
}

macro_rules! impl_lob_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for LOB<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(self.inner.as_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCILobLocator>() }
        }
        impl ToSql for &LOB<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(self.inner.as_ptr() as _) }
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
            fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.as_mut_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCILobLocator>() }
        }
        impl ToSqlOut for LOB<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.inner.locator.as_mut_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCILobLocator>() }
        }
    };
}

impl_lob_to_sql_output!{ OCICLobLocator  => SQLT_CLOB  }
impl_lob_to_sql_output!{ OCIBLobLocator  => SQLT_BLOB  }
impl_lob_to_sql_output!{ OCIBFileLocator => SQLT_BFILE }

impl LOB<'_,OCICLobLocator> {
    /// Debug helper that fetches first 50 (at most) bytes of CLOB content
    fn content_head(&self) -> Result<String> {
        const MAX_LEN : usize = 50;
        let len = self.len()?;
        let len = std::cmp::min(len, MAX_LEN);
        let mut buf = String::with_capacity(len * 4);
        let len = self.read(0, len, &mut buf)?;
        if len == MAX_LEN {
            buf.push_str("...");
        }
        Ok(buf)
    }
}

impl std::fmt::Debug for LOB<'_,OCICLobLocator> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.content_head() {
            Ok(text) => f.write_fmt(format_args!("CLOB {}", text)),
            Err(err) => f.write_fmt(format_args!("CLOB {:?}", err))
        }
    }
}

impl LOB<'_,OCIBLobLocator> {
    /// Debug helper that fetches first 50 (at most) bytes of BLOB content
    fn content_head(&self) -> Result<String> {
        const MAX_LEN : usize = 50;
        let len = self.len()?;
        let len = std::cmp::min(len, MAX_LEN);
        let mut buf = Vec::with_capacity(len);
        let len = self.read(0, len, &mut buf)?;
        let res = &buf[..len];
        let res = if len == MAX_LEN { format!("{:?}...", res) } else { format!("{:?}", res) };
        Ok(res)
    }
}

impl std::fmt::Debug for LOB<'_,OCIBLobLocator> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.content_head() {
            Ok(text) => f.write_fmt(format_args!("BLOB {}", text)),
            Err(err) => f.write_fmt(format_args!("BLOB {:?}", err))
        }
    }
}

impl LOB<'_,OCIBFileLocator> {
    /// Debug helper that fetches first 50 (at most) bytes of BFILE content
    fn content_head(&self) -> Result<String> {
        const MAX_LEN : usize = 50;
        let len = self.len()?;
        let len = std::cmp::min(len, MAX_LEN);
        let mut buf = Vec::with_capacity(len);
        let len = self.read(0, len, &mut buf)?;
        let res = &buf[..len];
        let res = if len == MAX_LEN { format!("{:?}...", res) } else { format!("{:?}", res) };
        Ok(res)
    }
}

impl std::fmt::Debug for LOB<'_,OCIBFileLocator> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.content_head() {
            Ok(text) => f.write_fmt(format_args!("BFILE {}", text)),
            Err(err) => f.write_fmt(format_args!("BFILE {:?}", err))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn partial_eq() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let oracle = crate::env()?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = conn.prepare("
            declare
                name_already_used exception; pragma exception_init(name_already_used, -955);
            begin
                execute immediate '
                    create table test_lobs (
                        id       number generated always as identity,
                        text     clob,
                        data     blob,
                        ext_file bfile
                    )
                ';
            exception
              when name_already_used then
                execute immediate '
                    truncate table test_lobs
                ';
            end;
        ")?;
        stmt.execute(&[])?;

        let stmt = conn.prepare("
            insert into test_lobs (text) values (empty_clob()) returning id, text into :id, :text
        ")?;
        let mut id1 : usize = 0;
        let mut lob1 = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id1, &mut lob1 ])?;

        let text = [
            "To see a World in a Grain of Sand\n",
            "And a Heaven in a Wild Flower\n",
            "Hold Infinity in the palm of your hand\n",
            "And Eternity in an hour\n"
        ];

        let written = lob1.append(text[0])?;
        assert_eq!(written, text[0].len());
        assert_eq!(lob1.len()?, text[0].len());

        let lob2 = lob1.clone()?;

        // They point to the same value and at this time they are completely in sync
        assert!(lob1.is_equal(&lob2)?);

        let written = lob2.append(text[1])?;
        assert_eq!(written, text[1].len());

        // Now they are out of sync
        assert!(!lob1.is_equal(&lob2)?);

        // At this time `lob1` is not yet aware that `lob2` added more text the LOB they "share".
        assert_eq!(lob2.len()?, text[0].len() + text[1].len());
        assert_eq!(lob1.len()?, text[0].len());

        let written = lob1.append(text[2])?;
        assert_eq!(written, text[2].len());

        // Now, after writing text[2], `lob1` has caught up with `lob2` prior writing and added more
        // text on its own. But now it's `lob2` turn to lag behind and not be aware of the added text.
        assert_eq!(lob1.len()?, text[0].len() + text[1].len() + text[2].len());
        assert_eq!(lob2.len()?, text[0].len() + text[1].len());

        // Let's save `lob2` now. It is still only knows about `text[0]` and `text[1]` fragments.
        let stmt = conn.prepare("
            insert into test_lobs (text) values (:text) returning id, text into :id, :saved_text
        ")?;
        let mut saved_lob_id : usize = 0;
        let mut saved_lob = CLOB::new(&conn)?;
        let res = stmt.execute_into(&[ &lob2 ], &mut [ &mut saved_lob_id, &mut saved_lob ])?;
        assert_eq!(res, 1);
        assert!(!stmt.is_null(":id")?, "id is not null");
        assert!(!stmt.is_null(":saved_text")?, "text is not null");

        // And thus `saved_lob` locator points to a distinct LOB value ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... that has only the `text[0]` and `text[1]`
        assert_eq!(saved_lob.len()?, text[0].len() + text[1].len());

        let written = lob2.append(text[3])?;
        assert_eq!(written, text[3].len());

        assert_eq!(lob2.len()?, text[0].len() + text[1].len() + text[2].len() + text[3].len());
        assert_eq!(lob1.len()?, text[0].len() + text[1].len() + text[2].len());

        // As `saved_lob` points to the enturely different LOB ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... it is not affected by `lob1` and `lob2` additions.
        assert_eq!(saved_lob.len()?, text[0].len() + text[1].len());
        Ok(())
    }
}
