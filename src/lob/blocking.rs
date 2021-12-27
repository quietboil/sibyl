use super::*;
use crate::*;
use crate::oci::*;
use std::sync::atomic::Ordering;

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

impl<T> LobInner<T> where T: DescriptorType<OCIType=OCILobLocator> {
    pub(super) fn clone(&self) -> Result<Self> {
        let mut locator = Descriptor::<T>::new(self)?;
        oci::lob_locator_assign(self.as_ref(), self.as_ref(), self.as_ref(), locator.as_mut_ptr())?;
        let svc = self.svc.clone();
        Ok(Self { locator, svc } )
    }
}

impl<'a,T> LOB<'a,T> where T: DescriptorType<OCIType=OCILobLocator> {
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
            DECLARE
                row_id ROWID;
            BEGIN
                INSERT INTO test_lobs (text) VALUES (Empty_Clob()) RETURNING rowid into row_id;
                SELECT text INTO :TXT FROM test_lobs WHERE rowid = row_id FOR UPDATE;
            END;
        ")?;
        let mut lob1 = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut lob1 ])?;

        let text = [
            "To see a World in a Grain of Sand\n",
            "And a Heaven in a Wild Flower\n",
            "Hold Infinity in the palm of your hand\n",
            "And Eternity in an hour\n"
        ];

        lob1.open()?;
        let written = lob1.append(text[0])?;
        assert_eq!(written, text[0].len());
        assert_eq!(lob1.len()?, text[0].len());

        let lob2 = lob1.clone()?;
        // Note that clone also makes lob2 open (as lob1 was open).
        assert!(lob2.is_open()?, "lob2 is already open");
        // They both will be auto-closed when they go out of scope
        // at end of this test.

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

        // Let's save `lob2` now. It is still only aware of `text[0]` and `text[1]` fragments.
        let stmt = conn.prepare("
            insert into test_lobs (text) values (:new_lob) returning id, text into :id, :saved_text
        ")?;
        let mut saved_lob_id : usize = 0;
        let mut saved_lob = CLOB::new(&conn)?;
        stmt.execute_into(&[ &lob2 ], &mut [ &mut saved_lob_id, &mut saved_lob ])?;

        // And thus `saved_lob` locator points to a distinct LOB value ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... which has only `text[0]` and `text[1]`
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
        Ok(Self { inner, chunk_size: AtomicU32::new(chunk_size), ..*self })
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
        use sibyl::{ CLOB, RowID };

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
            INSERT INTO test_lobs (text) VALUES (Empty_Clob()) RETURNING rowid INTO :row_id
        ")?;
        let mut rowid = RowID::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut rowid ])?;

        // must lock LOB's row before writing into it
        let stmt = conn.prepare("
            SELECT text FROM test_lobs WHERE rowid = :row_id FOR UPDATE
        ")?;
        let rows = stmt.query(&[ &rowid ])?;
        let row = rows.next()?.expect("a single row");
        let mut lob : CLOB = row.get(0)?.expect("CLOB for writing");

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
        use sibyl::{CLOB, Cache, CharSetForm, BFile, RowID};

        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
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
            DECLARE
                row_id ROWID;
            BEGIN
                INSERT INTO test_lobs (text) VALUES (Empty_Clob()) RETURNING rowid into row_id;
                SELECT text INTO :TXT FROM test_lobs WHERE rowid = row_id FOR UPDATE;
            END;
        ")?;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut lob ])?;

        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        let file_len = file.len()?;

        lob.open()?;
        file.open_file()?;
        lob.load_from_file(&file, 0, file_len, 0)?;
        file.close_file()?;

        let lob_len = lob.len()?;
        assert_eq!(lob_len, 13);

        let mut text = String::new();
        lob.read(0, lob_len, &mut text)?;

        assert_eq!(text, "Hello, World!");

        file.set_file_name("MEDIA_DIR", "hello_world_cyrillic.txt")?;
        let file_len = file.len()?;

        file.open_file()?;
        lob.load_from_file(&file, 0, file_len, 0)?;
        file.close()?;

        let lob_len = lob.len()?;
        assert_eq!(lob_len, 16);

        text.clear();
        lob.read(0, lob_len, &mut text)?;

        assert_eq!(text, "Здравствуй, Мир!");

        file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
        let file_len = file.len()?;

        lob.trim(0)?;
        file.open_file()?;
        lob.load_from_file(&file, 0, file_len, 0)?;
        file.close()?;

        let lob_len = lob.len()?;
        // Note that Oracle encoded 4 symbols (see below) into 8 characters
        assert_eq!(lob_len, 8);

        text.clear();
        // The reading stops at the end of LOB value if we request more
        // characters than the LOB contains
        let num_read = lob.read(0, 100, &mut text)?;
        assert_eq!(num_read, 8);

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
}

