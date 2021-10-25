/*!
Sibyl is an [OCI](https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/index.html)-based
driver for Rust applications to interface with Oracle databases.

## Example

```
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
    let mut rows = stmt.query(&[ &date ])?;
    if let Some( row ) = rows.next()? {
        let first_name : Option<&str> = row.get(0)?;
        let last_name : &str = row.get(1)?.unwrap();
        let name = first_name.map_or(last_name.to_string(),
            |first_name| format!("{}, {}", last_name, first_name)
        );
        let hire_date : oracle::Date = row.get(2)?.unwrap();
        let hire_date = hire_date.to_string("fmMonth DD, YYYY")?;

        println!("{} was hired on {}", name, hire_date);
    } else {
        println!("No one was hired after {}", date.to_string("fmMonth DD, YYYY")?);
    }
    Ok(())
}
```

## Notes on Building

Sibyl needs an installed Oracle client in order to link either to `OCI.DLL` on Windows or to `libclntsh.so`
on Linux. The cargo build needs to know where that library is. You can provide that information via environment
variable `OCI_LIB_DIR` in Windows or `LIBRARY_PATH` in Linux. In Linux `LIBRARY_PATH` would include the path to
the `lib` directory with `libclntsh.so`. For example,
you might build sibyl's example as:

```bash
LIBRARY_PATH=/usr/lib/oracle/21/client64/lib cargo build --examples
```

In Windows the process is similar if the target environment is `gnu`. There the `OCI_LIB_DIR` would point to the
directory with `oci.dll`:

```bat
set OCI_LIB_DIR=%ORACLE_HOME%\bin
cargo build --examples
```

However, for `msvc` environment the `OCI_LIB_DIR` must point to the directory with `oci.lib`. For example,
you might build provided example application as:

```bat
set OCI_LIB_DIR=%ORACLE_HOME%\oci\lib\msvc
cargo build --examples
```

## Usage

### Environment

The OCI environment handle must be created before any other OCI function can be called. While there can
be many environments - for example, one can create an environment per connection - usually one is enought.
Sibyl initializes it to be the most compatible with Rust requirements - thread-safe using UTF8 character
encoding. That single environment handle can be created in `main` and then passed around:

```
fn main() {
    let oracle = sibyl::env().expect("Oracle OCI environment");
    // ...
}
```

Note that some functions will need a direct reference to this handle, so instead of passing it around
some applications might prefer to create it statically:

```ignore
use sibyl::Environment;
use lazy_static::lazy_static;

lazy_static!{
    pub static ref ORACLE : Environment = sibyl::env().expect("Oracle OCI environment");
}
```

Then later one would be able to create, for example, a current timestamp as:

```
# use sibyl as oracle;
# let ORACLE : oracle::Environment = oracle::env()?;
use sibyl::TimestampTZ;

let current_timestamp = TimestampTZ::from_systimestamp(&ORACLE)?;
# Ok::<(),oracle::Error>(())
```

### Connections

Use `Environment::connect` method to connect to a database:

```
fn main() -> Result<(),Box<dyn std::error::Error>> {
    let dbname = std::env::var("DBNAME")?;
    let dbuser = std::env::var("DBUSER")?;
    let dbpass = std::env::var("DBPASS")?;
    let oracle = sibyl::env()?;

    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    // ...
    Ok(())
}
```

Where `dbname` can be any database reference/address that is acceptable to Oracle
clients - from local TNS name to Eazy Connect identifier to a connect descriptor.

### SQL Statement Execution

All SQL or PL/SQL statements must be prepared before they can be executed:

```
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = sibyl::env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
let stmt = conn.prepare("
    SELECT employee_id, last_name, first_name
      FROM hr.employees
     WHERE manager_id = :id
  ORDER BY employee_id
")?;
# Ok::<(),Box<dyn std::error::Error>>(())
```

A prepared statement can be executed either with the `query` or `execute` or `execute_into` methods:
- `query` is used for `SELECT` statements. In fact, it will complain if you try to `query` any other statement.
- `execute` is used for all other, non-SELECT, DML and DDL that do not have OUT parameters.
- `execute_into` is used with DML and DDL that have OUT parameters.

`query` and `execute` take a slice of IN arguments, which can be specified as positional
arguments or as name-value tuples. For example, to execute the above SELECT we can call
`query` using a positional argument as:

```
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = sibyl::env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
# let stmt = conn.prepare("
#    SELECT employee_id, last_name, first_name
#      FROM hr.employees
#     WHERE manager_id = :id
#  ORDER BY employee_id
# ")?;
let rows = stmt.query(&[ &103 ])?;
# Ok::<(),Box<dyn std::error::Error>>(())
```

or binding `:id` by name as:

```
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = sibyl::env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
# let stmt = conn.prepare("
#    SELECT employee_id, last_name, first_name
#      FROM hr.employees
#     WHERE manager_id = :id
#  ORDER BY employee_id
# ")?;
let rows = stmt.query(&[
    &( ":ID", 103 )
])?;
# Ok::<(),Box<dyn std::error::Error>>(())
```

In most cases which binding style to use is a matter of convenience and/or personal preferences. However,
in some cases named arguments would be preferable and less ambiguous. For example, statement changes during
development might force the change in argument positions. Also SQL and PL/SQL statements have different
interpretation of a parameter position. SQL statements create positions for every parameter but allow a
single argument to be used for the primary parameter and all its duplicares. PL/SQL on the other hand creates
positions for unique parameter names and this might make positioning arguments correctly a bit awkward when
there is more than one "duplicate" name in a statement.

`execute_into` allows execution of statements with OUT parameters. For example:

```
# use sibyl::*;
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
let stmt = conn.prepare("
    INSERT INTO hr.departments
            ( department_id, department_name, manager_id, location_id )
    VALUES ( hr.departments_seq.nextval, :department_name, :manager_id, :location_id )
    RETURNING department_id
        INTO :department_id
")?;
let mut department_id: u32 = 0;
let num_rows = stmt.execute_into(&[
    &( ":DEPARTMENT_NAME", "Security" ),
    &( ":MANAGER_ID",      ""         ),
    &( ":LOCATION_ID",     1700      ),
], &mut [
    &mut ( ":DEPARTMENT_ID", &mut department_id )
])?;
assert_eq!(num_rows, 1);
assert!(!stmt.is_null(":DEPARTMENT_ID")?);
assert!(department_id > 0);
# conn.rollback()?;
# Ok::<(),Box<dyn std::error::Error>>(())
```

`execute` and `execute_into` return the number of rows affected by the statement. `query` returns what
is colloquially called a "streaming iterator" which is typically iterated using `while`. For example
(continuing the previous SELECT example):

```
# use std::collections::HashMap;
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = sibyl::env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
# let stmt = conn.prepare("
#    SELECT employee_id, last_name, first_name
#      FROM hr.employees
#     WHERE manager_id = :id
#  ORDER BY employee_id
# ")?;
let mut employees = HashMap::new();

let mut rows = stmt.query(&[ &103 ])?;
while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get(0)?.unwrap();
    let last_name : &str = row.get(1)?.unwrap();
    let first_name : Option<&str> = row.get(2)?;
    let name = first_name.map_or(last_name.to_string(),
        |first_name| format!("{}, {}", last_name, first_name)
    );
    employees.insert(employee_id, name);
}
# Ok::<(),Box<dyn std::error::Error>>(())
```

There are a few notable points of interest in the last example:
- Sibyl uses 0-based column indexing in a projection.
- Column values are returned as an `Option`. However, if a column is declared as NOT NULL,
like EMPLOYEE_ID and LAST_NAME, the result will always be `Some` and therefore can be safely
unwrapped.
- LAST_NAME and FIRST_NAME are retrieved as `&str`. This is fast as they are borrowed directly
from the respective column buffers. However those values will only be valid during the lifetime
of the row. If the value needs to continue to exist beyond the lifetime of a row, it should be
retrieved as a `String`.

**Note** that sibyl can also identify columns by their names. The row processing loop of the
previous example can be written as:

```rust
# use std::collections::HashMap;
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = sibyl::env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
# let stmt = conn.prepare("
#    SELECT employee_id, last_name, first_name
#      FROM hr.employees
#     WHERE manager_id = :id
#  ORDER BY employee_id
# ")?;
# let mut employees = HashMap::new();
# let mut rows = stmt.query(&[ &103 ])?;
while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get("EMPLOYEE_ID")?.unwrap();
    let last_name : &str  = row.get("LAST_NAME")?.unwrap();
    let first_name : Option<&str> = row.get("FIRST_NAME")?;
    let name = first_name.map_or(last_name.to_string(), |first_name| format!("{}, {}", last_name, first_name));
    employees.insert(employee_id, name);
}
# Ok::<(),Box<dyn std::error::Error>>(())
```

## Oracle Data Types

Sibyl provides API to access several Oracle native data types.

### Number

```
use sibyl::Number;
let oracle = sibyl::env()?;

let pi = Number::pi(&oracle);
let two = Number::from_int(2, &oracle)?;
let two_pi = pi.mul(&two)?;
let h = Number::from_string("6.62607004E-34", "9D999999999EEEE", &oracle)?;
let hbar = h.div(&two_pi)?;

assert_eq!(hbar.to_string("TME")?, "1.05457180013911265115394106872506677375E-34");
# Ok::<(),sibyl::Error>(())
```

### Date
```
use sibyl::Date;
let oracle = sibyl::env()?;

let mar28_1996 = Date::from_string("28-MAR-1996", "DD-MON-YYYY", &oracle)?;
let next_monday = mar28_1996.next_week_day("MONDAY")?;

assert_eq!(next_monday.to_string("DL")?, "Monday, April 01, 1996");
# Ok::<(),sibyl::Error>(())
```

### Timestamp

There are 3 types of timestamps:
- `Timestamp` which is equivalent to Oracle's TIMESTAMP,
- `TimestampTZ` - TIMESTAMP WITH TIME ZONE,
- `TimestampLTZ` - TIMESTAMP WITH LOCAL TIME ZONE

```
use sibyl::TimestampTZ;
let oracle = sibyl::env()?;

let ts = TimestampTZ::from_string(
    "July 20, 1969 8:18:04.16 pm UTC",
    "MONTH DD, YYYY HH:MI:SS.FF PM TZR",
    &oracle
)?;
assert_eq!(
    ts.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR", 3)?,
    "1969-07-20 20:18:04.160 UTC"
);
# Ok::<(),sibyl::Error>(())
```

### Interval

There are 2 types of intervals:
- `IntervalYM` which is eqivalent to Oracle's INTERVAL YEAR TO MONTH,
- `IntervalDS` - INTERVAL DAY TO SECOND

```
use sibyl::{ TimestampTZ, IntervalDS };
let oracle = sibyl::env()?;

let launch  = TimestampTZ::with_datetime(1969, 7, 16, 13, 32,  0, 0, "UTC", &oracle)?;
let landing = TimestampTZ::with_datetime(1969, 7, 24, 16, 50, 35, 0, "UTC", &oracle)?;
let duration : IntervalDS = landing.subtract(&launch)?;

assert_eq!(duration.to_string(1,3)?, "+8 03:18:35.000");
# Ok::<(),sibyl::Error>(())
```

### RowID

Oracle ROWID can be selected and retrieved explicitly into an instance of the `RowID`.
However, one interesting case is SELECT FOR UPDATE queries where Oracle returns ROWIDs
implicitly. Those can be retrieved using `Row::get_rowid` method.

```
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = sibyl::env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
let stmt = conn.prepare("
    SELECT manager_id
      FROM hr.employees
     WHERE employee_id = :id
       FOR UPDATE
")?;
let mut rows = stmt.query(&[ &107 ])?;
if let Some( row ) = rows.next()? {
    let rowid = row.get_rowid()?;

    let manager_id: u32 = row.get(0)?.unwrap();
    assert_eq!(manager_id, 102);

    let stmt = conn.prepare("
        UPDATE hr.employees
           SET manager_id = :manager_id
         WHERE rowid = :row_id
    ")?;
    let num_updated = stmt.execute(&[
        &( ":MANAGER_ID", 102 ),
        &( ":ROW_ID",  &rowid )
    ])?;
    assert_eq!(num_updated, 1);
}
# Ok::<(),Box<dyn std::error::Error>>(())
```

### Cursors

Cursors can be returned explicitly:
```
# use sibyl::*;
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
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
# Ok::<(),Box<dyn std::error::Error>>(())
```

Or, beginning with Oracle 12.1, implicitly:
```
# let dbname = std::env::var("DBNAME")?;
# let dbuser = std::env::var("DBUSER")?;
# let dbpass = std::env::var("DBPASS")?;
# let oracle = sibyl::env()?;
# let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
let stmt = conn.prepare("
    DECLARE
        emp SYS_REFCURSOR;
    BEGIN
        OPEN emp FOR
            SELECT department_name, first_name, last_name, salary
              FROM hr.employees e
              JOIN hr.departments d
                ON d.department_id = e.department_id;
        DBMS_SQL.RETURN_RESULT(emp);
    END;
")?;
stmt.execute(&[])?;
if let Some( cursor ) = stmt.next_result()? {
    let rows = cursor.rows()?;
    // ...
}
# Ok::<(),Box<dyn std::error::Error>>(())
```
*/

