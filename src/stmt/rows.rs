//! Rows (result set) of a query (Statement) or a cursor.

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::sync::atomic::AtomicI32;

use super::{cols::Columns, data::FromSql, Position};
use crate::{Cursor, Error, Result, RowID, Statement, oci::{*, attr}, types::Ctx, Session};
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

pub(crate) enum DataSource<'a> {
    Statement(&'a Statement<'a>),
    Cursor(&'a Cursor<'a>)
}

impl AsRef<OCIEnv> for DataSource<'_> {
    fn as_ref(&self) -> &OCIEnv {
        match self {
            &Self::Statement(stmt) => stmt.as_ref(),
            &Self::Cursor(cursor)  => cursor.as_ref(),
        }
    }
}

impl AsRef<OCIError> for DataSource<'_> {
    fn as_ref(&self) -> &OCIError {
        match self {
            &Self::Statement(stmt) => stmt.as_ref(),
            &Self::Cursor(cursor)  => cursor.as_ref(),
        }
    }
}

impl AsRef<OCISvcCtx> for DataSource<'_> {
    fn as_ref(&self) -> &OCISvcCtx {
        match self {
            &Self::Statement(stmt) => stmt.as_ref(),
            &Self::Cursor(cursor)  => cursor.as_ref(),
        }
    }
}

impl AsRef<OCIStmt> for DataSource<'_> {
    fn as_ref(&self) -> &OCIStmt {
        match self {
            &Self::Statement(stmt) => stmt.as_ref(),
            &Self::Cursor(cursor)  => cursor.as_ref(),
        }
    }
}

impl Ctx for DataSource<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        match self {
            &Self::Statement(stmt) => stmt.try_as_session(),
            &Self::Cursor(cursor)  => cursor.try_as_session(),
        }
    }
}

impl DataSource<'_> {
    pub(crate) fn read_columns(&self) -> RwLockReadGuard<Columns> {
        match self {
            &Self::Statement(stmt) => stmt.read_columns(),
            &Self::Cursor(cursor)  => cursor.read_columns(),
        }
    }

    pub(crate) fn write_columns(&self) -> RwLockWriteGuard<Columns> {
        match self {
            &Self::Statement(stmt) => stmt.write_columns(),
            &Self::Cursor(cursor)  => cursor.write_columns(),
        }
    }

    pub(crate) fn session(&self) -> &Session {
        match self {
            &Self::Statement(stmt) => stmt.session(),
            &Self::Cursor(cursor)  => cursor.session(),
        }
    }
}

/// Result set of a query
pub struct Rows<'a> {
    rset: DataSource<'a>,
    last_result: AtomicI32,
}

impl<'a> Rows<'a> {
    pub(crate) fn from_query(query_result: i32, stmt: &'a Statement<'a>) -> Self {
        Self { rset: DataSource::Statement(stmt), last_result: AtomicI32::new(query_result) }
    }

    pub(crate) fn from_cursor(query_result: i32, cursor: &'a Cursor<'a>) -> Self {
        Self { rset: DataSource::Cursor(cursor), last_result: AtomicI32::new(query_result) }
    }
}

/// A row in the returned result set
pub struct Row<'a> {
    rset: &'a DataSource<'a>,
}

impl AsRef<OCIEnv> for Row<'_> {
    fn as_ref(&self) -> &OCIEnv {
        self.rset.as_ref()
    }
}

impl AsRef<OCIError> for Row<'_> {
    fn as_ref(&self) -> &OCIError {
        self.rset.as_ref()
    }
}

impl AsRef<OCISvcCtx> for Row<'_> {
    fn as_ref(&self) -> &OCISvcCtx {
        self.rset.as_ref()
    }
}

impl AsRef<OCIStmt> for Row<'_> {
    fn as_ref(&self) -> &OCIStmt {
        self.rset.as_ref()
    }
}

impl Ctx for Row<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        self.rset.try_as_session()
    }
}

impl<'a> Row<'a> {
    fn new(rows: &'a Rows) -> Self {
        Self { rset: &rows.rset }
    }

    pub(crate) fn session(&self) -> &Session {
        self.rset.session()
    }

    // `get` helper to ensure that the read lock is released when we have the index
    fn col_index_if_not_null(&self, pos: impl Position) -> Option<(usize, bool)> {
        let cols = self.rset.read_columns();
        pos.name().and_then(|name| cols.col_index(name)).or(pos.index())
            .map(|ix| (ix, cols.is_null(ix)))
    }

