//! Blocking cursor

use parking_lot::RwLock;

use crate::{Cursor, Result, Rows, oci::*, stmt::cols::Columns};

impl<'a> Cursor<'a> {
    /**
    Returns rows selected by this cursor

    # Example

    ```
    use sibyl::Cursor;

    # let session = sibyl::test_env::get_session()?;
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
    ")?;
    let row = stmt.query_single("King")?.unwrap();

    let last_name : &str = row.get(0)?;
    assert_eq!(last_name, "King");

    let departments : Cursor = row.get(1)?;
    let mut dept_rows = departments.rows()?;
    let dept_row = dept_rows.next()?.unwrap();

    let department_name : &str = dept_row.get(0)?;
    assert_eq!(department_name, "Executive");

    let dept_row = dept_rows.next()?.unwrap();
    let department_name : &str = dept_row.get(0)?;
    assert_eq!(department_name, "Sales");

    assert!(dept_rows.next()?.is_none());
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn rows(&self) -> Result<Rows<'_>> {
        if self.cols.get().is_none() {
            let cols = Columns::new(Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), Ptr::from(self.as_ref()), self.max_long)?;
            self.cols.get_or_init(|| RwLock::new(cols));
        };
        Ok( Rows::from_cursor(OCI_SUCCESS, self) )
    }
}