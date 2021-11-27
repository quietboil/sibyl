//! Rows (result set) of a query (Statement) or a cursor.

use super::{ResultSetColumns, ResultSetConnection, cols::{Columns, Position}, fromsql::FromSql};
use crate::{Cursor, Error, Result, RowID, Statement, env::Env, conn::Connection, stmt::Stmt, oci::{*, attr}, types::Ctx};
use libc::c_void;
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

pub(crate) enum ResultSetSource<'a> {
    Statement(&'a Statement<'a>),
    Cursor(&'a Cursor<'a>)
}

impl ResultSetColumns for ResultSetSource<'_> {
    fn read_columns(&self) -> RwLockReadGuard<Columns> {
        match self {
            Self::Statement(stmt) => stmt.read_columns(),
            Self::Cursor(cursor) => cursor.read_columns(),
        }
    }

    fn write_columns(&self) -> RwLockWriteGuard<Columns> {
        match self {
            Self::Statement(stmt) => stmt.write_columns(),
            Self::Cursor(cursor) => cursor.write_columns(),
        }
    }
}

impl ResultSetConnection for ResultSetSource<'_> {
    fn conn(&self) -> &Connection {
        match self {
            Self::Statement(stmt) => stmt.conn(),
            Self::Cursor(cursor) => cursor.conn(),
        }
    }

}

/// Result set of a query
pub struct Rows<'a> {
    rset: ResultSetSource<'a>,
    last_result: i32,
}

impl Env for Rows<'_> {
    fn env_ptr(&self) -> *mut OCIEnv {
        match &self.rset {
            ResultSetSource::Statement(stmt) => stmt.env_ptr(),
            ResultSetSource::Cursor(cursor)  => cursor.env_ptr(),
        }
    }

    fn err_ptr(&self) -> *mut OCIError {
        match &self.rset {
            ResultSetSource::Statement(stmt) => stmt.err_ptr(),
            ResultSetSource::Cursor(cursor)  => cursor.err_ptr(),
        }
    }
}

impl Ctx for Rows<'_> {
    fn ctx_ptr(&self) -> *mut c_void {
        match &self.rset {
            ResultSetSource::Statement(stmt) => stmt.ctx_ptr(),
            ResultSetSource::Cursor(cursor)  => cursor.ctx_ptr(),
        }
    }
}

impl Stmt for Rows<'_> {
    fn stmt_ptr(&self) -> *mut OCIStmt {
        match &self.rset {
            ResultSetSource::Statement(stmt) => stmt.stmt_ptr(),
            ResultSetSource::Cursor(cursor)  => cursor.stmt_ptr(),
        }
    }
}

impl<'a> Rows<'a> {
    pub(crate) fn from_query(query_result: i32, stmt: &'a Statement<'a>) -> Self {
        Self { rset: ResultSetSource::Statement(stmt), last_result: query_result }
    }

    pub(crate) fn from_cursor(query_result: i32, cursor: &'a Cursor<'a>) -> Self {
        Self { rset: ResultSetSource::Cursor(cursor), last_result: query_result }
    }

    /**
        Returns the next row in the SELECT's result set.
        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT street_address, postal_code, city, state_province
              FROM hr.locations
             WHERE country_id = :id
          ORDER BY location_id
        ")?;
        let mut rows = stmt.query(&[&(":ID", "CA")])?;
        let mut res = Vec::new();
        while let Some( row ) = rows.next()? {
            // &str does not live long enough to be useful for
            // the `street_address`
            let street_address : Option<String> = row.get(0)?;
            let postal_code    : Option<&str>   = row.get(1)?;
            let city           : Option<&str>   = row.get(2)?;
            let state_province : Option<&str>   = row.get(3)?;
            let city_address = format!("{} {} {}",
                city           .unwrap_or_default(),
                state_province .unwrap_or_default(),
                postal_code    .unwrap_or_default(),
            );
            res.push((street_address.unwrap_or_default(), city_address));
        }
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].1, "Toronto Ontario M5V 2L7");
        assert_eq!(res[1].1, "Whitehorse Yukon YSW 9T2");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn next(&mut self) -> Result<Option<Row>> {
        if self.last_result == OCI_NO_DATA {
            Ok( None )
        } else {
            self.last_result = unsafe {
                OCIStmtFetch2(self.stmt_ptr(), self.err_ptr(), 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
            };
            match self.last_result {
                OCI_NO_DATA => Ok( None ),
                OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Ok( Some(Row::new(self)) ),
                _ => Err( Error::oci(self.err_ptr(), self.last_result) )
            }
        }
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

    pub(crate) fn conn(&self) -> &Connection {
        self.rows.rset.conn()
    }

    pub(crate) fn get_ctx(&self) -> &dyn Ctx {
        self.conn()
    }

    pub(crate) fn get_env(&self) ->&dyn Env {
        self.conn()
    }

    /**
        Returns `true` if the value in the specified column is NULL.

        # Example

        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT MAX(commission_pct)
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        let mut rows = stmt.query(&[ &120 ])?;
        let row = rows.next()?.unwrap();
        let commission_exists = !row.is_null(0);
        assert!(!commission_exists);
        # Ok::<(),Box<dyn std::error::Error>>(())
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
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT manager_id
              FROM hr.employees
             WHERE employee_id = :id
        ")?;
        let mut rows = stmt.query(&[ &107 ])?;
        let row = rows.next()?.expect("first (and only) row");

        // Either a 0-based column position...
        let manager_id: Option<u32> = row.get(0)?;
        assert!(manager_id.is_some());
        assert_eq!(manager_id.unwrap(), 103);

        // Or the column name can be used to get the data
        let manager_id: Option<u32> = row.get("MANAGER_ID")?;
        assert!(manager_id.is_some());
        assert_eq!(manager_id.unwrap(), 103);
        # Ok::<(),Box<dyn std::error::Error>>(())
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
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT manager_id
              FROM hr.employees
             WHERE employee_id = :id
               FOR UPDATE
        ")?;
        let mut rows = stmt.query(&[ &107 ])?;
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
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn rowid(&self) -> Result<RowID> {
        let mut rowid = RowID::new(self.get_env().env_ptr())?;
        attr::get_into(OCI_ATTR_ROWID, &mut rowid, OCI_HTYPE_STMT, self.rows.stmt_ptr() as *const c_void, self.err_ptr())?;
        Ok( rowid )
    }
}