    /**
    Returns `true` if the value in the specified column is NULL.

    # Parameters

    * `pos` - column name or a zero-based column index

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = session.prepare("
        SELECT MAX(commission_pct)
          FROM hr.employees
         WHERE manager_id = :id
    ")?;
    let rows = stmt.query(120)?;
    let row = rows.next()?.unwrap();

    let commission_exists = !row.is_null(0);
    assert!(!commission_exists);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # let stmt = session.prepare("
    #     SELECT MAX(commission_pct)
    #       FROM hr.employees
    #      WHERE manager_id = :id
    # ").await?;
    # let rows = stmt.query(120).await?;
    # let row = rows.next().await?.unwrap();
    # let commission_exists = !row.is_null(0);
    # assert!(!commission_exists);
    # Ok(()) })
    # }
    ```

    ## Note

    This method considers the out of bounds or unknown/misnamed "columns" to be NULL.
    */
    pub fn is_null(&self, pos: impl Position) -> bool {
        let cols = self.rset.read_columns();
        pos.name().and_then(|name| cols.col_index(name)).or(pos.index())
            .map(|ix| cols.is_null(ix))
            .unwrap_or(true)
    }

    /**
    Returns `Option`-al value of the specified column in the current row.
    The returned value is `None` when the SQL value is `NULL`

    # Parameters

    * `pos` - column name or a zero-based column index

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest)

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = session.prepare("
        SELECT manager_id
          FROM hr.employees
         WHERE employee_id = :id
    ")?;
    let rows = stmt.query(107)?;
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
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # let stmt = session.prepare("
    #     SELECT manager_id
    #       FROM hr.employees
    #      WHERE employee_id = :id
    # ").await?;
    # let rows = stmt.query(107).await?;
    # let row = rows.next().await?.expect("first (and only) row");
    # let manager_id: Option<u32> = row.get(0)?;
    # assert!(manager_id.is_some());
    # assert_eq!(manager_id.unwrap(), 103);
    # let manager_id: Option<u32> = row.get("MANAGER_ID")?;
    # assert!(manager_id.is_some());
    # assert_eq!(manager_id.unwrap(), 103);
    # Ok(()) })
    # }
    ```
    */
    pub fn get<T: FromSql<'a>, P: Position>(&'a self, pos: P) -> Result<Option<T>> {
        match self.col_index_if_not_null(pos) {
            None => Err(Error::new("no such column")),
            Some((ix, is_null)) => {
                if is_null {
                    Ok(None)
                } else {
                    self.rset.write_columns().get(self, ix)
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

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest)

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = session.prepare("
        SELECT manager_id
          FROM hr.employees
         WHERE employee_id = :id
           FOR UPDATE
    ")?;
    let rows = stmt.query(107)?;
    let row = rows.next()?.expect("first (and only) row");
    let manager_id: u32 = row.get(0)?.unwrap();
    assert_eq!(manager_id, 103);

    let rowid = row.rowid()?;

    let stmt = session.prepare("
        UPDATE hr.employees
           SET manager_id = :mgr_id
         WHERE rowid = :row_id
    ")?;
    let num_updated = stmt.execute((
        (":MGR_ID", 103),
        (":ROW_ID", &rowid),
    ))?;
    assert_eq!(num_updated, 1);
    # session.rollback()?;
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # let stmt = session.prepare("
    #     SELECT manager_id
    #       FROM hr.employees
    #      WHERE employee_id = :id
    #        FOR UPDATE
    # ").await?;
    # let rows = stmt.query(107).await?;
    # let row = rows.next().await?.expect("first (and only) row");
    # let manager_id: u32 = row.get(0)?.unwrap();
    # assert_eq!(manager_id, 103);
    # let rowid = row.rowid()?;
    # let stmt = session.prepare("
    #     UPDATE hr.employees
    #        SET manager_id = :mgr_id
    #      WHERE rowid = :row_id
    # ").await?;
    # let num_updated = stmt.execute((
    #     (":MGR_ID", 103),
    #     (":ROW_ID", &rowid),
    # )).await?;
    # assert_eq!(num_updated, 1);
    # session.rollback().await?;
    # Ok(()) })
    # }
    ```
    */
    pub fn rowid(&self) -> Result<RowID> {
        let mut rowid = RowID::new(self)?;
        let stmt : &OCIStmt = self.as_ref();
        attr::get_into(OCI_ATTR_ROWID, &mut rowid, OCI_HTYPE_STMT, stmt, self.as_ref())?;
        Ok( rowid )
    }
}
