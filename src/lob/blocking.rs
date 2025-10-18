use libc::c_void;

use super::*;
use crate::*;
use crate::oci::*;
use std::sync::atomic::Ordering;

impl<'a,T> LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
    /**
    Closes a previously opened internal or external LOB.

    Closing a LOB requires a round-trip to the server for both internal and external LOBs.
    For internal LOBs, `close` triggers other code that relies on the close call and for external
    LOBs (BFILEs), close actually closes the server-side operating system file.

    If you open a LOB, you must close it before you commit the transaction; an error is produced
    if you do not. When an internal LOB is closed, it updates the functional and domain indexes
    on the LOB column.

    It is an error to commit the transaction before closing all opened LOBs that were opened by
    the transaction. When the error is returned, the openness of the open LOBs is discarded, but
    the transaction is successfully committed. Hence, all the changes made to the LOB and non-LOB
    data in the transaction are committed, but the domain and function-based indexes are not updated.
    If this happens, you should rebuild the functional and domain indexes on the LOB column.

    # Failures

    - The internal LOB is not open.

    No error is returned if the BFILE exists but is not opened.

    # Example

    See [`LOB<T>::open()`]
    */
    pub fn close(&self) -> Result<()> {
        oci::lob_close(self.as_ref(), self.as_ref(), self.as_ref())?;
        self.inner.status_flags.fetch_and(!LOB_IS_OPEN, Ordering::Relaxed);
        Ok(())
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
        let mut flag = oci::Aligned::new(0u8);
        oci::lob_is_open(self.as_ref(), self.as_ref(), self.as_ref(), flag.as_mut_ptr())?;
        Ok( <u8>::from(flag) != 0 )
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
        oci::lob_open(self.as_ref(), self.as_ref(), self.as_ref(), OCI_LOB_READONLY)?;
        self.inner.status_flags.fetch_or(LOB_IS_OPEN, Ordering::Relaxed);
        Ok(())
    }

    /**
    For CLOBs and NCLOBs, if you do not pass `char_len`, then `char_len` is calculated internally as
    `byte_len/max char width`, so if max char width is 4, `char_len` is calculated as `byte_len/4`.
    OCILobRead2() does not calculate how many bytes are required for each character. Instead, OCILobRead2()
    fetches the number of characters that in the worst case can fit in `byte_len`. To fill the buffer, check
    the returned value to see how much of the buffer is filled, then call OCILobRead2() again to fetch the
    remaining bytes.
    */
    fn read_piece(&self, piece: u8, piece_size: usize, offset: usize, byte_len: usize, char_len: usize, cs_form: u8, buf: &mut Vec<u8>) -> Result<(bool,usize,usize)> {
        let space_available = buf.capacity() - buf.len();
        if piece_size > space_available {
            buf.reserve(piece_size - space_available);
        }
        let mut byte_cnt = byte_len as u64;
        let mut char_cnt = char_len as u64;
        let res = unsafe {
            oci::lob_read(
                self.as_ref(), self.as_ref(), self.as_ref(),
                &mut byte_cnt, &mut char_cnt, (offset + 1) as u64,
                buf.as_mut_ptr().add(buf.len()), piece_size as u64, piece,
                AL32UTF8, cs_form
            )
        }?;
        unsafe {
            buf.set_len(buf.len() + byte_cnt as usize);
        }
        Ok( (res == OCI_NEED_DATA, byte_cnt as usize, char_cnt as usize) )
    }
}

