//! Nonblocking mode LOB methods.

use super::{LOB, InternalLob, LobInner};
use crate::{Result, BFile, oci::{self, *}, env::Env, conn::Connection, task};
use std::marker::PhantomData;


impl<T> Drop for LobInner<T>
    where T: DescriptorType<OCIType=OCILobLocator> + 'static
{
    fn drop(&mut self) {
        let conn = self.conn.clone();
        let locator = Descriptor::take_over(&mut self.locator);

        let async_drop = async move {
            let conn = conn;
            let locator = locator;
            let loc_ptr = locator.get_ptr();
            let svc_ptr = Ptr::new(conn.svc_ptr());
            let err_ptr = Ptr::new(conn.err_ptr());

            match oci::futures::LobIsOpen::new(svc_ptr.clone(), err_ptr.clone(), loc_ptr.clone()).await {
                Ok(is_open) if is_open => {
                    let _res = oci::futures::LobClose::new(svc_ptr.clone(), err_ptr.clone(), loc_ptr.clone()).await;
                },
                _ => {}
            };
            match oci::futures::LobIsTemporary::new(svc_ptr.clone(), err_ptr.clone(), loc_ptr.clone()).await {
                Ok(is_temp) if is_temp => {
                    let _res = oci::futures::LobFreeTemporary::new(svc_ptr.clone(), err_ptr.clone(), loc_ptr.clone()).await;
                },
                _ => {}
            };
        };
        task::spawn(async_drop);
    }
}

impl<T> LobInner<T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    fn get_ptr(&self) -> Ptr<OCILobLocator> {
        Ptr::new(self.locator.get())
    }

    fn get_err_ptr(&self) -> Ptr<OCIError> {
        Ptr::new(self.conn.err_ptr())
    }

    fn get_env_ptr(&self) -> Ptr<OCIEnv> {
        Ptr::new(self.conn.env_ptr())
    }

    fn get_svc_ptr(&self) -> Ptr<OCISvcCtx> {
        Ptr::new(self.conn.svc_ptr())
    }

    async fn clone(&self) -> Result<Self> {
        let mut locator = Descriptor::new(self.env_ptr())?;
        let new_locator_ptr = oci::futures::LobLocatorAssign::new(
            self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), locator.get_ptr()
        ).await?;
        locator.replace(new_locator_ptr);
        let conn = self.conn.clone();
        Ok(Self { locator, conn } )
    }
}

