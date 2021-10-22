use super::{ Stmt, cols::Columns };
use crate::{ 
    Position, RowID, Result,
    attr,
    oci::*,
    err::Error,
    env::Env,
    conn::Connection,
    types::Ctx,
    fromsql::FromSql,
};
use libc::c_void;

/// Methods that the provider of the returned results (`Statement` or `Cursor`) must implement.
pub trait ResultSetProvider : Stmt {
    fn get_cols(&self) -> Option<&Columns>;
    fn get_ctx(&self) -> &dyn Ctx;
    fn get_env(&self) -> &dyn Env;
    fn conn(&self) -> &Connection;
}

/// Result set of a query
pub struct Rows<'a> {
    rset: &'a dyn ResultSetProvider,
    last_result: i32,
}

impl<'a> Rows<'a> {
    pub(crate) fn new(res: i32, rset: &'a dyn ResultSetProvider) -> Self {
        Self { rset, last_result: res }
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
    pub fn next(&mut self) -> Result<Option<Row<'a>>> {
        if self.last_result == OCI_NO_DATA {
            Ok( None )
        } else {
            self.last_result = unsafe {
                OCIStmtFetch2(self.rset.stmt_ptr(), self.rset.err_ptr(), 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
            };
            match self.last_result {
                OCI_NO_DATA => Ok( None ),
                OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Ok( Some(Row::new(self.rset)) ),
                _ => Err( Error::oci(self.rset.err_ptr(), self.last_result) )
            }
        }
    }
}

/// A row in the returned result set
pub struct Row<'a> {
    rset: &'a dyn ResultSetProvider,
}

impl<'a> Row<'a> {
    fn new(rset: &'a dyn ResultSetProvider) -> Self {
        Self { rset }
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
        self.rset.get_cols().and_then(|cols| {
            pos.name().and_then(|name| cols.col_index(name)).or(pos.index())
                .map(|ix| cols.is_null(ix))
        }).unwrap_or(true)
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
        assert_eq!(manager_id.unwrap(), 102);

        // Or a column name can be used to get the data
        let manager_id: Option<u32> = row.get("MANAGER_ID")?;
        assert!(manager_id.is_some());
        assert_eq!(manager_id.unwrap(), 102);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get<T: FromSql<'a>, P: Position>(&'a self, pos: P) -> Result<Option<T>> {
        if let Some(cols) = self.rset.get_cols() {
            if let Some(pos) = pos.name().and_then(|name| cols.col_index(name)).or(pos.index()) {
                if cols.is_null(pos) {
                    Ok(None)
                } else {
                    cols.get(self.rset, pos)
                }
            } else {
                Err(Error::new("no such column"))
            }
        } else {
            Err(Error::new("projection is not initialized"))
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
        assert_eq!(manager_id, 102);

        let rowid = row.get_rowid()?;

        let stmt = conn.prepare("
            UPDATE hr.employees
               SET manager_id = :mgr_id
             WHERE rowid = :row_id
        ")?;
        let num_updated = stmt.execute(&[
            &( ":MGR_ID", 102 ),
            &( ":ROW_ID", &rowid )
        ])?;
        assert_eq!(num_updated, 1);
        # conn.rollback()?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_rowid(&self) -> Result<RowID> {
        let mut rowid = RowID::new(self.rset.env_ptr())?;
        attr::get_into(OCI_ATTR_ROWID, &mut rowid, OCI_HTYPE_STMT, self.rset.stmt_ptr() as *const c_void, self.rset.err_ptr())?;
        Ok( rowid )
    }
}
