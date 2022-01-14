use std::io::Read;

fn read_text_file(path: &str) -> String {
    let mut text = String::new();
    let mut file = std::fs::File::open(path).expect("open source file");
    file.read_to_string(&mut text).expect("file content is read");
    return text;
}

#[cfg(feature="blocking")]
mod blocking {

    use sibyl::*;

    use crate::read_text_file;

    fn connect(oracle: &Environment) -> Result<Session> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        oracle.connect(&dbname, &dbuser, &dbpass)
    }

    fn check_or_create_test_table(session: &Session) -> Result<()> {
        let stmt = session.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE test_large_object_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        bin     BLOB,
                        text    CLOB,
                        ntxt    NCLOB,
                        fbin    BFILE
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN NULL;
            END;
        ")?;
        stmt.execute(())?;
        Ok(())
    }

    fn check_file(lob: BFile) -> Result<()> {
        assert!(lob.is_initialized()?, "is initialized");
        assert!(lob.file_exists()?, "file exists");
        let file_len = lob.len()?;
        assert_eq!(file_len, 539977);

        let (dir, name) = lob.file_name()?;
        assert_eq!(dir, "MEDIA_DIR");
        assert_eq!(name, "mousepad_comp_ad.pdf");

        let mut data = Vec::new();

        let res = lob.read(0, file_len, &mut data);
        assert!(res.is_err(), "expected 'read' error");
        match res.unwrap_err() {
            Error::Oracle(code, _msg) => { assert_eq!(code, 22289, "cannot perform FILEREAD operation on an unopened file or LOB"); },
            _ => { panic!("unexpected 'read' error"); },
        }

        lob.open_file()?;
        assert!(lob.is_file_open()?, "file is open");

        let num_bytes = lob.read(0, file_len, &mut data)?;
        assert_eq!(num_bytes, file_len);
        assert_eq!(data.len(), file_len);

        let text = std::str::from_utf8(&data.as_slice()[0..8]).expect("first 8 bytes as str");
        assert_eq!(text, "%PDF-1.3");
        let text = std::str::from_utf8(&data.as_slice()[539971..539977]).expect("last 6 bytes as str");
        assert_eq!(text, "%%EOF\r");

        data.truncate(0);
        let mut num_read : usize = 0;
        let mut has_more = lob.read_first(8192, 0, file_len, &mut data, &mut num_read)?;
        assert!(num_read >= 8, "read at least first 8 bytes");
        assert_eq!(data.len(), num_read);
        let mut total_read = num_read;

        let text = std::str::from_utf8(&data.as_slice()[0..8]).expect("first 8 bytes as str");
        assert_eq!(text, "%PDF-1.3");

        while has_more {
            has_more = lob.read_next(8192, &mut data, &mut num_read)?;
            total_read += num_read;
        }
        assert_eq!(total_read, file_len);
        assert_eq!(data.len(), file_len);

        let text = std::str::from_utf8(&data.as_slice()[539971..539977]).expect("last 6 bytes as str");
        assert_eq!(text, "%%EOF\r");

        lob.close_file()?;
        assert!(!lob.is_file_open()?, "file is closed");

        Ok(())
    }

    fn check_blob(lob: BLOB) -> Result<()> {
        assert!(lob.is_initialized()?, "is initialized");
        let lob_len = lob.len()?;
        assert_eq!(lob_len, 539977);

        let mut data = Vec::new();

        let num_bytes = lob.read(0, lob_len, &mut data)?;
        assert_eq!(num_bytes, lob_len);
        assert_eq!(data.len(), lob_len);

        let text = std::str::from_utf8(&data.as_slice()[0..8]).expect("first 8 bytes as str");
        assert_eq!(text, "%PDF-1.3");
        let text = std::str::from_utf8(&data.as_slice()[539971..539977]).expect("last 6 bytes as str");
        assert_eq!(text, "%%EOF\r");

        data.truncate(0);

        let chunk_size = lob.chunk_size()?;
        assert!(chunk_size > 0);

        let mut num_read : usize = 0;
        let mut has_more = lob.read_first(chunk_size, 0, lob_len, &mut data, &mut num_read)?;
        assert!(num_read >= 8, "read at least first 8 bytes");
        assert_eq!(data.len(), num_read);
        let mut total_read = num_read;

        let text = std::str::from_utf8(&data.as_slice()[0..8]).expect("first 8 bytes as str");
        assert_eq!(text, "%PDF-1.3");

        while has_more {
            has_more = lob.read_next(chunk_size, &mut data, &mut num_read)?;
            total_read += num_read;
        }
        assert_eq!(total_read, lob_len);
        assert_eq!(data.len(), lob_len);

        let text = std::str::from_utf8(&data.as_slice()[539971..539977]).expect("last 6 bytes as str");
        assert_eq!(text, "%%EOF\r");

        Ok(())
    }

    #[test]
    fn read_file() -> Result<()> {
        let oracle = sibyl::env()?;
        let session = connect(&oracle)?;
        check_or_create_test_table(&session)?;

        let stmt = session.prepare("INSERT INTO test_large_object_data (fbin) VALUES (BFileName(:DIR,:FILENAME)) RETURNING id, fbin INTO :ID, :NEW_BFILE")?;
        let mut id : usize = 0;
        let mut lob : BFile = BFile::new(&session)?;
        stmt.execute(("MEDIA_DIR", "mousepad_comp_ad.pdf", &mut id, &mut lob))?;

        check_file(lob)?;

        let lob : BFile = BFile::new(&session)?;
        lob.set_file_name("MEDIA_DIR", "mousepad_comp_ad.pdf")?;

        check_file(lob)?;

        Ok(())
    }

    #[test]
    fn read_blob() -> Result<()> {
        let oracle = sibyl::env()?;
        let session = connect(&oracle)?;
        check_or_create_test_table(&session)?;

        let stmt = session.prepare("INSERT INTO test_large_object_data (bin) VALUES (Empty_Blob()) RETURNING id INTO :ID")?;
        let mut id : usize = 0;
        stmt.execute(&mut id)?;

        // retrieve BLOB and lock its row so we could write into it
        let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
        let row = stmt.query_single(&id)?.expect("one row");
        let lob : BLOB = row.get_not_null(0)?;

        let file = BFile::new(&session)?;
        file.set_file_name("MEDIA_DIR", "mousepad_comp_ad.pdf")?;
        let file_len = file.len()?;

        lob.open()?;
        file.open_file()?;
        lob.load_from_file(&file, 0, file_len, 0)?;
        file.close_file()?;
        lob.close()?;
        session.commit()?;

        check_blob(lob)?;

        Ok(())
    }

    #[test]
    fn write_blob() -> Result<()> {
        let oracle = sibyl::env()?;
        let session = connect(&oracle)?;
        check_or_create_test_table(&session)?;

        // load the data
        let file = BFile::new(&session)?;
        file.set_file_name("MEDIA_DIR", "mousepad_comp_ad.pdf")?;
        let file_len = file.len()?;

        file.open_file()?;
        assert!(file.is_open()?, "source file is open");

        let mut data = Vec::new();
        let num_read = file.read(0, file_len, &mut data)?;
        assert_eq!(num_read, file_len);
        assert_eq!(data.len(), file_len);

        file.close_file()?;
        assert!(!file.is_open()?, "source file is closed");

        // make 4 blobs - one for "one piece" writing, another for piece-wise writing
        // and the last 2 for appending and piece-wise appending.
        let stmt = session.prepare("INSERT INTO test_large_object_data (bin) VALUES (Empty_Blob()) RETURNING id INTO :ID")?;
        let mut ids = [0usize; 4];
        stmt.execute(&mut ids[0])?;
        stmt.execute(&mut ids[1])?;
        stmt.execute(&mut ids[2])?;
        stmt.execute(&mut ids[3])?;

        // retrieve BLOB and lock its row so we could write into it
        let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
        let row = stmt.query_single(&ids[0])?.expect("one row");
        let lob : BLOB = row.get_not_null(0)?;

        lob.open()?;
        let written = lob.write(0, &data)?;
        lob.close()?;
        assert_eq!(written, file_len);

        let row = stmt.query_single(&ids[1])?.expect("one row");
        let lob : BLOB = row.get_not_null(0)?;

        let chunk_size = lob.chunk_size()?;
        assert!(chunk_size > 0, "chunk size");
        assert!(chunk_size < file_len, "chunk size is smaller than the data we have"); // otherwise we need a better test data

        lob.open()?;
        let mut start_index = 0usize;
        let mut end_index = chunk_size;
        let written = lob.write_first(0, &data[start_index..end_index])?;
        assert!(written > 0, "first written chunk is not empty");
        let mut total_written = written;
        start_index += written;
        end_index += written;
        while end_index < file_len {
            let written = lob.write_next(&data[start_index..end_index])?;
            start_index += written;
            end_index += written;
            total_written += written;
        }
        let written = lob.write_last(&data[start_index..])?;
        total_written += written;
        lob.close()?;
        assert_eq!(total_written, file_len);

        let row = stmt.query_single(&ids[2])?.expect("one row");
        let lob : BLOB = row.get_not_null(0)?;

        lob.open()?;
        let written = lob.append(&data)?;
        lob.close()?;
        assert_eq!(written, file_len);

        let row = stmt.query_single(&ids[3])?.expect("one row");
        let lob : BLOB = row.get_not_null(0)?;

        lob.open()?;
        start_index = 0usize;
        end_index = chunk_size;
        let written = lob.append_first(&data[start_index..end_index])?;
        assert!(written > 0, "first written chunk is not empty");
        total_written = written;
        start_index += written;
        end_index += written;
        while end_index < file_len {
            let written = lob.append_next(&data[start_index..end_index])?;
            start_index += written;
            end_index += written;
            total_written += written;
        }
        let written = lob.append_last(&data[start_index..])?;
        total_written += written;
        lob.close()?;
        assert_eq!(total_written, file_len);

        session.commit()?;

        // read them back and check that they all match the source
        let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID")?;
        for id in ids {
            let row = stmt.query_single(&id)?.expect("one row");
            let lob : BLOB = row.get_not_null(0)?;

            check_blob(lob)?;
        }

        Ok(())
    }

    #[test]
    fn read_write_clob() -> Result<()> {
        let oracle = sibyl::env()?;
        let session = connect(&oracle)?;
        check_or_create_test_table(&session)?;

        // load the data
        let text = read_text_file("src/oci.rs");
        let text_char_len = text.chars().count();
        let expected_lob_char_len = text_char_len + 24; // +24 accounts for 2-characters per supplementary symbol encoding

        // make 4 clobs - one for "one piece" writing, another for piece-wise writing
        // and the last 2 for appending and piece-wise appending.
        let stmt = session.prepare("INSERT INTO test_large_object_data (text) VALUES (Empty_Clob()) RETURNING id INTO :ID")?;
        let mut ids = [0usize; 4];
        stmt.execute(&mut ids[0])?;
        stmt.execute(&mut ids[1])?;
        stmt.execute(&mut ids[2])?;
        stmt.execute(&mut ids[3])?;

        let stmt = session.prepare("SELECT text FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
        let row = stmt.query_single(&ids[0])?.expect("one row");
        let lob : CLOB = row.get_not_null(0)?;

        lob.open()?;
        let written = lob.write(0, &text)?;
        lob.close()?;
        assert_eq!(written, expected_lob_char_len);

        let row = stmt.query_single(&ids[1])?.expect("one row");
        let lob : CLOB = row.get_not_null(0)?;

        lob.open()?;
        let mut lines = text.split_inclusive('\n');
        let mut total_written = 0usize;
        if let Some(line) = lines.next() {
            let written = lob.write_first(0, line)?;
            total_written += written;
            while let Some(line) = lines.next() {
                let written = lob.write_next(line)?;
                total_written += written;
            }
            lob.write_last("")?;
        }
        lob.close()?;
        assert_eq!(total_written, expected_lob_char_len);

        let row = stmt.query_single(&ids[2])?.expect("one row");
        let lob : CLOB = row.get_not_null(0)?;

        lob.open()?;
        let written = lob.append(&text)?;
        lob.close()?;
        assert_eq!(written, expected_lob_char_len);

        let row = stmt.query_single(&ids[3])?.expect("one row");
        let lob : CLOB = row.get_not_null(0)?;

        lob.open()?;
        let mut lines = text.split_inclusive('\n');
        let mut total_written = 0usize;
        if let Some(line) = lines.next() {
            let written = lob.append_first(line)?;
            total_written += written;
            while let Some(line) = lines.next() {
                let written = lob.append_next(line)?;
                total_written += written;
            }
            lob.append_last("")?;
        }
        lob.close()?;
        assert_eq!(total_written, expected_lob_char_len);

        session.commit()?;

        // read them back and check that they all match the source
        let stmt = session.prepare("SELECT text FROM test_large_object_data WHERE id = :ID")?;
        for id in ids {
            let row = stmt.query_single(&id)?.expect("one row");
            let lob : CLOB = row.get_not_null(0)?;

            let mut lob_content = String::new();
            let num_chars = lob.read(0, expected_lob_char_len, &mut lob_content)?;
            assert_eq!(num_chars, expected_lob_char_len);
            assert_eq!(lob_content, text);
        }
        Ok(())
    }
}

