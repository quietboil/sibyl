# Row ID

Oracle ROWID can be selected and retrieved explicitly into an instance of the `RowID`. However, one interesting case is SELECT FOR UPDATE queries where Oracle returns ROWIDs implicitly. Those can be retrieved using `Row::rowid` method.

```rust,ignore
let stmt = session.prepare("
    SELECT manager_id
      FROM hr.employees
     WHERE employee_id = :id
       FOR UPDATE
")?;
let row = stmt.query_single(107)?;
let rowid = row.rowid()?;

let manager_id: u32 = row.get(0)?;
assert_eq!(manager_id, 103);

let stmt = session.prepare("
    UPDATE hr.employees
       SET manager_id = :manager_id
     WHERE rowid = :row_id
")?;
let num_updated = stmt.execute((
    ( ":MANAGER_ID", 102 ),
    ( ":ROW_ID",  &rowid ),
))?;
assert_eq!(num_updated, 1);
```
