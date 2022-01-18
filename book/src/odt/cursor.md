# Cursor

Cursors can be returned explicitly:

```rust,ignore
let stmt = session.prepare("
    BEGIN
        OPEN :emp FOR
            SELECT department_name, first_name, last_name, salary
              FROM hr.employees e
              JOIN hr.departments d
                ON d.department_id = e.department_id;
    END;
")?;
let mut cursor = Cursor::new(&stmt)?;
stmt.execute(&mut cursor)?;
let rows = cursor.rows()?;
// ...
```

Or, beginning with Oracle 12.1, implicitly:

```rust,ignore
let stmt = session.prepare("
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
