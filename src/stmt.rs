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

pub use args::ToSql;
pub use data::FromSql;
pub use bind::Params;
pub use cursor::Cursor;
pub use rows::{Row, Rows};
pub use cols::ColumnType;

use once_cell::sync::OnceCell;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{Result, session::SvcCtx, oci::*, Session, types::Ctx};

use std::{sync::Arc, fmt::Display};

use cols::{Columns, ColumnInfo};

/// Allows column or output variable identification by either
/// its numeric position or its name.
pub trait Position: Display {
    fn index(&self) -> Option<usize>;
    fn name(&self)  -> Option<&str>  { None }
}

impl Position for usize {
    fn index(&self) -> Option<usize> { Some(*self) }
}

impl Position for &str {
    fn index(&self) -> Option<usize> { None }
    fn name(&self)  -> Option<&str>  { Some(*self) }
}

/// Represents a prepared for execution SQL or PL/SQL statement
pub struct Statement<'a> {
    session:  &'a Session<'a>,
    stmt:     Ptr<OCIStmt>,
    params:   Option<RwLock<Params>>,
    cols:     OnceCell<RwLock<Columns>>,
    err:      Handle<OCIError>,
    svc:      Arc<SvcCtx>,
    max_long: u32,
}

#[cfg(not(docsrs))]
impl Drop for Statement<'_> {
    fn drop(&mut self) {
        #[cfg(feature="nonblocking")]
        let _ = self.svc.set_blocking_mode();

        oci_stmt_release(&self.stmt, &self.err);

        #[cfg(feature="nonblocking")]
        let _ = self.svc.set_nonblocking_mode();
    }
}

impl AsRef<OCIEnv> for Statement<'_> {
    fn as_ref(&self) -> &OCIEnv {
        self.session.as_ref()
    }
}

impl AsRef<OCIError> for Statement<'_> {
    fn as_ref(&self) -> &OCIError {
        self.session.as_ref()
    }
}

impl AsRef<OCISvcCtx> for Statement<'_> {
    fn as_ref(&self) -> &OCISvcCtx {
        self.session.as_ref()
    }
}

impl AsRef<OCIStmt> for Statement<'_> {
    fn as_ref(&self) -> &OCIStmt {
        self.stmt.as_ref()
    }
}

impl Ctx for Statement<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        self.session.try_as_session()
    }
}

impl<'a> Statement<'a> {
    fn get_attr<T: attr::AttrGet>(&self, attr_type: u32) -> Result<T> {
        attr::get(attr_type, OCI_HTYPE_STMT, self.stmt.as_ref(), self.as_ref())
    }

    fn set_attr<T: attr::AttrSet>(&self, attr_type: u32, attr_val: T) -> Result<()> {
        attr::set(attr_type, attr_val, OCI_HTYPE_STMT, self.stmt.as_ref(), self.as_ref())
    }