impl<'a,T> LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    fn get_ptr(&self) -> Ptr<OCILobLocator> {
        self.inner.get_ptr()
    }

    fn get_err_ptr(&self) -> Ptr<OCIError> {
        self.inner.get_err_ptr()
    }

    fn get_env_ptr(&self) -> Ptr<OCIEnv> {
        self.inner.get_env_ptr()
    }

    fn get_svc_ptr(&self) -> Ptr<OCISvcCtx> {
        self.inner.get_svc_ptr()
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
        use sibyl::CLOB;

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
        let stmt = conn.prepare("
            insert into test_lobs (text) values (empty_clob()) returning id, text into :id, :text
        ").await?;
        let mut id1 : usize = 0;
        let mut lob1 = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id1, &mut lob1 ]).await?;

        let text = [
            "To see a World in a Grain of Sand\n",
            "And a Heaven in a Wild Flower\n",
            "Hold Infinity in the palm of your hand\n",
            "And Eternity in an hour\n"
        ];

        let written = lob1.append(text[0]).await?;
        assert_eq!(written, text[0].len());
        assert_eq!(lob1.len().await?, text[0].len());

        let lob2 = lob1.clone().await?;

        // They point to the same value and at this time they are completely in sync
        assert!(lob2.is_equal(&lob1)?);

        let written = lob2.append(text[1]).await?;
        assert_eq!(written, text[1].len());

        // Now they are out of sync
        assert!(!lob2.is_equal(&lob1)?);
        // At this time `lob1` is not yet aware that `lob2` added more text the LOB they "share".
        assert_eq!(lob2.len().await?, text[0].len() + text[1].len());
        assert_eq!(lob1.len().await?, text[0].len());

        let written = lob1.append(text[2]).await?;
        assert_eq!(written, text[2].len());

        // Now, after writing, `lob1` has caught up with `lob2` prior writing and added more text
        // on its own. But now it's `lob2` turn to lag behind and not be aware of the added text.
        assert_eq!(lob1.len().await?, text[0].len() + text[1].len() + text[2].len());
        assert_eq!(lob2.len().await?, text[0].len() + text[1].len());

        // Let's save `lob2` now. It is still only knows about `text[0]` and `text[1]` fragments.
        let stmt = conn.prepare("
            insert into test_lobs (text) values (:text) returning id, text into :id, :saved_text
        ").await?;
        let mut saved_lob_id : usize = 0;
        let mut saved_lob = CLOB::new(&conn)?;
        stmt.execute_into(&[ &lob2 ], &mut [ &mut saved_lob_id, &mut saved_lob ]).await?;

        // And thus `saved_lob` locator points to a distinct LOB value ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... which has only the `text[0]` and `text[1]`
        assert_eq!(saved_lob.len().await?, text[0].len() + text[1].len());

        let written = lob2.append(text[3]).await?;
        assert_eq!(written, text[3].len());

        assert_eq!(lob2.len().await?, text[0].len() + text[1].len() + text[2].len() + text[3].len());
        assert_eq!(lob1.len().await?, text[0].len() + text[1].len() + text[2].len());

        // As `saved_lob` points to the enturely different LOB ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... it is not affected by `lob1` and `lob2` additions.
        assert_eq!(saved_lob.len().await?, text[0].len() + text[1].len());
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn clone(&'a self) -> Result<LOB<'a,T>> {
        let inner = self.inner.clone().await?;
        Ok(Self { inner, phantom_conn: PhantomData } )
    }

    /**
        Closes a previously opened internal or external LOB.

        Closing a LOB requires a round-trip to the server for both internal and external LOBs.
        For internal LOBs, `close` triggers other code that relies on the close call and for external
        LOBs (BFILEs), close actually closes the server-side operating system file.

        It is not required to close a LOB explicitly as it will be automatically closed when Rust drops
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
        use sibyl::CLOB;

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
        let stmt = conn.prepare("
            insert into test_lobs (text) values (empty_clob()) returning id, text into :id, :text
        ").await?;
        let mut id : usize = 0;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id, &mut lob ]).await?;

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
        oci::futures::LobClose::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr()).await
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
    pub async fn len(&self) -> Result<usize> {
        let len = oci::futures::LobGetLength::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr()).await?;
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
        oci::futures::LobIsOpen::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr()).await
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
        oci::futures::LobOpen::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), OCI_LOB_READONLY).await
    }

    /**
        For CLOBs and NCLOBs, if you do not pass `char_len`, then `char_len` is calculated internally as
        `byte_len/max char width`, so if max char width is 4, `char_len` is calculated as `byte_len/4`.

        **Note** that OCILobRead2() does not calculate how many bytes are required for each character. Instead, OCILobRead2()
        fetches the number of characters that in the worst case can fit in `byte_len`.
    */
    async fn read_piece(&self, piece: u8, piece_size: usize, offset: usize, byte_len: &mut usize, char_len: usize, char_form: u8, buf: &mut Vec<u8>) -> Result<bool> {
        buf.reserve(piece_size);
        let (res, read_len) = oci::futures::LobRead::new(
            self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), piece, piece_size, offset, *byte_len, char_len, char_form, buf
        ).await?;
        unsafe {
            buf.set_len(buf.len() + read_len);
        }
        *byte_len = read_len;
        Ok( res == OCI_NEED_DATA )
    }
}

