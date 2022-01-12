//! Blocking mode row fetch

use std::sync::atomic::Ordering;

use crate::{Result, Error, Rows, Row, oci::*};

impl<'a> Rows<'a> {
    /**
    Returns the next row in the SELECT's result set.

    # Example

    ```
    # let dbname = std::env::var("DBNAME")?;
    # let dbuser = std::env::var("DBUSER")?;
    # let dbpass = std::env::var("DBPASS")?;
    # let oracle = sibyl::env()?;
    # let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = session.prepare("
        SELECT street_address, postal_code, city, state_province
          FROM hr.locations
         WHERE country_id = :id
      ORDER BY location_id
    ")?;

    let rows = stmt.query("CA")?;

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
    pub fn next(&self) -> Result<Option<Row>> {
        if self.last_result.load(Ordering::Relaxed) == OCI_NO_DATA {
            Ok( None )
        } else {
            let res = unsafe {
                OCIStmtFetch2(self.rset.as_ref(), self.rset.as_ref(), 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
            };
            self.last_result.store(res, Ordering::Relaxed);
            match res {
                OCI_NO_DATA => Ok( None ),
                OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Ok( Some(Row::new(self)) ),
                _ => Err( Error::oci(self.rset.as_ref(), res) )
            }
        }
    }
}
