# Sibyl

Sibyl is an [OCI][1]-based driver for Rust applications to interface with Oracle databases.

## Example

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
                     , row_number() OVER (ORDER BY hire_date) ord
                  FROM hr.employees
                 WHERE hire_date >= :hire_date
               )
         WHERE ord = 1
    ")?;
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &oracle)?;
    let rows = stmt.query(&[ &date ])?;
    if let Some( row ) = rows.next()? {
        let first_name : Option<&str> = row.get("FIRST_NAME")?;
        let last_name : &str = row.get("LAST_NAME")?.unwrap();
        let name = first_name.map_or(last_name.to_string(), |first_name| format!("{}, {}", last_name, first_name));
        let hire_date : oracle::Date = row.get("HIRE_DATE")?.unwrap();
        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

        println!("{} was hired on {}", name, hire_date);
    } else {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}
```

## Notes on Building

Sibyl needs an installed Oracle client in order to link either `OCI.DLL` on Windows or `libclntsh.so` on Linux. The cargo build needs to know where that library is. You can supply that information via environment variable `OCI_LIB_DIR`. On Linux it would be the path to the `lib` directory with `libclntsh.so`. For example, you might build sibyl's example as:
```bash
OCI_LIB_DIR=/usr/lib/oracle/19.12/client64/lib cargo build --examples
```

On Windows the process is similar if the target environment is `gnu`. The `OCI_LIB_DIR` would point to the directory with `oci.dll`:
```bat
set OCI_LIB_DIR=%ORACLE_HOME%\bin
cargo build --examples
```

However, for `msvc` environment the `OCI_LIB_DIR` must point to the directory with `oci.lib`. For example, you might build that example as:
```bat
set OCI_LIB_DIR=%ORACLE_HOME%\oci\lib\msvc
cargo build --examples
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
fn main() {
    let oracle = sibyl::env().expect("Oracle OCI environment");
    let conn = oracle.connect("dbname", "username", "password").expect("New database connection");
    // ...
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
- `query` is used for `SELECT` statements. In fact, sibyl will complain if you try to `query` any other statement.
- `execute` is used for all other, non-SELECT, DML and DDL that do not have OUT parameters.
- `execute_into` is used with DML and DDL that have OUT parameters.

`query` and `execute` take a slice of IN arguments, which can be specified as positional arguments or as name-value tuples. For example, to execute the above SELECT we can call `query` using a positional argument as:

```rust
let rows = stmt.query(&[ &103 ])?;
```

or binding `:id` by name as:

```rust
let rows = stmt.query(&[
    &( ":ID", 103 )
])?;
```

In most cases which binding style to use is a matter of convenience and/or personal preferences. However, in some cases named arguments would be preferable and less ambiguous. For example, statement changes during development might force the change in argument positions. Also SQL and PL/SQL statements have different interpretation of a parameter position. SQL statements create positions for every parameter but allow a single argument to be used for the primary parameter and all its duplicares. PL/SQL on the other hand creates positions for unique parameter names and this might make positioning arguments correctly a bit awkward when there is more than one "duplicate" name in a statement.

`execute_into` allows execution of statements with OUT parameters. For example:

```rust
let stmt = conn.prepare("
    INSERT INTO hr.departments
           ( department_id, department_name, manager_id, location_id )
    VALUES ( hr.departments_seq.nextval, :department_name, :manager_id, :location_id )
 RETURNING department_id
      INTO :department_id
")?;
let mut department_id: u32 = 0;
let num_inserted = stmt.execute(&[
    &( ":DEPARTMENT_NAME", "Security" ),
    &( ":MANAGER_ID",      ""         ),
    &( ":LOCATION_ID",     1700       ),
], &mut [
    &mut ( ":DEPARTMENT_ID", &mut department_id )
])?;
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
let rows = stmt.query(&[ &103 ])?;
while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get(0)?.unwrap();
    let last_name : &str  = row.get(1)?.unwrap();
    let first_name : Option<&str> = row.get(2)?;
    let name = first_name.map_or(last_name.to_string(), |first_name| format!("{}, {}", last_name, first_name));
    employees.insert(employee_id, name);
}
```
There are a few notable points of interest in the last example:
- Sibyl uses 0-based column indexing in a projection.
- Column value are returned as `Option`s. However, if a column is declared as `NOT NULL`, like `EMPLOYEE_ID` and `LAST_NAME`, the result will always be `Some` and therefore can be safely unwrapped.
- `LAST_NAME` and `FIRST_NAME` are retrieved as `&str`. This is fast as they are borrowed directly from the respective column buffers. However, those values will only be valid during the lifetime of the row. If the value needs to continue to exist beyond the lifetime of a row, it should be retrieved as a `String`.

**Note** that instead of column indexes sibyl also accept column names. The row processing loop of the previous example can be written as:

```rust
while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get("EMPLOYEE_ID")?.unwrap();
    let last_name : &str  = row.get("LAST_NAME")?.unwrap();
    let first_name : Option<&str> = row.get("FIRST_NAME")?;
    let name = first_name.map_or(last_name.to_string(), |first_name| format!("{}, {}", last_name, first_name));
    employees.insert(employee_id, name);
}
```

## Oracle Data Types

Sibyl provides API to access several Oracle native data types.

### Number
```rust
use sibyl::Number;

let pi = Number::pi(&oracle);
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

### Interval

There are 2 types of intervals:
- `IntervalYM` which is eqivalent to Oracle's INTERVAL YEAR TO MONTH,
- `IntervalDS` - INTERVAL DAY TO SECOND

```rust
use sibyl::{ TimestampTZ, IntervalDS };

let launch  = TimestampTZ::with_datetime(1969, 7, 16, 13, 32,  0, 0, "UTC", &oracle)?;
let landing = TimestampTZ::with_datetime(1969, 7, 24, 16, 50, 35, 0, "UTC", &oracle)?;
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
let rows = stmt.query(&[ &107 ])?;
let cur_row = rows.next()?.unwrap();
let rowid = row.get_rowid()?;

let manager_id: u32 = row.get(0)?.unwrap();
assert_eq!(manager_id, 103);

let stmt = conn.prepare("
    UPDATE hr.employees
       SET manager_id = :manager_id
     WHERE rowid = :row_id
")?;
let num_updated = stmt.execute(&[
    &( ":MANAGER_ID", 102 ),
    &( ":ROW_ID",  &rowid )
])?;
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
stmt.execute_into(&[], &mut [ &mut cursor ])?;
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
stmt.execute(&[])?;
if let Some( cursor ) = stmt.next_result()? {
    let rows = cursor.rows()?;
    // ...
}
```

## Testing

Some of sibyl's tests connect to the database and expect certain objects to exist in it and certain privileges granted:
- At least the HR demo schema should be [installed][2].
- While there is no need to install other demo schemas at least `MEDIA_DIR` should be created (see `$ORACLE_HOME/demo/schema/mk_dir.sql`) and point to the directory with demo files that can be found in `product_media` in the [db-sample-schemas.zip][3].
- Some of the LOB tests need text files with the the expected content. Those can be found in `etc/media` and copied into `MEDIA_DIR`.
- A test user should be created. That user needs acccess to the HR schema and to the `MEDIA_DIR` directory. See `etc/create_sandbox.sql` for an example of how it can be accomplished.
- Tests that connect to the database use environment variables - DBNAME, DBUSER and DBPASS - to identify the database, user and password respectively. These variables should be set before executing `cargo test`.

## Limitations

At this time sibyl provides only the most commonly needed means to interface with the Oracle database. Some of the missing features are:
- Non-blocking execution
- Array interface for multi-row operations
- User defined data types
- PL/SQL collections and tables
- Objects
- JSON data
- LDAP and proxy authentications
- Global transactions
- Session and connection pooling
- High Availability
- Continuous query and publish-subscribe notifications
- Advanced queuing
- Shards
- Direct path load

Some of these features will be added in the upcoming releases. Some will be likely kept on a backburner until the need arises or they are explicitly requested. And some might never be implemented.

[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/index.html
[2]: https://docs.oracle.com/en/database/oracle/oracle-database/19/comsc/installing-sample-schemas.html#GUID-1E645D09-F91F-4BA6-A286-57C5EC66321D
[3]: https://github.com/oracle/db-sample-schemas/releases/latest
