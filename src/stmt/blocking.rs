//! Blocking SQL statement methods

use super::{
    Statement, Cursor, Params, StmtInArg, StmtOutArg, Columns, Rows,
    cols::DEFAULT_LONG_BUFFER_SIZE,
};
use crate::{Error, Result, oci::{self, *}, Connection};
use parking_lot::RwLock;
use once_cell::sync::OnceCell;

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        let _ = self.svc;
        oci_stmt_release(&self.stmt, &self.err);
    }
}

impl<'a> Statement<'a> {
    pub(crate) fn conn(&self) -> &Connection {
        self.conn
    }

    /// Creates a new statement
    pub(crate) fn new(sql: &str, conn: &'a Connection) -> Result<Self> {
        let err = Handle::<OCIError>::new(conn)?;
        let mut stmt = Ptr::<OCIStmt>::null();
        oci::stmt_prepare(
            conn.as_ref(), stmt.as_mut_ptr(), &err,
            sql.as_ptr(), sql.len() as u32,
            OCI_NTV_SYNTAX, OCI_DEFAULT
        )?;
        let params = Params::new(&stmt, &err)?.map(|params| RwLock::new(params));
        Ok(Self {conn, svc: conn.get_svc(), stmt, params, cols: OnceCell::new(), err, max_long: DEFAULT_LONG_BUFFER_SIZE})
    }

    /// Binds provided arguments to SQL parameter placeholders. Returns indexes of parameter placeholders for the OUT args.
    fn bind_args(&self, in_args: &[&dyn StmtInArg], out_args: &mut [&mut dyn StmtOutArg]) -> Result<Option<Vec<usize>>> {
        self.params.as_ref()
            .map(|params| params.write().bind_args(&self.stmt, &self.err, in_args, out_args))
            .unwrap_or_else(|| 
                if in_args.len() == 0 && out_args.len() == 0 {
                    Ok(None)
                } else {
                    Err(Error::new("Statement has no parameters"))
                }
            )
    }

