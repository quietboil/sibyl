//! Nonblocking mode row fetch

use std::sync::atomic::Ordering;

use crate::{Result, Error, Rows, Row, oci::*, task::execute_blocking};

impl<'a> Rows<'a> {
    /**
    Returns the next row in the SELECT's result set.

    # Example

    ```
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
        SELECT street_address, postal_code, city, state_province
          FROM hr.locations
         WHERE country_id = :id
      ORDER BY location_id
    ").await?;

    let rows = stmt.query("CA").await?;

    let mut res = Vec::new();
    while let Some( row ) = rows.next().await? {
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
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn next(&'a self) -> Result<Option<Row<'a>>> {
        if self.last_result.load(Ordering::Acquire) == OCI_NO_DATA {
            Ok( None )
        } else {
            let stmt: &OCIStmt  = self.rset.as_ref();
            let err:  &OCIError = self.rset.as_ref();

            let res = if self.rset.read_columns().has_lob_col() {
                self.rset.session().set_blocking_mode()?;
                let stmt= Ptr::from(stmt);
                let err = Ptr::from(err);
                let res = execute_blocking(move || -> i32 {
                    unsafe {
                        OCIStmtFetch2(stmt.get(), err.get(), 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
                    }
                }).await;
                self.rset.session().set_nonblocking_mode()?;
                res?
            } else {
                futures::StmtFetch::new(self.rset.session().get_svc(), stmt, err).await?
            };

            self.last_result.store(res, Ordering::Release);
            match res {
                OCI_NO_DATA => Ok( None ),
                OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Ok( Some(Row::new(self)) ),
                _ => Err( Error::oci(self.rset.as_ref(), res) )
            }
        }
    }

    /// Variant of [`Rows::next()`] for a single row query
    pub(in crate::stmt) async fn single(self) -> Result<Option<Row<'a>>> {
        if self.last_result.load(Ordering::Relaxed) == OCI_NO_DATA {
            Ok( None )
        } else {
            let stmt: &OCIStmt  = self.rset.as_ref();
            let err:  &OCIError = self.rset.as_ref();

            let res = if self.rset.read_columns().has_lob_col() {
                self.rset.session().set_blocking_mode()?;
                let stmt= Ptr::from(stmt);
                let err = Ptr::from(err);
                let res = execute_blocking(move || -> i32 {
                    unsafe {
                        OCIStmtFetch2(stmt.get(), err.get(), 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
                    }
                }).await;
                self.rset.session().set_nonblocking_mode()?;
                res?
            } else {
                futures::StmtFetch::new(self.rset.session().get_svc(), stmt, err).await?
            };
            
            match res {
                OCI_NO_DATA => Ok( None ),
                OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Ok( Some(Row::single(self)) ),
                _ => Err( Error::oci(self.rset.as_ref(), res) )
            }
        }
    }
}
