# Statement Arguments

`query`, `query_single` and `execute` take a tuple of arguments (or a single argument). The latter can be specified as positional arguments or as name-value tuples. For example, to execute the above SELECT we can call `query` using a positional argument as:

```rust,ignore
let row = stmt.query_single(103)?;
```

or bind a value to `:id` by name as:

```rust,ignore
let row = stmt.query_single((":ID", 103))?;
```

The leading colon in name part of the name-value argument tuple is optional. Depending on your preferences and/or tooling you might specify parameter placeholder name to bind argument value to without a colon:

```rust,ignore
let row = stmt.query_single(("ID", 103))?;
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