impl<'a, T> LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> + InternalLob {
    /**
        Tests if a locator points to a temporary LOB.

        # Returns

        * `true` - if this LOB locator points to a temporary LOB
        * `flase` - if it does not.
     */
    pub async fn is_temp(&self) -> Result<bool> {
        let mut is_temp = oci::Aligned::new(0u8);
        oci::lob_is_temporary(self.as_ref(), self.as_ref(), self.as_ref(), is_temp.as_mut_ptr())?;
        Ok( <u8>::from(is_temp) != 0 )
    }

    /**
    Appends another LOB value at the end of this LOB.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # let session = sibyl::test_env::get_session()?;
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
    let lob1 = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;
    lob1.append(text1)?;
    assert_eq!(lob1.len()?, text1.len());

    let lob2 = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;
    lob2.append(text2)?;
    // Cannot use `len` shortcut with `text2` because of `RIGHT SINGLE QUOTATION MARK`
    // after "bells"
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

    # Parameters

    - `src` - souce LOB
    - `src_offset`- the absolute offset for the source LOB.
    - `amount` - The number of characters for CLOBs or NCLOBs or the number of bytes for BLOBs to be copied from the source LOB to the destination LOB.
    - `offset` - The absolute offset for the destination LOB

    For character LOBs, the offset is the number of characters from the beginning of the LOB. For binary LOBs, it is the number of bytes.

    If the data exists at the destination's start position, it is overwritten with the source data.

    If the destination's start position is beyond the end of the current data, zero-byte fillers (for BLOBs)
    or spaces (for CLOBs) are written into the destination LOB from the end of the current data to the beginning
    of the newly written data from the source.

    The destination LOB is extended to accommodate the newly written data if it extends beyond the current
    length of the destination LOB.

    LOB buffering must not be enabled for either locator.

    Notes:
    - To copy the entire source LOB specify `amount` as `std::usize::MAX`.
    - You can call `len` to determine the length of the source LOB.

    # Example
    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # let session = sibyl::test_env::get_session()?;
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
    let lob1 = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;
    lob1.append(text)?;

    let lost_text = "
    Now timely sing, ere the rude Bird of Hate
    Foretell my hopeles doom, in som Grove ny:
    As thou from yeer to yeer hast sung too late
    ";
    let lob2 = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;
    lob2.append(lost_text)?;

    // Set source offset to 1 to skip leading \n:
    lob1.copy(&lob2, 1, 131, 364)?;

    // Read back the overwriten fragment.
    // Start with the leading \n, to make comparison easier:
    let mut fragment = String::new();
    lob1.read(363, 132, &mut fragment)?;

    assert_eq!(fragment, lost_text);

    let mut sonnet = String::new();
    // ASCII only, so we can use `len` as a "shortcut".
    let text_len = lob1.len()?;
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

    # Parameters

    - `src` - souce BFILE
    - `src_offset`- the absolute offset for the source BFILE. It is the number of bytes from the beginning of the BFILE.
    - `amount` - The number of characters for CLOBs or NCLOBs or the number of bytes for BLOBs to be copied from the source LOB to the destination LOB.
    - `offset` - The absolute offset for the destination LOB. For character LOBs, it is the number of characters from the beginning of the LOB at which
         to begin writing. For binary LOBs, it is the number of bytes from the beginning of the LOB.

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
    use sibyl::*;

    # let session = sibyl::test_env::get_session()?;
    #
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;

    //-------------------

    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
    let file_len = file.len()?;

    file.open_file()?;
    lob.load_from_file(&file, 0, file_len, 0)?;
    file.close_file()?;

    let lob_len = lob.len()?;
    assert_eq!(lob_len, 14);

    let mut text = String::new();
    lob.read(0, lob_len, &mut text)?;

    assert_eq!(text, "Hello, World!\n");

    //-------------------

    file.set_file_name("MEDIA_DIR", "konnichiwa_sekai.txt")?;
    let file_len = file.len()?;

    file.open_file()?;
    lob.load_from_file(&file, 0, file_len, lob_len)?;
    file.close()?;

    let lob_len = lob.len()?;
    assert_eq!(lob_len, 14 + 9);

    text.clear();
    lob.read(0, lob_len, &mut text)?;

    assert_eq!(text, "Hello, World!\nこんにちは世界！\n");

    //-------------------

    file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
    let file_len = file.len()?;

    file.open_file()?;
    lob.load_from_file(&file, 0, file_len, lob_len)?;
    file.close()?;

    let lob_len = lob.len()?;
    // Note that Oracle encoded 4 symbols (see below) into 8 "characters"
    assert_eq!(lob_len, 14 + 9 + 9);

    text.clear();
    // The reading stops at the end of the LOB value if we request more
    // characters than the LOB contains
    let num_read = lob.read(0, 100, &mut text)?;
    assert_eq!(num_read, 14 + 9 + 9);

    assert_eq!(text, "Hello, World!\nこんにちは世界！\n🚲🛠📬🎓\n");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn load_from_file(&self, src: &BFile, src_offset: usize, amount: usize, dst_offset: usize) -> Result<()> {
        oci::lob_load_from_file(
            self.as_ref(), self.as_ref(),
            self.as_ref(), src.as_ref(),
            amount as u64, (dst_offset + 1) as u64, (src_offset + 1) as u64
        )
    }

    /**
    Erases a specified portion of the internal LOB data starting at a specified offset.

    For BLOBs, erasing means that zero-byte fillers overwrite the existing LOB value.
    For CLOBs, erasing means that spaces overwrite the existing LOB value.

    # Parameters

    - `offset` - Absolute offset in characters (for CLOBs or NCLOBs) or bytes (for BLOBs)
        to the start of the LOB fragment to erase
    - `amount` - The number of characters or bytes to erase.

    Returns the actual number of characters or bytes erased.
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

    # Example

    See [`LOB<T>::write_first()`]
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
        let ptr = if len > 0 { content_type.as_ptr() } else { std::ptr::null::<u8>() };
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

    # Example

    ```
    use sibyl::CLOB;
    /*
        CREATE TABLE test_lobs (
            id       INTEGER GENERATED ALWAYS AS IDENTITY,
            text     CLOB,
            data     BLOB,
            ext_file BFILE
        );
     */
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        DECLARE
            row_id ROWID;
        BEGIN
            INSERT INTO test_lobs (text) VALUES (Empty_Blob()) RETURNING rowid INTO row_id;
            SELECT text INTO :NEW_LOB FROM test_lobs WHERE rowid = row_id FOR UPDATE;
        END;
    ")?;
    let mut lob = CLOB::new(&session)?;
    stmt.execute(&mut lob)?;

    let text = [
        "Love seeketh not itself to please,\n",
        "Nor for itself hath any care,\n",
        "But for another gives its ease,\n",
        "And builds a Heaven in Hell's despair.\n",
    ];

    lob.open()?;
    /*
     * It is not necessary to open a LOB to perform operations on it.
     * When using function-based indexes, extensible indexes or context,
     * and making multiple calls to update or write to the LOB, you should
     * first call `open`, then update the LOB as many times as you want,
     * and finally call `close`. This sequence of operations ensures that
     * the indexes are only updated once at the end of all the write
     * operations instead of once for each write operation.
     *
     * If you do not wrap your LOB operations inside the open or close API,
     * then the functional and domain indexes are updated each time you write
     * to the LOB. This can adversely affect performance.
     *
     * If you have functional or domain indexes, Oracle recommends that you
     * enclose write operations to the LOB within the open or close statements.
     */
    lob.append(text[0])?;
    lob.append(text[1])?;
    lob.append(text[2])?;
    lob.append(text[3])?;
    lob.close()?;

    session.commit()?;

    assert_eq!(lob.len()?, text[0].len() + text[1].len() + text[2].len() + text[3].len());
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn open(&self) -> Result<()> {
        oci::lob_open(self.as_ref(), self.as_ref(), self.as_ref(), OCI_LOB_READWRITE)?;
        self.inner.status_flags.fetch_or(LOB_IS_OPEN, Ordering::Relaxed);
        Ok(())
    }

    /**
    Truncates the LOB value to a shorter length.

    # Parameters

    - `new_len` - new LOB length.

    For character LOBs, `new_len` is the number of characters; for binary LOBs and BFILEs, it is the number
    of bytes in the LOB.
    */
    pub fn trim(&self, new_len: usize) -> Result<()> {
        oci::lob_trim(self.as_ref(), self.as_ref(), self.as_ref(), new_len as u64)
    }

    fn write_piece(&self, piece: u8, offset: usize, cs_form: u8, data: &[u8]) -> Result<(usize,usize)> {
        let mut byte_cnt = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        let mut char_cnt = 0u64;
        oci::lob_write(
            self.as_ref(), self.as_ref(), self.as_ref(),
            &mut byte_cnt, &mut char_cnt, (offset + 1) as u64,
            data.as_ptr(), data.len() as u64, piece,
            std::ptr::null_mut::<c_void>(), std::ptr::null::<c_void>(),
            AL32UTF8, cs_form
        )?;
        Ok( (byte_cnt as usize, char_cnt as usize) )
    }

    fn append_piece(&self, piece: u8, cs_form: u8, data: &[u8]) -> Result<(usize,usize)> {
        let mut byte_cnt = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        let mut char_cnt = 0u64;
        oci::lob_write_append(
            self.as_ref(), self.as_ref(), self.as_ref(),
            &mut byte_cnt, &mut char_cnt,
            data.as_ptr(), data.len() as u64, piece,
            std::ptr::null_mut::<c_void>(), std::ptr::null::<c_void>(),
            AL32UTF8, cs_form
        )?;
        Ok( (byte_cnt as usize, char_cnt as usize) )
    }

    fn create_temp(session: &'a Session<'a>, csform: u8, lob_type: u8, cache: Cache) -> Result<LOB<'a,T>> {
        let lob_descriptor = Descriptor::<T>::new(session)?;
        oci::lob_create_temporary(
            session.as_ref(), session.as_ref(), lob_descriptor.as_ref(),
            OCI_DEFAULT as _, csform, lob_type, cache as _, OCI_DURATION_SESSION
        )?;
        Ok(Self::make_temp(lob_descriptor, session))
    }
}

