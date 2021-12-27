//! REF CURSOR

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use super::{Statement, args::ToSqlOut, cols::{Columns, ColumnInfo, DEFAULT_LONG_BUFFER_SIZE}, rows::Row};
use crate::{Result, oci::*, types::Ctx, Connection};
use libc::c_void;
use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

impl ToSqlOut for Handle<OCIStmt> {
    fn sql_type(&self) -> u16 { SQLT_RSET }
    fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.as_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIStmt>() }
}

pub(crate) enum RefCursor {
    Handle( Handle<OCIStmt> ),
    Ptr( Ptr<OCIStmt> )
}

impl AsRef<OCIStmt> for RefCursor {
    fn as_ref(&self) -> &OCIStmt {
        match self {
            RefCursor::Handle( handle ) => handle.as_ref(),
            RefCursor::Ptr( ptr )       => ptr.as_ref(),
        }
    }
}

impl RefCursor {
    // fn get_ptr(&self) -> Ptr<OCIStmt> {
    //     match self {
    //         RefCursor::Handle( handle ) => handle.get_ptr(),
    //         RefCursor::Ptr( ptr )       => *ptr,
    //     }
    // }

    fn as_mut_ptr(&mut self) -> *mut *mut OCIStmt {
        match self {
            RefCursor::Handle( handle ) => handle.as_mut_ptr(),
            RefCursor::Ptr( ptr )       => ptr.as_mut_ptr(),
        }
    }
}

enum CursorSource<'a> {
    Statement(&'a Statement<'a>),
    Row(&'a Row<'a>)
}

impl AsRef<OCIEnv> for CursorSource<'_> {
    fn as_ref(&self) -> &OCIEnv {
        match self {
            &Self::Statement(stmt) => stmt.as_ref(),
            &Self::Row(row)        => row.as_ref(),
        }
    }
}

impl AsRef<OCIError> for CursorSource<'_> {
    fn as_ref(&self) -> &OCIError {
        match self {
            &Self::Statement(stmt) => stmt.as_ref(),
            &Self::Row(row)        => row.as_ref(),
        }
    }
}

impl AsRef<OCISvcCtx> for CursorSource<'_> {
    fn as_ref(&self) -> &OCISvcCtx {
        match self {
            &Self::Statement(stmt) => stmt.as_ref(),
            &Self::Row(row)        => row.as_ref(),
        }
    }
}

impl Ctx for CursorSource<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        match self {
            &Self::Statement(stmt) => stmt.try_as_session(),
            &Self::Row(row)        => row.try_as_session(),
        }
    }
}

impl CursorSource<'_> {
    pub(crate) fn conn(&self) -> &Connection {
        match self {
            &Self::Statement(stmt) => stmt.conn(),
            &Self::Row(row)        => row.conn(),
        }
    }
}

/// Cursors - implicit results and REF CURSOR - from an executed PL/SQL statement
pub struct Cursor<'a> {
    source: CursorSource<'a>,
    cursor: RefCursor,
    cols:   OnceCell<RwLock<Columns>>,
    max_long: u32,
}

impl AsRef<OCIEnv> for Cursor<'_> {
    fn as_ref(&self) -> &OCIEnv {
        self.source.as_ref()
    }
}

impl AsRef<OCIError> for Cursor<'_> {
    fn as_ref(&self) -> &OCIError {
        self.source.as_ref()
    }
}

impl AsRef<OCISvcCtx> for Cursor<'_> {
    fn as_ref(&self) -> &OCISvcCtx {
        self.source.as_ref()
    }
}

impl AsRef<OCIStmt> for Cursor<'_> {
    fn as_ref(&self) -> &OCIStmt {
        self.cursor.as_ref()
    }
}

impl Ctx for Cursor<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        self.source.try_as_session()
    }
}

impl ToSqlOut for Cursor<'_> {
    fn sql_type(&self) -> u16 { SQLT_RSET }
    fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.cursor.as_mut_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIStmt>() }
}

impl<'a> Cursor<'a> {
    pub(crate) fn read_columns(&self) -> RwLockReadGuard<Columns> {
        self.cols.get().expect("locked columns").read()
    }

    pub(crate) fn write_columns(&self) -> RwLockWriteGuard<Columns> {
        self.cols.get().expect("locked columns").write()
    }

    pub(crate) fn conn(&self) -> &Connection {
        self.source.conn()
    }