impl<'a, T> LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator> + InternalLob
{
    /**
        Appends another LOB value at the end of this LOB.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
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
        let lob1 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        lob1.append(text1).await?;
        assert_eq!(lob1.len().await?, text1.len());

        let lob2 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        lob2.append(text2).await?;
        // Cannot use `len` shortcut with `text2` because of the `RIGHT SINGLE QUOTATION MARK` in it
        assert_eq!(lob2.len().await?, text2.chars().count());

        lob1.append_lob(&lob2).await?;
        assert_eq!(lob1.len().await?, text1.len() + text2.chars().count());
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn append_lob(&self, lob: &Self) -> Result<()> {
        oci::futures::LobAppend::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), lob.get_ptr()).await
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

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
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
        let lob1 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        lob1.append(text).await?;

        let lost_text = "
        Now timely sing, ere the rude Bird of Hate
        Foretell my hopeles doom, in som Grove ny:
        As thou from yeer to yeer hast sung too late
        ";
        let lob2 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        lob2.append(lost_text).await?;

        // Set source offset to 1 to skip leading \n:
        lob1.copy(&lob2, 1, 131, 364).await?;

        // Read back the overwriten fragment.
        // Start with the leading \n, to make comparison easier:
        let mut fragment = String::with_capacity(135*4);
        lob1.read(363, 132, &mut fragment).await?;

        assert_eq!(fragment, lost_text);

        // ASCII only. That means we can use `len` as a "shortcut".
        let text_len = lob1.len().await?;
        // Recall that the buffer needs to be allocated for the worst case
        let mut sonnet = String::with_capacity(text_len*4);
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
        oci::futures::LobCopy::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), src.get_ptr(), offset, src_offset, amount).await
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

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        assert!(file.file_exists().await?, "hello_world.txt");

        // To load this file into a temporary CLOB Oracle expects it to be UTF-16 BE encoded.

        // File must be opened first
        file.open_readonly().await?;
        assert!(file.is_open().await?);

        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        lob.load_from_file(&file, 0, 28, 0).await?;
        file.close().await?;

        assert_eq!(13, lob.len().await?);

        let mut text = String::with_capacity(64);
        // Even though we are asking for more characters than the LOB has it should only return what it has
        lob.read(0, 20, &mut text).await?;

        assert_eq!(text, "Hello, World!");

        // Do it again in Cyrillic. No need to erase it. We'll just load new content over the old one
        file.set_file_name("MEDIA_DIR", "hello_world_cyrillic.txt")?;
        assert!(file.file_exists().await?, "hello_world_cyrillic.txt");
        file.open_readonly().await?;
        assert!(file.is_open().await?);

        lob.load_from_file(&file, 0, 34, 0).await?;
        file.close().await?;

        assert_eq!(16, lob.len().await?);

        text.clear();
        lob.read(0, 20, &mut text).await?;

        assert_eq!(text, "Здравствуй, Мир!");

        // And the supplemental plane
        file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
        assert!(file.file_exists().await?, "hello_supplemental.txt");
        file.open_readonly().await?;
        assert!(file.is_open().await?);

        // As the text in the "cyrillic" test was longer...
        lob.trim(0).await?;
        lob.load_from_file(&file, 0, 18, 0).await?;
        file.close().await?;

        // Note that "char count" for all characters used in this test is defined in Unicode as 2.
        assert_eq!(8, lob.len().await?);

        text.clear();
        lob.read(0, 20, &mut text).await?;

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
        # ").await?;
        # stmt.execute(&[]).await?;
        let stmt = conn.prepare("
            insert into test_lobs (text) values (empty_clob()) returning text into :text
        ").await?;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut lob ]).await?;

        lob.open().await?;

        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        assert!(file.file_exists().await?, "hello_world.txt");
        file.open_readonly().await?;
        assert!(file.is_open().await?);

        lob.load_from_file(&file, 0, 28, 0).await?;
        file.close().await?;

        assert_eq!(13, lob.len().await?);

        text.clear();
        lob.read(0, 20, &mut text).await?;

        assert_eq!(text, "Hello, World!");

        file.set_file_name("MEDIA_DIR", "hello_world_cyrillic.txt")?;
        assert!(file.file_exists().await?, "hello_world_cyrillic.txt");
        file.open_readonly().await?;
        assert!(file.is_open().await?);

        lob.load_from_file(&file, 0, 34, 0).await?;
        file.close().await?;

        assert_eq!(16, lob.len().await?);

        text.clear();
        lob.read(0, 20, &mut text).await?;

        assert_eq!(text, "Здравствуй, Мир!");

        file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
        assert!(file.file_exists().await?, "hello_supplemental.txt");
        file.open_readonly().await?;
        assert!(file.is_open().await?);

        lob.trim(0).await?;
        lob.load_from_file(&file, 0, 18, 0).await?;
        file.close().await?;

        assert_eq!(8, lob.len().await?);

        text.clear();
        lob.read(0, 20, &mut text).await?;

        assert_eq!(text, "🚲🛠📬🎓");
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn load_from_file(&self, src: &'a BFile<'a>, src_offset: usize, amount: usize, offset: usize) -> Result<()> {
        oci::futures::LobLoadFromFile::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), src.get_ptr(), src_offset, amount , offset).await
    }

    /**
        Erases a specified portion of the internal LOB data starting at a specified offset.
        Returns the actual number of characters or bytes erased.

        For BLOBs, erasing means that zero-byte fillers overwrite the existing LOB value.
        For CLOBs, erasing means that spaces overwrite the existing LOB value.
    */
    pub async fn erase(&self, offset: usize, amount: usize) -> Result<usize> {
        let count = oci::futures::LobErase::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), offset, amount).await?;
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
        let size = oci::futures::LobGetChunkSize::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr()).await?;
        Ok( size as usize )
    }

    /**
        Returns he user-specified content type string for the data in a SecureFile, if set.

        This function only works on SecureFiles.
    */
    pub async fn content_type(&self) -> Result<String> {
        oci::futures::LobGetContentType::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_env_ptr(), self.get_ptr()).await
    }

    /**
        Sets a content type string for the data in the SecureFile to something that can be used by an application.

        This function only works on SecureFiles.
    */
    pub async    fn set_content_type(&self, content_type: &str) -> Result<()> {
        oci::futures::LobSetContentType::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_env_ptr(), self.get_ptr(), content_type).await
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
        oci::futures::LobOpen::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), OCI_LOB_READWRITE).await
    }

    /**
        Truncates the LOB value to a shorter length.

        For character LOBs, `new_len` is the number of characters; for binary LOBs and BFILEs, it is the number
        of bytes in the LOB.
    */
    pub async fn trim(&self, new_len: usize) -> Result<()> {
        oci::futures::LobTrim::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), new_len).await
    }

    async fn write_piece(&self, piece: u8, offset: usize, data: &[u8]) -> Result<usize> {
        let charset_form = self.charset_form()? as u8;
        oci::futures::LobWrite::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), piece, charset_form, offset, data).await
    }

    async fn append_piece(&self, piece: u8, data: &[u8]) -> Result<usize> {
        let charset_form = self.charset_form()? as u8;
        oci::futures::LobWriteAppend::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr(), piece, charset_form, data).await
    }
}