impl<'a> LOB<'a,OCICLobLocator> {
    /**
    Creates an empty temporary CLOB or NCLOB and its corresponding index in the user's temporary tablespace.

    # Parameters

    - `csform` - The LOB character set form of the data.
    - `cache` - Indicates whether the temporary LOB should be read into the cache.

    The temporary LOB is freed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub fn temp(session: &'a Session, csform: CharSetForm, cache: Cache) -> Result<Self> {
        Self::create_temp(session, csform as _, OCI_TEMP_CLOB, cache)
    }

    /**
    Writes a buffer into a LOB.

    # Parameters

    - `offset` - the absolute offset (in number of characters) from the beginning of the LOB,
    - `text` - slice of text to be written into this LOB.

    # Returns

    The number of characters written to the database.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # let session = sibyl::test_env::get_session()?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;
    let text = "tête-à-tête";
    assert_eq!(text.len(), 14); // byte count

    let written = lob.write(4, text)?;
    // Note that auto inserted spaces at 0..4 are not included
    // in the number of characters written
    assert_eq!(written, 11);    // char count

    // Note that initially the CLOB was empty, so writing at offset 4
    // inserted 4 spaces before the text we were writing:
    let lob_len_in_chars = lob.len()?;
    let mut lob_content = String::new();
    let num_chars_read = lob.read(0, lob_len_in_chars, &mut lob_content)?;
    assert_eq!(num_chars_read, 15);
    assert_eq!(lob_content, "    tête-à-tête");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn write(&self, offset: usize, text: &str) -> Result<usize> {
        let cs_form = self.charset_form()? as u8;
        let (_, char_count) = self.write_piece(OCI_ONE_PIECE, offset, cs_form, text.as_bytes())?;
        Ok(char_count)
    }

    /**
    Starts piece-wise writing into a LOB.

    The application must call `write_next` to write more pieces into the LOB.
    `write_last` terminates the piecewise write.

    # Parameters

    - `offset` - The absolute offset from the beginning of the LOB. For character LOBs, it is the number of characters
            from the beginning of the LOB; for binary LOBs, it is the number of bytes
    - `text` - First text piece to write into the LOB.

    # Returns

    The number of characters written to the database for the first piece.

    # Example
    ```
    use sibyl::{ CLOB, CharSetForm, Cache };

    # let session = sibyl::test_env::get_session()?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;

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

    lob.close()?;

    assert_eq!(lob.len()?, chunk_size * 10);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn write_first(&self, offset: usize, text: &str) -> Result<usize> {
        let cs_form = self.charset_form()? as u8;
        let (_, char_count) = self.write_piece(OCI_FIRST_PIECE, offset, cs_form, text.as_bytes())?;
        Ok(char_count)
    }

    /**
    Continues piece-wise writing into a LOB.

    The application must call `write_next` to write more pieces into the LOB.
    `write_last` terminates the piecewise write.

    # Parameters

    - `text` - Next text piece to write into the LOB.

    # Returns

    The number of characters written to the database for this piece.

    # Example

    See [`LOB<T>::write_first()`]
    */
    pub fn write_next(&self, text: &str) -> Result<usize> {
        let (_, char_count) = self.write_piece(OCI_NEXT_PIECE, 0, 0, text.as_bytes())?;
        Ok(char_count)
    }

