//! SQL or PL/SQL statement

mod args;
mod bind;
mod cols;
mod cursor;
mod rows;
mod data;

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

pub use args::{StmtInArg, StmtOutArg, ToSql, ToSqlOut};
pub use cursor::Cursor;
pub use rows::{Row, Rows};
pub use cols::ColumnType;

use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{Result, conn::SvcCtx, oci::*, Connection, types::Ctx};

use std::sync::Arc;

use self::{bind::Params, cols::{Columns, ColumnInfo}};

/// Allows column or output variable identification by either
/// its numeric position or its name.
pub trait Position {
    fn index(&self) -> Option<usize>;
    fn name(&self)  -> Option<&str>;
}

impl Position for usize {
    fn index(&self) -> Option<usize> { Some(*self) }
    fn name(&self)  -> Option<&str>  { None }
}

impl Position for &str {
    fn index(&self) -> Option<usize> { None }
    fn name(&self)  -> Option<&str>  { Some(*self) }
}

/// Represents a prepared for execution SQL or PL/SQL statement
pub struct Statement<'a> {
    conn:     &'a Connection<'a>,
    svc:      Arc<SvcCtx>,
    stmt:     Ptr<OCIStmt>,
    params:   Option<RwLock<Params>>,
    cols:     OnceCell<RwLock<Columns>>,
    err:      Handle<OCIError>,
    max_long: u32,
}

impl AsRef<OCIEnv> for Statement<'_> {
    fn as_ref(&self) -> &OCIEnv {
        self.conn.as_ref()
    }
}

impl AsRef<OCIError> for Statement<'_> {
    fn as_ref(&self) -> &OCIError {
        self.conn.as_ref()
    }
}

impl AsRef<OCISvcCtx> for Statement<'_> {
    fn as_ref(&self) -> &OCISvcCtx {
        self.conn.as_ref()
    }
}

impl AsRef<OCIStmt> for Statement<'_> {
    fn as_ref(&self) -> &OCIStmt {
        self.stmt.as_ref()
    }
}

impl Ctx for Statement<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        self.conn.try_as_session()
    }
}

impl<'a> Statement<'a> {
    fn get_attr<T: attr::AttrGet>(&self, attr_type: u32) -> Result<T> {
        attr::get(attr_type, OCI_HTYPE_STMT, self.stmt.as_ref(), self.as_ref())
    }

    fn set_attr<T: attr::AttrSet>(&self, attr_type: u32, attr_val: T) -> Result<()> {
        attr::set(attr_type, attr_val, OCI_HTYPE_STMT, self.stmt.as_ref(), self.as_ref())
    }

    pub(crate) fn read_columns(&self) -> RwLockReadGuard<Columns> {
        self.cols.get().expect("locked columns").read()
    }

    pub(crate) fn write_columns(&self) -> RwLockWriteGuard<Columns> {
        self.cols.get().expect("locked columns").write()
    }

    pub(crate) fn conn(&self) -> &Connection {
        self.conn
    }

    /**
        Sets the number of top-level rows to be prefetched. The default value is 1 row.

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
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, first_name, last_name
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        stmt.set_prefetch_rows(10)?;
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
        #     SELECT employee_id, first_name, last_name
        #       FROM hr.employees
        #      WHERE manager_id = :id
        # ").await?;
        # stmt.set_prefetch_rows(10)?;
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
        has to be changed **before** the `query` is run.

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
        let mut stmt = conn.prepare("
            SELECT text
              FROM test_long_and_raw_data
             WHERE id = :id
        ")?;
        stmt.set_max_long_size(100_000);
        let rows = stmt.query(&[ &id ])?;
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
        # let mut stmt = conn.prepare("
        #     SELECT text
        #       FROM test_long_and_raw_data
        #      WHERE id = :id
        # ").await?;
        # stmt.set_max_long_size(100_000);
        # let rows = stmt.query(&[ &id ]).await?;
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

    /**
        Returns he number of columns in the select-list of this statement.

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
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, last_name, first_name
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        let rows = stmt.query(&[ &103 ])?;
        let num_cols = stmt.column_count()?;
        assert_eq!(num_cols, 3);
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
        #     SELECT employee_id, last_name, first_name
        #       FROM hr.employees
        #      WHERE manager_id = :id
        # ").await?;
        # let rows = stmt.query(&[ &103 ]).await?;
        # let num_cols = stmt.column_count()?;
        # assert_eq!(num_cols, 3);
        # Ok(()) })
        # }
        ```
    */
    pub fn column_count(&self) -> Result<usize> {
        let num_columns = self.get_attr::<u32>(OCI_ATTR_PARAM_COUNT)? as usize;
        Ok( num_columns )
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
        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, first_name, last_name
              FROM hr.employees
             WHERE manager_id = :id
          ORDER BY employee_id
        ")?;
        stmt.set_prefetch_rows(5)?;
        let rows = stmt.query(&[ &103 ])?;
        let mut ids = Vec::new();
        while let Some( row ) = rows.next()? {
            // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
            let id : u32 = row.get(0)?.unwrap();
            ids.push(id);
        }
        assert_eq!(stmt.row_count()?, 4);
        assert_eq!(ids.len(), 4);
        assert_eq!(ids.as_slice(), &[104 as u32, 105, 106, 107]);
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
        #     SELECT employee_id, first_name, last_name
        #       FROM hr.employees
        #      WHERE manager_id = :id
        #   ORDER BY employee_id
        # ").await?;
        # stmt.set_prefetch_rows(5)?;
        # let rows = stmt.query(&[ &103 ]).await?;
        # let mut ids = Vec::new();
        # while let Some( row ) = rows.next().await? {
        #     let id : u32 = row.get(0)?.unwrap();
        #     ids.push(id);
        # }
        # assert_eq!(stmt.row_count()?, 4);
        # assert_eq!(ids.len(), 4);
        # assert_eq!(ids.as_slice(), &[104 as u32, 105, 106, 107]);
        # Ok(()) })
        # }
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

    /**
        Checks whether the value returned for the output parameter is NULL.
    */
    pub fn is_null(&self, pos: impl Position) -> Result<bool> {
        self.params.as_ref().map(|params| params.read().is_null(pos)).unwrap_or(Ok(true))
    }

    /**
        Returns `pos` column meta data handler. `pos` is 0-based. Returns None if
        `pos` is greater than the number of columns in the query or if the prepared
        statement is not a SELECT and has no columns.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        use sibyl::ColumnType;

        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, last_name, first_name
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        let rows = stmt.query(&[ &103 ])?;
        let col = stmt.column(0).expect("employee_id column info");
        assert_eq!(col.name()?, "EMPLOYEE_ID");
        assert_eq!(col.data_type()?, ColumnType::Number);
        assert_eq!(col.precision()?, 6);
        assert_eq!(col.scale()?, 0);
        assert!(!col.is_null()?);
        assert!(col.is_visible()?);
        assert!(!col.is_identity()?);
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
        #     SELECT employee_id, last_name, first_name
        #       FROM hr.employees
        #      WHERE manager_id = :id
        # ").await?;
        # let rows = stmt.query(&[ &103 ]).await?;
        # let col = stmt.column(0).expect("employee_id column info");
        # assert_eq!(col.name()?, "EMPLOYEE_ID");
        # assert_eq!(col.data_type()?, ColumnType::Number);
        # assert_eq!(col.precision()?, 6);
        # assert_eq!(col.scale()?, 0);
        # assert!(!col.is_null()?);
        # assert!(col.is_visible()?);
        # assert!(!col.is_identity()?);
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
}

