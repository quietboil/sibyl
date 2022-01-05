# Sibyl

Sibyl is an [OCI][1]-based interface between Rust applications and Oracle databases.

[![crates.io](https://img.shields.io/crates/v/sibyl)](https://crates.io/crates/sibyl)
[![Documentation](https://docs.rs/sibyl/badge.svg)](https://docs.rs/sibyl)
![MIT](https://img.shields.io/crates/l/sibyl.svg)

## Example

### Blocking Mode

```rust
use sibyl as oracle; // pun intended :)

fn main() -> Result<(),Box<dyn std::error::Error>> {
    let dbname = std::env::var("DBNAME")?;
    let dbuser = std::env::var("DBUSER")?;
    let dbpass = std::env::var("DBPASS")?;

    let oracle = oracle::env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
        SELECT first_name, last_name, hire_date
          FROM (
                SELECT first_name, last_name, hire_date
                     , Row_Number() OVER (ORDER BY hire_date) hire_date_rank
                  FROM hr.employees
                 WHERE hire_date >= :hire_date
               )
         WHERE hire_date_rank = 1
    ")?;
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &oracle)?;
    let rows = stmt.query(&date)?;
    if let Some( row ) = rows.next()? {
        let first_name : Option<&str> = row.get("FIRST_NAME")?;
        let last_name : &str = row.get("LAST_NAME")?.unwrap();
        let name = first_name.map_or(last_name.to_string(),
            |first_name| format!("{}, {}", last_name, first_name)
        );
        let hire_date : oracle::Date = row.get("HIRE_DATE")?.unwrap();
        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

        println!("{} was hired on {}", name, hire_date);
    } else {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}
```

### Nonblocking Mode

```rust
use sibyl as oracle;

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {
    let dbname = std::env::var("DBNAME")?;
    let dbuser = std::env::var("DBUSER")?;
    let dbpass = std::env::var("DBPASS")?;

    let oracle = oracle::env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = conn.prepare("
        SELECT first_name, last_name, hire_date
          FROM (
                SELECT first_name, last_name, hire_date
                     , Row_Number() OVER (ORDER BY hire_date) hire_date_rank
                  FROM hr.employees
                 WHERE hire_date >= :hire_date
               )
         WHERE hire_date_rank = 1
    ").await?;
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &oracle)?;
    let rows = stmt.query(&date).await?;
    if let Some( row ) = rows.next().await? {
        let first_name : Option<&str> = row.get("FIRST_NAME")?;
        let last_name : &str = row.get("LAST_NAME")?.unwrap();
        let name = first_name.map_or(last_name.to_string(),
            |first_name| format!("{}, {}", last_name, first_name)
        );
        let hire_date : oracle::Date = row.get("HIRE_DATE")?.unwrap();
        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

        println!("{} was hired on {}", name, hire_date);
    } else {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}
```

> Note that:
> - The nonblocking mode example is almost a verbatim copy of the blocking mode example (see above) with `await`s added.
> - The example below uses and depends on [Tokio](https://crates.io/crates/tokio)
> - For the moment, Sibyl can use only Tokio as an async runtime.


## Notes on Building

Sibyl needs an installed Oracle client in order to link either `OCI.DLL` on Windows or `libclntsh.so` on Linux. The cargo build needs to know where that library is. You can supply that information via environment variable `OCI_LIB_DIR` on Windows or `LIBRARY_PATH` on Linux. On Linux `LIBRARY_PATH` would include the path to the `lib` directory with `libclntsh.so`. For example, you might build Sibyl's example as:

```bash
LIBRARY_PATH=/usr/lib/oracle/19.13/client64/lib cargo build --examples --features=blocking
```

On Windows the process is similar if the target environment is `gnu`. The `OCI_LIB_DIR` would point to the directory with `oci.dll`:

```bat
set OCI_LIB_DIR=%ORACLE_HOME%\bin
cargo build --examples --features=blocking
```

However, for `msvc` environment the `OCI_LIB_DIR` must point to the directory with `oci.lib`. For example, you might build that example as:

```bat
set OCI_LIB_DIR=%ORACLE_HOME%\oci\lib\msvc
cargo build --examples --features=blocking
```

> Note that Sibyl has 2 features - `blocking` and `nonblocking`. They are exclusive and one must be explictly selected. Thus, when Sibyl is used as a dependency it might be included as:

```toml
[dependencies]
sibyl = { version = "0.5", features = "blocking" }
```

## Usage

### Environment

The OCI environment handle must be created before any other OCI function can be called. While there can be many environments - for example, one can create an environment per connection - usually one is enought. Sibyl initializes it to be the most compatible with Rust requirements - thread-safe using UTF8 character encoding. That single environment handle can be created in `main` and then passed around:

```rust
fn main() {
    let oracle = sibyl::env().expect("Oracle OCI environment");
    // ...
}
```

Note however that some functions will need a direct reference to this handle, so instead of passing it around some applications might prefer to create it statically:

```rust
use sibyl::Environment;
use lazy_static::lazy_static;

lazy_static!{
    pub static ref ORACLE : Environment = sibyl::env().expect("Oracle OCI environment");
}
```

Then later one would be able to create, for example, a current timestamp as:

```rust
use sibyl::TimestampTZ;

let current_timestamp = TimestampTZ::from_systimestamp(&ORACLE)?;
```

### Connections

Use `Environment::connect` method to connect to a database:

```rust
fn main() -> sibyl::Result<()> {
    let oracle = sibyl::env()?;
    let conn = oracle.connect("dbname", "username", "password")?;
    // ...
    Ok(())
}
```

Where `dbname` can be any name that is acceptable to Oracle clients - from local TNS name to EZConnect identifier to a connect descriptor.

### SQL Statement Execution

All SQL or PL/SQL statements must be prepared before they can be executed:

```rust
let stmt = conn.prepare("
    SELECT employee_id, last_name, first_name
      FROM hr.employees
     WHERE manager_id = :id
  ORDER BY employee_id
")?;
```

A prepared statement can be executed either with the `query` or `execute` or `execute_into` methods:
- `query` is used for `SELECT` statements. In fact, Sibyl will complain if you try to `query` any other statement.
- `execute` is used for all other, non-SELECT, DML and DDL that do not have OUT parameters.
- `execute_into` is used with DML that have OUT parameters.

`query` and `execute` take a tuple of IN arguments, which can be specified as positional arguments or as name-value tuples. For example, to execute the above SELECT we can call `query` using a positional argument as:

```rust
let rows = stmt.query(103)?;
```

or bind a value to `:id` by name as:

```rust
let rows = stmt.query((":ID", 103))?;
```

In most cases which binding style to use is a matter of convenience and/or personal preferences. However, in some cases named arguments would be preferable and less ambiguous. For example, statement might change during development and thus force the change in argument positions. Also SQL and PL/SQL statements have different interpretation of a parameter position. SQL statements create positions for every parameter but allow a single argument to be used for the primary parameter and all its duplicares. PL/SQL on the other hand creates positions for unique parameter names and this might make positioning arguments correctly a bit awkward when there is more than one "duplicate" name in a statement.

> Note one caveat - until [min_specialization][5] is stabilized Sibyl has no way to distinguish whether a 2-item tuple is used to pass a named argument or 2 positional arguments. At the moment you'll have to use a 3-item tuple with a unit type as the last item when you are passing 2 positional arguments. The unit type is treated as "nothing", so effectively only first 2 arguments are used. For example:

```rust
let stmt = conn.prepare("
    SELECT department_id, manager_id
      FROM hr.departments
     WHERE department_name = :DEPARTMENT_NAME
       AND location_id = :LOCATION_ID
")?;
let rows = stmt.query(("Security", 1700, ()))?;
```

`execute_into` allows execution of statements with OUT (or INOUT) parameters. For example:

```rust
let stmt = conn.prepare("
    INSERT INTO hr.departments
           ( department_id, department_name, manager_id, location_id )
    VALUES ( hr.departments_seq.nextval, :department_name, :manager_id, :location_id )
 RETURNING department_id
      INTO :department_id
")?;
let mut department_id: u32 = 0;
let num_inserted = stmt.execute(
    (
        (":DEPARTMENT_NAME", "Security"),
        (":MANAGER_ID",      ""        ),
        (":LOCATION_ID",     1700      ),
    ), 
        (":DEPARTMENT_ID",   &mut department_id)
)?;
```

`execute` and `execute_into` return the number of rows affected by the statement. `query` returns what is colloquially called a "streaming iterator" which is typically iterated using `while`. For example (continuing the SELECT example from above):

```rust
let mut employees = HashMap::new();
let stmt = conn.prepare("
    SELECT employee_id, last_name, first_name
        FROM hr.employees
    WHERE manager_id = :id
    ORDER BY employee_id
")?;
let rows = stmt.query(103)?;
while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get(0)?.unwrap();
    let last_name : &str  = row.get(1)?.unwrap();
    let first_name : Option<&str> = row.get(2)?;
    let name = first_name.map_or(last_name.to_string(),
        |first_name| format!("{}, {}", last_name, first_name)
    );
    employees.insert(employee_id, name);
}
```

There are a few notable points of interest in the last example:
- Sibyl uses 0-based column indexing in a projection.
- Column value is returned as an `Option`. However, if a column is declared as `NOT NULL`, like `EMPLOYEE_ID` and `LAST_NAME`, the result will always be `Some` and therefore can be safely unwrapped.
- `LAST_NAME` and `FIRST_NAME` are retrieved as `&str`. This is fast as they are borrowed directly from the respective column buffers. However, those values will only be valid during the lifetime of the row. If the value needs to continue to exist beyond the lifetime of a row, it should be retrieved as a `String`.

> Note that while Sibyl expects 0-based indexes to reference projection columns, it also accepts column names. Thus, the row processing loop of the previous example can be written as:

```rust
while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get("EMPLOYEE_ID")?.unwrap();
    let last_name : &str  = row.get("LAST_NAME")?.unwrap();
    let first_name : Option<&str> = row.get("FIRST_NAME")?;
    let name = first_name.map_or(last_name.to_string(),
        |first_name| format!("{}, {}", last_name, first_name)
    );
    employees.insert(employee_id, name);
}
```

## Oracle Data Types

Sibyl provides API to access several Oracle native data types.

### Number
```rust
use sibyl::Number;

let oracle = sibyl::env()?;

let pi = Number::pi(&oracle)?;
let two = Number::from_int(2, &oracle);
let two_pi = pi.mul(&two)?;
let h = Number::from_string("6.62607004E-34", "9D999999999EEEE", &oracle)?;
let hbar = h.div(&two_pi)?;

assert_eq!(hbar.to_string("TME")?, "1.05457180013911265115394106872506677375E-34");
```

### Date
```rust
use sibyl::Date;

let mar28_1996 = Date::from_string("28-MAR-1996", "DD-MON-YYYY", &oracle)?;
let next_monday = mar28_1996.next_week_day("MONDAY")?;

assert_eq!(next_monday.to_string("DL")?, "Monday, April 01, 1996");
```

### Timestamp

There are 3 types of timestamps:
- `Timestamp` which is equivalent to Oracle's TIMESTAMP,
- `TimestampTZ` - for TIMESTAMP WITH TIME ZONE, and
- `TimestampLTZ` - for TIMESTAMP WITH LOCAL TIME ZONE

```rust
use sibyl::TimestampTZ;

let ts = oracle::TimestampTZ::from_string(
    "July 20, 1969 8:18:04.16 pm UTC",
    "MONTH DD, YYYY HH:MI:SS.FF PM TZR",
    &oracle
)?;
assert_eq!(
    ts.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR", 3)?,
    "1969-07-20 20:18:04.160 UTC"
);
```

> Note that if you are getting `ORA-01805` when timestamp with time zone is used, then most likely your local client and the server it is connected to are using different versions of the time zone file. This [stackoverflow answer][4] should help you in setting up your local client with the correct time zone file.

### Interval

There are 2 types of intervals:
- `IntervalYM` which is eqivalent to Oracle's INTERVAL YEAR TO MONTH,
- `IntervalDS` - INTERVAL DAY TO SECOND

```rust
use sibyl::{ TimestampTZ, IntervalDS };

let launch  = TimestampTZ::with_date_and_time(1969, 7, 16, 13, 32,  0, 0, "UTC", &oracle)?;
let landing = TimestampTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 0, "UTC", &oracle)?;
let duration : IntervalDS = landing.subtract(&launch)?;

assert_eq!("+8 03:18:35.000", duration.to_string(1,3)?);
```

### RowID

Oracle ROWID can be selected and retrieved explicitly into an instance of the `RowID`. However, one interesting case is SELECT FOR UPDATE queries where Oracle returns ROWIDs implicitly. Those can be retrieved using `Row::get_rowid` method.

```rust
let stmt = conn.prepare("
    SELECT manager_id
      FROM hr.employees
     WHERE employee_id = :id
       FOR UPDATE
")?;
let rows = stmt.query(107)?;
let row = rows.next()?.unwrap();
let rowid = row.rowid()?;

let manager_id: u32 = row.get(0)?.unwrap();
assert_eq!(manager_id, 103);

let stmt = conn.prepare("
    UPDATE hr.employees
       SET manager_id = :manager_id
     WHERE rowid = :row_id
")?;
let num_updated = stmt.execute((
    ( ":MANAGER_ID", 102 ),
    ( ":ROW_ID",  &rowid ),
))?;
assert_eq!(1, num_updated);
```

### Cursors

Cursors can be returned explicitly:

```rust
let stmt = conn.prepare("
    BEGIN
        OPEN :emp FOR
            SELECT department_name, first_name, last_name, salary
              FROM hr.employees e
              JOIN hr.departments d
                ON d.department_id = e.department_id;
    END;
")?;
let mut cursor = Cursor::new(&stmt)?;
stmt.execute_into((), &mut cursor)?;
let rows = cursor.rows()?;
// ...
```

Or, beginning with Oracle 12.1, implicitly:

```rust
let stmt = conn.prepare("
    DECLARE
        emp SYS_REFCURSOR;
    BEGIN
        OPEN emp FOR
            SELECT department_name, first_name, last_name, salary
              FROM hr.employees e
              JOIN hr.departments d
                ON d.department_id = e.department_id;
        ;
        DBMS_SQL.RETURN_RESULT(emp);
    END;
")?;
stmt.execute(())?;
if let Some( cursor ) = stmt.next_result()? {
    let rows = cursor.rows()?;
    // ...
}
```

### CLOBs, BLOBs, BFILEs

Let's assume a table was created:

```sql
CREATE TABLE lob_example (
    id  NUMBER GENERATED ALWAYS AS IDENTITY,
    bin BLOB
);
```

We can then create and write data into that LOB as:

```rust
// ... create OCI environment, connect to the database, etc.

let file = BFile::new(&conn)?;
file.set_file_name("MEDIA_DIR", "mousepad_comp_ad.pdf")?;
let file_len = file.len().await?;

file.open_file().await?;
let mut data = Vec::new();
let num_read = file.read(0, file_len, &mut data).await?;
file.close_file().await?;
// ... or do not close now as it will be closed
// automatically when `file` goes out of scope

// Insert new BLOB and lock its row
let stmt = conn.prepare("
    DECLARE
        row_id ROWID;
    BEGIN
        INSERT INTO lob_example (bin) VALUES (Empty_Blob()) RETURNING rowid INTO row_id;
        SELECT bin INTO :NEW_BLOB FROM lob_example WHERE rowid = row_id FOR UPDATE;
    END;
").await?;
let mut lob = BLOB::new(&conn)?;
stmt.execute_into((), &mut lob).await?;

lob.open().await?;
let num_bytes_written = lob.write(0, &data).await?;
lob.close().await?;

conn.commit().await?;
```

And then later it could be read as:

```rust
let id: usize = 1234; // assume it was retrieved from somewhere...
let stmt = conn.prepare("SELECT bin FROM lob_example WHERE id = :ID").await?;
let rows = stmt.query(&id).await?;
if let Some(row) = rows.next().await? {
    if let Some(lob) = row.get(0)? {
        let data = read_blob(lob)?;
        // ...
    }
}
```

Where `read_blob` could be this:

```rust
async fn read_blob(lob: BLOB<'_>) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let lob_len = lob.len().await?;
    let offset = 0;
    lob.read(offset, lob_len, &mut data).await?;
    Ok(data)
}
```


## Testing

Some of Sibyl's tests connect to the database and expect certain objects to exist in it and certain privileges granted:
- At least the HR demo schema should be [installed][2].
- While there is no need to install other demo schemas at least `MEDIA_DIR` should be created (see `$ORACLE_HOME/demo/schema/mk_dir.sql`) and point to the directory with demo files that can be found in `product_media` in the [db-sample-schemas.zip][3].
- Some of the LOB tests need text files with the the expected content. Those can be found in `etc/media` and copied into `MEDIA_DIR`.
- A test user should be created. That user needs acccess to the HR schema and to the `MEDIA_DIR` directory. See `etc/create_sandbox.sql` for an example of how it can be accomplished.
- Tests that connect to the database use environment variables - DBNAME, DBUSER and DBPASS - to identify the database, user and password respectively. These variables should be set before executing `cargo test`.


## Supported Clients

The minimal supported client is 12.2 as Sibyl uses some API functions that are not available in earlier clients. While suporting those is definitely feasible, it was not a priority.

Sibyl tests are routinely executed on x64 Linux with Instant Clients 12.2, 18.5, 19.13 and 21.4 that connect to the 19.3 database. Sibyl is also tested on x64 Windows with Instant CLient 19.12.

### Known Issues with Some Clients

`SessionPool::session_max_use_count` and `SessionPool::set_session_max_use_count` will fail on 12.2 client with `ORA-24315: illegal attribute type`.

Client 21.4 (at least with 19.3 database) is strangely picky about names of parameter placeholders for LOB columns. For example, if a table was created with the following LOB column:

```sql
CREATE TABLE table_with_lob (
    id   NUMBER GENERATED ALWAYS AS IDENTITY,
    txt  CLOB
);
```

and if an SQL parameter name is the same as the LOB column name (as in this example):

```rust
let stmt = conn.prepare("
    INSERT INTO table_with_lob (txt) VALUES (:TXT) RETURNING id INTO :ID
")?;
```

Then 21.4 client will fail executing this SQL with `ORA-03120: two-task conversion routine: integer overflow`. Renaming the parameter placeholder resolves this:

```rust
let stmt = conn.prepare("
    INSERT INTO table_with_lob (txt) VALUES (:NEW_TXT) RETURNING id INTO :ID
")?;
```

21.4 also does not "like" some specific parameter names like `:NAME` which makes it fail with the same `ORA-03120`.

> Note that 12.2 through 19.13 clients (as far as Sibyl's tests showed) do not exhibit this issue.

21.4 client (at least when it is connected to the 19.3 database) cannot read **CLOBs** piece-wize - something bad happens in `OCILobRead2` as it reads the last piece and the process gets killed. 21.4 client has no issues executing piece-wise reads from BFILEs and BLOBs.

## Limitations

At this time Sibyl provides only the most commonly needed means to interface with the Oracle database. Some of the missing features are:
- Array interface for multi-row operations
- User defined data types
- PL/SQL collections and tables
- Objects
- JSON data
- LDAP and proxy authentications
- Global transactions
- High Availability
- Continuous query and publish-subscribe notifications
- Advanced queuing
- Shards
- Direct path load

Some of these features might be added in the upcoming releases if the need arises or if they are explicitly requested. Some, however, will never be implemented. The latter category includes those that are incompatible with nonblocking execution.

[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/index.html
[2]: https://docs.oracle.com/en/database/oracle/oracle-database/19/comsc/installing-sample-schemas.html#GUID-1E645D09-F91F-4BA6-A286-57C5EC66321D
[3]: https://github.com/oracle/db-sample-schemas/releases/latest
[4]: https://stackoverflow.com/questions/69381749/where-is-the-oracle-instant-client-timezone-file-located
[5]: https://doc.rust-lang.org/stable/unstable-book/language-features/min-specialization.html#min_specialization