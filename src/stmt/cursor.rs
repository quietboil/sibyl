use super::cols::{Columns, ColumnInfo};
use super::rows::{Rows, ResultSetProvider};
use super::Stmt;
use crate::*;
use crate::types::*;
use libc::c_void;
use std::cell::RefCell;

pub(crate) enum RefCursor {
    Handle( Handle<OCIStmt> ),
    Ptr( *mut OCIStmt )
}

impl RefCursor {
    fn get(&self) -> *mut OCIStmt {
        match self {
            RefCursor::Handle( handle ) => handle.get(),
            RefCursor::Ptr( ptr )       => *ptr,
        }
    }

    fn as_ptr(&mut self) -> *mut *mut OCIStmt {
        match self {
            RefCursor::Handle( handle ) => handle.as_ptr(),
            RefCursor::Ptr( ptr )       => ptr,
        }
    }
}

/// Cursors - implicit results and REF CURSOR - from an executed PL/SQL statement
pub struct Cursor<'a> {
    stmt:       &'a dyn ResultSetProvider,
    cursor:     RefCursor,
    cols:       RefCell<Columns>,
}

impl Drop for Cursor<'_> {
    fn drop(&mut self) {
        self.cols.borrow_mut().drop_output_buffers(self.env_ptr(), self.err_ptr());
    }
}

impl Env for Cursor<'_> {
    fn env_ptr(&self) -> *mut OCIEnv {
        self.stmt.env_ptr()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.stmt.err_ptr()
    }
}

impl Stmt for Cursor<'_> {
    fn stmt_ptr(&self) -> *mut OCIStmt {
        self.cursor.get()
    }
}

impl Ctx for Cursor<'_> {
    fn as_ptr(&self) -> *mut c_void {
        self.stmt.get_ctx().as_ptr()
    }
}

impl<'a> ResultSetProvider for Cursor<'a> {
    fn get_cols(&self) -> &RefCell<Columns> {
        &self.cols
    }

    fn get_ctx(&self) -> &dyn Ctx {
        self
    }

    fn get_env(&self) -> &dyn Env {
        self
    }

    fn conn(&self) -> &Connection {
        self.stmt.conn()
    }
}

impl ToSqlOut for Cursor<'_> {
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
        (SQLT_RSET, (*self).cursor.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCIStmt>(), std::mem::size_of::<*mut OCIStmt>())
    }
}

impl<'a> Cursor<'a> {
    /**
        Creates a Cursor that can be used as an OUT argument to receive a returning REF CURSOR.

        # Example
        ```
        use sibyl::{Cursor, Number};
        use std::cmp::Ordering::Equal;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            BEGIN
                OPEN :lowest_payed_employee FOR
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
                OPEN :median_salary_employees FOR
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
            END;
        ")?;

        let mut lowest_payed_employee   = Cursor::new(&stmt)?;
        let mut median_salary_employees = Cursor::new(&stmt)?;

        stmt.execute_into(&[], &mut [
            &mut ( ":LOWEST_PAYED_EMPLOYEE",   &mut lowest_payed_employee   ),
            &mut ( ":MEDIAN_SALARY_EMPLOYEES", &mut median_salary_employees ),
        ])?;

        let expected_lowest_salary = Number::from_int(2100, &conn)?;
        let expected_median_salary = Number::from_int(6200, &conn)?;

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
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
        See also `Statement::next_result` for another method to return REF CURSORs.
    */
    pub fn new(stmt: &'a Statement) -> Result<Self> {
        let handle = Handle::<OCIStmt>::new(stmt.env_ptr())?;
        Ok( Self::from_handle(handle, stmt) )
    }

    pub(crate) fn implicit(istmt: *mut OCIStmt, stmt: &'a dyn ResultSetProvider) -> Self {
        Self {
            cursor: RefCursor::Ptr( istmt ),
            cols: RefCell::new(Columns::new()),
            stmt,
        }
    }

    pub(crate) fn from_handle(handle: Handle<OCIStmt>, stmt: &'a dyn ResultSetProvider) -> Self {
        Self {
            cursor: RefCursor::Handle( handle ),
            cols: RefCell::new(Columns::new()),
            stmt,
        }
    }

    fn get_attr<V: attr::AttrGet>(&self, attr_type: u32) -> Result<V> {
        attr::get::<V>(attr_type, OCI_HTYPE_STMT, self.stmt_ptr() as *const c_void, self.err_ptr())
    }

    fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V) -> Result<()> {
        attr::set::<V>(attr_type, attr_val, OCI_HTYPE_STMT, self.stmt_ptr() as *mut c_void, self.err_ptr())
    }

    /**
        Sets the buffer size for fetching LONG and LONG RAW via the data interface.

        By default 32768 bytes are allocated for values from LONG and LONG RAW columns.
        If the actual value is expected to be larger than that, then the "column size"
        has to be changed before `query` is run.

        # Example

        ```rust
        use sibyl::Cursor;

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
        let stmt = conn.prepare("
            BEGIN
                OPEN :long_texts FOR
                    SELECT text
                      FROM test_long_and_raw_data
                     WHERE id = :id
                ;
            END;
        ")?;
        let mut long_texts = Cursor::new(&stmt)?;
        stmt.execute_into(&[&(":ID", &id)], &mut [
            &mut (":LONG_TEXTS", &mut long_texts),
        ])?;
        long_texts.set_column_size(0, 100_000);
        let mut rows = long_texts.rows()?;
        let row = rows.next()?.expect("first (and only) row");
        let txt : &str = row.get(0)?.expect("long text");
        # assert_eq!(txt, text);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_column_size(&self, pos: usize, size: usize) {
        self.cols.borrow_mut().set_long_column_size(pos, size as u32);
    }

    /**
        Returns he number of columns in the select-list of this statement.

        # Example

        ```
        use sibyl::Cursor;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            BEGIN
                OPEN :subordinates FOR
                    SELECT employee_id, last_name, first_name
                      FROM hr.employees
                     WHERE manager_id = :id
                ;
            END;
        ")?;
        let mut subordinates = Cursor::new(&stmt)?;
        stmt.execute_into(&[&(":ID", 103)], &mut [
            &mut (":SUBORDINATES", &mut subordinates),
        ])?;
        assert_eq!(subordinates.get_column_count()?, 3);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_column_count(&self) -> Result<usize> {
        let num_columns = self.get_attr::<u32>(OCI_ATTR_PARAM_COUNT)? as usize;
        Ok( num_columns )
    }

    /**
        Returns `pos` column meta data handler. `pos` is 0-based. Returns None if
        `pos` is greater than the number of columns in the query or if the prepared
        statement is not a SELECT and has no columns.

        # Example
        ```
        use sibyl::{Cursor, ColumnType};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            BEGIN
                OPEN :subordinates FOR
                    SELECT employee_id, last_name, first_name
                      FROM hr.employees
                     WHERE manager_id = :id
                ;
            END;
        ")?;
        let mut subordinates = Cursor::new(&stmt)?;
        stmt.execute_into(&[&(":ID", 103)], &mut [
            &mut (":SUBORDINATES", &mut subordinates),
        ])?;
        let mut _rows = subordinates.rows()?;        
        let col = subordinates.get_column(0).expect("ID column info");
        assert_eq!(col.name()?, "EMPLOYEE_ID", "column name");
        assert_eq!(col.data_type()?, ColumnType::Number, "column type");
        assert_eq!(col.precision()?, 6, "number precision");
        assert_eq!(col.scale()?, 0, "number scale");
        assert!(!col.is_null()?, "not null");
        assert!(col.is_visible()?, "is visible");
        assert!(!col.is_identity()?, "not an identity column");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_column(&self, pos: usize) -> Option<ColumnInfo> {
        self.cols.borrow().get_column_info(self, pos)
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
        use sibyl::Cursor;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            BEGIN
                OPEN :subordinates FOR
                    SELECT employee_id, last_name, first_name
                      FROM hr.employees
                     WHERE manager_id = :id
                  ORDER BY employee_id
                ;
            END;
        ")?;
        let mut subordinates = Cursor::new(&stmt)?;
        stmt.execute_into(&[&(":ID", 103)], &mut [
            &mut (":SUBORDINATES", &mut subordinates),
        ])?;
        let mut rows = subordinates.rows()?;
        let mut ids = Vec::new();
        while let Some( row ) = rows.next()? {
            // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
            let id : usize = row.get(0)?.unwrap();
            ids.push(id);
        }
        assert_eq!(subordinates.get_row_count()?, 3);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids.as_slice(), &[104 as usize, 105, 106]);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_row_count(&self) -> Result<usize> {
        let num_rows = self.get_attr::<u64>(OCI_ATTR_UB8_ROW_COUNT)? as usize;
        Ok( num_rows )
    }

    /**
        Sets the number of top-level rows to be prefetched. The default value is 1 row.

        # Example
        ```
        use sibyl::Cursor;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            BEGIN
                OPEN :subordinates FOR
                    SELECT employee_id, last_name, first_name
                      FROM hr.employees
                     WHERE manager_id = :id
                  ORDER BY employee_id
                ;
            END;
        ")?;
        let mut subordinates = Cursor::new(&stmt)?;
        stmt.execute_into(&[&(":ID", 103)], &mut [
            &mut (":SUBORDINATES", &mut subordinates),
        ])?;
        subordinates.set_prefetch_rows(10)?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_prefetch_rows(&self, num_rows: u32) -> Result<()> {
        self.set_attr(OCI_ATTR_PREFETCH_ROWS, num_rows)
    }

    /**
        Returns rows selected by this cursor

        # Example
        ```
        use sibyl::Cursor;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT last_name
                 , CURSOR(
                        SELECT department_name
                          FROM hr.departments
                         WHERE department_id IN (
                                    SELECT department_id
                                      FROM hr.employees
                                     WHERE last_name = e.last_name)
                      ORDER BY department_name
                   ) AS departments
              FROM (
                    SELECT DISTINCT last_name
                      FROM hr.employees
                     WHERE last_name = :last_name
                   ) e
        ")?;
        let mut rows = stmt.query(&[ &"King" ])?;

        let row = rows.next()?.unwrap();
        let last_name : &str = row.get(0)?.unwrap();
        assert_eq!(last_name, "King");

        let departments : Cursor = row.get(1)?.unwrap();
        let mut dept_rows = departments.rows()?;
        let dept_row = dept_rows.next()?.unwrap();

        let department_name : &str = dept_row.get(0)?.unwrap();
        assert_eq!(department_name, "Executive");

        let dept_row = dept_rows.next()?.unwrap();
        let department_name : &str = dept_row.get(0)?.unwrap();
        assert_eq!(department_name, "Sales");

        assert!(dept_rows.next()?.is_none());
        assert!(rows.next()?.is_none());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn rows(&'a self) -> Result<Rows<'a>> {
        self.cols.borrow_mut().setup(self)?;
        Ok( Rows::new(OCI_SUCCESS, self) )
    }
}