impl<'a> LOB<'a,OCICLobLocator> {
    /**
        Creates an empty temporary CLOB or NCLOB.

        The LOB is feed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub async fn temp(conn: &'a Connection<'a>, csform: CharSetForm, cache: Cache) -> Result<LOB<'a,OCICLobLocator>> {
        let locator = Descriptor::new(conn.env_ptr())?;
        oci::futures::LobCreateTemporary::new(conn.get_svc_ptr(), conn.get_err_ptr(), locator.get_ptr(), OCI_TEMP_CLOB, csform as u8, cache as u8).await?;
        Ok(Self::make(locator, conn))
    }

    /**
        Writes a buffer into a LOB.

        Returns the number of bytes written to the database.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        let written = lob.write(3, "Hello, World!").await?;
        assert_eq!(13, written);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn write(&self, offset: usize, text: &str) -> Result<usize> {
        self.write_piece(OCI_ONE_PIECE, offset, text.as_bytes()).await
    }

    /**
        Starts piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::{ CLOB, CharSetForm, Cache };

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        lob.open().await?;
        let chunk_size = lob.chunk_size().await?;
        let data = vec![42u8;chunk_size];
        let text = std::str::from_utf8(&data)?;

        let written = lob.write_first(0, text).await?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.write_next(text).await?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.write_last(text).await?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len().await?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(()) }).expect("Ok from async");
        ```
    */
    pub async fn write_first(&self, offset: usize, text: &str) -> Result<usize> {
        self.write_piece(OCI_FIRST_PIECE, offset, text.as_bytes()).await
    }