    /**
    Terminates piece-wise writing into a LOB.

    # Parameters

    - `text` - Last text piece to write into the LOB.

    # Returns

    The number of characters written to the database for the last piece.

    # Example

    See [`LOB<T>::write_first()`]
    */
    pub fn write_last(&self, text: &str) -> Result<usize> {
        let (_, char_count) = self.write_piece(OCI_LAST_PIECE, 0, 0, text.as_bytes())?;
        Ok(char_count)
    }

    /**
    Writes data starting at the end of a LOB.

    # Parameters

    - `text` - Text piece to append to the LOB.

    # Returns

    The number of characters written to the database.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # let session = sibyl::test_env::get_session()?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;

    let written = lob.append("Hello, World!")?;

    assert_eq!(written, 13);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn append(&self, text: &str) -> Result<usize> {
        let cs_form = self.charset_form()? as u8;
        let (_, char_count) = self.append_piece(OCI_ONE_PIECE, cs_form, text.as_bytes())?;
        Ok(char_count)
    }

    /**
    Starts piece-wise writing at the end of a LOB.

    The application must call `append_next` to write more pieces into the LOB.
    `append_last` terminates the piecewise write.

    # Parameters

    - `text` - First text piece to append to the LOB.

    # Returns

    The number of characters written to the database for the first piece.

    # Example

    ```
    use sibyl::{ CLOB, CharSetForm, Cache };

    # let session = sibyl::test_env::get_session()?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;
    let chunk_size = lob.chunk_size()?;
    let data = vec![42u8;chunk_size];
    let text = std::str::from_utf8(&data)?;

    lob.open()?;

    let written = lob.append_first(text)?;
    assert_eq!(written, chunk_size);
    for i in 0..8 {
        let written = lob.append_next(text)?;
        assert_eq!(written, chunk_size);
    }
    let written = lob.append_last(text)?;
    assert_eq!(written, chunk_size);

    lob.close()?;

    assert_eq!(lob.len()?, chunk_size * 10);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn append_first(&self, text: &str) -> Result<usize> {
        let cs_form = self.charset_form()? as u8;
        let (_, char_count) = self.append_piece(OCI_FIRST_PIECE, cs_form, text.as_bytes())?;
        Ok(char_count)
    }

    /**
    Continues piece-wise writing at the end of a LOB.

    The application must call `append_next` to write more pieces into the LOB.
    `append_last` terminates the piecewise write.

    # Parameters

    - `text` - Next text piece to append to the LOB.

    # Returns

    The number of characters written to the database for this piece.

    # Example

    See [`LOB<T>::append_first()`]
    */
    pub fn append_next(&self, text: &str) -> Result<usize> {
        let (_, char_count) = self.append_piece(OCI_NEXT_PIECE, 0, text.as_bytes())?;
        Ok(char_count)
    }

