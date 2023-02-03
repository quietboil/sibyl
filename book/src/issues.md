# Known Issues with Some Clients

12.2 client does not support `OCI_ATTR_SPOOL_MAX_USE_SESSION` and thus `SessionPool`'s `session_max_use_count` and `set_session_max_use_count` will fail on it with `ORA-24315: illegal attribute type`.

19.15 and all later clients do not return current schema (via `Session::current_schema`) until it is explicitly set via `Session::set_current_schema`. `Session::current_schema` works as expected in 19.13 client.

21c clients (at least with 19.3 database) is strangely picky about names of parameter placeholders for LOB columns. For example, if a table was created with the following LOB column:

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

Then 21c clients will fail executing this SQL with `ORA-03120: two-task conversion routine: integer overflow`. Renaming the parameter placeholder resolves this:

```rust,ignore
let stmt = session.prepare("
    INSERT INTO table_with_lob (txt) VALUES (:NEW_TXT) RETURNING id INTO :ID
")?;
```

21c clients also do not "like" some specific parameter names like `:NAME` which makes it fail with the same `ORA-03120`.

> Note that 12.2 through 19.18 clients (as far as Sibyl's tests showed) do not exhibit this issue.

21c clients (at least when connected to the 19.3 database) cannot read **LOBs** piece-wize - something bad happens while it reads the very last byte of the last piece and the execution is aborted with SIGSEGV. Notably, they have no problem reading the second to last piece even if it has all the bytes but the very last one. Subsequently, an attempt to read the last one-byte piece gets aborted with memory violation.

All tested clients behave erratically in `nonblocking` mode when they execute piece-wize LOB operations. Therefore, in `nonblocking` mode Sibyl does not support LOBs piece-wise reading and writing.
