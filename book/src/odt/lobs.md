# CLOBs, BLOBs, BFILEs

Let's assume a table was created:

```sql
CREATE TABLE lob_example (
    id  NUMBER GENERATED ALWAYS AS IDENTITY,
    bin BLOB
);
```

We can then create and write data into that LOB as:

```rust,ignore
// ... create OCI environment, connect to the database, etc.

let file = BFile::new(&session)?;
file.set_file_name("MEDIA_DIR", "mousepad_comp_ad.pdf")?;
let file_len = file.len()?;

file.open_file()?;
let mut data = Vec::new();
let num_read = file.read(0, file_len, &mut data)?;
file.close_file()?;
// ... or do not close now as it will be closed
// automatically when `file` goes out of scope

// Insert new BLOB and lock its row
let stmt = session.prepare("
    DECLARE
        row_id ROWID;
    BEGIN
        INSERT INTO lob_example (bin) VALUES (Empty_Blob()) RETURNING rowid INTO row_id;
        SELECT bin INTO :NEW_BLOB FROM lob_example WHERE rowid = row_id FOR UPDATE;
    END;
")?;
let mut lob = BLOB::new(&session)?;
stmt.execute(&mut lob)?;

lob.open()?;
let num_bytes_written = lob.write(0, &data)?;
lob.close()?;

session.commit()?;
```

And then later it could be read as:

```rust,ignore
let id: usize = 1234; // assume it was retrieved from somewhere...
let stmt = session.prepare("SELECT bin FROM lob_example WHERE id = :ID")?;
let row = stmt.query_single(&id)?;
if let Some(row) = row {
    if let Some(lob) = row.get(0)? {
        let data = read_blob(lob)?;
        // ...
    }
}

// Where `read_blob` could be this:
fn read_blob(lob: BLOB<'_>) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let lob_len = lob.len()?;
    let offset = 0;
    lob.read(offset, lob_len, &mut data)?;
    Ok(data)
}
```