    /**
    Terminates piece-wise writing at the end of a LOB.

    # Parameters

    - `text` - Last text piece to append to the LOB.

    # Returns

    The number of charcaters written to the database for the last piece.

    # Example

    See [`LOB<T>::append_first()`]
    */
    pub fn append_last(&self, text: &str) -> Result<usize> {
        let (_, char_count) = self.append_piece(OCI_LAST_PIECE, 0, text.as_bytes())?;
        Ok(char_count)
    }

    /**
    Reads specified number of characters from this LOB, appending them to `buf`.

    # Parameters

    - `offset` - The absolute offset (in characters) from the beginning of the LOB value.
    - `len` - The  maximum number of characters to read into the buffer.
    - `buf` - The output buffer.

    # Returns

    The total number of characters read.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # let session = sibyl::test_env::get_session()?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;
    lob.write(4, "tête-à-tête")?;

    let mut lob_content = String::new();
    // There are 15 characters in the LOB - 11 from the text we
    // inserted and 4 padding spaced at 0..4.
    // We can request to read more than that. LOB will stop at the
    // end of its value and return the actual number of characters
    // that were read.
    let num_chars_read = lob.read(0, 100, &mut lob_content)?;
    assert_eq!(num_chars_read, 15);
    assert_eq!(lob_content, "    tête-à-tête");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn read(&self, offset: usize, len: usize, buf: &mut String) -> Result<usize> {
        if len == 0 {
            return Ok(len);
        }
        let buf = unsafe { buf.as_mut_vec() };
        let bytes_available = buf.capacity() - buf.len();
        let bytes_needed = len * 4;
        if bytes_needed > bytes_available {
            buf.reserve(bytes_needed - bytes_available);
        }
        let cs_form = self.charset_form()? as u8;
        let (_, _, char_count) = self.read_piece(OCI_ONE_PIECE, bytes_needed, offset, bytes_needed, len, cs_form, buf)?;
        Ok(char_count)
    }

    /**
    Starts piece-wise reading of the specified number of characters from this LOB into the provided buffer,
    returning a flag which indicates whether there are more pieces to read until the requested piece is
    complete. Application should call `read_next` (and **only** `read_next`) repeatedly until `read_next`
    returns `false`.

    # Parameters

    - `piece_size` - number of characters to read for the first piece
    - `offset` - The absolute offset (in characters) from the beginning of the LOB value.
    - `len` - The total maximum number of characters to read.
    - `buf` - The output buffer.
    - `num_read` - The number of characters actually read for this piece.

    **Note** that the output buffer does not need to be the same for subsequent reading (even though that's
    what the example below does :-))

    # Returns

    `true` if `read_next` should be called to continue reading the specified LOB fragment.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # let session = sibyl::test_env::get_session()?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No)?;
    let chunk_size = lob.chunk_size()?;
    let fragment = vec![42u8;chunk_size];
    let fragment_as_text = String::from_utf8(fragment)?;

    lob.open()?;

    let mut written = lob.append_first(&fragment_as_text)?;
    for _i in 0..10 {
        written += lob.append_next(&fragment_as_text)?;
    }
    written += lob.append_last(&fragment_as_text)?;
    assert_eq!(written, chunk_size * 12);

    let mut text = String::new();
    let piece_size = chunk_size;
    let offset = chunk_size * 2;
    let length = chunk_size * 5;
    let mut read_len = 0usize;
    let mut has_next = lob.read_first(piece_size, offset, length, &mut text, &mut read_len)?;
    let mut text_len = read_len;
    while has_next {
        has_next = lob.read_next(piece_size, &mut text, &mut read_len)?;
        text_len += read_len;
    }
    assert_eq!(text_len, chunk_size * 5);
    assert_eq!(text_len, text.len());

    lob.close()?;
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut String, num_read: &mut usize) -> Result<bool> {
        let buf = unsafe { buf.as_mut_vec() };
        let cs_form = self.charset_form()? as u8;
        let (has_more, _, char_count) = self.read_piece(OCI_FIRST_PIECE, piece_size * 4, offset, len * 4, len, cs_form, buf)?;
        *num_read = char_count;
        Ok(has_more)
    }

    /**
    Continues piece-wise reading of the LOB fragment started by `read_first`, returning a flag which indicates whether
    there are more pieces to read until the requested fragment is complete. Application should keep calling
    `read_next` until it returns `false`.

    # Parameters

    - `piece_size` - number of characters to read for the next piece
    - `buf` - The output buffer.
    - `num_read` - The number of characters actually read for this piece.

    # Returns

    `true` if `read_next` should be called again to continue reading the specified LOB fragment.

    # Example

    See [`LOB<T>::read_first()`]
    */
    pub fn read_next(&self, piece_size: usize, buf: &mut String, num_read: &mut usize) -> Result<bool> {
        let buf = unsafe { buf.as_mut_vec() };
        let (has_more, _, char_count) = self.read_piece(OCI_NEXT_PIECE, piece_size * 4, 0, 0, 0, 0, buf)?;
        *num_read = char_count;
        Ok(has_more)
    }
}

