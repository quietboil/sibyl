//! Nonblocking SQL statement methods

use super::{Statement, bind::Params, cols::{DEFAULT_LONG_BUFFER_SIZE, Columns}};
use crate::{Result, oci::*, Session, Error, Rows, Cursor, ToSql, Row};
use parking_lot::RwLock;
use once_cell::sync::OnceCell;

impl<'a> Statement<'a> {
    /// Creates a new statement
    pub(crate) async fn new(sql: &str, session: &'a Session<'a>) -> Result<Statement<'a>> {
        let err = Handle::<OCIError>::new(session)?;
        let stmt = futures::StmtPrepare::new(session.get_svc(), &err, sql).await?;
        let params = Params::new(&stmt, &err)?.map(|params| RwLock::new(params));
        let stmt = Self {session, svc: session.get_svc(), stmt, params, cols: OnceCell::new(), err, max_long: DEFAULT_LONG_BUFFER_SIZE};
        stmt.set_prefetch_rows(10)?;
        Ok(stmt)
    }

    /// Binds provided arguments to SQL parameter placeholders. Returns indexes of parameter placeholders for the OUT args.
    fn bind_args(&self, args: &mut impl ToSql) -> Result<()> {
        if let Some(params) = &self.params {
            params.write().bind_args(&self.stmt, &self.err, args)
        } else {
            Ok(())
        }
    }

    /// Executes the prepared statement. Returns the OCI result code from OCIStmtExecute.
    async fn exec(&self, stmt_type: u16, args: &mut impl ToSql) -> Result<i32> {
        self.bind_args(args)?;
        futures::StmtExecute::new(self.svc.clone(), &self.err, &self.stmt, stmt_type).await
    }

    /**
    Executes the prepared statement. Returns the number of rows affected.

    # Parameters

    * `args` - SQL statement arguments - a single argument or a tuple of arguments

    Where each argument can be represented by:
    - a value: `val` (IN)
    - a reference: `&val` (IN)
    - a mutable reference: `&mut val` (OUT or INOUT)
    - a 2-item tuple where first item is a parameter name: `(":NAME", val)`

    # Example

    ```
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = session.prepare("
        INSERT INTO hr.departments
               ( department_id, department_name, manager_id, location_id )
        VALUES ( hr.departments_seq.nextval, :department_name, :manager_id
               , (SELECT location_id FROM hr.locations WHERE city = :city)
               )
        RETURNING department_id INTO :department_id
    ").await?;
    let mut department_id = 0u32;

    let num_updated_rows = stmt.execute((
        ( ":DEPARTMENT_NAME", "Security"         ),
        ( ":MANAGER_ID",      ""                 ),
        ( ":CITY",            "Seattle"          ),
        ( ":DEPARTMENT_ID",   &mut department_id ),
    )).await?;

    assert_eq!(num_updated_rows, 1);
    assert!(!stmt.is_null(":DEPARTMENT_ID")?);
    assert!(department_id > 0);
    # session.rollback().await?;
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn execute(&self, mut args: impl ToSql) -> Result<usize> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type == OCI_STMT_SELECT {
            return Err( Error::new("Use `query` to execute SELECT") );
        }
        self.exec(stmt_type, &mut args).await?;
        let num_rows = self.row_count()?;
        if let Some(params) = &self.params {
            if num_rows == 0 {
                params.write().set_out_to_null();
            }
            params.read().set_out_data_len(&mut args);
        }
        Ok(num_rows)
    }

    /**
    Executes the prepared statement. Returns "streaming iterator" over the returned rows.

    # Parameters

    * `args` - SQL statement arguments - a single argument or a tuple of arguments

    Where each argument can be represented by:
    - a value: `val` (IN)
    - a reference: `&val` (IN)
    - a 2-item tuple where first item is a parameter name: `(":NAME", val)`

    # Example

    ```
    # use std::collections::HashMap;
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = session.prepare("
        SELECT employee_id, last_name, first_name
          FROM hr.employees
         WHERE manager_id = :id
      ORDER BY employee_id
    ").await?;
    stmt.set_prefetch_rows(5)?;

    let rows = stmt.query(103).await?; // 103 is Alexander Hunold

    let mut subs = HashMap::new();
    while let Some( row ) = rows.next().await? {
        // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
        let id : u32 = row.get(0)?;
        // Same for the LAST_NAME.
        // Note that `last_name` is retrieved as a slice. This is fast as it
        // borrows directly from the column buffer, but it can only live until
        // the end of the current scope, i.e. only during the lifetime of the
        // current row.
        let last_name : &str = row.get(1)?;
        // FIRST_NAME is NULL-able...
        let first_name : Option<&str> = row.get(2)?;
        let name = first_name.map_or(last_name.to_string(),
            |first_name| format!("{}, {}", last_name, first_name)
        );
        subs.insert(id, name);
    }
    assert_eq!(stmt.row_count()?, 4);
    assert_eq!(subs.len(), 4);
    assert!(subs.contains_key(&104), "Bruce Ernst");
    assert!(subs.contains_key(&105), "David Austin");
    assert!(subs.contains_key(&106), "Valli Pataballa");
    assert!(subs.contains_key(&107), "Diana Lorentz");
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn query(&'a self, mut args: impl ToSql) -> Result<Rows<'a>> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Use `execute` to execute statements other than SELECT") );
        }
        let res = self.exec(stmt_type, &mut args).await?;

        if self.cols.get().is_none() {
            let cols = Columns::new(Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), self.max_long)?;
            self.cols.get_or_init(|| RwLock::new(cols));
        }

        match res {
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO | OCI_NO_DATA => {
                Ok( Rows::from_query(res, self) )
            }
            _ => Err( Error::oci(&self.err, res) )
        }
    }

    /**
    Convenience method to execute a query that returns a single rows.

    If the query returns more than one row, `query_single` will return only the first
    row and ignore the rest.

    # Parameters

    * `args` - SQL statement arguments - a single argument or a tuple of arguments

    Where each argument can be represented by:
    - a value: `val` (IN)
    - a reference: `&val` (IN)
    - a 2-item tuple where first item is a parameter name: `(":NAME", val)`

    # Returns

    - `None` - if query did not return any rows
    - `Some(row) - a single row (even if query returned more than one row)

    # Example

    ```
    use std::collections::HashMap;

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = session.prepare("
        SELECT country_id, state_province, city, postal_code, street_address
          FROM hr.locations
         WHERE location_id = :id
    ").await?;

    let row = stmt.query_single(1800).await?;

    assert!(row.is_some());
    let row = row.unwrap();
    let country_id     : &str = row.get(0)?;
    let state_province : &str = row.get(1)?;
    let city           : &str = row.get(2)?;
    let postal_code    : &str = row.get(3)?;
    let street_address : &str = row.get(4)?;
    assert_eq!(country_id, "CA");
    assert_eq!(state_province, "Ontario");
    assert_eq!(city, "Toronto");
    assert_eq!(postal_code, "M5V 2L7");
    assert_eq!(street_address, "147 Spadina Ave");
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn query_single(&'a self, mut args: impl ToSql) -> Result<Option<Row<'a>>> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Use `execute` to execute statements other than SELECT") );
        }
        self.set_prefetch_rows(1)?;
        let res = self.exec(stmt_type, &mut args).await?;

        if self.cols.get().is_none() {
            let cols = Columns::new(Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), self.max_long)?;
            self.cols.get_or_init(|| RwLock::new(cols));
        }

        match res {
            OCI_NO_DATA => Ok(None),
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Rows::from_query(res, self).single().await,
            _ => Err( Error::oci(&self.err, res) )
        }
    }

    /**
    Retrieves a single implicit result (cursor) in the order in which they were returned
    from the PL/SQL procedure or block. If no more results are available, then `None` is
    returned.

    PL/SQL provides a subprogram RETURN_RESULT in the DBMS_SQL package to return the result
    of an executed statement. Only SELECT query result-sets can be implicitly returned by a
    PL/SQL procedure or block.

    `next_result` can be called iteratively by the application to retrieve each implicit
    result from an executed PL/SQL statement. Applications retrieve each result-set sequentially
    but can fetch rows from any result-set independently.

    # Example

    ```
    use sibyl::Number;
    use std::cmp::Ordering::Equal;

    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = session.prepare("
        DECLARE
            c1 SYS_REFCURSOR;
            c2 SYS_REFCURSOR;
        BEGIN
            OPEN c1 FOR
                SELECT department_name, first_name, last_name, salary
                  FROM (
                        SELECT first_name, last_name, salary, department_id
                             , ROW_NUMBER() OVER (ORDER BY salary) ord
                          FROM hr.employees
                       ) e
                  JOIN hr.departments d
                    ON d.department_id = e.department_id
                 WHERE ord = 1
            ;
            DBMS_SQL.RETURN_RESULT (c1);

            OPEN c2 FOR
                SELECT department_name, first_name, last_name, salary
                  FROM (
                        SELECT first_name, last_name, salary, department_id
                             , MEDIAN(salary) OVER () median_salary
                          FROM hr.employees
                       ) e
                  JOIN hr.departments d
                    ON d.department_id = e.department_id
                 WHERE salary = median_salary
              ORDER BY department_name, last_name, first_name
            ;
            DBMS_SQL.RETURN_RESULT (c2);
        END;
    ").await?;
    let expected_lowest_salary = Number::from_int(2100, &session)?;
    let expected_median_salary = Number::from_int(6200, &session)?;

    stmt.execute(()).await?;

    let lowest_payed_employee = stmt.next_result().await?.unwrap();

    let rows = lowest_payed_employee.rows().await?;
    let row = rows.next().await?.unwrap();

    let department_name : &str = row.get(0)?;
    let first_name : &str = row.get(1)?;
    let last_name : &str = row.get(2)?;
    let salary : Number = row.get(3)?;

    assert_eq!(department_name, "Shipping");
    assert_eq!(first_name, "TJ");
    assert_eq!(last_name, "Olson");
    assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

    let row = rows.next().await?;
    assert!(row.is_none());

    let median_salary_employees = stmt.next_result().await?.unwrap();

    let rows = median_salary_employees.rows().await?;

    let row = rows.next().await?.unwrap();
    let department_name : &str = row.get(0)?;
    let first_name : &str = row.get(1)?;
    let last_name : &str = row.get(2)?;
    let salary : Number = row.get(3)?;

    assert_eq!(department_name, "Sales");
    assert_eq!(first_name, "Amit");
    assert_eq!(last_name, "Banda");
    assert_eq!(salary.compare(&expected_median_salary)?, Equal);

    let row = rows.next().await?.unwrap();

    let department_name : &str = row.get(0)?;
    let first_name : &str = row.get(1)?;
    let last_name : &str = row.get(2)?;
    let salary : Number = row.get(3)?;

    assert_eq!(department_name, "Sales");
    assert_eq!(first_name, "Charles");
    assert_eq!(last_name, "Johnson");
    assert_eq!(salary.compare(&expected_median_salary)?, Equal);

    let row = rows.next().await?;
    assert!(row.is_none());

    assert!(stmt.next_result().await?.is_none());
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn next_result(&'a self) -> Result<Option<Cursor<'a>>> {
        let res = futures::StmtGetNextResult::new(self.svc.clone(), &self.stmt, &self.err).await?;
        if let Some(stmt) = res {
            Ok(Some(Cursor::implicit(stmt, self)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn async_query() -> Result<()> {
        block_on(async {
            use std::env;

            let oracle = crate::env()?;

            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            let stmt = session.prepare("
                SELECT employee_id
                  FROM (
                        SELECT employee_id
                             , row_number() OVER (ORDER BY hire_date) AS hire_date_rank
                          FROM hr.employees
                       )
                 WHERE hire_date_rank = 1
            ").await?;
            let row = stmt.query_single(()).await?.unwrap();
            let id : usize = row.get(0)?;
            assert_eq!(id, 102);

            Ok(())
        })
    }

    #[test]
    fn plsql_args() -> std::result::Result<(),Box<dyn std::error::Error>> {
        block_on(async {
            let dbname = std::env::var("DBNAME")?;
            let dbuser = std::env::var("DBUSER")?;
            let dbpass = std::env::var("DBPASS")?;
            let oracle = env()?;
            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                BEGIN
                    SELECT city, street_address
                      INTO :city, :street_address
                      FROM hr.locations
                     WHERE location_id = :location_id;
                EXCEPTION
                    WHEN NO_DATA_FOUND THEN
                        :city := 'Unknown';
                        :street_address := NULL;
                END;
            ").await?;
            let mut city = String::with_capacity(30);
            let mut addr = String::with_capacity(40);
            let num_rows = stmt.execute((
                ( ":LOCATION_ID",    2500      ),
                ( ":CITY",           &mut city ),
                ( ":STREET_ADDRESS", &mut addr )
            )).await?;

            assert_eq!(num_rows, 1);
            assert!(!stmt.is_null(":CITY")?);
            assert_eq!(city, "Oxford");
            assert!(!stmt.is_null(":STREET_ADDRESS")?);
            assert_eq!(addr, "Magdalen Centre, The Oxford Science Park");

            let num_rows = stmt.execute((
                ( ":LOCATION_ID",    2400      ),
                ( ":CITY",           &mut city ),
                ( ":STREET_ADDRESS", &mut addr )
            )).await?;

            assert_eq!(num_rows, 2); // no idea why... blocking mode is fine
            assert!(!stmt.is_null(":CITY")?);
            assert_eq!(city, "London");
            assert!(!stmt.is_null(":STREET_ADDRESS")?);
            assert_eq!(addr, "8204 Arthur St");

            let num_rows = stmt.execute((
                ( ":LOCATION_ID",    2200      ),
                ( ":CITY",           &mut city ),
                ( ":STREET_ADDRESS", &mut addr )
            )).await?;

            assert_eq!(num_rows, 2);
            assert!(!stmt.is_null(":CITY")?);
            assert_eq!(city, "Sydney");
            assert!(!stmt.is_null(":STREET_ADDRESS")?);
            assert_eq!(addr, "12-98 Victoria Street");

            let num_rows = stmt.execute((
                ( ":LOCATION_ID",    3300      ),
                ( ":CITY",           &mut city ),
                ( ":STREET_ADDRESS", &mut addr )
            )).await?;

            assert_eq!(num_rows, 2);
            assert!(!stmt.is_null(":CITY")?);
            assert_eq!(city, "Unknown");
            assert!(stmt.is_null(":STREET_ADDRESS")?);

            session.rollback().await?;
            Ok(())
        })
    }

    #[test]
    fn plsql_one_out_arg() -> std::result::Result<(),Box<dyn std::error::Error>> {
        block_on(async {
            let dbname = std::env::var("DBNAME")?;
            let dbuser = std::env::var("DBUSER")?;
            let dbpass = std::env::var("DBPASS")?;
            let oracle = env()?;
            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                BEGIN
                    SELECT street_address
                      INTO :addr
                      FROM hr.locations
                     WHERE location_id = :id;
                END;
            ").await?;

            let mut addr = String::with_capacity(40);

            let num_rows = stmt.execute((
                ( ":ID",   2500      ),
                ( ":ADDR", &mut addr )
            )).await?;

            assert_eq!(num_rows, 1);
            assert!(!stmt.is_null(":ADDR")?);
            assert_eq!(addr, "Magdalen Centre, The Oxford Science Park");

            let num_rows = stmt.execute((
                ( ":ID",   2400      ),
                ( ":ADDR", &mut addr )
            )).await?;

            assert_eq!(num_rows, 2);
            assert!(!stmt.is_null(":ADDR")?);
            assert_eq!(addr, "8204 Arthur St");

            let num_rows = stmt.execute((
                ( ":ID",   2200      ),
                ( ":ADDR", &mut addr )
            )).await?;

            assert_eq!(num_rows, 2);
            assert!(!stmt.is_null(":ADDR")?);
            assert_eq!(addr, "12-98 Victoria Street");

            let num_rows = stmt.execute((
                ( ":ID",   3300      ),
                ( ":ADDR", &mut addr )
            )).await?;

            assert_eq!(num_rows, 0);
            assert!(stmt.is_null(":ADDR")?);

            let num_rows = stmt.execute((
                ( ":ID",   2200      ),
                ( ":ADDR", &mut addr )
            )).await?;

            assert_eq!(num_rows, 2);
            assert!(!stmt.is_null(":ADDR")?);
            assert_eq!(addr, "12-98 Victoria Street");

            let num_rows = stmt.execute((
                ( ":ID",   2500      ),
                ( ":ADDR", &mut addr )
            )).await?;

            assert_eq!(num_rows, 2);
            assert!(!stmt.is_null(":ADDR")?);
            assert_eq!(addr, "Magdalen Centre, The Oxford Science Park");

            session.rollback().await?;
            Ok(())
        })
    }

    #[test]
    fn update_many_rows() -> std::result::Result<(),Box<dyn std::error::Error>> {
        block_on(async {
            let dbname = std::env::var("DBNAME")?;
            let dbuser = std::env::var("DBUSER")?;
            let dbpass = std::env::var("DBPASS")?;
            let oracle = env()?;
            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                UPDATE hr.employees
                   SET salary = Round(salary * :rate, -2)
                 WHERE manager_id = :manager_id
            ").await?;

            let num_rows = stmt.execute((
                (":MANAGER_ID", 103  ),
                (":RATE",       1.02 ),
            )).await?;
            assert_eq!(num_rows, 4);

            let num_rows = stmt.execute((
                (":MANAGER_ID", 108  ),
                (":RATE",       1.03 ),
            )).await?;
            assert_eq!(num_rows, 5);

            session.rollback().await?;
            Ok(())
        })
    }

    #[test]
    fn single_row_query() -> Result<()> {
        block_on(async {
            let oracle = env()?;
            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");
            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                SELECT country_id, state_province, city, postal_code, street_address
                  FROM hr.locations
                 WHERE location_id = :id
            ").await?;
            let row = stmt.query_single(1800).await?;
            assert!(row.is_some());
            let row = row.unwrap();
            let country_id     : &str = row.get(0)?;
            let state_province : &str = row.get(1)?;
            let city           : &str = row.get(2)?;
            let postal_code    : &str = row.get(3)?;
            let street_address : &str = row.get(4)?;
            assert_eq!(country_id, "CA");
            assert_eq!(state_province, "Ontario");
            assert_eq!(city, "Toronto");
            assert_eq!(postal_code, "M5V 2L7");
            assert_eq!(street_address, "147 Spadina Ave");

            let stmt = session.prepare("
                SELECT location_id, state_province, city, postal_code, street_address
                  FROM hr.locations
                 WHERE country_id = :country_id
              ORDER BY location_id
            ").await?;
            let row = stmt.query_single("CA").await?;
            assert!(row.is_some());
            let row = row.unwrap();
            let location_id    : u16  = row.get(0)?;
            let state_province : &str = row.get(1)?;
            let city           : &str = row.get(2)?;
            let postal_code    : &str = row.get(3)?;
            let street_address : &str = row.get(4)?;
            assert_eq!(location_id, 1800);
            assert_eq!(state_province, "Ontario");
            assert_eq!(city, "Toronto");
            assert_eq!(postal_code, "M5V 2L7");
            assert_eq!(street_address, "147 Spadina Ave");

            Ok(())
        })
    }
}
