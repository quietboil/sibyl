# SQL Statement Execution

All SQL or PL/SQL statements must be prepared before they can be executed:

```rust,ignore
let stmt = session.prepare("
    SELECT employee_id, last_name, first_name
      FROM hr.employees
     WHERE manager_id = :id
  ORDER BY employee_id
")?;
```

A prepared statement can be executed either with the `query`, `query_single` or `execute` methods:
- `query` is used for `SELECT` statements. In fact, Sibyl will complain if you try to `query` any other statement.
- `query_single` is a variant of `query` that returns a single row. It's a convenience method that allows skipping boilerplate of extracting only one row from a result set when it is known upfront that only one row (or none) is expected.
- `execute` is used for all other, non-SELECT, DML and DDL.

## Arguments

`query`, `query_single` and `execute` take a tuple of arguments (or a single argument). The latter can be specified as positional arguments or as name-value tuples. For example, to execute the above SELECT we can call `query` using a positional argument as:

```rust,ignore
let row = stmt.query_single(103)?;
```

or bind a value to `:id` by name as:

```rust,ignore
let row = stmt.query_single((":ID", 103))?;
```

> Note one caveat - until [min_specialization][1] is stabilized Sibyl has no way of distinguishing whether a 2-item tuple is used to pass a single named argument or 2 positional arguments. For the moment you must use a 3-item tuple with a unit type as the last item when you are passing 2 positional arguments. The unit type is skipped, so effectively only first 2 arguments are used. For example:

```rust,ignore
let stmt = session.prepare("
    SELECT department_id, manager_id
      FROM hr.departments
     WHERE department_name = :DEPARTMENT_NAME
       AND location_id = :LOCATION_ID
")?;
let rows = stmt.query(( "Administration", 1700, () ))?;
```

In most cases which binding style to use is a matter of convenience and/or personal preferences. However, in some cases named arguments would be preferable and less ambiguous. For example, statement might change during development and thus force the change in argument positions. Also SQL and PL/SQL statements have different interpretation of a parameter position. SQL statements create positions for every parameter but allow a single argument to be used for the primary parameter and all its duplicates. PL/SQL on the other hand creates positions for unique parameter names and this might make positioning arguments correctly a bit awkward when there is more than one "duplicate" name in a statement.

For example, the following (contrived) `INSERT` would need its arguments to be bound differently depending on whether it is defined as a standalone SQL or as a (part of a) PL/SQL:

```rust,ignore
let stmt = session.prepare("
    INSERT INTO hr.locations
        (location_id, state_province, city, postal_code, street_address)
    VALUES
        (:id, :na, :na, :code, :na)
")?;
stmt.execute( (3333, "N/A", (), "00000", ()) )?;
// :NA'a first pos __^---^
// while ___________________^^____and____^^
// are its duplicate positions
```

The duplicate position can be skipped using `()` as in the example. However, when it is a part of PL/SQL:

```rust,ignore
let stmt = session.prepare("
  BEGIN
    INSERT INTO hr.locations
        (location_id, state_province, city, postal_code, street_address)
    VALUES
        (:id, :na, :na, :code, :na);
  END;
")?;
stmt.execute( ( 3333, "N/A", "00000" ) )?;
```

Only 3 position are possible as there are only 3 unique names.

`execute` also allows execution of statements with OUT (or INOUT) parameters. For example:

```rust,ignore
let stmt = session.prepare("
    INSERT INTO hr.departments
           ( department_id, department_name, manager_id, location_id )
    VALUES ( hr.departments_seq.nextval, :department_name, :manager_id, :location_id )
 RETURNING department_id
      INTO :department_id
")?;
let mut department_id: u32 = 0;
let num_inserted = stmt.execute(
    (
        (":DEPARTMENT_NAME", "Security"         ),
        (":MANAGER_ID",      ""                 ),
        (":LOCATION_ID",     1700               ),
        (":DEPARTMENT_ID",   &mut department_id ),
    )
)?;
```

`execute` returns the number of rows affected by the statement. `query` returns what is colloquially called a "streaming iterator" which is typically iterated using `while`. For example (continuing the SELECT example from above):

```rust,ignore
let mut employees = HashMap::new();
let stmt = session.prepare("
    SELECT employee_id, last_name, first_name
      FROM hr.employees
     WHERE manager_id = :id
  ORDER BY employee_id
")?;
let rows = stmt.query(103)?;
while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get(0)?;
    let last_name : &str  = row.get(1)?;
    let first_name : Option<&str> = row.get(2)?;
    let name = first_name.map_or(last_name.to_string(),
        |first_name| format!("{}, {}", last_name, first_name)
    );
    employees.insert(employee_id, name);
}
```

There are a few notable points of interest in the last example:
- Sibyl uses 0-based column indexing in a projection.
- `LAST_NAME` and `FIRST_NAME` are retrieved as `&str`. This is fast as they borrow directly from the respective column buffers. However, those values will only be valid during the lifetime of the row. If the value needs to continue to exist beyond the lifetime of a row, it should be retrieved as a `String`.

> Note that while Sibyl expects 0-based indexes to reference projection columns, it also accepts column names. Thus, the row processing loop of the previous example can be written as:

```rust,ignore
while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get("EMPLOYEE_ID")?;
    let last_name : &str  = row.get("LAST_NAME")?;
    let first_name : Option<&str> = row.get("FIRST_NAME")?;
    let name = first_name.map_or(last_name.to_string(),
        |first_name| format!("{}, {}", last_name, first_name)
    );
    employees.insert(employee_id, name);
}
```

**Note** that all examples use all upper case column and parameter names. This is not really necessary as Sibyl treat them as case-insensitive. However, using all upper case gives Sibyl a chance to locate a column (or a parameter placeholder) without converting the name to upper case first (to match the Oracle reported names), thus avoiding temporary string allocation and upper case conversion. Of course, you can always maintain an `enum` for a select list, thus using indexes, which are the speediest way to get to the data anyway.

```rust,ignore
enum Col { EmployeeId, LastName, FirstName }

while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get(Col::EmployeeId as usize)?;
    let last_name : &str  = row.get(Col::LastName as usize)?;
    let first_name : Option<&str> = row.get(Col::FirstName as usize)?;
    // ...
}
```

Or to be extra fancy:

```rust,ignore
#[derive(Clone,Copy)]
enum Col { EmployeeId, LastName, FirstName }

impl sibyl::Position for Col {
    fn index(&self) -> Option<usize> { Some(*self as _) }
}

impl std::fmt::Display for Col {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        static COLS : [&str;3] = ["EMPLOYEE_ID", "LAST_NAME", "FIRST_NAME"];
        let i = *self as usize;
        f.write_str(COLS[i])
    }
}

while let Some( row ) = rows.next()? {
    let employee_id : u32 = row.get(Col::EmployeeId)?;
    let last_name : &str  = row.get(Col::LastName)?;
    let first_name : Option<&str> = row.get(Col::FirstName)?;
    // ...
}
```

Of course, that's a lot of boilerplate, which would benefit from a `derive` macro. Maybe we'll get to that eventually :-)

[1]: https://doc.rust-lang.org/stable/unstable-book/language-features/min-specialization.html#min_specialization