//! Nonblocking cursor

use parking_lot::RwLock;

use crate::{Cursor, Result, Rows, oci::*, stmt::cols::Columns};

impl<'a> Cursor<'a> {
    /**
        Returns rows selected by this cursor

        # Example
        ```
        use sibyl::Cursor;

        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let stmt = conn.prepare("
            SELECT last_name
                 , CURSOR(
                        SELECT department_name
                          FROM hr.departments
                         WHERE department_id IN (
                                    SELECT department_id
                                      FROM hr.employees
                                     WHERE last_name = e.last_name)
                      ORDER BY department_name
                   ) AS departments
              FROM (
                    SELECT DISTINCT last_name
                      FROM hr.employees
                     WHERE last_name = :last_name
                   ) e
        ").await?;
        let rows = stmt.query(&[ &"King" ]).await?;

        let row = rows.next().await?.unwrap();
        let last_name : &str = row.get(0)?.unwrap();
        assert_eq!(last_name, "King");

        let departments : Cursor = row.get(1)?.unwrap();
        let mut dept_rows = departments.rows().await?;

        let dept_row = dept_rows.next().await?.unwrap();
        let department_name : &str = dept_row.get(0)?.unwrap();
        assert_eq!(department_name, "Executive");

        let dept_row = dept_rows.next().await?.unwrap();
        let department_name : &str = dept_row.get(0)?.unwrap();
        assert_eq!(department_name, "Sales");

        assert!(dept_rows.next().await?.is_none());
        assert!(rows.next().await?.is_none());
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn rows(&'a self) -> Result<Rows<'a>> {
        // We do not really need this async, but it makes the API look better :-)
        // Cursor::rows is called in the same fashion as Statement::rows - via .await
        async {
            if self.cols.get().is_none() {
                let cols = Columns::new(self, self.max_long)?;
                self.cols.get_or_init(|| RwLock::new(cols));
            }
            Ok( Rows::from_cursor(OCI_SUCCESS, self) )
        }.await
    }
}