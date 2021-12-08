//! Rows (result set) of a query (Statement) or a cursor.

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::sync::atomic::AtomicI32;

use super::{ResultSetColumns, cols::{Columns, Position}, fromsql::FromSql, ResultSetConnection};
use crate::{Cursor, Error, Result, RowID, Statement, env::Env, oci::{*, attr}, stmt::Stmt, types::Ctx, Connection};
use libc::c_void;
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

pub(crate) enum ResultSetSource<'a> {
    Statement(&'a Statement<'a>),
    Cursor(&'a Cursor<'a>)
}

impl ResultSetColumns for ResultSetSource<'_> {
    fn read_columns(&self) -> RwLockReadGuard<Columns> {
        match self {
            &Self::Statement(stmt) => stmt.read_columns(),
            &Self::Cursor(cursor) => cursor.read_columns(),
        }
    }

    fn write_columns(&self) -> RwLockWriteGuard<Columns> {
        match self {
            &Self::Statement(stmt) => stmt.write_columns(),
            &Self::Cursor(cursor) => cursor.write_columns(),
        }
    }
}

impl ResultSetConnection for ResultSetSource<'_> {
    fn conn(&self) -> &Connection {
        match self {
            &Self::Statement(stmt) => stmt.conn(),
            &Self::Cursor(cursor) => cursor.conn(),
        }
    }
}

/// Result set of a query
pub struct Rows<'a> {
    rset: ResultSetSource<'a>,
    last_result: AtomicI32,
}

impl Env for Rows<'_> {
    fn env_ptr(&self) -> *mut OCIEnv {
        match &self.rset {
            &ResultSetSource::Statement(stmt) => stmt.env_ptr(),
            &ResultSetSource::Cursor(cursor)  => cursor.env_ptr(),
        }
    }

    fn err_ptr(&self) -> *mut OCIError {
        match &self.rset {
            &ResultSetSource::Statement(stmt) => stmt.err_ptr(),
            &ResultSetSource::Cursor(cursor)  => cursor.err_ptr(),
        }
    }

    fn get_env_ptr(&self) -> Ptr<OCIEnv> {
        Ptr::new(self.env_ptr())
    }

    fn get_err_ptr(&self) -> Ptr<OCIError> {
        Ptr::new(self.err_ptr())
    }
}

impl Ctx for Rows<'_> {
    fn ctx_ptr(&self) -> *mut c_void {
        match &self.rset {
            &ResultSetSource::Statement(stmt) => stmt.ctx_ptr(),
            &ResultSetSource::Cursor(cursor)  => cursor.ctx_ptr(),
        }
    }
}

impl Stmt for Rows<'_> {
    fn stmt_ptr(&self) -> *mut OCIStmt {
        match &self.rset {
            &ResultSetSource::Statement(stmt) => stmt.stmt_ptr(),
            &ResultSetSource::Cursor(cursor)  => cursor.stmt_ptr(),
        }
    }

    fn get_stmt_ptr(&self) -> Ptr<OCIStmt> {
        Ptr::new(self.stmt_ptr())
    }
}

impl<'a> Rows<'a> {
    pub(crate) fn from_query(query_result: i32, stmt: &'a Statement<'a>) -> Self {
        Self { rset: ResultSetSource::Statement(stmt), last_result: AtomicI32::new(query_result) }
    }

    pub(crate) fn from_cursor(query_result: i32, cursor: &'a Cursor<'a>) -> Self {
        Self { rset: ResultSetSource::Cursor(cursor), last_result: AtomicI32::new(query_result) }
    }
}

/// A row in the returned result set
pub struct Row<'a> {
    rows: &'a Rows<'a>,
}

impl<'a> Row<'a> {
    fn new(rows: &'a Rows) -> Self {
        Self { rows }
    }

    fn get_col_index(&self, pos: impl Position) -> Option<usize> {
        let cols = self.rows.rset.read_columns();
        pos.name().and_then(|name| cols.col_index(name)).or(pos.index())
    }

    fn col_is_null(&self, ix: usize) -> bool {
        self.rows.rset.read_columns().is_null(ix)
    }

    pub(crate) fn err_ptr(&self) -> *mut OCIError {
        self.rows.err_ptr()
    }

    pub(crate) fn env_ptr(&self) -> *mut OCIEnv {
        self.rows.env_ptr()
    }

    pub(crate) fn get_ctx(&self) -> &dyn Ctx {
        self.rows
    }

    pub(crate) fn get_env(&self) -> &dyn Env {
        self.rows
    }

    pub(crate) fn conn(&self) -> &Connection {
        self.rows.rset.conn()
    }

