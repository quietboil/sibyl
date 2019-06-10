use crate::*;
use crate::types::*;
use crate::env::Env;
use crate::conn::Conn;
use crate::stmt::Stmt;
use crate::column::Column;
use crate::rows::Rows;
use libc::c_void;
use std::cell::{ RefCell, Ref };

pub(crate) enum RefCursor {
    Handle( Handle<OCIStmt> ),
    Ptr( *mut OCIStmt )
}

impl ToSqlOut for Handle<OCIStmt> {
    fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_RSET, self.as_ptr() as *mut c_void, 0)
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
    cursor: RefCursor,
    cols: RefCell<Vec<Column>>,
    stmt: &'a dyn Stmt,
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
    fn stmt_ptr(&self) -> *mut OCIStmt    { self.cursor.get()            }
    fn conn(&self) -> &dyn Conn           { self.stmt.conn()             }
    fn get_max_col_size(&self) -> usize   { self.stmt.get_max_col_size() }
    fn usr_env(&self) -> &dyn UsrEnv      { self                         }
}

impl ToSqlOut for Cursor<'_> {
    fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_RSET, self.cursor.as_ptr() as *mut c_void, 0)
    }
}

impl<'a> Cursor<'a> {
    /// Creates a Cursor that can be used as an OUT argument to receive a returning REF CURSOR.
    ///
    /// ## Example
    /// ```
    /// use sibyl::{Cursor, Number};
    /// use std::cmp::Ordering::Equal;
    ///
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     BEGIN
    ///         OPEN :lowest_payed_employee FOR
    ///             SELECT department_name, first_name, last_name, salary
    ///              FROM (
    ///                    SELECT first_name, last_name, salary, department_id
    ///                         , ROW_NUMBER() OVER (ORDER BY salary) ord
    ///                      FROM hr.employees
    ///                   ) e
    ///              JOIN hr.departments d
    ///                ON d.department_id = e.department_id
    ///             WHERE ord = 1
    ///         ;
    ///         OPEN :median_salary_employees FOR
    ///             SELECT department_name, first_name, last_name, salary
    ///               FROM (
    ///                     SELECT first_name, last_name, salary, department_id
    ///                          , MEDIAN(salary) OVER () median_salary
    ///                       FROM hr.employees
    ///                    ) e
    ///               JOIN hr.departments d
    ///                 ON d.department_id = e.department_id
    ///              WHERE salary = median_salary
    ///           ORDER BY department_name, last_name, first_name
    ///         ;
    ///     END;
    /// ")?;
    ///
    /// let mut lowest_payed_employee   = Cursor::new(&stmt)?;
    /// let mut median_salary_employees = Cursor::new(&stmt)?;
    ///
    /// stmt.execute_into(&[], &mut [
    ///     &mut ( ":lowest_payed_employee",   &mut lowest_payed_employee   ),
    ///     &mut ( ":median_salary_employees", &mut median_salary_employees ),
    /// ])?;
    ///
    /// let rows = lowest_payed_employee.rows()?;
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    ///
    /// let department_name = row.get::<&str>(0)?.unwrap();
    /// assert_eq!(department_name, "Shipping");
    ///
    /// let first_name = row.get::<&str>(1)?;
    /// assert!(first_name.is_some());
    /// let first_name = first_name.unwrap();
    /// assert_eq!(first_name, "TJ");
    ///
    /// let last_name = row.get::<&str>(2)?.unwrap();
    /// assert_eq!(last_name, "Olson");
    ///
    /// let salary = row.get::<Number>(3)?.unwrap();
    /// let expected = Number::from_int(2100, &oracle);
    /// assert!(salary.cmp(&expected)? == Equal);
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_none());
    ///
    /// let rows = median_salary_employees.rows()?;
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    ///
    /// let department_name = row.get::<&str>(0)?.unwrap();
    /// assert_eq!(department_name, "Sales");
    ///
    /// let first_name = row.get::<&str>(1)?;
    /// assert!(first_name.is_some());
    /// let first_name = first_name.unwrap();
    /// assert_eq!(first_name, "Amit");
    ///
    /// let last_name = row.get::<&str>(2)?.unwrap();
    /// assert_eq!(last_name, "Banda");
    ///
    /// let expected = Number::from_int(6200, &oracle);
    ///
    /// let salary = row.get::<Number>(3)?.unwrap();
    /// assert!(salary.cmp(&expected)? == Equal);
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    ///
    /// let department_name = row.get::<&str>(0)?.unwrap();
    /// assert_eq!(department_name, "Sales");
    ///
    /// let first_name = row.get::<&str>(1)?;
    /// assert!(first_name.is_some());
    /// let first_name = first_name.unwrap();
    /// assert_eq!(first_name, "Charles");
    ///
    /// let last_name = row.get::<&str>(2)?.unwrap();
    /// assert_eq!(last_name, "Johnson");
    ///
    /// let salary = row.get::<Number>(3)?.unwrap();
    /// assert!(salary.cmp(&expected)? == Equal);
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_none());
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    /// See also `Statement::next_result` for another method to return REF CURSORs.
    pub fn new(stmt: &'a dyn Stmt) -> Result<Self> {
        let handle = Handle::<OCIStmt>::new(stmt.env_ptr())?;
        Ok( Self {
            cursor: RefCursor::Handle( handle ),
            cols: RefCell::new(Vec::new()),
            stmt
        } )
    }

    pub(crate) fn implicit(istmt: *mut OCIStmt, stmt: &'a dyn Stmt) -> Self {
        Self {
            cursor: RefCursor::Ptr( istmt ),
            cols: RefCell::new(Vec::new()),
            stmt,
        }
    }

    pub(crate) fn from_handle(handle: Handle<OCIStmt>, stmt: &'a dyn Stmt) -> Self {
        Self {
            cursor: RefCursor::Handle( handle ),
            cols: RefCell::new(Vec::new()),
            stmt,
        }
    }

    pub(crate) fn borrow_columns(&self) -> Result<Ref<Vec<Column>>> {
        let borrow = self.cols.try_borrow();
        if borrow.is_err() {
            Err( Error::new("cannot borrow projection") )
        } else {
            Ok( borrow.unwrap() )
        }
    }

    /// Returns rows selected by this cursor
    /// 
    /// ## Example
    /// ```
    /// use sibyl::Cursor;
    ///
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     SELECT last_name
    ///          , CURSOR(
    ///                 SELECT department_name
    ///                   FROM hr.departments
    ///                  WHERE department_id IN (
    ///                             SELECT department_id
    ///                               FROM hr.employees
    ///                              WHERE last_name = e.last_name)
    ///               ORDER BY department_name
    ///            ) AS departments
    ///       FROM (
    ///             SELECT distinct last_name
    ///               FROM hr.employees
    ///              WHERE last_name = :last_name
    ///            ) e
    /// ")?;
    /// let rows = stmt.query(&[ &"King" ])?;
    /// 
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    /// 
    /// let last_name = row.get::<&str>(0)?.unwrap();
    /// assert_eq!(last_name, "King");
    /// 
    /// let departments = row.get::<Cursor>(1)?.unwrap();
    /// let dept_rows = departments.rows()?;
    /// 
    /// let dept_row = dept_rows.next()?;
    /// assert!(dept_row.is_some());
    /// let dept_row = dept_row.unwrap();
    /// 
    /// let department_name = dept_row.get::<&str>(0)?.unwrap();
    /// assert_eq!("Executive", department_name);
    /// 
    /// let dept_row = dept_rows.next()?;
    /// assert!(dept_row.is_some());
    /// let dept_row = dept_row.unwrap();
    /// 
    /// let department_name = dept_row.get::<&str>(0)?.unwrap();
    /// assert_eq!("Sales", department_name);
    /// 
    /// let dept_row = dept_rows.next()?;
    /// assert!(dept_row.is_none());
    /// 
    /// let row = rows.next()?;
    /// assert!(row.is_none());
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn rows(&self) -> Result<Rows> {
        self.define_columns()?;
        let cols = self.borrow_columns()?;
        Ok( Rows::new(OCI_SUCCESS, cols, self) )
    }

    /// Initializes, if necessary, the internal vector of columns
    fn define_columns(&self) -> Result<()> {
        let mut cols = self.cols.borrow_mut();
        if cols.is_empty() {
            let num_columns = attr::get::<u32>(OCI_ATTR_PARAM_COUNT, OCI_HTYPE_STMT, self.stmt_ptr() as *const c_void, self.err_ptr())? as usize;
            cols.reserve_exact(num_columns);
            for pos in 1..=num_columns {
                let col = param::get::<OCIParam>(pos as u32, OCI_HTYPE_STMT, self.stmt_ptr() as *const c_void, self.err_ptr())?;
                let mut col = Column::new(pos, col)?;
                col.define_output_buffer(self)?;
                cols.push(col)
            }
        }
        Ok(())
    }
}