impl<'a> LOB<'a,OCICLobLocator> {
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
        let cs_form = self.charset_form()? as u8;
        let (_, char_count) = self.write_piece(OCI_FIRST_PIECE, offset, cs_form, text.as_bytes())?;
        Ok(char_count)
    }

    /**
        Continues piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn write_next(&self, text: &str) -> Result<usize> {
        let (_, char_count) = self.write_piece(OCI_NEXT_PIECE, 0, 0, text.as_bytes())?;
        Ok(char_count)
    }

    /**
        Terminates piece-wise writing into a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn write_last(&self, text: &str) -> Result<usize> {
        let (_, char_count) = self.write_piece(OCI_LAST_PIECE, 0, 0, text.as_bytes())?;
        Ok(char_count)
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
        let cs_form = self.charset_form()? as u8;
        let (_, char_count) = self.append_piece(OCI_FIRST_PIECE, cs_form, text.as_bytes())?;
        Ok(char_count)
    }

    /**
        Continues piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn append_next(&self, text: &str) -> Result<usize> {
        let (_, char_count) = self.append_piece(OCI_NEXT_PIECE, 0, text.as_bytes())?;
        Ok(char_count)
    }

    /**
        Terminates piece-wise writing at the end of a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn append_last(&self, text: &str) -> Result<usize> {
        let (_, char_count) = self.append_piece(OCI_LAST_PIECE, 0, text.as_bytes())?;
        Ok(char_count)
    }

    /**
        Reads specified number of characters from this LOB, appending them to `buf`.
        If successful, this function will return the total number of bytes read.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
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
        let fragment = vec![42u8;chunk_size];
        let fragment_as_text = String::from_utf8(fragment)?;
        let written = lob.append_first(&fragment_as_text)?;
        let mut total_written = written;
        for _i in 0..10 {
            let written = lob.append_next(&fragment_as_text)?;
            total_written += written;
        }
        let written = lob.append_last(&fragment_as_text)?;
        total_written += written;
        assert_eq!(chunk_size * 12, total_written);

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
        Continues piece-wise reading of the fragment started by `read_first`, returning a flag which indicates
        whether there are more pieces to read until the requested fragment is complete. Application should keep
        calling `read_next` until it returns `false`.
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
        let (byte_count, _) = self.write_piece(OCI_ONE_PIECE, offset, 0, data)?;
        Ok(byte_count)
    }

    /**
        Starts piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::{BLOB, RowID};

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
            insert into test_lobs (data) values (empty_blob()) returning rowid into :row_id
        ")?;
        let mut rowid = RowID::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut rowid ])?;

        // must lock LOB's row before writing the LOB
        let stmt = conn.prepare("
            SELECT data FROM test_lobs WHERE rowid = :row_id FOR UPDATE
        ")?;
        let rows = stmt.query(&[ &rowid ])?;
        let row = rows.next()?.expect("a single row");
        let mut lob : BLOB = row.get(0)?.expect("BLOB for writing");

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
        let (byte_count, _) = self.write_piece(OCI_FIRST_PIECE, offset, 0, data)?;
        Ok(byte_count)
    }

    /**
        Continues piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn write_next(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.write_piece(OCI_NEXT_PIECE, 0, 0, data)?;
        Ok(byte_count)
    }

    /**
        Terminates piece-wise writing into a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn write_last(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.write_piece(OCI_LAST_PIECE, 0, 0, data)?;
        Ok(byte_count)
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
        let (byte_count, _) = self.append_piece(OCI_ONE_PIECE, 0, data)?;
        Ok(byte_count)
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
        let (byte_count, _) = self.append_piece(OCI_FIRST_PIECE, 0, data)?;
        Ok(byte_count)
    }

    /**
        Continues piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn append_next(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.append_piece(OCI_NEXT_PIECE, 0, data)?;
        Ok(byte_count)
    }

    /**
        Terminates piece-wise writing at the end of a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn append_last(&self, data: &[u8]) -> Result<usize> {
        let (byte_count, _) = self.append_piece(OCI_LAST_PIECE, 0, data)?;
        Ok(byte_count)
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

        let mut data = Vec::new();
        let num_read = lob.read(0, file_len, &mut data)?;

        assert_eq!(num_read, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read(&self, offset: usize, len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        let (_, byte_count, _) = self.read_piece(OCI_ONE_PIECE, len, offset, len, 0, 0, buf)?;
        Ok( byte_count )
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
        let mut data = Vec::new();
        let piece_size = chunk_size;
        let offset = 0;
        let length = file_len;
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
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
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

        let mut data = Vec::new();
        let num_read = file.read(0, file_len, &mut data)?;

        assert_eq!(num_read, file_len);
        assert_eq!(data, [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21]);
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

        let mut data = Vec::new();
        let piece_size = 8192;
        let offset = 0;
        let length = file_len;
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
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
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
    */
    pub fn read_next(&self, piece_size: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        let (has_more, byte_count, _) = self.read_piece(OCI_NEXT_PIECE, piece_size, 0, 0, 0, 0, buf)?;
        *num_read = byte_count;
        Ok(has_more)
    }
}

impl LOB<'_,OCICLobLocator> {
    /// Debug helper that fetches first 50 (at most) bytes of CLOB content
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
        let mut buf = Vec::new();
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
        let mut buf = Vec::new();
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