mod oci;
#[macro_use] mod err;
mod handle;
mod desc;
mod attr;
mod param;
mod env;
mod types;
mod tosql;
mod tosqlout;
mod fromsql;
mod conn;
mod stmt;
mod rowid;
mod lob;

/**
    Allows parameter or column identification by either
    its numeric position or its name
*/
pub trait Position {
    fn index(&self) -> Option<usize>;
    fn name(&self)  -> Option<&str>;
}

impl Position for usize {
    fn index(&self) -> Option<usize> { Some(*self) }
    fn name(&self)  -> Option<&str>  { None }
}

impl Position for &str {
    fn index(&self) -> Option<usize> { None }
    fn name(&self)  -> Option<&str>  { Some(*self) }
}

/// Character set form
pub enum CharSetForm {
    Undefined = 0,
    Implicit = 1,
    NChar = 2
}

/// LOB cache control flags
pub enum Cache {
    No  = 0,
    Yes = 1,
}

/**
    Returns a new environment handle, which is then used by the OCI functions.

    While there can be multiple environments, most applications most likely will
    need only one.

    As nothing can outlive its environment, when only one environment is used,
    it might be created either in `main` function:

    ```
    use sibyl as oracle; // pun intended :)
    fn main() {
        let oracle = oracle::env().expect("Oracle OCI environment");
        // ...
    }
    ```

    and passed around, or it might be created statically:

    ```ignore
    use sibyl::Environment;
    lazy_static! {
        pub static ref ORACLE : Environment = sibyl::env().expect("Oracle OCI environment");
    }
    ```
*/
pub fn env() -> Result<Environment> {
    Environment::new()
}

