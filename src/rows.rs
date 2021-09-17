use crate::*;
use crate::stmt::Stmt;
use libc::c_void;
use std::cell::Cell;

const OCI_ATTR_ROWID : u32 = 19;

const OCI_FETCH_NEXT : u16 = 2;

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-DF585B90-58BA-45FC-B7CE-6F7F987C03B9
    fn OCIStmtFetch2(
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        nrows:      u32,
        orient:     u16,
        offset:     i16,
        mode:       u32
    ) -> i32;
}

/// Result set of a query
pub struct Rows<'s> {
    stmt: &'s dyn Stmt,
    last_result: Cell<i32>,
}

impl<'s> Rows<'s> {
    pub(crate) fn new(res: i32, stmt: &'s dyn Stmt) -> Self {
        Self { stmt, last_result: Cell::new(res) }
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
        let rows = stmt.query(&[ &"CA" ])?;
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

        assert_eq!(2, res.len());
        assert_eq!("Toronto Ontario M5V 2L7",  res[0].1);
        assert_eq!("Whitehorse Yukon YSW 9T2", res[1].1);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn next(&self) -> Result<Option<Row>> {
        let res = self.last_result.get();
        if res == OCI_NO_DATA {
            Ok( None )
        } else {
            let res = unsafe {
                OCIStmtFetch2(self.stmt.stmt_ptr(), self.stmt.err_ptr(), 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
            };
            self.last_result.set(res);
            match res {
                OCI_NO_DATA => Ok( None ),
                OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Ok( Some(Row::new(self.stmt)) ),
                _ => Err( Error::oci(self.stmt.err_ptr(), res) )
            }
        }
    }
}

/// A row in the returned result set
pub struct Row<'a> {
    stmt: &'a dyn Stmt,
}

impl<'a> Row<'a> {
    fn new(stmt: &'a dyn Stmt) -> Self {
        Self { stmt }
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
        let rows = stmt.query(&[ &120 ])?;
        let cur_row = rows.next()?;

        assert!(cur_row.is_some());

        let row = cur_row.unwrap();
        let commission_exists = !row.is_null(0);

        assert!(!commission_exists);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```

        ## Note

        This method considers the out of bounds or unknown/misnamed
        "columns" to be NULL.
    */
    pub fn is_null(&self, pos: impl Position) -> bool {
        let index = pos.name().and_then(|name| self.stmt.col_index(name)).or(pos.index());
        if let Some(index) = index {
            if let Some(col) = self.stmt.get_cols().borrow().get(index) {
                return col.is_null();
            }
        }
        true
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
        let rows = stmt.query(&[ &107 ])?;
        let cur_row = rows.next()?;

        assert!(cur_row.is_some());

        let row = cur_row.unwrap();
        let manager_id: u32 = row.get(0)?.unwrap_or_default();

        assert_eq!(manager_id, 102);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get<T: FromSql<'a>, P: Position>(&'a self, pos: P) -> Result<Option<T>> {
        let index = pos.name().and_then(|name| self.stmt.col_index(name)).or(pos.index());
        if let Some(index) = index {
            if let Some(col) = self.stmt.get_cols().borrow().get(index) {
                if col.is_null() {
                    return Ok(None);
                } else {
                    let value = FromSql::value(col.get_column_buffer(), self.stmt)?;
                    return Ok(Some(value));
                }
            }
        }
        Err( Error::new("column position is out of bounds") )
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
        let rows = stmt.query(&[ &107 ])?;
        let cur_row = rows.next()?;

        assert!(cur_row.is_some());

        let row = cur_row.unwrap();
        let manager_id: u32 = row.get(0)?.unwrap_or_default();

        assert_eq!(manager_id, 102);

        let rowid = row.get_rowid()?;

        let stmt = conn.prepare("
            UPDATE hr.employees
               SET manager_id = :mid
             WHERE rowid = :rid
        ")?;
        let num_updated = stmt.execute(&[
            &( ":mid", 102 ),
            &( ":rid", &rowid )
        ])?;
        assert_eq!(num_updated, 1);
        # conn.rollback()?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_rowid(&self) -> Result<RowID> {
        let mut rowid = RowID::new(self.stmt.env_ptr())?;
        attr::get_into(OCI_ATTR_ROWID, &mut rowid, OCI_HTYPE_STMT, self.stmt.stmt_ptr() as *const c_void, self.stmt.err_ptr())?;
        Ok( rowid )
    }
}
