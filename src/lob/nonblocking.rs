/*!
Nonblocking mode LOB methods.

> **Note** that for various reasons piece-wise LOB content operations - i.e. `read_first`, `read_next`
and `write_first`, `write_next`, `write_last` methods - are not supported in nonblocking mode.
*/

use super::{LOB, InternalLob, LOB_IS_OPEN, LOB_FILE_IS_OPEN, LOB_IS_TEMP};
use crate::{Result, BFile, oci::*, session::{Session, SvcCtx}, Error};
use std::sync::{atomic::Ordering, Arc};

impl<'a,T> LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn get_svc(&self) -> Arc<SvcCtx> {
        self.inner.svc.clone()
    }

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

    ```
    use sibyl::CLOB;

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
    let stmt = session.prepare("
        DECLARE
            row_id ROWID;
        BEGIN
            INSERT INTO test_lobs (text) VALUES (Empty_Clob()) RETURNING rowid into row_id;
            SELECT text INTO :NEW_LOB FROM test_lobs WHERE rowid = row_id FOR UPDATE;
        END;
    ").await?;
    let mut lob = CLOB::new(&session)?;
    stmt.execute(&mut lob).await?;

    let text = [
        "Love seeketh not itself to please,\n",
        "Nor for itself hath any care,\n",
        "But for another gives its ease,\n",
        "And builds a Heaven in Hell's despair.\n",
    ];

    lob.open().await?;
    lob.append(text[0]).await?;
    lob.append(text[1]).await?;
    lob.append(text[2]).await?;
    lob.append(text[3]).await?;
    lob.close().await?;

    assert_eq!(lob.len().await?, text[0].len() + text[1].len() + text[2].len() + text[3].len());
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn close(&self) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobClose::new(self.get_svc(), lob).await?;
        self.inner.status_flags.fetch_and(!LOB_IS_OPEN, Ordering::Relaxed);
        Ok(())
    }

    /**
    Returns the length of a LOB.

    For character LOBs, it is the number of characters; for binary LOBs and BFILEs, it is the number of
    bytes in the LOB.

    ## Notes

    - If the LOB is NULL, the length is undefined.
    - The length of a BFILE includes the EOF, if it exists.
    - The length of an empty internal LOB is zero.
    - Any zero-byte or space fillers in the LOB written by previous calls to `erase` or `write` are also
        included in the length count.
    */
    pub async fn len(&self) -> Result<usize> {
        let lob: &OCILobLocator = self.as_ref();
        let len = futures::LobGetLength::new(self.get_svc(), lob).await?;
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
    pub async fn is_open(&self) -> Result<bool> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobIsOpen::new(self.get_svc(), lob).await
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
    pub async fn open_readonly(&self) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobOpen::new(self.get_svc(), lob, OCI_LOB_READONLY).await?;
        self.inner.status_flags.fetch_or(LOB_IS_OPEN, Ordering::Relaxed);
        Ok(())
    }

    /**
    For CLOBs and NCLOBs, if you do not pass `char_len`, then `char_len` is calculated internally as
    `byte_len/max char width`, so if max char width is 4, `char_len` is calculated as `byte_len/4`.

    **Note** that OCILobRead2() does not calculate how many bytes are required for each character. Instead, OCILobRead2()
    fetches the number of characters that in the worst case can fit in `byte_len`.
    */
    async fn read_piece(&self, piece: u8, piece_size: usize, offset: usize, byte_len: usize, char_len: usize, cs_form: u8, buf: &mut Vec<u8>) -> Result<(bool,usize,usize)> {
        let space_available = buf.capacity() - buf.len();
        if piece_size > space_available {
            buf.reserve(piece_size - space_available);
        }
        let lob: &OCILobLocator = self.as_ref();
        let (res, num_bytes, num_chars) = futures::LobRead::new(self.get_svc(), lob, piece, piece_size, offset, byte_len, char_len, cs_form, buf).await?;
        unsafe {
            buf.set_len(buf.len() + num_bytes);
        }
        Ok( (res == OCI_NEED_DATA, num_bytes, num_chars) )
    }
}

impl<'a, T> LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> + InternalLob {
    pub(crate) fn make(locator: Descriptor<T>, session: &'a Session) -> Self {
        let this = Self::make_new(locator, session);
        let _ = this.is_temp();
        this
    }

    pub async fn is_temp(&self) -> Result<bool> {
        let stmt = self.session.prepare("BEGIN :RES := DBMS_LOB.ISTEMPORARY(:LOC); END;").await?;
        let mut flag = 0;
        stmt.execute((&mut flag, self, ())).await?;
        let is_temp = flag != 0;
        if is_temp {
            self.inner.status_flags.fetch_or(LOB_IS_TEMP, Ordering::Release);
        } else {
            self.inner.status_flags.fetch_and(!LOB_IS_TEMP, Ordering::Release);
        }
        Ok(is_temp)
    }

    /**
    Appends another LOB value at the end of this LOB.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
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
    let lob1 = CLOB::temp(&session, CharSetForm::Implicit, Cache::No).await?;
    lob1.append(text1).await?;
    assert_eq!(lob1.len().await?, text1.len());

    let lob2 = CLOB::temp(&session, CharSetForm::Implicit, Cache::No).await?;
    lob2.append(text2).await?;
    // Cannot use `len` shortcut with `text2` because of the `RIGHT SINGLE QUOTATION MARK` in it
    assert_eq!(lob2.len().await?, text2.chars().count());

    lob1.append_lob(&lob2).await?;

    assert_eq!(lob1.len().await?, text1.len() + text2.chars().count());
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn append_lob(&self, other_lob: &Self) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        let src: &OCILobLocator = other_lob.as_ref();
        futures::LobAppend::new(self.get_svc(), lob, src).await
    }

    /**
    Copies all or a portion of another LOB value.

    # Parameters

    - `src` - souce LOB
    - `src_offset`- the absolute offset for the source LOB.
    - `amount` - The number of characters for CLOBs or NCLOBs or the number of bytes for BLOBs to be copied from the source LOB to the destination LOB.
    - `offset` - The absolute offset for the destination LOB

    If the data exists at the destination's start position, it is overwritten with the source data.

    If the destination's start position is beyond the end of the current data, zero-byte fillers (for BLOBs)
    or spaces (for CLOBs) are written into the destination LOB from the end of the current data to the beginning
    of the newly written data from the source.

    The destination LOB is extended to accommodate the newly written data if it extends beyond the current
    length of the destination LOB.

    LOB buffering must not be enabled for either locator.

    ## Notes

    - To copy the entire source LOB specify `amount` as `std::usize::MAX`.
    - `offset` and `src_offset` - the number of characters (character LOB) or bytes (binary LOB) from the
        beginning of the LOB - start at 0.
    - You can call `len` to determine the length of the source LOB.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
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
    let lob1 = CLOB::temp(&session, CharSetForm::Implicit, Cache::No).await?;
    lob1.append(text).await?;

    let lost_text = "
    Now timely sing, ere the rude Bird of Hate
    Foretell my hopeles doom, in som Grove ny:
    As thou from yeer to yeer hast sung too late
    ";
    let lob2 = CLOB::temp(&session, CharSetForm::Implicit, Cache::No).await?;
    lob2.append(lost_text).await?;

    // Set source offset to 1 to skip leading \n:
    lob1.copy(&lob2, 1, 131, 364).await?;

    // Read back the overwriten fragment.
    // Start with the leading \n, to make comparison easier:
    let mut fragment = String::new();
    lob1.read(363, 132, &mut fragment).await?;

    assert_eq!(fragment, lost_text);

    // ASCII only. That means we can use `len` as a "shortcut".
    let text_len = lob1.len().await?;
    // Recall that the buffer needs to be allocated for the worst case
    let mut sonnet = String::new();
    lob1.read(0, text_len, &mut sonnet).await?;

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
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn copy(&self, src: &Self, src_offset: usize, amount: usize, offset: usize) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobCopy::new(self.get_svc(), lob, offset, src.as_ref(), src_offset, amount).await
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
    use sibyl::{CLOB, Cache, CharSetForm, BFile};

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
    let stmt = session.prepare("
        DECLARE
            row_id ROWID;
        BEGIN
            INSERT INTO test_lobs (text) VALUES (Empty_Clob()) RETURNING rowid into row_id;
            SELECT text INTO :NEW_LOB FROM test_lobs WHERE rowid = row_id FOR UPDATE;
        END;
    ").await?;
    let mut lob = CLOB::new(&session)?;
    stmt.execute(&mut lob).await?;

    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
    let file_len = file.len().await?;

    lob.open().await?;
    file.open_file().await?;
    lob.load_from_file(&file, 0, file_len, 0).await?;
    file.close_file().await?;

    let lob_len = lob.len().await?;
    assert_eq!(lob_len, 13);

    let mut text = String::new();
    lob.read(0, lob_len, &mut text).await?;

    assert_eq!(text, "Hello, World!");

    file.set_file_name("MEDIA_DIR", "hello_world_cyrillic.txt")?;
    let file_len = file.len().await?;

    file.open_file().await?;
    lob.load_from_file(&file, 0, file_len, 0).await?;
    file.close().await?;

    let lob_len = lob.len().await?;
    assert_eq!(lob_len, 16);

    text.clear();
    lob.read(0, lob_len, &mut text).await?;

    assert_eq!(text, "Здравствуй, Мир!");

    file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
    let file_len = file.len().await?;

    lob.trim(0).await?;
    file.open_file().await?;
    lob.load_from_file(&file, 0, file_len, 0).await?;
    file.close().await?;

    let lob_len = lob.len().await?;
    // Note that Oracle encoded 4 symbols (see below) into 8 characters
    assert_eq!(lob_len, 8);

    text.clear();
    // The reading stops at the end of LOB value if we request more
    // characters than the LOB contains
    let num_read = lob.read(0, 100, &mut text).await?;
    assert_eq!(num_read, 8);

    assert_eq!(text, "🚲🛠📬🎓");
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn load_from_file(&self, src: &'a BFile<'a>, src_offset: usize, amount: usize, offset: usize) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        let src: &OCILobLocator = src.as_ref();
        futures::LobLoadFromFile::new(self.get_svc(), lob, offset, src, src_offset, amount).await
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
    pub async fn erase(&self, offset: usize, amount: usize) -> Result<usize> {
        let lob: &OCILobLocator = self.as_ref();
        let count = futures::LobErase::new(self.get_svc(), lob, offset, amount).await?;
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
    pub async fn chunk_size(&self) -> Result<usize> {
        let mut size = self.chunk_size.load(Ordering::Relaxed);
        if size == 0 {
            let lob: &OCILobLocator = self.as_ref();
            size = futures::LobGetChunkSize::new(self.get_svc(), lob).await?;
            self.chunk_size.store(size, Ordering::Relaxed);
        }
        Ok( size as usize )
    }

    /**
    Returns he user-specified content type string for the data in a SecureFile, if set.

    This function only works on SecureFiles.
    */
    pub async fn content_type(&self) -> Result<String> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobGetContentType::new(self.get_svc(), lob).await
    }

    /**
    Sets a content type string for the data in the SecureFile to something that can be used by an application.

    This function only works on SecureFiles.
    */
    pub async    fn set_content_type(&self, content_type: &str) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobSetContentType::new(self.get_svc(), lob, content_type).await
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
    pub async fn open(&self) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobOpen::new(self.get_svc(), lob, OCI_LOB_READWRITE).await?;
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
    pub async fn trim(&self, new_len: usize) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobTrim::new(self.get_svc(), lob, new_len).await
    }

    async fn write_piece(&self, piece: u8, offset: usize, cs_form: u8, data: &[u8]) -> Result<(usize,usize)> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobWrite::new(self.get_svc(), lob, piece, cs_form, offset, data).await
    }

    async fn append_piece(&self, piece: u8, cs_form: u8, data: &[u8]) -> Result<(usize,usize)> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobWriteAppend::new(self.get_svc(), lob, piece, cs_form, data).await
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
    pub async fn temp(session: &'a Session<'a>, csform: CharSetForm, cache: Cache) -> Result<LOB<'a,OCICLobLocator>> {
        let locator = Descriptor::new(session)?;
        futures::LobCreateTemporary::new(session.get_svc(), &locator, OCI_TEMP_CLOB, csform as u8, cache as u8).await?;
        Ok(Self::make_temp(locator, session))
    }

    /**
    Writes a buffer into a LOB.

    # Parameters

    * `offset` - the absolute offset (in number of characters) from the beginning of the LOB,
    * `text` - slice of text to be written into this LOB.

    # Returns

    The number of characters written to the database.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No).await?;
    let text = "tête-à-tête";
    assert_eq!(text.len(), 14); // byte count

    let written = lob.write(4, text).await?;

    // Note that auto inserted spaces at 0..4 are not included
    // in the number of characters written
    assert_eq!(written, 11);    // char count

    // Note that initially the CLOB was empty, so writing at offset 4
    // inserted 4 spaces before the text we were writing:
    let lob_len_in_chars = lob.len().await?;
    let mut lob_content = String::new();
    let num_chars_read = lob.read(0, lob_len_in_chars, &mut lob_content).await?;
    assert_eq!(num_chars_read, 15);
    assert_eq!(lob_content, "    tête-à-tête");
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn write(&self, offset: usize, text: &str) -> Result<usize> {
        let cs_form = self.charset_form()? as u8;
        let (_, char_count) = self.write_piece(OCI_ONE_PIECE, offset, cs_form, text.as_bytes()).await?;
        Ok(char_count)
    }

    /**
    Writes data starting at the end of a LOB.

    # Parameters

    * `text` - the text to be written into this LOB.

    # Returns

    The number of characters written to the database.

    # Example

    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No).await?;

    let written = lob.append("Hello, World!").await?;

    assert_eq!(written, 13);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn append(&self, text: &str) -> Result<usize> {
        let cs_form = self.charset_form()? as u8;
        let (_, char_count) = self.append_piece(OCI_ONE_PIECE, cs_form, text.as_bytes()).await?;
        Ok(char_count)
    }

    /**
    Reads specified number of characters from this LOB, appending them to `buf`.

    # Parameters

    * `offset` - Offset in characters from the start of the LOB.
    * `len` - The number of characters to read
    * `out` - The output buffer

    # Returns

    The total number of characters read.

    # Example
    ```
    use sibyl::{CLOB, Cache, CharSetForm};

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let lob = CLOB::temp(&session, CharSetForm::Implicit, Cache::No).await?;
    lob.write(4, "tête-à-tête").await?;

    let mut lob_content = String::new();
    // There are 15 characters in the LOB - 11 from the text we
    // inserted and 4 padding spaced at 0..4.
    // We can request to read more than that. LOB will stop at the
    // end of its value and return the actual number of characters
    // that were read.
    let num_chars_read = lob.read(0, 100, &mut lob_content).await?;
    assert_eq!(num_chars_read, 15);
    assert_eq!(lob_content, "    tête-à-tête");
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn read(&self, mut offset: usize, len: usize, out: &mut String) -> Result<usize> {
        offset += 1;

        let space_available = out.capacity() - out.len();
        if len > space_available {
            out.reserve(len - space_available);
        }

        let stmt = self.session.prepare("BEGIN DBMS_LOB.READ(:LOC, :AMT, :POS, :DATA); END;").await?;

        let mut buf = String::with_capacity(32768);
        let mut remainder = len;
        while remainder > 0 {
            let mut amount = std::cmp::min(remainder, 32767);
            let res = stmt.execute((self, &mut amount, offset, &mut buf)).await;
            match res {
                Ok(num_rows) if num_rows == 0 => {
                    break;
                }
                Ok(_) => {
                    offset += amount;
                    remainder -= amount;
                    out.push_str(&buf);
                    buf.clear();
                },
                Err(Error::Oracle(NO_DATA_FOUND,_)) => {
                    break;
                },
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Ok( offset - 1 )
    }
}

impl<'a> LOB<'a,OCIBLobLocator> {
    /**
    Creates an empty temporary BLOB and its corresponding index in the user's temporary tablespace.

    # Parameters

    - `cache` - Indicates whether the temporary LOB should be read into the cache.

    The temporary LOB is freed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub async fn temp(session: &'a Session<'a>, cache: Cache) -> Result<LOB<'a,OCIBLobLocator>> {
        let locator = Descriptor::new(session)?;
       futures::LobCreateTemporary::new(session.get_svc(), &locator, OCI_TEMP_BLOB, 0u8, cache as u8).await?;
        Ok(Self::make_temp(locator, session))
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

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let lob = BLOB::temp(&session, Cache::No).await?;

    let written = lob.write(3, "Hello, World!".as_bytes()).await?;

    assert_eq!(written, 13);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn write(&self, offset: usize, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.write_piece(OCI_ONE_PIECE, offset, 0, data).await?;
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

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let lob = BLOB::temp(&session, Cache::No).await?;

    let written = lob.append("Hello, World!".as_bytes()).await?;

    assert_eq!(13, written);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn append(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.append_piece(OCI_ONE_PIECE, 0, data).await?;
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

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "modem.jpg")?;
    assert!(file.file_exists().await?);
    let file_len = file.len().await?;
    file.open_readonly().await?;
    let lob = BLOB::temp(&session, Cache::No).await?;
    lob.load_from_file(&file, 0, file_len, 0).await?;
    assert_eq!(lob.len().await?, file_len);

    let mut data = Vec::new();
    let num_read = lob.read(0, file_len, &mut data).await?;

    assert_eq!(num_read, file_len);
    assert_eq!(data.len(), file_len);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn read(&self, mut offset: usize, len: usize, out: &mut Vec<u8>) -> Result<usize> {
        offset += 1;

        let space_available = out.capacity() - out.len();
        if len > space_available {
            out.reserve(len - space_available);
        }
        let buf_ptr = out.as_mut_ptr();

        let stmt = self.session.prepare("BEGIN DBMS_LOB.READ(:LOC, :AMT, :POS, :DATA); END;").await?;

        let mut remainder = len;
        while remainder > 0 {
            let piece_ptr = unsafe { buf_ptr.add(out.len()) };
            let mut piece_len = std::cmp::min(remainder, 32767);
            let mut piece = unsafe { std::slice::from_raw_parts_mut(piece_ptr, piece_len) };

            let res = stmt.execute((self, &mut piece_len, offset, &mut piece)).await;
            match res {
                Ok(num_rows) if num_rows == 0 => {
                    break;
                }
                Ok(_) => {
                    offset += piece_len;
                    remainder -= piece_len;
                    unsafe {
                        out.set_len(out.len() + piece_len);
                    }
                },
                Err(Error::Oracle(NO_DATA_FOUND,_)) => {
                    break;
                },
                Err(err) => {
                    return Err(err);
                }
            }
        }
        let num_read = offset - 1;
        Ok( num_read )
    }
}

impl<'a> LOB<'a,OCIBFileLocator> {
    pub(crate) fn make(locator: Descriptor<OCIBFileLocator>, session: &'a Session) -> Self {
        Self::make_new(locator, session)
    }

    /**
    Closes a previously opened BFILE.

    No error is returned if the BFILE exists but is not opened.

    This function is only meaningful the first time it is called for a particular
    BFILE locator. Subsequent calls to this function using the same BFILE locator
    have no effect.
    */
    pub async fn close_file(&self) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobFileClose::new(self.get_svc(), lob).await?;
        self.inner.status_flags.fetch_and(!LOB_FILE_IS_OPEN, Ordering::Relaxed);
        Ok(())
    }

    /**
    Tests to see if the BFILE exists on the server's operating system.

    # Example

    ```
    use sibyl::BFile;

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "formatted_doc.txt")?;

    assert!(file.file_exists().await?);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn file_exists(&self) -> Result<bool> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobFileExists::new(self.get_svc(), lob).await
    }

    /**
    Returns `true` if the BFILE was opened using this particular locator.
    However, a different locator may have the file open. Openness is associated
    with a particular locator.

    # Example

    ```
    use sibyl::BFile;

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
    assert!(!file.is_file_open().await?);

    file.open_file().await?;
    assert!(file.is_file_open().await?);

    file.close_file().await?;
    assert!(!file.is_file_open().await?);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn is_file_open(&self) -> Result<bool> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobFileIsOpen::new(self.get_svc(), lob).await
    }

    /**
    Opens a BFILE on the file system of the server. The BFILE can only be opened
    for read-only access. BFILEs can not be written through Oracle Database.

    This function is only meaningful the first time it is called for a particular
    BFILE locator. Subsequent calls to this function using the same BFILE locator
    have no effect.

    # Example

    ```
    use sibyl::BFile;

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;

    file.open_file().await?;

    assert!(file.is_file_open().await?);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn open_file(&self) -> Result<()> {
        let lob: &OCILobLocator = self.as_ref();
        futures::LobFileOpen::new(self.get_svc(), lob).await?;
        self.inner.status_flags.fetch_or(LOB_FILE_IS_OPEN, Ordering::Relaxed);
        Ok(())
    }

    /**
    Reads specified number of bytes from this LOB, appending them to `buf`.

    # Parameters

    - `offset` - The absolute offset (in bytes) from the beginning of the LOB value.
    - `len` - The total maximum number of bytes to read.
    - `buf` - The output buffer.

    # Returns

    The number of bytes that were read and appended to `buf`.

    # Example

    ```
    use sibyl::BFile;

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let file = BFile::new(&session)?;
    file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
    let file_len = file.len().await?;
    file.open_file().await?;

    let mut data = Vec::new();
    let num_read = file.read(0, file_len, &mut data).await?;

    assert_eq!(num_read, file_len);
    assert_eq!(data, [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21]);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn read(&self, offset: usize, len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        let (_, byte_count, _) = self.read_piece(OCI_ONE_PIECE, len, offset, len, 0, 0, buf).await?;
        Ok(byte_count)
    }
}