pub use crate::{
    err::Error,
    env::Environment,
    conn::Connection,
    stmt::{
    Statement,
    cols::ColumnType,
    args::{ SqlInArg, SqlOutArg },
    rows::{ Rows, Row },
    cursor::Cursor
   },
    types::{
        number::Number, 
        date::Date, 
        raw::Raw, 
        varchar::Varchar
    },
    tosql::ToSql,
    tosqlout::ToSqlOut,
    fromsql::FromSql
};

pub type Result<T>          = std::result::Result<T, Error>;
pub type Timestamp<'a>      = types::timestamp::Timestamp<'a, oci::OCITimestamp>;
pub type TimestampTZ<'a>    = types::timestamp::Timestamp<'a, oci::OCITimestampTZ>;
pub type TimestampLTZ<'a>   = types::timestamp::Timestamp<'a, oci::OCITimestampLTZ>;
pub type IntervalYM<'a>     = types::interval::Interval<'a, oci::OCIIntervalYearToMonth>;
pub type IntervalDS<'a>     = types::interval::Interval<'a, oci::OCIIntervalDayToSecond>;
pub type CLOB<'a>           = lob::LOB<'a,oci::OCICLobLocator>;
pub type BLOB<'a>           = lob::LOB<'a,oci::OCIBLobLocator>;
pub type BFile<'a>          = lob::LOB<'a,oci::OCIBFileLocator>;
pub type RowID              = desc::Descriptor<oci::OCIRowid>;
