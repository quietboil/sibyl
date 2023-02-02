//! Blocking SQL statement methods

use super::{
    Statement, Cursor, Params, Columns, Rows,
    cols::DEFAULT_LONG_BUFFER_SIZE,
};
use crate::{Error, Result, oci::{self, *}, Session, ToSql, Row};
use parking_lot::RwLock;
use once_cell::sync::OnceCell;

impl<'a> Statement<'a> {
    /// Creates a new statement
    pub(crate) fn new(sql: &str, session: &'a Session) -> Result<Self> {
        let err = Handle::<OCIError>::new(session)?;
        let mut stmt = Ptr::<OCIStmt>::null();
        oci::stmt_prepare(
            session.as_ref(), stmt.as_mut_ptr(), &err,
            sql.as_ptr(), sql.len() as u32,
            OCI_NTV_SYNTAX, OCI_DEFAULT
        )?;
        let params = Params::new(&stmt, &err)?.map(|params| RwLock::new(params));
        let stmt = Self {session, svc: session.get_svc(), stmt, params, cols: OnceCell::new(), err, max_long: DEFAULT_LONG_BUFFER_SIZE};
        stmt.set_prefetch_rows(10)?;
        Ok(stmt)
    }

    /// Binds provided arguments to SQL parameter placeholders.
    fn bind_args(&self, args: &mut impl ToSql) -> Result<()> {
        if let Some(params) = &self.params {
            params.write().bind_args(&self.stmt, &self.err, args)
        } else {
            Ok(())
        }
    }

    /// Executes the prepared statement. Returns the OCI result code from OCIStmtExecute.
    fn exec(&self, stmt_type: u16, args: &mut impl ToSql) -> Result<i32>{
        self.bind_args(args)?;

        let iters: u32 = if stmt_type == OCI_STMT_SELECT { 0 } else { 1 };
        oci::stmt_execute(self.as_ref(), &self.stmt, &self.err, iters, 0, OCI_DEFAULT)
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
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        INSERT INTO hr.departments
               ( department_id, department_name, manager_id, location_id )
        VALUES ( hr.departments_seq.nextval, :department_name, :manager_id
               , (SELECT location_id FROM hr.locations WHERE city = :city)
               )
        RETURNING department_id INTO :department_id
    ")?;
    let mut department_id = 0u32;

    let num_updated_rows = stmt.execute((
        ( ":DEPARTMENT_NAME", "Security"         ),
        ( ":MANAGER_ID",      ""                 ),
        ( ":CITY",            "Seattle"          ),
        ( ":DEPARTMENT_ID",   &mut department_id ),
    ))?;

    assert_eq!(num_updated_rows, 1);
    assert!(!stmt.is_null(":DEPARTMENT_ID")?);
    assert!(department_id > 0);
    # session.rollback()?;
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn execute(&self, mut args: impl ToSql) -> Result<usize> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type == OCI_STMT_SELECT {
            return Err( Error::new("Use `query` to execute SELECT") );
        }
        self.exec(stmt_type, &mut args)?;
        let num_rows = self.row_count()?;
        if let Some(params) = &self.params {
            if num_rows == 0 {
                params.write().set_out_to_null();
            }
            params.read().update_out_args(&mut args)?;
        }
        Ok(num_rows)
    }

