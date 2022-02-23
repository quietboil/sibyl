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

`execute` returns the number of rows affected by the statement.

`query` returns what is colloquially called a "streaming iterator" which is typically iterated using `while`. For example (continuing the SELECT example from above):

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