    /**
        Continues piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub async fn write_next(&self, text: &str) -> Result<usize> {
        self.write_piece(OCI_NEXT_PIECE, 0, text.as_bytes()).await
    }

    /**
        Terminates piece-wise writing into a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub async fn write_last(&self, text: &str) -> Result<usize> {
        self.write_piece(OCI_LAST_PIECE, 0, text.as_bytes()).await
    }

    /**
        Writes data starting at the end of a LOB.

        Returns the number of bytes written to the database.

        # Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        let written = lob.append("Hello, World!").await?;
        assert_eq!(13, written);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn append(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_ONE_PIECE, text.as_bytes()).await
    }

    /**
        Starts piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::{ CLOB, CharSetForm, Cache };

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        lob.open().await?;
        let chunk_size = lob.chunk_size().await?;
        let data = vec![42u8;chunk_size];
        let text = std::str::from_utf8(&data)?;

        let written = lob.append_first(text).await?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.append_next(text).await?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.append_last(text).await?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len().await?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(()) }).expect("Ok from async");
        ```
    */
    pub async fn append_first(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_FIRST_PIECE, text.as_bytes()).await
    }

    /**
        Continues piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub async fn append_next(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_NEXT_PIECE, text.as_bytes()).await
    }

    /**
        Terminates piece-wise writing at the end of a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub async fn append_last(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_LAST_PIECE, text.as_bytes()).await
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

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        let written = lob.write(3, "Hello, World!").await?;
        assert_eq!(13, written);

        let mut buf = String::with_capacity(16*4);
        let read = lob.read(0, 16, &mut buf).await?;

        assert_eq!(16, read);
        assert_eq!(buf, "   Hello, World!");
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn read(&self, offset: usize, len: usize, buf: &mut String) -> Result<usize> {
        let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        let mut num_read : usize = 0;
        self.read_piece(OCI_ONE_PIECE, len * 4, offset, &mut num_read, len, form, buf).await?;
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

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No).await?;
        let chunk_size = lob.chunk_size().await?;
        lob.open().await?;
        let fragment = vec![52u8;chunk_size];
        let fragment_as_text = unsafe { String::from_utf8_unchecked(fragment) };
        let mut written = lob.append_first(&fragment_as_text).await?;
        for _i in 0..10 {
            written += lob.append_next(&fragment_as_text).await?;
        }
        written += lob.append_last(&fragment_as_text).await?;

        assert_eq!(chunk_size * 12, written);

        let mut text = String::new();
        let mut text_len : usize = 0;
        let mut read_len : usize = 0;
        let piece_size = chunk_size;
        let offset = chunk_size * 2;
        let length = chunk_size * 5;
        let mut has_next = lob.read_first(piece_size, offset, length, &mut text, &mut text_len).await?;
        while has_next {
            has_next = lob.read_next(piece_size, &mut text, &mut read_len).await?;
            text_len += read_len;
        }
        assert_eq!(text_len, chunk_size * 5);
        assert_eq!(text_len, text.len());
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut String, num_read: &mut usize) -> Result<bool> {
        let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        *num_read = 0;
        self.read_piece(OCI_FIRST_PIECE, piece_size, offset, num_read, len, form, buf).await
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a flag which indicates
        whether there are more pieces to read until the requested fragment is complete. Application should keep
        calling `read_next` until it returns `false`.
    */
    pub async fn read_next(&self, piece_size: usize, buf: &mut String, num_read: &mut usize) -> Result<bool> {
        // let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, piece_size, 0, num_read, 0, 0, buf).await
    }
}

