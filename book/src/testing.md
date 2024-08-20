# Testing

Some of Sibyl's tests connect to the database and expect certain objects to exist in it and certain privileges granted:
- At least the HR demo schema should be [installed][1].
- While there is no need to install other demo schemas at least `MEDIA_DIR` should be created (see `$ORACLE_HOME/demo/schema/mk_dir.sql`) and point to the directory with demo files. The latter can be found in `product_media` in the [db-sample-schemas.zip][2].
- Some of the LOB tests need text files with the the expected content. Those can be found in `etc/media` and copied into `MEDIA_DIR`.
- A test user should be created. That user needs access to the `HR` schema and to the `MEDIA_DIR` directory. See `etc/create_sandbox.sql` for an example of how it can be accomplished.
- The test user needs `SELECT` access to `V$SESSION` as some tests use it for validation.
```sql
GRANT SELECT ON V_$SESSION TO sibyl;
```
- Tests that connect to the database use environment variables - `DBNAME`, `DBUSER` and `DBPASS` - to identify the database, user and password respectively. These variables should be set before executing `cargo test`.

[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/comsc/installing-sample-schemas.html#GUID-1E645D09-F91F-4BA6-A286-57C5EC66321D
[2]: https://github.com/oracle/db-sample-schemas/releases/latest
