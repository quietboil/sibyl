use crate::*;
use crate::types::*;
use crate::env::Env;
use crate::conn::Conn;
use crate::stmt::{ColumnInfo, Stmt};
use crate::column::Column;
use crate::rows::Rows;
use libc::c_void;
use std::cell::RefCell;
use std::collections::HashMap;

pub(crate) enum RefCursor {
    Handle( Handle<OCIStmt> ),
    Ptr( *mut OCIStmt )
}

impl ToSqlOut for Handle<OCIStmt> {
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
        (SQLT_RSET, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCIStmt>(), std::mem::size_of::<*mut OCIStmt>())
    }
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
    cursor:     RefCursor,
    cols:       RefCell<Vec<Column>>,
    col_names:  RefCell<HashMap<String,usize>>,
    stmt:       &'a dyn Stmt,
}

impl Drop for Cursor<'_> {
    fn drop(&mut self) {
        let env = self.env_ptr();
        let err = self.err_ptr();
        for col in self.cols.borrow_mut().iter_mut() {
            col.drop_output_buffer(env, err);
        }
    }
}

impl Env for Cursor<'_> {
    fn env_ptr(&self) -> *mut OCIEnv      { self.stmt.env_ptr() }
    fn err_ptr(&self) -> *mut OCIError    { self.stmt.err_ptr() }
}

impl UsrEnv for Cursor<'_> {
    fn as_ptr(&self) -> *mut c_void         { self.stmt.usr_ptr() as *mut c_void }
    fn as_conn(&self) -> Option<&dyn Conn>  { Some( self.stmt.conn() )           }
}

impl Conn for Cursor<'_> {
    fn srv_ptr(&self) -> *mut OCIServer   { self.stmt.srv_ptr() }
    fn svc_ptr(&self) -> *mut OCISvcCtx   { self.stmt.svc_ptr() }
    fn usr_ptr(&self) -> *mut OCISession  { self.stmt.usr_ptr() }
}

impl Stmt for Cursor<'_> {
    fn stmt_ptr(&self) -> *mut OCIStmt    { self.cursor.get() }
    fn conn(&self) -> &dyn Conn           { self.stmt.conn()  }
    fn env(&self) -> &dyn Env             { self              }
    fn usr_env(&self) -> &dyn UsrEnv      { self              }
    fn get_cols(&self) -> &RefCell<Vec<Column>> { &self.cols }
    fn col_index(&self, name: &str) -> Option<usize> {
        self.col_names.borrow().get(name).map(|pos| *pos)
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
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
        See also `Statement::next_result` for another method to return REF CURSORs.
    */
    pub fn new(stmt: &'a dyn Stmt) -> Result<Self> {
        let handle = Handle::<OCIStmt>::new(stmt.env_ptr())?;
        Ok( Self::from_handle(handle, stmt) )
    }

    pub(crate) fn implicit(istmt: *mut OCIStmt, stmt: &'a dyn Stmt) -> Self {
        Self {
            cursor: RefCursor::Ptr( istmt ),
            cols: RefCell::new(Vec::new()),
            col_names: RefCell::new(HashMap::new()),
            stmt,
        }
    }

    pub(crate) fn from_handle(handle: Handle<OCIStmt>, stmt: &'a dyn Stmt) -> Self {
        Self {
            cursor: RefCursor::Handle( handle ),
            cols: RefCell::new(Vec::new()),
            col_names: RefCell::new(HashMap::new()),
            stmt,
        }
    }

    /// Initializes, if necessary, the internal vector of columns
    fn setup_columns(&self) -> Result<()> {
        let mut cols = self.cols.borrow_mut();
        if cols.is_empty() {
            let num_columns = attr::get::<u32>(OCI_ATTR_PARAM_COUNT, OCI_HTYPE_STMT, self.stmt_ptr() as *const c_void, self.err_ptr())? as usize;
            cols.reserve_exact(num_columns);
            for pos in 1..=num_columns {
                let col = Column::new(pos, self.stmt_ptr(), self.err_ptr())?;
                cols.push(col)
            }
            // Now that columns are in the vector and thus their locations in memory are fixed,
            // define their output buffers
            for col in cols.iter_mut() {
                col.setup_output_buffer(self.stmt_ptr(), self.env_ptr(), self.err_ptr())?;
            }
            let mut col_names = self.col_names.borrow_mut();
            col_names.reserve(num_columns);
            for col in cols.iter() {
                let col_info = ColumnInfo::new(self, col.as_ptr());
                let name = col_info.name()?;
                col_names.insert(name.to_string(), col.position() - 1);
            }
        }
        Ok(())
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
                    SELECT distinct last_name
                      FROM hr.employees
                     WHERE last_name = :last_name
                   ) e
        ")?;
        let rows = stmt.query(&[ &"King" ])?;

        let row = rows.next()?.unwrap();
        let last_name : &str = row.get(0)?.unwrap();
        assert_eq!(last_name, "King");

        let departments : Cursor = row.get(1)?.unwrap();
        let dept_rows = departments.rows()?;
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
    pub fn rows(&self) -> Result<Rows> {
        self.setup_columns()?;
        Ok( Rows::new(OCI_SUCCESS, self) )
    }
}