impl<'a> LOB<'a,OCIBLobLocator> {
    /**
        Creates an empty temporary BLOB.

        The LOB is feed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub async fn temp(conn: &'a Connection<'a>, cache: Cache) -> Result<LOB<'a,OCIBLobLocator>> {
        let locator = Descriptor::new(conn.env_ptr())?;
        oci::futures::LobCreateTemporary::new(conn.get_svc_ptr(), conn.get_err_ptr(), locator.get_ptr(), OCI_TEMP_BLOB, 0u8, cache as u8).await?;
        Ok(Self::make(locator, conn))
    }

    /**
        Writes a buffer into a LOB.

        Returns the number of bytes written to the database.

        # Example
        ```
        use sibyl::{BLOB, Cache};

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let lob = BLOB::temp(&conn, Cache::No).await?;
        let written = lob.write(3, "Hello, World!".as_bytes()).await?;
        assert_eq!(13, written);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn write(&self, offset: usize, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_ONE_PIECE, offset, data).await
    }

    /**
        Starts piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::BLOB;

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
        let stmt = conn.prepare("
            insert into test_lobs (data) values (empty_blob()) returning data into :data
        ").await?;
        let mut lob = BLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut lob ]).await?;
        lob.open().await?;
        let chunk_size = lob.chunk_size().await?;
        let data = vec![42u8;chunk_size];

        let written = lob.write_first(0, &data).await?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.write_next(&data).await?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.write_last(&data).await?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len().await?, chunk_size * 10);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn write_first(&self, offset: usize, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_FIRST_PIECE, offset, data).await
    }

    /**
        Continues piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub async fn write_next(&self, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_NEXT_PIECE, 0, data).await
    }

    /**
        Terminates piece-wise writing into a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub async fn write_last(&self, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_LAST_PIECE, 0, data).await
    }

    /**
        Writes data starting at the end of a LOB.

        Returns the number of bytes written to the database.

        # Example
        ```
        use sibyl::{BLOB, Cache};

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let lob = BLOB::temp(&conn, Cache::No).await?;
        let written = lob.append("Hello, World!".as_bytes()).await?;
        assert_eq!(13, written);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn append(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_ONE_PIECE, data).await
    }

    /**
        Starts piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        # Example
        ```
        use sibyl::{BLOB, Cache};

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let mut lob = BLOB::temp(&conn, Cache::No).await?;
        let chunk_size = lob.chunk_size().await?;
        let data = vec![165u8;chunk_size];

        let written = lob.append_first(&data).await?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.append_next(&data).await?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.append_last(&data).await?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len().await?, chunk_size * 10);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn append_first(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_FIRST_PIECE, data).await
    }

    /**
        Continues piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub async fn append_next(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_NEXT_PIECE, data).await
    }

    /**
        Terminates piece-wise writing at the end of a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub async fn append_last(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_LAST_PIECE, data).await
    }

    /**
        Reads specified number of bytes from this LOB, appending them to `buf`.
        If successful, this function returns the number of bytes that were read and appended to `buf`.

        # Example

        ```
        use sibyl::{BLOB, Cache, BFile};

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "modem.jpg")?;
        assert!(file.file_exists().await?);
        let file_len = file.len().await?;
        file.open_readonly().await?;
        let lob = BLOB::temp(&conn, Cache::No).await?;
        lob.load_from_file(&file, 0, file_len, 0).await?;
        assert_eq!(lob.len().await?, file_len);

        let mut data = Vec::with_capacity(file_len);
        let num_read = lob.read(0, file_len, &mut data).await?;

        assert_eq!(num_read, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn read(&self, offset: usize, mut len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.read_piece(OCI_ONE_PIECE, len, offset, &mut len, 0, 0, buf).await?;
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

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "monitor.jpg")?;
        assert!(file.file_exists().await?);
        let file_len = file.len().await?;
        file.open_readonly().await?;
        let lob = BLOB::temp(&conn, Cache::No).await?;
        lob.load_from_file(&file, 0, file_len, 0).await?;
        assert_eq!(lob.len().await?, file_len);

        let chunk_size = lob.chunk_size().await?;
        let mut data = Vec::with_capacity(file_len);
        let mut data_len : usize = 0;
        let piece_size = chunk_size;
        let offset = 0;
        let length = file_len;
        let mut has_next = lob.read_first(piece_size, offset, length, &mut data, &mut data_len).await?;
        assert_eq!(data_len, chunk_size);
        assert_eq!(data.len(), data_len);
        while has_next {
            let mut bytes_read : usize = 0;
            has_next = lob.read_next(piece_size, &mut data, &mut bytes_read).await?;
            data_len += bytes_read;
            assert_eq!(data_len, data.len());
        }

        assert_eq!(data_len, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = len;
        self.read_piece(OCI_FIRST_PIECE, piece_size, offset, num_read, 0, 0, buf).await
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a tuple with 2 elements - the number of
        bytes read in the current piece and the flag which indicates whether there are more pieces to read until the requested
        fragment is complete. Application should keep calling `read_next` until "more data" flag becomes `false`.
    */
    pub async fn read_next(&self, piece_size: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, piece_size, 0, num_read, 0, 0, buf).await
    }
}