    pub(crate) fn read_columns(&self) -> RwLockReadGuard<'_,Columns> {
        self.cols.get().expect("locked columns").read()
    }

    pub(crate) fn write_columns(&self) -> RwLockWriteGuard<'_,Columns> {
        self.cols.get().expect("locked columns").write()
    }

    pub(crate) fn session(&self) -> &Session<'_> {
        self.session
    }

    /**
    Sets the number of top-level rows to be prefetched. The default value is 10 rows.

    # Parameters

    * `num_rows` The number of top-level rows to be prefetched

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT employee_id, first_name, last_name
          FROM hr.employees
         WHERE manager_id = :id
    ")?;
    stmt.set_prefetch_rows(5)?;
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
        SELECT employee_id, first_name, last_name
          FROM hr.employees
         WHERE manager_id = :id
    ").await?;
    stmt.set_prefetch_rows(5)?;
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn set_prefetch_rows(&self, num_rows: u32) -> Result<()> {
        self.set_attr(OCI_ATTR_PREFETCH_ROWS, num_rows)
    }

    /**
    Sets the maximum size of data that will be fetched from LONG and LONG RAW.

    By default 32768 bytes are allocated for values from LONG and LONG RAW columns.
    If the actual value is expected to be larger than that, then the "max long size"
    has to be set **before** the `query` is run.

    # Parameters

    * `size` - The maximum sizeof data that will be fetched

    # Example

    ## Blocking

    ```
    /*
        CREATE TABLE long_and_raw_test_data (
            id      INTEGER GENERATED ALWAYS AS IDENTITY,
            bin     RAW(100),
            text    LONG
        )
     */
    static TEXT : &str = "
        When I have fears that I may cease to be
        Before my pen has gleaned my teeming brain,
        Before high-pilèd books, in charactery,
        Hold like rich garners the full ripened grain;
        When I behold, upon the night’s starred face,
        Huge cloudy symbols of a high romance,
        And think that I may never live to trace
        Their shadows with the magic hand of chance;
        And when I feel, fair creature of an hour,
        That I shall never look upon thee more,
        Never have relish in the faery power
        Of unreflecting love—then on the shore
        Of the wide world I stand alone, and think
        Till love and fame to nothingness do sink.
    ";
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let session = sibyl::test_env::get_session()?;
    # let stmt = session.prepare("
    #     INSERT INTO long_and_raw_test_data (text) VALUES (:TEXT) RETURNING id INTO :ID
    # ")?;
    # let mut id = 0;
    # let count = stmt.execute(((":TEXT", &TEXT), (":ID", &mut id)))?;
    let mut stmt = session.prepare("
        SELECT text
          FROM long_and_raw_test_data
         WHERE id = :ID
    ")?;
    stmt.set_max_long_size(100_000);
    let row = stmt.query_single(&id)?.unwrap();
    let txt : &str = row.get(0)?;
    assert_eq!(txt, TEXT);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    static TEXT : &str = "
        When I have fears that I may cease to be
        Before my pen has gleaned my teeming brain,
        Before high-pilèd books, in charactery,
        Hold like rich garners the full ripened grain;
        When I behold, upon the night’s starred face,
        Huge cloudy symbols of a high romance,
        And think that I may never live to trace
        Their shadows with the magic hand of chance;
        And when I feel, fair creature of an hour,
        That I shall never look upon thee more,
        Never have relish in the faery power
        Of unreflecting love—then on the shore
        Of the wide world I stand alone, and think
        Till love and fame to nothingness do sink.
    ";
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # let stmt = session.prepare("
    #     INSERT INTO long_and_raw_test_data (text) VALUES (:TEXT) RETURNING id INTO :ID
    # ").await?;
    # let mut id = 0;
    # let count = stmt.execute(((":TEXT", &TEXT), (":ID", &mut id))).await?;
    let mut stmt = session.prepare("
        SELECT text
          FROM long_and_raw_test_data
         WHERE id = :ID
    ").await?;
    stmt.set_max_long_size(100_000);
    let row = stmt.query_single(&id).await?.unwrap();
    let txt : &str = row.get(0)?;
    assert_eq!(txt, TEXT);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn set_max_long_size(&mut self, size: u32) {
        self.max_long = size;
    }

    /**
    Returns he number of columns in the select-list of this statement.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT employee_id, last_name, first_name
          FROM hr.employees
         WHERE manager_id = :id
    ")?;
    let rows = stmt.query(103)?;
    let num_cols = stmt.column_count()?;

    assert_eq!(num_cols, 3);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
        SELECT employee_id, last_name, first_name
          FROM hr.employees
         WHERE manager_id = :id
    ").await?;
    let rows = stmt.query(103).await?;
    let num_cols = stmt.column_count()?;

    assert_eq!(num_cols, 3);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn column_count(&self) -> Result<usize> {
        let num_columns = self.get_attr::<u32>(OCI_ATTR_PARAM_COUNT)? as usize;
        Ok( num_columns )
    }

    /**
    Returns the number of rows processed/seen so far in SELECT statements.

    For INSERT, UPDATE, and DELETE statements, it is the number of rows processed
    by the statement.

    For nonscrollable cursors, it is the total number of rows fetched into user buffers
    since this statement handle was executed. Because they are forward sequential only,
    this also represents the highest row number seen by the application.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT employee_id, first_name, last_name
          FROM hr.employees
         WHERE manager_id = :id
      ORDER BY employee_id
    ")?;
    stmt.set_prefetch_rows(5)?;
    let rows = stmt.query(103)?;
    let mut ids = Vec::new();
    while let Some( row ) = rows.next()? {
        // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
        let id : u32 = row.get(0)?;
        ids.push(id);
    }
    assert_eq!(stmt.row_count()?, 4);
    assert_eq!(ids.len(), 4);
    assert_eq!(ids.as_slice(), &[104 as u32, 105, 106, 107]);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
        SELECT employee_id, first_name, last_name
          FROM hr.employees
         WHERE manager_id = :id
      ORDER BY employee_id
    ").await?;
    stmt.set_prefetch_rows(5)?;
    let rows = stmt.query(103).await?;
    let mut ids = Vec::new();
    while let Some( row ) = rows.next().await? {
        let id : i32 = row.get(0)?;
        ids.push(id);
    }
    assert_eq!(stmt.row_count()?, 4);
    assert_eq!(ids.len(), 4);
    assert_eq!(ids.as_slice(), &[104, 105, 106, 107]);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn row_count(&self) -> Result<usize> {
        let num_rows = self.get_attr::<u64>(OCI_ATTR_UB8_ROW_COUNT)? as usize;
        Ok( num_rows )
    }

    // Indicates the number of rows that were successfully fetched into the user's buffers
    // in the last fetch or execute with nonzero iterations.
    //
    // This is not very useful in this implementation as we set up buffers for 1 row only.
    //
    // pub fn rows_fetched(&self) -> Result<usize> {
    //     let num_rows = self.get_attr::<u32>(OCI_ATTR_ROWS_FETCHED)? as usize;
    //     Ok( num_rows )
    // }

    /**
    Checks whether the value returned for the output parameter is NULL.

    # Parameters

    * `pos` - parameter "position" - either the parameter name or a zero-based index

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        UPDATE hr.employees
           SET manager_id = :NEW_MANAGER_ID
         WHERE employee_id = :EMPLOYEE_ID
        RETURN commission_pct
          INTO :COMMISSION_PCT
    ")?;
    let mut commission_pct = 0f64;
    stmt.execute((
        (":EMPLOYEE_ID", 133),
        (":NEW_MANAGER_ID", 120),
        (":COMMISSION_PCT", &mut commission_pct),
    ))?;
    let commission_pct_is_null = stmt.is_null(":COMMISSION_PCT")?;
    assert!(commission_pct_is_null);

    // Alternatively an Option could be bound to a RETURNING
    // parameter that might be NULL
    let mut commission_pct = Some(0u64);
    stmt.execute((
        (":EMPLOYEE_ID", 133),
        (":NEW_MANAGER_ID", 120),
        (":COMMISSION_PCT", &mut commission_pct),
    ))?;
    assert!(commission_pct.is_none());
    # session.rollback()?;
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
        UPDATE hr.employees
           SET manager_id = :NEW_MANAGER_ID
         WHERE employee_id = :EMPLOYEE_ID
        RETURN commission_pct
          INTO :COMMISSION_PCT
    ").await?;
    let mut commission_pct = 0f64;
    stmt.execute((
        (":EMPLOYEE_ID", 133),
        (":NEW_MANAGER_ID", 120),
        (":COMMISSION_PCT", &mut commission_pct)
    )).await?;
    let commission_pct_is_null = stmt.is_null(":COMMISSION_PCT")?;
    assert!(commission_pct_is_null);

    // Alternatively an Option could be bound to a RETURNING
    // parameter that might be NULL
    let mut commission_pct = Some(0u64);
    stmt.execute((
        (":EMPLOYEE_ID", 133),
        (":NEW_MANAGER_ID", 120),
        (":COMMISSION_PCT", &mut commission_pct),
    )).await?;
    assert!(commission_pct.is_none());
    # session.rollback().await?;
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn is_null(&self, pos: impl Position) -> Result<bool> {
        self.params.as_ref().map(|params| params.read().is_null(pos)).unwrap_or(Ok(true))
    }

    /**
    Returns the size of the data in bytes bound to the specified parameter placeholder.

    This is the most useful for byte arrays bound to OUT or INOUT parameters. Unlike `String`
    or `Vec` byte slices cannot adjust their length when the size of the returned data is
    smaller than their size. This method can be used to do so after the data are fetched.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
    BEGIN
        :VAL := Utl_Raw.Cast_To_Raw('data');
    END;
    ")?;
    let mut data = [0; 8];
    stmt.execute(data.as_mut())?;

    assert_eq!(data, [0x64, 0x61, 0x74, 0x61, 0x00, 0x00, 0x00, 0x00]);
    // Note the "trailing" original zeros ----^^^^--^^^^--^^^^--^^^^
    assert_eq!(stmt.len_of("VAL")?, 4);

    let res = data[0..stmt.len_of("VAL")?].as_ref();
    assert_eq!(res.len(), 4);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
    BEGIN
        :VAL := Utl_Raw.Cast_To_Raw('data');
    END;
    ").await?;
    let mut data = [0; 8];
    stmt.execute(data.as_mut()).await?;

    assert_eq!(data, [0x64, 0x61, 0x74, 0x61, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(stmt.len_of(0)?, 4);

    let res = data[0..stmt.len_of(0)?].as_ref();
    assert_eq!(res.len(), 4);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn len_of(&self, pos: impl Position) -> Result<usize> {
        self.params.as_ref().map(|params| params.read().data_len(pos)).unwrap_or(Ok(0))
    }

    /**
    Returns column meta data.

    Returns None if the specified position is greater than the number of columns in the query
    or if the prepared statement is not a SELECT and has no columns.

    # Parameters

    * `pos` - zero-based column position

    # Example

    ## Blocking

    ```
    use sibyl::ColumnType;

    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT employee_id, last_name, first_name
          FROM hr.employees
         WHERE manager_id = :id
    ")?;
    let rows = stmt.query(103)?;
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
    # fn main() {}
    ```

    ## Nonblocking

    ```
    use sibyl::ColumnType;

    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
        SELECT employee_id, last_name, first_name
          FROM hr.employees
         WHERE manager_id = :id
    ").await?;
    let rows = stmt.query(103).await?;
    let col = stmt.column(0).expect("employee_id column info");

    assert_eq!(col.name()?, "EMPLOYEE_ID");
    assert_eq!(col.data_type()?, ColumnType::Number);
    assert_eq!(col.precision()?, 6);
    assert_eq!(col.scale()?, 0);
    assert!(!col.is_null()?);
    assert!(col.is_visible()?);
    assert!(!col.is_identity()?);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn column(&self, pos: usize) -> Option<ColumnInfo<'_>> {
        self.cols.get()
            .and_then(|cols|
                cols.read().column_param(pos)
            ).map(|param|
                ColumnInfo::new(param, self.as_ref())
            )
    }
}