    /**
        Creates a Cursor that can be used as an OUT argument to receive a returning REF CURSOR.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        use sibyl::{Cursor, Number};
        use std::cmp::Ordering::Equal;

        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
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
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let stmt = conn.prepare("
        #     BEGIN
        #         OPEN :lowest_payed_employee FOR
        #             SELECT department_name, first_name, last_name, salary
        #               FROM (
        #                     SELECT first_name, last_name, salary, department_id
        #                          , ROW_NUMBER() OVER (ORDER BY salary) ord
        #                      FROM hr.employees
        #                    ) e
        #               JOIN hr.departments d
        #                 ON d.department_id = e.department_id
        #              WHERE ord = 1
        #         ;
        #         OPEN :median_salary_employees FOR
        #             SELECT department_name, first_name, last_name, salary
        #               FROM (
        #                     SELECT first_name, last_name, salary, department_id
        #                          , MEDIAN(salary) OVER () median_salary
        #                      FROM hr.employees
        #                    ) e
        #               JOIN hr.departments d
        #                 ON d.department_id = e.department_id
        #              WHERE salary = median_salary
        #           ORDER BY department_name, last_name, first_name
        #         ;
        #     END;
        # ").await?;
        # let mut lowest_payed_employee   = Cursor::new(&stmt)?;
        # let mut median_salary_employees = Cursor::new(&stmt)?;
        # stmt.execute_into(&[], &mut [
        #     &mut ( ":LOWEST_PAYED_EMPLOYEE",   &mut lowest_payed_employee   ),
        #     &mut ( ":MEDIAN_SALARY_EMPLOYEES", &mut median_salary_employees ),
        # ]).await?;
        # let expected_lowest_salary = Number::from_int(2100, &conn)?;
        # let expected_median_salary = Number::from_int(6200, &conn)?;
        # let rows = lowest_payed_employee.rows().await?;
        # let row = rows.next().await?.unwrap();
        # let department_name : &str = row.get(0)?.unwrap();
        # let first_name : &str = row.get(1)?.unwrap();
        # let last_name : &str = row.get(2)?.unwrap();
        # let salary : Number = row.get(3)?.unwrap();
        # assert_eq!(department_name, "Shipping");
        # assert_eq!(first_name, "TJ");
        # assert_eq!(last_name, "Olson");
        # assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);
        # let row = rows.next().await?;
        # assert!(row.is_none());
        # let rows = median_salary_employees.rows().await?;
        # let row = rows.next().await?.unwrap();
        # let department_name : &str = row.get(0)?.unwrap();
        # let first_name : &str = row.get(1)?.unwrap();
        # let last_name : &str = row.get(2)?.unwrap();
        # let salary : Number = row.get(3)?.unwrap();
        # assert_eq!(department_name, "Sales");
        # assert_eq!(first_name, "Amit");
        # assert_eq!(last_name, "Banda");
        # assert_eq!(salary.compare(&expected_median_salary)?, Equal);
        # let row = rows.next().await?.unwrap();
        # let department_name : &str = row.get(0)?.unwrap();
        # let first_name : &str = row.get(1)?.unwrap();
        # let last_name : &str = row.get(2)?.unwrap();
        # let salary : Number = row.get(3)?.unwrap();
        # assert_eq!(department_name, "Sales");
        # assert_eq!(first_name, "Charles");
        # assert_eq!(last_name, "Johnson");
        # assert_eq!(salary.compare(&expected_median_salary)?, Equal);
        # let row = rows.next().await?;
        # assert!(row.is_none());
        # Ok(()) })
        # }
        ```
        See also [`Statement::next_result`] for another method to return REF CURSORs.
    */
    pub fn new(stmt: &'a Statement) -> Result<Self> {
        let handle = Handle::<OCIStmt>::new(stmt)?;
        Ok(
            Self {
                source:   CursorSource::Statement(stmt),
                cursor:   RefCursor::Handle( handle ),
                cols:     OnceCell::new(),
                max_long: DEFAULT_LONG_BUFFER_SIZE
            }
        )
    }

    // next_result
    pub(crate) fn implicit(istmt: Ptr<OCIStmt>, stmt: &'a Statement) -> Self {
        Self {
            source:   CursorSource::Statement(stmt),
            cursor:   RefCursor::Ptr( istmt ),
            cols:     OnceCell::new(),
            max_long: DEFAULT_LONG_BUFFER_SIZE
        }
    }

    // column in a row
    pub(crate) fn explicit(handle: Handle<OCIStmt>, row: &'a Row<'a>) -> Self {
        Self {
            source:   CursorSource::Row(row),
            cursor:   RefCursor::Handle( handle ),
            cols:     OnceCell::new(),
            max_long: DEFAULT_LONG_BUFFER_SIZE
        }
    }

