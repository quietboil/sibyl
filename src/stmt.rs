//! SQL or PL/SQL statement

pub mod args;
pub mod bind;
pub mod fromsql;
pub mod cols;
pub mod cursor;
pub mod rows;
#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
pub mod blocking;

use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use libc::c_void;
pub use rows::{Rows, Row};
pub use cursor::Cursor;
pub use args::{ToSql, ToSqlOut, SqlInArg, SqlOutArg};
use bind::Params;
use cols::{Columns, Position, ColumnInfo};
use crate::{Result, Error, Connection, oci::*, env::Env, types::Ctx};
use std::ptr;

/// Represents a prepared for execution SQL or PL/SQL statement
pub struct Statement<'a> {
    conn:     &'a Connection<'a>,
    stmt:     Ptr<OCIStmt>,
    params:   Option<RwLock<Params>>,
    cols:     OnceCell<RwLock<Columns>>,
    err:      Handle<OCIError>,
    max_long: u32,
}

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        let ocistmt = self.stmt_ptr();
        if !ocistmt.is_null() {
            let res = unsafe {
                OCIStmtRelease(ocistmt, self.err_ptr(), ptr::null(), 0, OCI_DEFAULT)
            };
            assert_ne!(res, OCI_STILL_EXECUTING, "OCIStmtRelease is still executing");
        }
    }
}

impl Env for Statement<'_> {
    fn env_ptr(&self) -> *mut OCIEnv {
        self.conn.env_ptr()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.err.get()
    }
}

impl Ctx for Statement<'_> {
    fn ctx_ptr(&self) -> *mut c_void {
        self.conn.usr_ptr() as *mut c_void
    }
}

pub trait Stmt: Env {
    fn stmt_ptr(&self) -> *mut OCIStmt;
}

impl Stmt for Statement<'_> {
    fn stmt_ptr(&self) -> *mut OCIStmt {
        self.stmt.get()
    }
}

trait ResultSetColumns {
    fn read_columns(&self) -> RwLockReadGuard<Columns>;
    fn write_columns(&self) -> RwLockWriteGuard<Columns>;
}

impl ResultSetColumns for Statement<'_> {
    fn read_columns(&self) -> RwLockReadGuard<Columns> {
        self.cols.get().expect("protected columns").read()
    }

    fn write_columns(&self) -> RwLockWriteGuard<Columns> {
        self.cols.get().expect("protected columns").write()
    }
}

trait ResultSetConnection {
    fn conn(&self) -> &Connection;
}

impl ResultSetConnection for Statement<'_> {
    fn conn(&self) -> &Connection {
        self.conn
    }
}

impl<'a> Statement<'a> {

    fn get_attr<V: attr::AttrGet>(&self, attr_type: u32) -> Result<V> {
        attr::get::<V>(attr_type, OCI_HTYPE_STMT, self.stmt_ptr() as *const c_void, self.err_ptr())
    }

    fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V) -> Result<()> {
        attr::set::<V>(attr_type, attr_val, OCI_HTYPE_STMT, self.stmt_ptr() as *mut c_void, self.err_ptr())
    }

    /**
        Checks whether the value returned for the output parameter is NULL.
    */
    pub fn is_null(&self, pos: impl Position) -> Result<bool> {
        self.params.as_ref().map(|params| params.read().is_null(pos)).unwrap_or(Ok(true))
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

        let mut rows = lowest_payed_employee.rows()?;
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

        let mut rows = median_salary_employees.rows()?;

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
        let mut stmt = Ptr::null();
        let mut stmt_type = 0u32;
        let res = unsafe {
            OCIStmtGetNextResult(self.stmt_ptr(), self.err_ptr(), stmt.as_mut_ptr(), &mut stmt_type, OCI_DEFAULT)
        };
        match res {
            OCI_NO_DATA => Ok( None ),
            OCI_SUCCESS => Ok( Some ( Cursor::implicit(stmt, self) ) ),
            _ => Err( Error::oci(self.err_ptr(), res) )
        }
    }

    /**
        Sets the number of top-level rows to be prefetched. The default value is 1 row.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, first_name, last_name
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        stmt.set_prefetch_rows(10)?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_prefetch_rows(&self, num_rows: u32) -> Result<()> {
        self.set_attr(OCI_ATTR_PREFETCH_ROWS, num_rows)
    }

    /**
        Sets the maximum size of data that will be fetched from LONG and LONG RAW.

        By default 32768 bytes are allocated for values from LONG and LONG RAW columns.
        If the actual value is expected to be larger than that, then the "column size"
        has to be changed **before** the `query` is run.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
        #     DECLARE
        #         name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
        #     BEGIN
        #         EXECUTE IMMEDIATE '
        #             CREATE TABLE test_long_and_raw_data (
        #                 id      NUMBER GENERATED ALWAYS AS IDENTITY,
        #                 bin     RAW(100),
        #                 text    LONG
        #             )
        #         ';
        #     EXCEPTION
        #       WHEN name_already_used THEN
        #         EXECUTE IMMEDIATE '
        #             TRUNCATE TABLE test_long_and_raw_data
        #         ';
        #     END;
        # ")?;
        # stmt.execute(&[])?;
        # let stmt = conn.prepare("
        #     INSERT INTO test_long_and_raw_data (text) VALUES (:TEXT)
        #     RETURNING id INTO :ID
        # ")?;
        # let text = "When I have fears that I may cease to be Before my pen has gleaned my teeming brain, Before high-pilèd books, in charactery, Hold like rich garners the full ripened grain; When I behold, upon the night’s starred face, Huge cloudy symbols of a high romance, And think that I may never live to trace Their shadows with the magic hand of chance; And when I feel, fair creature of an hour, That I shall never look upon thee more, Never have relish in the faery power Of unreflecting love—then on the shore Of the wide world I stand alone, and think Till love and fame to nothingness do sink.";
        # let mut id = 0;
        # let count = stmt.execute_into(
        #     &[
        #         &(":TEXT", text)
        #     ], &mut [
        #         &mut (":ID", &mut id),
        #     ]
        # )?;
        let mut stmt = conn.prepare("
            SELECT text
              FROM test_long_and_raw_data
             WHERE id = :id
        ")?;
        stmt.set_max_long_size(100_000);
        let mut rows = stmt.query(&[ &id ])?;
        let row = rows.next()?.expect("first (and only) row");
        let txt : &str = row.get(0)?.expect("long text");
        # assert_eq!(txt, text);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_max_long_size(&mut self, size: u32) {
        self.max_long = size;
    }

    /**
        Returns he number of columns in the select-list of this statement.

        # Example
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
        ")?;
        let mut _rows = stmt.query(&[ &103 ])?;
        let num_cols = stmt.column_count()?;

        assert_eq!(num_cols, 3);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn column_count(&self) -> Result<usize> {
        let num_columns = self.get_attr::<u32>(OCI_ATTR_PARAM_COUNT)? as usize;
        Ok( num_columns )
    }

    /**
        Returns `pos` column meta data handler. `pos` is 0-based. Returns None if
        `pos` is greater than the number of columns in the query or if the prepared
        statement is not a SELECT and has no columns.

        # Example
        ```
        use sibyl::ColumnType;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, last_name, first_name
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        let mut _rows = stmt.query(&[ &103 ])?;
        let col = stmt.column(0).expect("employee_id column info");
        assert_eq!(col.name()?, "EMPLOYEE_ID");
        assert_eq!(col.data_type()?, ColumnType::Number);
        assert_eq!(col.precision()?, 6);
        assert_eq!(col.scale()?, 0);
        assert!(!col.is_null()?);
        assert!(col.is_visible()?);
        assert!(!col.is_identity()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn column(&self, pos: usize) -> Option<ColumnInfo> {
        self.cols.get().and_then(|cols| cols.read().column_info(self, pos))
    }

    /**
        Returns the number of rows processed/seen so far in SELECT statements.

        For INSERT, UPDATE, and DELETE statements, it is the number of rows processed
        by the most recent statement.

        For nonscrollable cursors, it is the total number of rows fetched into user buffers
        since this statement handle was executed. Because they are forward sequential only,
        this also represents the highest row number seen by the application.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, first_name, last_name
              FROM hr.employees
             WHERE manager_id = :id
          ORDER BY employee_id
        ")?;
        stmt.set_prefetch_rows(5)?;
        let mut rows = stmt.query(&[ &103 ])?;
        let mut ids = Vec::new();
        while let Some( row ) = rows.next()? {
            // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
            let id : u32 = row.get(0)?.unwrap();
            ids.push(id);
        }
        assert_eq!(stmt.row_count()?, 4);
        assert_eq!(ids.len(), 4);
        assert_eq!(ids.as_slice(), &[104 as u32, 105, 106, 107]);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn row_count(&self) -> Result<usize> {
        let num_rows = self.get_attr::<u64>(OCI_ATTR_UB8_ROW_COUNT)? as usize;
        Ok( num_rows )
    }

    // Indicates the number of rows that were successfully fetched into the user's buffers
    // in the last fetch or execute with nonzero iterations.
    // This is not very useful in this implementation as we set up buffers for 1 row only.
    // pub fn rows_fetched(&self) -> Result<usize> {
    //     let num_rows = self.get_attr::<u32>(OCI_ATTR_ROWS_FETCHED)? as usize;
    //     Ok( num_rows )
    // }
}
