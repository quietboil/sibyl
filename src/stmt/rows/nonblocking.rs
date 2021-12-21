//! Nonblocking mode row fetch

use std::sync::atomic::Ordering;

use crate::{Result, Error, Rows, Row, oci::{self, *}};

impl<'a> Rows<'a> {
    /**
        Returns the next row in the SELECT's result set.

        # Example

        ```
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let stmt = conn.prepare("
            SELECT street_address, postal_code, city, state_province
              FROM hr.locations
             WHERE country_id = :id
          ORDER BY location_id
        ").await?;
        let rows = stmt.query(&[&(":ID", "CA")]).await?;
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
        if self.last_result.load(Ordering::Relaxed) == OCI_NO_DATA {
            Ok( None )
        } else {
            let res = oci::futures::StmtFetch::new(self.rset.as_ref(), self.rset.as_ref()).await?;
            self.last_result.store(res, Ordering::Relaxed);
            match res {
                OCI_NO_DATA => Ok( None ),
                OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => Ok( Some(Row::new(self)) ),
                _ => Err( Error::oci(self.rset.as_ref(), res) )
            }
        }
    }
}