    fn get_attr<V: attr::AttrGet>(&self, attr_type: u32) -> Result<V> {
        let stmt: &OCIStmt = self.as_ref();
        attr::get(attr_type, OCI_HTYPE_STMT, stmt, self.as_ref())
    }

    fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V) -> Result<()> {
        let stmt: &OCIStmt = self.as_ref();
        attr::set(attr_type, attr_val, OCI_HTYPE_STMT, stmt, self.as_ref())
    }

    /**
        Returns he number of columns in the select-list of this statement.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        use sibyl::Cursor;

        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
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
        assert_eq!(subordinates.column_count()?, 3);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let stmt = conn.prepare("
        #     BEGIN
        #         OPEN :subordinates FOR
        #             SELECT employee_id, last_name, first_name
        #               FROM hr.employees
        #              WHERE manager_id = :id
        #         ;
        #     END;
        # ").await?;
        # let mut subordinates = Cursor::new(&stmt)?;
        # stmt.execute_into(&[&(":ID", 103)], &mut [
        #     &mut (":SUBORDINATES", &mut subordinates),
        # ]).await?;
        # assert_eq!(subordinates.column_count()?, 3);
        # Ok(()) })
        # }
        ```
    */
    pub fn column_count(&self) -> Result<usize> {
        let num_columns = self.get_attr::<u32>(OCI_ATTR_PARAM_COUNT)?;
        Ok( num_columns as usize )
    }

    /**
        Returns `pos` column meta data handler. `pos` is 0-based. Returns None if
        `pos` is greater than the number of columns in the query or if the prepared
        statement is not a SELECT and has no columns.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        use sibyl::{Cursor, ColumnType};

        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
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
        let col = subordinates.column(0).expect("ID column info");
        assert_eq!(col.name()?, "EMPLOYEE_ID", "column name");
        assert_eq!(col.data_type()?, ColumnType::Number, "column type");
        assert_eq!(col.precision()?, 6, "number precision");
        assert_eq!(col.scale()?, 0, "number scale");
        assert!(!col.is_null()?, "not null");
        assert!(col.is_visible()?, "is visible");
        assert!(!col.is_identity()?, "not an identity column");
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let stmt = conn.prepare("
        #     BEGIN
        #         OPEN :subordinates FOR
        #             SELECT employee_id, last_name, first_name
        #               FROM hr.employees
        #              WHERE manager_id = :id
        #         ;
        #     END;
        # ").await?;
        # let mut subordinates = Cursor::new(&stmt)?;
        # stmt.execute_into(&[&(":ID", 103)], &mut [
        #     &mut (":SUBORDINATES", &mut subordinates),
        # ]).await?;
        # let mut _rows = subordinates.rows().await?;
        # let col = subordinates.column(0).expect("ID column info");
        # assert_eq!(col.name()?, "EMPLOYEE_ID", "column name");
        # assert_eq!(col.data_type()?, ColumnType::Number, "column type");
        # assert_eq!(col.precision()?, 6, "number precision");
        # assert_eq!(col.scale()?, 0, "number scale");
        # assert!(!col.is_null()?, "not null");
        # assert!(col.is_visible()?, "is visible");
        # assert!(!col.is_identity()?, "not an identity column");
        # Ok(()) })
        # }
        ```
    */
    pub fn column(&self, pos: usize) -> Option<ColumnInfo> {
        self.cols.get()
            .and_then(|cols|
                cols.read().column_param(pos)
            ).map(|param|
                ColumnInfo::new(param, self.as_ref())
            )
    }

    /**
        Returns the number of rows processed/seen so far in SELECT statements.

        For INSERT, UPDATE, and DELETE statements, it is the number of rows processed
        by the most recent statement.

        For nonscrollable cursors, it is the total number of rows fetched into user buffers
        since this statement handle was executed. Because they are forward sequential only,
        this also represents the highest row number seen by the application.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        use sibyl::Cursor;

        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
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
        subordinates.set_prefetch_rows(5)?;
        let rows = subordinates.rows()?;
        let mut ids = Vec::new();
        while let Some( row ) = rows.next()? {
            // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
            let id : usize = row.get(0)?.unwrap();
            ids.push(id);
        }
        assert_eq!(subordinates.row_count()?, 4);
        assert_eq!(ids.len(), 4);
        assert_eq!(ids.as_slice(), &[104 as usize, 105, 106, 107]);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let stmt = conn.prepare("
        #     BEGIN
        #         OPEN :subordinates FOR
        #             SELECT employee_id, last_name, first_name
        #               FROM hr.employees
        #              WHERE manager_id = :id
        #           ORDER BY employee_id
        #         ;
        #     END;
        # ").await?;
        # let mut subordinates = Cursor::new(&stmt)?;
        # stmt.execute_into(&[&(":ID", 103)], &mut [
        #     &mut (":SUBORDINATES", &mut subordinates),
        # ]).await?;
        # subordinates.set_prefetch_rows(5)?;
        # let mut rows = subordinates.rows().await?;
        # let mut ids = Vec::new();
        # while let Some( row ) = rows.next().await? {
        #     let id : usize = row.get(0)?.unwrap();
        #     ids.push(id);
        # }
        # assert_eq!(subordinates.row_count()?, 4);
        # assert_eq!(ids.len(), 4);
        # assert_eq!(ids.as_slice(), &[104 as usize, 105, 106, 107]);
        # Ok(()) })
        # }
        ```
    */
    pub fn row_count(&self) -> Result<usize> {
        let num_rows = self.get_attr::<u64>(OCI_ATTR_UB8_ROW_COUNT)?;
        Ok( num_rows as usize )
    }

    /**
        Sets the number of top-level rows to be prefetched. The default value is 1 row.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        use sibyl::Cursor;

        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
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
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # let stmt = conn.prepare("
        #     BEGIN
        #         OPEN :subordinates FOR
        #             SELECT employee_id, last_name, first_name
        #               FROM hr.employees
        #              WHERE manager_id = :id
        #           ORDER BY employee_id
        #         ;
        #     END;
        # ").await?;
        # let mut subordinates = Cursor::new(&stmt)?;
        # stmt.execute_into(&[&(":ID", 103)], &mut [
        #     &mut (":SUBORDINATES", &mut subordinates),
        # ]).await?;
        # subordinates.set_prefetch_rows(10)?;
        # Ok(()) })
        # }
        ```
    */
    pub fn set_prefetch_rows(&self, num_rows: u32) -> Result<()> {
        self.set_attr(OCI_ATTR_PREFETCH_ROWS, num_rows)
    }

    /**
        Sets the maximum size of data that will be fetched from LONG and LONG RAW.

        By default 32768 bytes are allocated for values from LONG and LONG RAW columns.
        If the actual value is expected to be larger than that, then the "column size"
        has to be changed before `query` is run.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        use sibyl::Cursor;

        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
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
        #       WHEN name_already_used THEN NULL;
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
        long_texts.set_max_long_size(100_000);
        let rows = long_texts.rows()?;
        let row = rows.next()?.expect("first (and only) row");
        let txt : &str = row.get(0)?.expect("long text");
        # assert_eq!(txt, text);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
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
        #       WHEN name_already_used THEN NULL;
        #     END;
        # ").await?;
        # stmt.execute(&[]).await?;
        # let stmt = conn.prepare("
        #     INSERT INTO test_long_and_raw_data (text) VALUES (:TEXT)
        #     RETURNING id INTO :ID
        # ").await?;
        # let text = "When I have fears that I may cease to be Before my pen has gleaned my teeming brain, Before high-pilèd books, in charactery, Hold like rich garners the full ripened grain; When I behold, upon the night’s starred face, Huge cloudy symbols of a high romance, And think that I may never live to trace Their shadows with the magic hand of chance; And when I feel, fair creature of an hour, That I shall never look upon thee more, Never have relish in the faery power Of unreflecting love—then on the shore Of the wide world I stand alone, and think Till love and fame to nothingness do sink.";
        # let mut id = 0;
        # let count = stmt.execute_into(
        #     &[
        #         &(":TEXT", text)
        #     ], &mut [
        #         &mut (":ID", &mut id),
        #     ]
        # ).await?;
        # let stmt = conn.prepare("
        #     BEGIN
        #         OPEN :long_texts FOR
        #             SELECT text
        #               FROM test_long_and_raw_data
        #              WHERE id = :id
        #         ;
        #     END;
        # ").await?;
        # let mut long_texts = Cursor::new(&stmt)?;
        # stmt.execute_into(&[&(":ID", &id)], &mut [
        #     &mut (":LONG_TEXTS", &mut long_texts),
        # ]).await?;
        # long_texts.set_max_long_size(100_000);
        # let rows = long_texts.rows().await?;
        # let row = rows.next().await?.expect("first (and only) row");
        # let txt : &str = row.get(0)?.expect("long text");
        # assert_eq!(txt, text);
        # Ok(()) })
        # }
        ```
    */
    pub fn set_max_long_size(&mut self, size: u32) {
        self.max_long = size;
    }
}
