//! Nonblocking cursor

use parking_lot::RwLock;

use crate::{Cursor, Result, Rows, oci::*, stmt::cols::Columns};

impl<'a> Cursor<'a> {
    /**
    Returns rows selected by this cursor

    # Example

    ```
    use sibyl::Cursor;

    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    let stmt = session.prepare("
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
    let row = stmt.query_single("King").await?.unwrap();

    let last_name : &str = row.get(0)?;
    assert_eq!(last_name, "King");

    let departments : Cursor = row.get(1)?;
    let mut dept_rows = departments.rows().await?;

    let dept_row = dept_rows.next().await?.unwrap();
    let department_name : &str = dept_row.get(0)?;
    assert_eq!(department_name, "Executive");

    let dept_row = dept_rows.next().await?.unwrap();
    let department_name : &str = dept_row.get(0)?;
    assert_eq!(department_name, "Sales");

    assert!(dept_rows.next().await?.is_none());
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn rows(&'a self) -> Result<Rows<'a>> {
        // We do not really need this async, but it makes the API more consistent -
        // Cursor::rows will be .await-ed in the same fashion as Statement::rows is
        async {
            if self.cols.get().is_none() {
                let cols = Columns::new(Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), self.max_long)?;
                self.cols.get_or_init(|| RwLock::new(cols));
            }
            Ok( Rows::from_cursor(OCI_SUCCESS, self) )
        }.await
    }
}