impl LOB<'_,OCIBFileLocator> {
    /**
        Closes a previously opened BFILE.

        No error is returned if the BFILE exists but is not opened.

        This function is only meaningful the first time it is called for a particular
        BFILE locator. Subsequent calls to this function using the same BFILE locator
        have no effect.
    */
    pub async fn close_file(&self) -> Result<()> {
        oci::futures::LobClose::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr()).await
    }

    /**
        Tests to see if the BFILE exists on the server's operating system.

        # Example
        ```
        use sibyl::BFile;

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "formatted_doc.txt")?;

        assert!(file.file_exists().await?);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn file_exists(&self) -> Result<bool> {
        oci::futures::LobFileExists::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr()).await
    }

    /**
        Returns `true` if the BFILE was opened using this particular locator.
        However, a different locator may have the file open. Openness is associated
        with a particular locator.

        # Example
        ```
        use sibyl::BFile;

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let file = BFile::new(&conn)?;
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
        oci::futures::LobFileIsOpen::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr()).await
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

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;

        file.open_file().await?;

        assert!(file.is_file_open().await?);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn open_file(&self) -> Result<()> {
        oci::futures::LobFileOpen::new(self.get_svc_ptr(), self.get_err_ptr(), self.get_ptr()).await
    }

    /**
        Reads specified number of bytes from this LOB, appending them to `buf`.
        If successful, this function returns the number of bytes that were read and appended to `buf`.

        # Example

        ```
        use sibyl::BFile;

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        let file_len = file.len().await?;
        file.open_file().await?;

        let mut data = Vec::with_capacity(file_len);
        let num_read = file.read(0, file_len, &mut data).await?;

        assert_eq!(num_read, file_len);
        assert_eq!(data, [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21]);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn read(&self, offset: usize, mut len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.read_piece(OCI_ONE_PIECE, len, offset, &mut len, 0, 0, buf).await?;
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

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "keyboard.jpg")?;
        assert!(file.file_exists().await?);
        let file_len = file.len().await?;
        file.open_readonly().await?;

        let mut data = Vec::with_capacity(file_len);
        let mut data_len : usize = 0;
        let piece_size = 8192;
        let offset = 0;
        let length = file_len;
        let mut has_next = file.read_first(piece_size, offset, length, &mut data, &mut data_len).await?;
        assert_eq!(data.len(), data_len);
        while has_next {
            let mut bytes_read : usize = 0;
            has_next = file.read_next(piece_size, &mut data, &mut bytes_read).await?;
            data_len += bytes_read;
            assert_eq!(data_len, data.len());
        }

        assert_eq!(data_len, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn read_first(&self, piece_size: usize, offset: usize, len: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = len;
        self.read_piece(OCI_FIRST_PIECE, piece_size, offset, num_read, 0, 0, buf).await
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a tuple with 2 elements - the number of
        bytes read in the current piece and the flag which indicates whether there are more pieces to read until the requested
        fragment is complete. Application should keep calling `read_next` until "more data" flag becomes `false`.
    */
    pub async fn read_next(&self, piece_size: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, piece_size, 0, num_read, 0, 0, buf).await
    }
}

impl std::fmt::Debug for LOB<'_,OCICLobLocator> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CLOB")
    }
}

impl std::fmt::Debug for LOB<'_,OCIBLobLocator> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BLOB")
    }
}

impl std::fmt::Debug for LOB<'_,OCIBFileLocator> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BFILE")
    }
}
