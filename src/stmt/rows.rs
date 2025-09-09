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

macro_rules! impl_as_ref_for_data_source {
    ($($tname:ty),+) => {
        $(
            impl AsRef<$tname> for DataSource<'_> {
                fn as_ref(&self) -> &$tname {
                    match self {
                        &Self::Statement(stmt) => stmt.as_ref(),
                        &Self::Cursor(cursor)  => cursor.as_ref(),
                    }
                }
            }
        )+
    };
}

impl_as_ref_for_data_source!(OCIEnv, OCIError, OCISvcCtx, OCIStmt);

impl Ctx for DataSource<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        match self {
            &Self::Statement(stmt) => stmt.try_as_session(),
            &Self::Cursor(cursor)  => cursor.try_as_session(),
        }
    }
}

impl DataSource<'_> {
    pub(crate) fn read_columns(&self) -> RwLockReadGuard<'_, Columns> {
        match self {
            &Self::Statement(stmt) => stmt.read_columns(),
            &Self::Cursor(cursor)  => cursor.read_columns(),
        }
    }

    pub(crate) fn write_columns(&self) -> RwLockWriteGuard<'_, Columns> {
        match self {
            &Self::Statement(stmt) => stmt.write_columns(),
            &Self::Cursor(cursor)  => cursor.write_columns(),
        }
    }

    pub(crate) fn session(&self) -> &Session<'_> {
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

    fn src(self) -> DataSource<'a> {
        self.rset
    }
}

enum RowSource<'a> {
    Single(DataSource<'a>),
    Multi(&'a DataSource<'a>)
}

impl RowSource<'_> {
    fn rset(&self) -> &DataSource<'_> {
        match self {
            Self::Single(ds) => ds,
            &Self::Multi(ds) => ds,
        }
    }
}

macro_rules! impl_as_ref_for_row_source {
    ($($tname:ty),+) => {
        $(
            impl AsRef<$tname> for RowSource<'_> {
                fn as_ref(&self) -> &$tname {
                    match self {
                        Self::Single(ds) => ds.as_ref(),
                        &Self::Multi(ds) => ds.as_ref(),
                    }
                }
            }
        )+
    };
}

impl_as_ref_for_row_source!(OCIEnv, OCIError, OCISvcCtx, OCIStmt);


/// A row in the returned result set
pub struct Row<'a> {
    src: RowSource<'a>,
}

macro_rules! impl_as_ref_for_row {
    ($($tname:ty),+) => {
        $(
            impl AsRef<$tname> for Row<'_> {
                fn as_ref(&self) -> &$tname {
                    self.src.as_ref()
                }
            }
        )+
    };
}

impl_as_ref_for_row!(OCIEnv, OCIError, OCISvcCtx, OCIStmt);

impl Ctx for Row<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        self.src.rset().try_as_session()
    }
}

impl<'a> Row<'a> {
    fn new(rows: &'a Rows) -> Self {
        Self { src: RowSource::Multi(&rows.rset) }
    }

    fn single(rows: Rows<'a>) -> Self {
        Self { src: RowSource::Single(rows.src()) }
    }