impl<'a> LOB<'a,OCIBLobLocator> {
    /**
    Creates an empty temporary BLOB and its corresponding index in the user's temporary tablespace.

    # Parameters

    - `cache` - Indicates whether the temporary LOB should be read into the cache.

    The temporary LOB is freed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub fn temp(session: &'a Session, cache: Cache) -> Result<Self> {
        Self::create_temp(session, 0u8, OCI_TEMP_BLOB, cache)
    }

    /**
    Writes a buffer into a LOB.

    # Parameters

    - `offset` - the number of bytes from the beginning of the LOB
    - `data` - slice of bytes to write into this LOB

    # Returns

    The number of bytes written to the database.

    # Example
    ```
    use sibyl::{BLOB, Cache};

    # let session = sibyl::test_env::get_session()?;
    let lob = BLOB::temp(&session, Cache::No)?;
    let written = lob.write(3, "Hello, World!".as_bytes())?;
    assert_eq!(13, written);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.write_piece(OCI_ONE_PIECE, offset, 0, data)?;
        Ok(byte_count)
    }

    /**
    Starts piece-wise writing into a LOB.

    The application must call `write_next` to write more pieces into the LOB.
    `write_last` terminates the piecewise write.

    # Parameters

    - `offset` - the number of bytes from the beginning of the LOB
    - `data` - the first slice of bytes to write into this LOB

    # Returns

    The number of bytes written to the database for the first piece.

    # Example
    ```
    use sibyl::{BLOB, Cache};

    # let session = sibyl::test_env::get_session()?;
    let lob = BLOB::temp(&session, Cache::No)?;
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
        let (byte_count, _) = self.write_piece(OCI_FIRST_PIECE, offset, 0, data)?;
        Ok(byte_count)
    }

    /**
    Continues piece-wise writing into a LOB.

    The application must call `write_next` to write more pieces into the LOB.
    `write_last` terminates the piecewise write.

    # Parameters

    - `data` - the next slice of bytes to write into this LOB

    # Returns

    The number of bytes written to the database for this piece.
    */
    pub fn write_next(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.write_piece(OCI_NEXT_PIECE, 0, 0, data)?;
        Ok(byte_count)
    }

    /**
    Terminates piece-wise writing into a LOB.

    # Parameters

    - `data` - the last slice of bytes to write into this LOB

    # Returns

    The number of bytes written to the database for the last piece.
    */
    pub fn write_last(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.write_piece(OCI_LAST_PIECE, 0, 0, data)?;
        Ok(byte_count)
    }

    /**
    Writes data starting at the end of a LOB.

    # Parameters

    - `data` - bytes to append to this LOB

    # Returns

    The number of bytes written to the database.

    # Example

    ```
    use sibyl::{BLOB, Cache};

    # let session = sibyl::test_env::get_session()?;
    let lob = BLOB::temp(&session, Cache::No)?;

    let written = lob.append("Hello, World!".as_bytes())?;

    assert_eq!(13, written);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn append(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.append_piece(OCI_ONE_PIECE, 0, data)?;
        Ok(byte_count)
    }

    /**
    Starts piece-wise writing at the end of a LOB.

    The application must call `append_next` to write more pieces into the LOB.
    `append_last` terminates the piecewise write.

    # Parameters

    - `data` - the first slice of bytes to append to this LOB

    # Returns

    The number of bytes written to the database for the first piece.

    # Example

    ```
    use sibyl::{BLOB, Cache};

    # let session = sibyl::test_env::get_session()?;
    let mut lob = BLOB::temp(&session, Cache::No)?;
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
        let (byte_count, _) = self.append_piece(OCI_FIRST_PIECE, 0, data)?;
        Ok(byte_count)
    }

    /**
    Continues piece-wise writing at the end of a LOB.

    The application must call `append_next` to write more pieces into the LOB.
    `append_last` terminates the piecewise write.

    # Parameters

    - `data` - the next slice of bytes to append to this LOB

    # Returns

    The number of bytes written to the database for this piece.
    */
    pub fn append_next(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.append_piece(OCI_NEXT_PIECE, 0, data)?;
        Ok(byte_count)
    }

    /**
    Terminates piece-wise writing at the end of a LOB.

    # Parameters

    - `data` - the last slice of bytes to append to this LOB

    # Returns

    The number of bytes written to the database for the last piece.
    */
    pub fn append_last(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.append_piece(OCI_LAST_PIECE, 0, data)?;
        Ok(byte_count)
    }

