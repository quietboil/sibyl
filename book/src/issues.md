# Known Issues with Some Clients

12.2 client does not support `OCI_ATTR_SPOOL_MAX_USE_SESSION` and thus `SessionPool`'s `session_max_use_count` and `set_session_max_use_count` will fail on it with `ORA-24315: illegal attribute type`.

21.4 client (at least with 19.3 database) is strangely picky about names of parameter placeholders for LOB columns. For example, if a table was created with the following LOB column:

```sql
CREATE TABLE table_with_lob (
    id   NUMBER GENERATED ALWAYS AS IDENTITY,
    txt  CLOB
);
```

and if an SQL parameter name is the same as the LOB column name (as in this example):

```rust,ignore
let stmt = session.prepare("
    INSERT INTO table_with_lob (txt) VALUES (:TXT) RETURNING id INTO :ID
")?;
```

Then 21.4 client will fail executing this SQL with `ORA-03120: two-task conversion routine: integer overflow`. Renaming the parameter placeholder resolves this:

```rust,ignore
let stmt = session.prepare("
    INSERT INTO table_with_lob (txt) VALUES (:NEW_TXT) RETURNING id INTO :ID
")?;
```

21.4 also does not "like" some specific parameter names like `:NAME` which makes it fail with the same `ORA-03120`.

> Note that 12.2 through 19.13 clients (as far as Sibyl's tests showed) do not exhibit this issue.

21.4 client (at least when it is connected to the 19.3 database) cannot read **CLOBs** piece-wize - something bad happens in `OCILobRead2` as it reads the last piece and the process gets killed. 21.4 client has no issues executing piece-wise reads from BFILEs and BLOBs.

All tested clients behave erratically in `nonblocking` mode when they execute piece-wize LOB operations. Therefore, in `nonblocking` mode Sibyl does not support LOBs piece-wise reading and writing.