    /// Executes the prepared statement. Returns the OCI result code from OCIStmtExecute.
    fn exec(&self, stmt_type: u16, in_args: &[&dyn StmtInArg], out_args: &mut [&mut dyn StmtOutArg]) -> Result<i32>{
        let out_idxs = self.bind_args(in_args, out_args)?;

        let iters: u32 = if stmt_type == OCI_STMT_SELECT { 0 } else { 1 };
        let res = oci::stmt_execute(self.as_ref(), &self.stmt, &self.err, iters, 0, OCI_DEFAULT)?;
        match res {
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                if let Some(idxs) = out_idxs {
                    if let Some(params) = self.params.as_ref() {
                        let params = params.read();
                        for (out_arg_ix, out_param_ix) in idxs.into_iter().enumerate() {
                            out_args[out_arg_ix].to_sql_out().sql_set_len(params.out_data_len(out_param_ix));
                        }
                    }
                }
                Ok(res)
            },
            _ => Ok(res)
        }
    }

    /**
        Executes the prepared statement. Returns the number of rows affected.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            UPDATE hr.departments
               SET manager_id = :manager_id
             WHERE department_id = :department_id
        ")?;
        let num_updated_rows = stmt.execute(&[
            &( ":DEPARTMENT_ID", 120 ),
            &( ":MANAGER_ID",    101 ),
        ])?;
        assert_eq!(num_updated_rows, 1);
        # conn.rollback()?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn execute(&self, args: &[&dyn StmtInArg]) -> Result<usize> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type == OCI_STMT_SELECT {
            return Err( Error::new("Use `query` to execute SELECT") );
        }
        let is_returning: u8 = self.get_attr(OCI_ATTR_STMT_IS_RETURNING)?;
        if is_returning != 0 {
            return Err( Error::new("Use `execute_into` with output arguments to execute a RETURNING statement") );
        }
        self.exec(stmt_type, args, &mut [])?;
        self.row_count()
    }

    /**
        Executes a prepared RETURNING statement. Returns the number of rows affected.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            INSERT INTO hr.departments
                   ( department_id, department_name, manager_id, location_id )
            VALUES ( hr.departments_seq.nextval, :department_name, :manager_id, :location_id )
         RETURNING department_id
              INTO :department_id
        ")?;
        let mut department_id : usize = 0;
        // In this case (no duplicates in the statement parameters and the OUT parameter follows
        // the IN parameters) we could have used positional arguments. However, there are many
        // cases when positional is too difficult to use correcty with `execute_into`. For example,
        // OUT is used as an IN-OUT parameter, OUT precedes or in the middle of the IN parameter
        // list, parameter list is very long, etc. This example shows the call with the named
        // arguments as this might be a more typical use case for it.
        let num_rows = stmt.execute_into(&[
            &( ":DEPARTMENT_NAME", "Security" ),
            &( ":MANAGER_ID",      ""         ),
            &( ":LOCATION_ID",     1700       ),
        ], &mut [
            &mut ( ":DEPARTMENT_ID", &mut department_id )
        ])?;
        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":DEPARTMENT_ID")?);
        assert!(department_id > 0);
        # conn.rollback()?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn execute_into(&self, in_args: &[&dyn StmtInArg], out_args: &mut [&mut dyn StmtOutArg]) -> Result<usize> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type == OCI_STMT_SELECT {
            return Err( Error::new("Use `query` to execute SELECT") );
        }
        self.exec(stmt_type, in_args, out_args)?;
        self.row_count()
    }

    /**
        Executes the prepared statement. Returns "streaming iterator" over the returned rows.

        # Example
        ```
        # use std::collections::HashMap;
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
        stmt.set_prefetch_rows(5)?;
        let rows = stmt.query(&[ &103 ])?; // Alexander Hunold
        let mut subs = HashMap::new();
        while let Some( row ) = rows.next()? {
            // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
            let id : u32 = row.get(0)?.unwrap();
            // Same for the LAST_NAME.
            // Note that `last_name` is retrieved as a slice. This is fast as it
            // borrows directly from the column buffer, but it can only live until
            // the end of the current scope, i.e. only during the lifetime of the
            // current row.
            let last_name : &str = row.get(1)?.unwrap();
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
    pub fn query(&'a self, args: &[&dyn StmtInArg]) -> Result<Rows> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Use `execute` or `execute_into` to execute statements other than SELECT") );
        }
        let res = self.exec(stmt_type, args, &mut [])?;

        if self.cols.get().is_none() {
            let cols = Columns::new(Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), self.max_long)?;
            self.cols.get_or_init(|| RwLock::new(cols));
        };
        match res {
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO | OCI_NO_DATA => {
                Ok( Rows::from_query(res, self) )
            }
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

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
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
        let expected_lowest_salary = Number::from_int(2100, &conn)?;
        let expected_median_salary = Number::from_int(6200, &conn)?;

        stmt.execute(&[])?;

        let lowest_payed_employee = stmt.next_result()?.unwrap();

        let rows = lowest_payed_employee.rows()?;
        let row = rows.next()?.unwrap();

        let department_name : &str = row.get(0)?.unwrap();
        let first_name : &str = row.get(1)?.unwrap();
        let last_name : &str = row.get(2)?.unwrap();
        let salary : Number = row.get(3)?.unwrap();

        assert_eq!(department_name, "Shipping");
        assert_eq!(first_name, "TJ");
        assert_eq!(last_name, "Olson");
        assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

        let row = rows.next()?;
        assert!(row.is_none());

        let median_salary_employees = stmt.next_result()?.unwrap();

        let rows = median_salary_employees.rows()?;

        let row = rows.next()?.unwrap();
        let department_name : &str = row.get(0)?.unwrap();
        let first_name : &str = row.get(1)?.unwrap();
        let last_name : &str = row.get(2)?.unwrap();
        let salary : Number = row.get(3)?.unwrap();

        assert_eq!(department_name, "Sales");
        assert_eq!(first_name, "Amit");
        assert_eq!(last_name, "Banda");
        assert_eq!(salary.compare(&expected_median_salary)?, Equal);

        let row = rows.next()?.unwrap();

        let department_name : &str = row.get(0)?.unwrap();
        let first_name : &str = row.get(1)?.unwrap();
        let last_name : &str = row.get(2)?.unwrap();
        let salary : Number = row.get(3)?.unwrap();

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
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let oracle = env()?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            INSERT INTO hr.departments
                   ( department_id, department_name, manager_id, location_id )
            VALUES ( 9, :department_name, :manager_id, :location_id )
         RETURNING department_id
              INTO :department_id
        ")?;
        let mut department_id : i32 = 0;
        let num_rows = stmt.execute_into(&[
            &( ":department_name", "Security" ),
            &( ":manager_id",      ""         ),
            &( ":location_id",     1700       ),
        ], &mut [
            &mut ( ":department_id", &mut department_id )
        ])?;
        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":department_id")?);
        assert_eq!(department_id, 9);
        conn.rollback()?;
        Ok(())
    }
}