    /**
    Reads specified number of bytes from this LOB, appending them to `buf`.

    # Parameters

    - `offset` - The absolute offset (in bytes) from the beginning of the LOB value.
    - `len` - The  maximum number of bytes to read into the buffer.
    - `buf` - The output buffer.

    # Returns

    The number of bytes that were read and appended to `buf`.

    # Example

    ```
    use sibyl::{BLOB, Cache, BFile};

    # let session = sibyl::test_env::get_session()?;
    let lob = BLOB::temp(&session, Cache::No)?;

    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "modem.jpg")?;
    assert!(file.file_exists()?);
    let file_len = file.len()?;
    file.open_readonly()?;
    lob.load_from_file(&file, 0, file_len, 0)?;
    file.close_file()?;
    assert_eq!(lob.len()?, file_len);

    let mut data = Vec::new();
    let num_read = lob.read(0, file_len, &mut data)?;

    assert_eq!(num_read, file_len);
    assert_eq!(data.len(), file_len);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn read(&self, offset: usize, len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        if len == 0 {
            return Ok(0);
        }
        let (_, byte_count, _) = self.read_piece(OCI_ONE_PIECE, len, offset, len, 0, 0, buf)?;
        Ok( byte_count )
    }

    /**
    Starts piece-wise reading of the specified number of bytes from this LOB into the provided buffer, returning a tuple
    with 2 elements - the number of bytes read in the current piece and the flag which indicates whether there are more
    pieces to read until the requested fragment is complete. Application should call `read_next` (and **only** `read_next`)
    repeatedly until "more data" flag becomes `false`.

    # Parameters

    - `piece_size` - number of bytes to read for the first piece
    - `offset` - The absolute offset (in bytes) from the beginning of the LOB value.
    - `len` - The total maximum number of bytes to read.
    - `buf` - The output buffer.
    - `num_read` - The number of bytes actually read for this piece.

    # Returns

    `true` if `read_next` should be called to continue reading the specified LOB fragment.

    # Example

    ```
    use sibyl::{BLOB, Cache, BFile};

    # let (feature_release, _, _, _, _) = sibyl::client_version();
    # // if feature_release <= 19 {
    # let session = sibyl::test_env::get_session()?;
    let lob = BLOB::temp(&session, Cache::No)?;
    let chunk_size = lob.chunk_size()?;

    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "monitor.jpg")?;
    assert!(file.file_exists()?);
    let file_len = file.len()?;
    file.open_readonly()?;
    lob.load_from_file(&file, 0, file_len, 0)?;
    file.close_file()?;
    assert_eq!(lob.len()?, file_len);


    let piece_size = chunk_size;
    let offset = 0;
    let length = file_len;

    let mut data = Vec::new();
    let mut bytes_read = 0usize;
    let mut has_next = lob.read_first(piece_size, offset, length, &mut data, &mut bytes_read)?;

    let mut data_len = bytes_read;
    assert_eq!(data_len, chunk_size);
    assert_eq!(data.len(), data_len);
    while has_next {
        has_next = lob.read_next(piece_size, &mut data, &mut bytes_read)?;
        data_len += bytes_read;
        assert_eq!(data_len, data.len());
    }

    assert_eq!(data_len, file_len);
    assert_eq!(data.len(), file_len);
    # // }
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```

    # Known Issues

    - 21c clients fail with SIGSEGV when reading the last piece from the 19c server.
    */
    pub fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        let (has_more, byte_count, _) = self.read_piece(OCI_FIRST_PIECE, piece_size, offset, len, 0, 0, buf)?;
        *num_read = byte_count;
        Ok(has_more)
    }

    /**
    Continues piece-wise reading of the fragment started by `read_first`, returning a tuple with 2 elements - the number of
    bytes read in the current piece and the flag which indicates whether there are more pieces to read until the requested
    fragment is complete. Application should keep calling `read_next` until "more data" flag becomes `false`.

    # Parameters

    - `piece_size` - number of bytes to read for the next piece
    - `buf` - The output buffer.
    - `num_read` - The number of bytes actually read for this piece.

    # Returns

    `true` if `read_next` should be called again to continue reading the specified LOB fragment.

    # Known Issues

    - 21c clients (tested with instant clients 21.4 and 21.9) are getting SIGSEGV when they are reading the last piece from the
      19c server.
    */
    pub fn read_next(&self, piece_size: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        let (has_more, byte_count, _) = self.read_piece(OCI_NEXT_PIECE, piece_size, 0, 0, 0, 0, buf)?;
        *num_read = byte_count;
        Ok(has_more)
    }
}

impl<'a> LOB<'a,OCIBFileLocator> {
    /**
    Closes a previously opened BFILE.

    No error is returned if the BFILE exists but is not opened.

    This function is only meaningful the first time it is called for a particular
    BFILE locator. Subsequent calls to this function using the same BFILE locator
    have no effect.
    */
    pub fn close_file(&self) -> Result<()> {
        oci::lob_file_close(self.as_ref(), self.as_ref(), self.as_ref())?;
        self.inner.status_flags.fetch_and(!LOB_IS_OPEN, Ordering::Relaxed);
        Ok(())
    }