#[cfg(feature="nonblocking")]
mod nonblocking {

    use sibyl::*;

    use crate::read_text_file;

    async fn connect<'a>(oracle: &'a Environment) -> Result<Session<'a>> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        oracle.connect(&dbname, &dbuser, &dbpass).await
    }

    async fn check_or_create_test_table(session: &Session<'_>) -> Result<()> {
        let stmt = session.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE test_large_object_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        bin     BLOB,
                        text    CLOB,
                        ntxt    NCLOB,
                        fbin    BFILE
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN NULL;
            END;
        ").await ?;
        stmt.execute(()).await?;
        Ok(())
    }

    async fn check_file(lob: BFile<'_>) -> Result<()> {
        assert!(lob.is_initialized()?, "is initialized");
        assert!(lob.file_exists().await?, "file exists");

        let file_len = lob.len().await?;
        assert_eq!(file_len, 539977);

        let (dir, name) = lob.file_name()?;
        assert_eq!(dir, "MEDIA_DIR");
        assert_eq!(name, "mousepad_comp_ad.pdf");

        let mut data = Vec::new();

        lob.open_file().await?;
        assert!(lob.is_file_open().await?, "file is open");

        let num_bytes = lob.read(0, file_len, &mut data).await?;
        assert_eq!(num_bytes, file_len);
        assert_eq!(data.len(), file_len);

        let text = std::str::from_utf8(&data.as_slice()[0..8]).expect("first 8 bytes as str");
        assert_eq!(text, "%PDF-1.3");
        let text = std::str::from_utf8(&data.as_slice()[539971..539977]).expect("last 6 bytes as str");
        assert_eq!(text, "%%EOF\r");

        lob.close_file().await?;
        assert!(!lob.is_file_open().await?, "file is closed");

        Ok(())
    }

    #[test]
    fn read_file() -> Result<()> {
        block_on(async {
            let oracle = sibyl::env()?;
            let session = connect(&oracle).await?;
            check_or_create_test_table(&session).await?;

            let stmt = session.prepare("INSERT INTO test_large_object_data (fbin) VALUES (BFileName(:DIRNAME,:FILENAME)) RETURNING id, fbin INTO :ID, :NEW_BFILE").await?;
            let mut id : usize = 0;
            let mut lob : BFile = BFile::new(&session)?;
            stmt.execute(((":DIRNAME", "MEDIA_DIR"), (":FILENAME", "mousepad_comp_ad.pdf"), (":ID", &mut id), (":NEW_BFILE", &mut lob))).await?;

            check_file(lob).await?;

            let lob : BFile = BFile::new(&session)?;
            lob.set_file_name("MEDIA_DIR", "mousepad_comp_ad.pdf")?;

            check_file(lob).await?;

            Ok(())
        })
    }

    async fn check_blob(lob: BLOB<'_>) -> Result<()> {
        assert!(lob.is_initialized()?, "is initialized");
        let lob_len = lob.len().await?;
        assert_eq!(lob_len, 539977);

        let mut data = Vec::new();

        let num_bytes = lob.read(0, lob_len, &mut data).await?;
        assert_eq!(num_bytes, lob_len);
        assert_eq!(data.len(), lob_len);

        let text = std::str::from_utf8(&data.as_slice()[0..8]).expect("first 8 bytes as str");
        assert_eq!(text, "%PDF-1.3");
        let text = std::str::from_utf8(&data.as_slice()[539971..539977]).expect("last 6 bytes as str");
        assert_eq!(text, "%%EOF\r");
        Ok(())
    }

    #[test]
    fn read_blob() -> Result<()> {
        block_on(async {
            let oracle = sibyl::env()?;
            let session = connect(&oracle).await?;
            check_or_create_test_table(&session).await?;

            let stmt = session.prepare("
                DECLARE
                    row_id ROWID;
                BEGIN
                    INSERT INTO test_large_object_data (bin) VALUES (Empty_Blob()) RETURNING rowid into row_id;
                    SELECT bin INTO :NEW_BLOB FROM test_large_object_data WHERE rowid = row_id FOR UPDATE;
                END;
            ").await?;
            let mut lob = BLOB::new(&session)?;
            stmt.execute(&mut lob).await?;

            let file = BFile::new(&session)?;
            file.set_file_name("MEDIA_DIR", "mousepad_comp_ad.pdf")?;
            let file_len = file.len().await?;

            lob.open().await?;
            file.open_file().await?;
            lob.load_from_file(&file, 0, file_len, 0).await?;
            file.close_file().await?;
            lob.close().await?;
            session.commit().await?;

            check_blob(lob).await?;

            Ok(())
        })
    }

    #[test]
    fn write_blob() -> Result<()> {
        block_on(async {
            let oracle = sibyl::env()?;
            let session = connect(&oracle).await?;
            check_or_create_test_table(&session).await?;

            // load the data
            let file = BFile::new(&session)?;
            file.set_file_name("MEDIA_DIR", "mousepad_comp_ad.pdf")?;
            let file_len = file.len().await?;

            file.open_file().await?;
            assert!(file.is_open().await?, "source file is open");

            let mut data = Vec::new();
            let num_read = file.read(0, file_len, &mut data).await?;
            assert_eq!(num_read, file_len);
            assert_eq!(data.len(), file_len);

            file.close_file().await?;
            assert!(!file.is_open().await?, "source file is closed");

            // make 2 blobs - one for writing and another for appending
            let stmt = session.prepare("INSERT INTO test_large_object_data (bin) VALUES (Empty_Blob()) RETURNING id INTO :ID").await?;
            let mut ids = [0usize; 2];
            stmt.execute(&mut ids[0]).await?;
            stmt.execute(&mut ids[1]).await?;

            // retrieve BLOB and lock its row so we could write into it
            let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID FOR UPDATE").await?;
            let row = stmt.query_single(ids[0]).await?.expect("one row");
            let lob : BLOB = row.get_not_null(0)?;

            lob.open().await?;
            let written = lob.write(0, &data).await?;
            lob.close().await?;
            assert_eq!(written, file_len);

            let row = stmt.query_single(ids[1]).await?.expect("one row");
            let lob : BLOB = row.get_not_null(0)?;

            lob.open().await?;
            let written = lob.append(&data).await?;
            lob.close().await?;
            assert_eq!(written, file_len);

            session.commit().await?;

            // read them back and check that they all match the source
            let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID").await?;
            for id in ids {
                if id > 0 {
                    let row = stmt.query_single(&id).await?.expect("one row");
                    let lob : BLOB = row.get_not_null(0)?;
                    check_blob(lob).await?;
                }
            }

            Ok(())
        })
    }

    #[test]
    fn read_write_clob() -> Result<()> {
        let text = read_text_file("src/oci.rs");
        let text_char_len = text.chars().count();
        // Note that 24 supplemental symbols in `oci.rs` are encoded as 2 "characters" by Oracle.
        let expected_lob_char_len = text_char_len + 24;

        block_on(async {
            let oracle = sibyl::env()?;
            let session = connect(&oracle).await?;
            check_or_create_test_table(&session).await?;

            // make 2 clobs - one for writing and another for appending
            let stmt = session.prepare("INSERT INTO test_large_object_data (text) VALUES (Empty_Clob()) RETURNING id INTO :ID").await?;
            let mut ids = [0usize; 2];
            stmt.execute(&mut ids[0]).await?;
            stmt.execute(&mut ids[1]).await?;

            let stmt = session.prepare("SELECT text FROM test_large_object_data WHERE id = :ID FOR UPDATE").await?;
            let row = stmt.query_single(ids[0]).await?.expect("one row");
            let lob : CLOB = row.get_not_null(0)?;

            lob.open().await?;
            let written = lob.write(0, &text).await?;
            lob.close().await?;
            assert_eq!(written, expected_lob_char_len);

            let row = stmt.query_single(ids[1]).await?.expect("one row");
            let lob : CLOB = row.get_not_null(0)?;

            lob.open().await?;
            let written = lob.append(&text).await?;
            lob.close().await?;
            assert_eq!(written, expected_lob_char_len);

            session.commit().await?;

            // read them back and check that they all match the source
            let stmt = session.prepare("SELECT text FROM test_large_object_data WHERE id = :ID").await?;
            for id in ids {
                let row = stmt.query_single(&id).await?.expect("one row");
                let lob : CLOB = row.get_not_null(0)?;
                let lob_len = lob.len().await?;

                let mut lob_content = String::new();
                let num_chars = lob.read(0, lob_len, &mut lob_content).await?;
                assert_eq!(num_chars, expected_lob_char_len);
                assert_eq!(lob_content, text);
            }

            Ok(())
        })
    }

    #[test]
    fn temp_blob() -> Result<()> {
        block_on(async {
            let oracle = sibyl::env()?;
            let session = connect(&oracle).await?;
            check_or_create_test_table(&session).await?;

            let lob = BLOB::temp(&session, Cache::No).await?;

            let is_temp = lob.is_temp().await?;
            assert!(is_temp);

            let mut lob = BLOB::empty(&session)?;
            let is_temp = lob.is_temp().await?;
            assert!(!is_temp);

            let stmt = session.prepare("BEGIN DBMS_LOB.CREATETEMPORARY(:LOC, FALSE); END;").await?;
            stmt.execute(&mut lob).await?;
            let is_temp = lob.is_temp().await?;
            assert!(is_temp);

            Ok(())
        })
    }
}