    pub(crate) fn session(&self) -> &Session<'_> {
        self.src.rset().session()
    }

    // `get` helper to ensure that the read lock is released when we have the index
    fn col_index(&self, pos: &impl Position) -> Option<usize> {
        let cols = self.src.rset().read_columns();
        pos.name().and_then(|name| cols.col_index(name)).or(pos.index())
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
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT MAX(commission_pct)
          FROM hr.employees
         WHERE manager_id = :id
    ")?;
    let row = stmt.query_single(120)?.unwrap();

    let commission_exists = !row.is_null(0);
    assert!(!commission_exists);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # let stmt = session.prepare("
    #     SELECT MAX(commission_pct)
    #       FROM hr.employees
    #      WHERE manager_id = :id
    # ").await?;
    # let row = stmt.query_single(120).await?.unwrap();
    # let commission_exists = !row.is_null(0);
    # assert!(!commission_exists);
    # Ok(()) })
    # }
    ```

    ## Note

    This method considers the out of bounds or unknown/misnamed "columns" to be NULL.
    */
    pub fn is_null(&self, pos: impl Position) -> bool {
        let cols = self.src.rset().read_columns();
        pos.name().and_then(|name| cols.col_index(name)).or(pos.index())
            .map(|ix| cols.is_null(ix))
            .unwrap_or(true)
    }

    /**
    Returns value of the specified column in the row.

    The column can be specified either by its numeric index in the row, or by its column name.

    To fetch data from NULL-able columns save the returned data into `Option` of the approrpriate
    type. If the value in the column was NULL, then the saved value will be `None`.

    # Parameters

    * `pos` - column name or a zero-based column index

    # Failures

    * `Column does not exist` - the column as specified was not found
    * `Column is null` - method was used to fetch data from a NULL-able column **and**
        the column's value was NULL **and** the type of the returned value is not an `Option`

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest)

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT postal_code, country_id
          FROM hr.locations
         WHERE location_id = :id
    ")?;
    let row = stmt.query_single(2400)?.unwrap();

    // Either a 0-based column position...
    let postal_code : Option<&str> = row.get(0)?;
    assert!(postal_code.is_none());
    let country_id  : Option<&str> = row.get(1)?;
    assert!(country_id.is_some());
    let country_id = country_id.unwrap();
    assert_eq!(country_id, "UK");

    // Or the column name can be used to get the data
    let postal_code : Option<&str> = row.get("POSTAL_CODE")?;
    assert!(postal_code.is_none());
    let country_id  : Option<&str> = row.get("COUNTRY_ID")?;
    assert!(country_id.is_some());
    let country_id = country_id.unwrap();
    assert_eq!(country_id, "UK");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # let stmt = session.prepare("
    #     SELECT postal_code, country_id
    #       FROM hr.locations
    #      WHERE location_id = :id
    # ").await?;
    # let row = stmt.query_single(2400).await?.unwrap();
    # let postal_code : Option<&str> = row.get(0)?;
    # assert!(postal_code.is_none());
    # let country_id  : Option<&str> = row.get(1)?;
    # assert!(country_id.is_some());
    # let country_id = country_id.unwrap();
    # assert_eq!(country_id, "UK");
    # let postal_code : Option<&str> = row.get("POSTAL_CODE")?;
    # assert!(postal_code.is_none());
    # let country_id  : Option<&str> = row.get("COUNTRY_ID")?;
    # assert!(country_id.is_some());
    # let country_id = country_id.unwrap();
    # assert_eq!(country_id, "UK");
    # Ok(()) })
    # }
    ```
    */
    pub fn get<T: FromSql<'a>, P: Position>(&'a self, pos: P) -> Result<T> {
        match self.col_index(&pos) {
            None => Err(Error::msg(format!("Column {} does not exist", pos))),
            Some(index) => {
                if let Some(result) = self.src.rset().write_columns().col_mut(index).map(|col| FromSql::value(self, col)) {
                    result
                } else {
                    Err(Error::msg(format!("Column {} cannot be found", pos)))
                }
            }
        }
    }

    /**
    Returns value of the specified column in the current row.

    This method used to provides a friendlier alternative to [`get`](Row::get) to fetch data
    from `NOT NULL` columns at the time when `get` was always returning an `Option`.

    # Parameters

    * `pos` - column name or a zero-based column index

    # Failures

    * `Column does not exist` - the column as specified was not found
    * `Column is null` - method was used to fetch data from a NULL-able column **and**
        the column's value was NULL.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest)

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT postal_code, city, state_province, country_id
          FROM hr.locations
         WHERE location_id = :id
    ")?;
    let row = stmt.query_single(2400)?.unwrap();

    // CITY is NOT NULL
    let city : &str = row.get("CITY")?;

    assert_eq!(city, "London");

    // POSTAL_CODE, STATE_PROVINCE and COUNTRY_ID are all NULL-able
    let postal_code    : Option<&str> = row.get("POSTAL_CODE")?;
    let state_province : Option<&str> = row.get("STATE_PROVINCE")?;
    let country_id     : Option<&str> = row.get("COUNTRY_ID")?;

    assert!(postal_code.is_none());     // this one is NULL
    assert!(state_province.is_none());  // also NULL
    assert!(country_id.is_some());      // not NULL then
    let country_id = country_id.unwrap();
    assert_eq!(country_id, "UK");

    // We could have used `get` without `Option` to get `COUNTRY_ID`
    // even if it is NULL-able provided we had a posteriori knowledge
    // that all country IDs have values despite column being NOT NULL:

    let country_id : &str = row.get("COUNTRY_ID")?;

    assert_eq!(country_id, "UK");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # let stmt = session.prepare("
    #     SELECT postal_code, city, state_province, country_id
    #       FROM hr.locations
    #      WHERE location_id = :id
    # ").await?;
    # let row = stmt.query_single(2400).await?.unwrap();
    # let city : &str = row.get("CITY")?;
    # assert_eq!(city, "London");
    # let postal_code    : Option<&str> = row.get("POSTAL_CODE")?;
    # let state_province : Option<&str> = row.get("STATE_PROVINCE")?;
    # let country_id     : Option<&str> = row.get("COUNTRY_ID")?;
    # assert!(postal_code.is_none());
    # assert!(state_province.is_none());
    # assert!(country_id.is_some());
    # let country_id = country_id.unwrap();
    # assert_eq!(country_id, "UK");
    # let country_id : &str = row.get("COUNTRY_ID")?;
    # assert_eq!(country_id, "UK");
    # Ok(()) })
    # }
    ```
    */
    #[deprecated = "Use [`get`](Row::get) instead."]
    pub fn get_not_null<T: FromSql<'a>, P: Position>(&'a self, pos: P) -> Result<T> {
        self.get(pos)
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
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT manager_id
          FROM hr.employees
         WHERE employee_id = :id
           FOR UPDATE
    ")?;
    let row = stmt.query_single(107)?.unwrap();
    let manager_id: u32 = row.get(0)?;
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
    # let session = sibyl::test_env::get_session().await?;
    # let stmt = session.prepare("
    #     SELECT manager_id
    #       FROM hr.employees
    #      WHERE employee_id = :id
    #        FOR UPDATE
    # ").await?;
    # let row = stmt.query_single(107).await?.unwrap();
    # let manager_id: u32 = row.get(0)?;
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

#[cfg(all(test,feature="blocking"))]
mod tests {
    use crate::*;

    #[test]
    fn get_null() -> Result<()> {
        let session = crate::test_env::get_session()?;

        let stmt = session.prepare("
            SELECT postal_code, city, state_province, country_id
              FROM hr.locations
             WHERE location_id = :id
        ")?;
        let row = stmt.query_single(2400)?.unwrap();

        assert!(row.is_null("POSTAL_CODE"));
        assert!(!row.is_null("CITY"));
        assert!(row.is_null("STATE_PROVINCE"));
        assert!(!row.is_null("COUNTRY_ID"));

        let postal_code : Option<&str> = row.get("POSTAL_CODE")?;
        assert!(postal_code.is_none());
        let state_province : Option<&str> = row.get("STATE_PROVINCE")?;
        assert!(state_province.is_none());

        let city : &str = row.get("CITY")?;
        assert_eq!(city, "London");
        let country_id : &str = row.get("COUNTRY_ID")?;
        assert_eq!(country_id, "UK");

        let res : Result<&str> = row.get("POSTAL_CODE");
        assert!(res.is_err());
        match res {
            Err(Error::Interface(msg)) => assert_eq!(msg, "Column POSTAL_CODE is null"),
            _ => panic!("unexpected result {:?}", res),
        }

        Ok(())
    }

    #[test]
    fn column_indexing() -> Result<()> {
        use std::fmt::Display;

        let session = crate::test_env::get_session()?;

        let stmt = session.prepare("
            SELECT postal_code, city, state_province, country_id
              FROM hr.locations
             WHERE location_id = :id
        ")?;
        let row = stmt.query_single(2400)?.unwrap();

        #[derive(Clone,Copy)]
        enum Col {
            PostalCode, City, StateProvince, CountryId
        }
        impl Position for Col {
            fn index(&self) -> Option<usize> { Some(*self as _) }
        }
        impl Display for Col {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                static COLS : [&str;4] = ["POSTAL_CODE", "CITY", "STATE_PROVINCE", "COUNTRY_ID"];
                let i = *self as usize;
                f.write_str(COLS[i])
            }
        }

        assert!(row.is_null(Col::PostalCode));
        assert!(!row.is_null(Col::City));
        assert!(row.is_null(Col::StateProvince));
        assert!(!row.is_null(Col::CountryId));

        let postal_code : Option<&str> = row.get(Col::PostalCode)?;
        assert!(postal_code.is_none());
        let state_province : Option<&str> = row.get(Col::StateProvince)?;
        assert!(state_province.is_none());

        let city : &str = row.get(Col::City)?;
        assert_eq!(city, "London");
        let country_id : &str = row.get(Col::CountryId)?;
        assert_eq!(country_id, "UK");

        let res : Result<&str> = row.get(Col::PostalCode);
        assert!(res.is_err());
        match res {
            Err(Error::Interface(msg)) => assert_eq!(msg, "Column POSTAL_CODE is null"),
            _ => panic!("unexpected result {:?}", res),
        }

        Ok(())
    }
}