    /**
    Tests to see if the BFILE exists on the server's operating system.

    # Example

    ```
    use sibyl::BFile;

    # let session = sibyl::test_env::get_session()?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "formatted_doc.txt")?;

    assert!(file.file_exists()?);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn file_exists(&self) -> Result<bool> {
        let mut exists = oci::Aligned::new(0u8);
        oci::lob_file_exists(self.as_ref(), self.as_ref(), self.as_ref(), exists.as_mut_ptr())?;
        Ok( <u8>::from(exists) != 0 )
    }

    /**
    Returns `true` if the BFILE was opened using this particular locator.
    However, a different locator may have the file open. Openness is associated
    with a particular locator.

    # Example

    ```
    use sibyl::BFile;

    # let session = sibyl::test_env::get_session()?;
    let file = BFile::new(&session)?;
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
        let mut is_open = oci::Aligned::new(0u8);
        oci::lob_file_is_open(self.as_ref(), self.as_ref(), self.as_ref(), is_open.as_mut_ptr())?;
        Ok( <u8>::from(is_open) != 0 )
    }

    /**
    Opens a BFILE on the file system of the server. The BFILE can only be opened
    for read-only access. BFILEs can not be written through Oracle Database.

    This function is only meaningful the first time it is called for a particular
    BFILE locator. Subsequent calls to this function using the same BFILE locator
    have no effect.

    # Example

    See [`LOB<T>::is_file_open()`]
    */
    pub fn open_file(&self) -> Result<()> {
        oci::lob_file_open(self.as_ref(), self.as_ref(), self.as_ref(), OCI_FILE_READONLY)?;
        self.inner.status_flags.fetch_or(LOB_IS_OPEN, Ordering::Relaxed);
        Ok(())
    }

    /**
    Reads specified number of bytes from this BFILE, appending them to `buf`.

    # Parameters

    - `offset` - The absolute offset (in bytes) from the beginning of the LOB value.
    - `len` - The total maximum number of bytes to read.
    - `buf` - The output buffer.

    # Returns

    The number of bytes that were read and appended to `buf`.

    # Example

    ```
    use sibyl::BFile;

    # let session = sibyl::test_env::get_session()?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
    let file_len = file.len()?;
    file.open_file()?;

    let mut data = Vec::new();
    let num_read = file.read(0, file_len, &mut data)?;

    assert_eq!(num_read, file_len);
    assert_eq!(data, [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21, 0x00, 0x0a]);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn read(&self, offset: usize, len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        let (_, byte_count, _) = self.read_piece(OCI_ONE_PIECE, len, offset, len, 0, 0, buf)?;
        Ok( byte_count )
    }

    /**
    Starts piece-wise reading of the specified number of bytes from this LOB into the provided buffer, returning a flag
    which indicates whether there are more data to read until the requested fragment is complete. Application should
    call `read_next` (and **only** `read_next`) repeatedly until "more data" flag becomes `false`.

    # Parameters

    - `piece_size` - number of bytes to read for the first piece
    - `offset` - The absolute offset (in bytes) from the beginning of the LOB value.
    - `len` - The total maximum number of bytes to read.
    - `buf` - The output buffer.
    - `num_read` - The number of bytes actually read for this piece.

    # Returns

    `true` if `read_next` should be called to continue reading the specified LOB fragment.

    # Example

    ```
    use sibyl::BFile;

    # let (feature_release, _, _, _, _) = sibyl::client_version();
    # if feature_release <= 19 {
    # let session = sibyl::test_env::get_session()?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "keyboard.jpg")?;
    assert!(file.file_exists()?);
    let file_len = file.len()?;
    file.open_readonly()?;

    let piece_size = 8192;
    let offset = 0;
    let length = file_len;

    let mut data = Vec::new();
    let mut bytes_read = 0usize;
    let mut has_next = file.read_first(piece_size, offset, length, &mut data, &mut bytes_read)?;
    let mut data_len = bytes_read;
    assert_eq!(data.len(), data_len);
    while has_next {
        has_next = file.read_next(piece_size, &mut data, &mut bytes_read)?;
        data_len += bytes_read;
        assert_eq!(data_len, data.len());
    }

    assert_eq!(data_len, file_len);
    assert_eq!(data.len(), file_len);
    # }
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```

    # Known Issues

    - 21c clients fail with SIGSEGV when reading the last piece from the 19c server.
    */
    pub fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        let (has_more, byte_count, _) = self.read_piece(OCI_FIRST_PIECE, piece_size, offset, len, 0, 0, buf)?;
        *num_read = byte_count;
        Ok(has_more)
    }

    /**
    Continues piece-wise reading of the fragment started by `read_first`, returning a flag which indicates whether
    there are more data to read until the requested fragment is complete. Application should keep calling `read_next`
    until "more data" flag becomes `false`.

    # Parameters

    - `piece_size` - number of bytes to read for the next piece
    - `buf` - The output buffer.
    - `num_read` - The number of bytes actually read for this piece.

    # Returns

    `true` if `read_next` should be called again to continue reading the specified LOB fragment.

    # Known Issues

    - 21c clients fail with SIGSEGV when reading the last piece from the 19c server.
    */
    pub fn read_next(&self, piece_size: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        let (has_more, byte_count, _) = self.read_piece(OCI_NEXT_PIECE, piece_size, 0, 0, 0, 0, buf)?;
        *num_read = byte_count;
        Ok(has_more)
    }
}