    /**
        Returns `true` if the value in the specified column is NULL.

        # Example

        ```
        # use sibyl::Result;
        // === Blocking mode variant ===
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = conn.prepare("
            SELECT MAX(commission_pct)
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        let rows = stmt.query(&[ &120 ])?;
        let row = rows.next()?.unwrap();

        let commission_exists = !row.is_null(0);
        assert!(!commission_exists);
        # Ok(())
        # }

        // === Nonblocking mode variant ===
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;

        let stmt = conn.prepare("
            SELECT MAX(commission_pct)
              FROM hr.employees
             WHERE manager_id = :id
        ").await?;
        let rows = stmt.query(&[ &120 ]).await?;
        let row = rows.next().await?.unwrap();

        let commission_exists = !row.is_null(0);
        assert!(!commission_exists);
        # Ok(()) })
        # }
        # fn main() -> Result<()> { test() }
        ```

        ## Note

        This method considers the out of bounds or unknown/misnamed
        "columns" to be NULL.
    */
    pub fn is_null(&self, pos: impl Position) -> bool {
        let cols = self.rows.rset.read_columns();
        pos.name().and_then(|name| cols.col_index(name)).or(pos.index())
            .map(|ix| cols.is_null(ix))
            .unwrap_or(true)
    }

    /**
        Returns `Option`-al value of the specified column in the current row.
        The returned value is `None` when the SQL value is `NULL`

        # Example
        ```
        # use sibyl::Result;
        // === Blocking mode variant ===
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = conn.prepare("
            SELECT manager_id
              FROM hr.employees
             WHERE employee_id = :id
        ")?;
        let rows = stmt.query(&[ &107 ])?;
        let row = rows.next()?.expect("first (and only) row");

        // Either a 0-based column position...
        let manager_id: Option<u32> = row.get(0)?;
        assert!(manager_id.is_some());
        assert_eq!(manager_id.unwrap(), 103);

        // Or the column name can be used to get the data
        let manager_id: Option<u32> = row.get("MANAGER_ID")?;
        assert!(manager_id.is_some());
        assert_eq!(manager_id.unwrap(), 103);
        # Ok(())
        # }

        // === Nonblocking mode variant ===
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;

        let stmt = conn.prepare("
            SELECT manager_id
              FROM hr.employees
             WHERE employee_id = :id
        ").await?;
        let rows = stmt.query(&[ &107 ]).await?;
        let row = rows.next().await?.expect("first (and only) row");

        let manager_id: Option<u32> = row.get(0)?;
        assert!(manager_id.is_some());
        assert_eq!(manager_id.unwrap(), 103);

        let manager_id: Option<u32> = row.get("MANAGER_ID")?;
        assert!(manager_id.is_some());
        assert_eq!(manager_id.unwrap(), 103);
        # Ok(()) })
        # }
        # fn main() -> Result<()> { test() }
        ```
    */
    pub fn get<T: FromSql<'a>, P: Position>(&'a self, pos: P) -> Result<Option<T>> {
        match self.get_col_index(pos) {
            None => Err(Error::new("no such column")),
            Some(ix) => {
                if self.col_is_null(ix) {
                    Ok(None)
                } else {
                    self.rows.rset.write_columns().get(self, ix)
                }
            }
        }
    }

    /**
        Returns the implicitily returned `RowID` of the current row in the SELECT...FOR UPDATE results.
        The returned `RowID` can be used in a later UPDATE or DELETE statement.

        # Notes
        This method is only valid for the SELECT...FOR UPDATE results as only those return ROWIDs implicitly.
        For all others the returned `RowID` will be empty (one might think about it as NULL).

        # Example
        ```
        # use sibyl::Result;
        // === Blocking mode variant ===
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = conn.prepare("
            SELECT manager_id
              FROM hr.employees
             WHERE employee_id = :id
               FOR UPDATE
        ")?;
        let rows = stmt.query(&[ &107 ])?;
        let row = rows.next()?.expect("first (and only) row");
        let manager_id: u32 = row.get(0)?.unwrap();
        assert_eq!(manager_id, 103);

        let rowid = row.rowid()?;

        let stmt = conn.prepare("
            UPDATE hr.employees
               SET manager_id = :mgr_id
             WHERE rowid = :row_id
        ")?;
        let num_updated = stmt.execute(&[
            &( ":MGR_ID", 103 ),
            &( ":ROW_ID", &rowid )
        ])?;
        assert_eq!(num_updated, 1);
        # conn.rollback()?;
        # Ok(())
        # }

        // === Nonblocking mode variant ===
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;

        let stmt = conn.prepare("
            SELECT manager_id
              FROM hr.employees
             WHERE employee_id = :id
               FOR UPDATE
        ").await?;
        let rows = stmt.query(&[ &107 ]).await?;
        let row = rows.next().await?.expect("first (and only) row");
        let manager_id: u32 = row.get(0)?.unwrap();
        assert_eq!(manager_id, 103);

        let rowid = row.rowid()?;

        let stmt = conn.prepare("
            UPDATE hr.employees
               SET manager_id = :mgr_id
             WHERE rowid = :row_id
        ").await?;
        let num_updated = stmt.execute(&[
            &( ":MGR_ID", 103 ),
            &( ":ROW_ID", &rowid )
        ]).await?;
        assert_eq!(num_updated, 1);
        # conn.rollback().await?;
        # Ok(()) })
        # }
        # fn main() -> Result<()> { test() }
        ```
    */
    pub fn rowid(&self) -> Result<RowID> {
        let mut rowid = RowID::new(self.get_env().env_ptr())?;
        attr::get_into(OCI_ATTR_ROWID, &mut rowid, OCI_HTYPE_STMT, self.rows.stmt_ptr() as *const c_void, self.err_ptr())?;
        Ok( rowid )
    }
}