    /**
    Executes the prepared SELECT statement. Returns "streaming iterator" over the returned rows.

    # Parameters

    * `args` - SQL statement arguments - a single argument or a tuple of arguments

    Where each argument can be represented by:
    - a value: `val` (IN)
    - a reference: `&val` (IN)
    - a 2-item tuple where first item is a parameter name: `(":NAME", val)`

    # Example

    ```
    use std::collections::HashMap;

    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT employee_id, last_name, first_name
          FROM hr.employees
         WHERE manager_id = :id
      ORDER BY employee_id
    ")?;
    stmt.set_prefetch_rows(5)?;

    let rows = stmt.query(103)?; // 103 is Alexander Hunold

    let mut subs = HashMap::new();
    while let Some( row ) = rows.next()? {
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
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn query(&'a self, mut args: impl ToSql) -> Result<Rows> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Use `execute` to execute statements other than SELECT") );
        }
        let res = self.exec(stmt_type, &mut args)?;

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

    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT country_id, state_province, city, postal_code, street_address
          FROM hr.locations
         WHERE location_id = :id
    ")?;

    let row = stmt.query_single(1800)?;

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
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn query_single(&'a self, mut args: impl ToSql) -> Result<Option<Row>> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Use `execute` to execute statements other than SELECT") );
        }
        self.set_prefetch_rows(1)?;
        let res = self.exec(stmt_type, &mut args)?;

        if self.cols.get().is_none() {
            let cols = Columns::new(Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), self.max_long)?;
            self.cols.get_or_init(|| RwLock::new(cols));
        }

        match res {
            OCI_NO_DATA => Ok(None),
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Rows::from_query(res, self).single(),
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

    # let session = sibyl::test_env::get_session()?;
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
    ")?;
    let expected_lowest_salary = Number::from_int(2100, &session)?;
    let expected_median_salary = Number::from_int(6200, &session)?;

    stmt.execute(())?;

    let lowest_payed_employee = stmt.next_result()?.unwrap();

    let rows = lowest_payed_employee.rows()?;
    let row = rows.next()?.unwrap();

    let department_name : &str = row.get(0)?;
    let first_name : &str = row.get(1)?;
    let last_name : &str = row.get(2)?;
    let salary : Number = row.get(3)?;

    assert_eq!(department_name, "Shipping");
    assert_eq!(first_name, "TJ");
    assert_eq!(last_name, "Olson");
    assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

    let row = rows.next()?;
    assert!(row.is_none());

    let median_salary_employees = stmt.next_result()?.unwrap();

    let rows = median_salary_employees.rows()?;

    let row = rows.next()?.unwrap();
    let department_name : &str = row.get(0)?;
    let first_name : &str = row.get(1)?;
    let last_name : &str = row.get(2)?;
    let salary : Number = row.get(3)?;

    assert_eq!(department_name, "Sales");
    assert_eq!(first_name, "Amit");
    assert_eq!(last_name, "Banda");
    assert_eq!(salary.compare(&expected_median_salary)?, Equal);

    let row = rows.next()?.unwrap();

    let department_name : &str = row.get(0)?;
    let first_name : &str = row.get(1)?;
    let last_name : &str = row.get(2)?;
    let salary : Number = row.get(3)?;

    assert_eq!(department_name, "Sales");
    assert_eq!(first_name, "Charles");
    assert_eq!(last_name, "Johnson");
    assert_eq!(salary.compare(&expected_median_salary)?, Equal);

    let row = rows.next()?;
    assert!(row.is_none());

    assert!(stmt.next_result()?.is_none());
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn next_result(&'a self) -> Result<Option<Cursor>> {
        let mut stmt = Ptr::<OCIStmt>::null();
        let mut stmt_type = 0u32;
        let res = unsafe {
            OCIStmtGetNextResult(self.stmt.as_ref(), self.err.as_ref(), stmt.as_mut_ptr(), &mut stmt_type, OCI_DEFAULT)
        };
        match res {
            OCI_NO_DATA => Ok( None ),
            OCI_SUCCESS => Ok( Some ( Cursor::implicit(stmt, self) ) ),
            _ => Err( Error::oci(&self.err, res) )
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn stmt_args() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let session = crate::test_env::get_session()?;

        let stmt = session.prepare("
            INSERT INTO hr.countries
                   ( country_id, country_name, region_id )
            VALUES ( :country_id, :country_name
                   , (
                       SELECT region_id
                         FROM hr.regions
                        WHERE region_name = :region_name
                     )
                   )
         RETURNING region_id
              INTO :region_id
        ")?;
        let mut region_id : usize = 0;
        let num_rows = stmt.execute((
            ( ":COUNTRY_ID",    "IE"           ),
            ( ":COUNTRY_NAME",  "Ireland"      ),
            ( ":REGION_NAME",   "Europe"       ),
            ( ":REGION_ID",     &mut region_id )
        ))?;

        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":REGION_ID")?);
        assert_eq!(region_id, 1);

        session.rollback()?;
        Ok(())
    }

    #[test]
    fn int_arg() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let session = crate::test_env::get_session()?;

        let stmt = session.prepare("
            SELECT city, street_address
              FROM hr.locations
             WHERE location_id = :location_id
        ")?;
        let row = stmt.query_single(
            (":LOCATION_ID", 2500 )
        )?;
        assert!(row.is_some());
        let row = row.unwrap();
        assert!(!row.is_null(0));
        assert!(!row.is_null(1));
        let city : &str = row.get(0)?;
        let addr : &str = row.get(1)?;
        assert_eq!(city, "Oxford");
        assert_eq!(addr, "Magdalen Centre, The Oxford Science Park");

        Ok(())
    }

    #[test]
    fn plsql_args() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let session = crate::test_env::get_session()?;

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
        ")?;
        let mut city = String::with_capacity(30);
        let mut addr = String::with_capacity(40);
        let num_rows = stmt.execute((
            ( ":LOCATION_ID",    2500      ),
            ( ":CITY",           &mut city ),
            ( ":STREET_ADDRESS", &mut addr )
        ))?;

        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":CITY")?);
        assert_eq!(city, "Oxford");
        assert!(!stmt.is_null(":STREET_ADDRESS")?);
        assert_eq!(addr, "Magdalen Centre, The Oxford Science Park");

        let num_rows = stmt.execute((
            ( ":LOCATION_ID",    2400      ),
            ( ":CITY",           &mut city ),
            ( ":STREET_ADDRESS", &mut addr )
        ))?;

        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":CITY")?);
        assert_eq!(city, "London");
        assert!(!stmt.is_null(":STREET_ADDRESS")?);
        assert_eq!(addr, "8204 Arthur St");

        let num_rows = stmt.execute((
            ( ":LOCATION_ID",    2200      ),
            ( ":CITY",           &mut city ),
            ( ":STREET_ADDRESS", &mut addr )
        ))?;

        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":CITY")?);
        assert_eq!(city, "Sydney");
        assert!(!stmt.is_null(":STREET_ADDRESS")?);
        assert_eq!(addr, "12-98 Victoria Street");

        let num_rows = stmt.execute((
            ( ":LOCATION_ID",    3300      ),
            ( ":CITY",           &mut city ),
            ( ":STREET_ADDRESS", &mut addr )
        ))?;

        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":CITY")?);
        assert_eq!(city, "Unknown");
        assert!(stmt.is_null(":STREET_ADDRESS")?);

        session.rollback()?;
        Ok(())
    }

    #[test]
    fn out_arg_no_rows() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let session = crate::test_env::get_session()?;

        let stmt = session.prepare("
            UPDATE hr.employees
               SET salary = Round(salary * :rate, -2)
             WHERE employee_id = :id
            RETURN salary INTO :new_salary
        ")?;
        let mut new_salary = 0u16;
        let num_updated = stmt.execute((
            (":ID",         107             ),
            (":RATE",       1.07            ),
            (":NEW_SALARY", &mut new_salary ),
        ))?;

        assert_eq!(num_updated, 1);
        assert!(!stmt.is_null(":NEW_SALARY")?);
        assert_eq!(new_salary, 4500);

        let num_updated = stmt.execute((
            (":ID",         99              ),
            (":RATE",       1.03            ),
            (":NEW_SALARY", &mut new_salary ),
        ))?;

        assert_eq!(num_updated, 0);
        assert!(stmt.is_null(":NEW_SALARY")?);

        session.rollback()?;
        Ok(())
    }

    /// Unless (or until) dynamic array binding is implemented this would be failing
    #[test]
    fn out_arg_many_rows() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let session = crate::test_env::get_session()?;

        let stmt = session.prepare("
            UPDATE hr.employees
               SET salary = Round(salary * :rate, -2)
             WHERE manager_id = :manager_id
            RETURN salary INTO :new_salary
        ")?;
        let mut new_salary = 0u16;
        let res = stmt.execute((
            (":MANAGER_ID", 103             ),
            (":RATE",       1.02            ),
            (":NEW_SALARY", &mut new_salary ),
        ));
        assert!(res.is_err());
        match res.unwrap_err() {
            Error::Oracle(code,_) => assert_eq!(code, 24369),
            err => panic!("unexpected error {:?}", err),
        }

        session.rollback()?;
        Ok(())
    }

    #[test]
    fn single_row_query() -> Result<()> {
        let session = crate::test_env::get_session()?;

        let stmt = session.prepare("
            SELECT country_id, state_province, city, postal_code, street_address
              FROM hr.locations
             WHERE location_id = :id
        ")?;
        let row = stmt.query_single(1800)?;
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
        ")?;
        let row = stmt.query_single("CA")?;
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
    }
}
