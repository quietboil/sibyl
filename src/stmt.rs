//! SQL or PL/SQL statement

pub mod args;
pub mod fromsql;
pub mod cols;
pub mod cursor;
pub mod rows;
#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
pub mod blocking;

pub use rows::{Rows, Row};
pub use cursor::Cursor;
pub use args::{ToSql, ToSqlOut, SqlInArg, SqlOutArg};
use rows::ResultSetProvider;
use cols::{Columns, Position, ColumnInfo};
use crate::{Result, catch, Error, Connection, oci::*, env::Env, types::Ctx};
use libc::c_void;
use std::{cell::Cell, collections::{HashMap, HashSet}, ptr};
use once_cell::unsync::OnceCell;

// type OCICallbackInBindFn = extern "C" fn(
//     ictxp:  *mut c_void,
//     bindp:  *mut OCIBind,
//     iter:   u32,
//     index:  u32,
//     bufpp:  &*mut c_void,
//     alenp:  *mut u32,
//     piecep: &mut u8,
//     indp:   &*mut c_void
// ) -> i32;
// type OCICallbackInBind = Option<OCICallbackInBindFn>;

// type OCICallbackOutBindFn = extern "C" fn(
//     octxp:  *mut c_void,
//     bindp:  *mut OCIBind,
//     iter:   u32,
//     index:  u32,
//     bufpp:  &*mut c_void,
//     alenp:  &*mut u32,
//     piecep: &mut u8,
//     indp:   &*mut c_void,
//     rcodep: &*mut u16
// ) -> i32;
// type OCICallbackOutBind = Option<OCICallbackOutBindFn>;

/// Represents a prepared for execution SQL or PL/SQL statement
pub struct Statement<'a> {
    conn:        &'a Connection<'a>,
    stmt:        Ptr<OCIStmt>,
    param_idxs:  HashMap<String,usize>,
    args_binds:  Vec<Ptr<OCIBind>>,

    indicators:  Vec<Cell<i16>>,
    data_sizes:  Vec<Cell<u32>>,
    cols:        OnceCell<Columns>,

    max_long:    Cell<u32>,
}

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        let env = self.env_ptr();
        let err = self.err_ptr();
        if let Some(cols) = self.cols.get_mut() {
            cols.drop_output_buffers(env, err);
        }
        let ocistmt = self.stmt.get();
        if !ocistmt.is_null() {
            unsafe {
                OCIStmtRelease(ocistmt, err, ptr::null(), 0, OCI_DEFAULT);
            }
        }
    }
}

impl Env for Statement<'_> {
    fn env_ptr(&self) -> *mut OCIEnv {
        self.conn.env_ptr()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.conn.err_ptr()
    }
}

pub trait Stmt: Env {
    fn stmt_ptr(&self) -> *mut OCIStmt;
}

impl Stmt for Statement<'_> {
    fn stmt_ptr(&self) -> *mut OCIStmt {
        self.stmt.get()
    }
}

impl Ctx for Statement<'_> {
    fn as_ptr(&self) -> *mut c_void {
        self.conn.usr_ptr() as *mut c_void
    }
}

impl ResultSetProvider for Statement<'_> {
    fn get_cols(&self) -> Option<&Columns> {
        self.cols.get()
    }

    fn get_ctx(&self) -> &dyn Ctx {
        self
    }

    fn get_env(&self) -> &dyn Env {
        self
    }

    fn conn(&self) -> &Connection {
        &self.conn
    }
}

fn get_bind_info(stmt: *mut OCIStmt, err: *mut OCIError) -> Result<(HashMap<String,usize>, Vec<Ptr<OCIBind>>, Vec<Cell<i16>>, Vec<Cell<u32>>)> {
    let num_binds = attr::get::<u32>(OCI_ATTR_BIND_COUNT, OCI_HTYPE_STMT, stmt as *const c_void, err)? as usize;
    let mut param_idxs = HashMap::with_capacity(num_binds);
    let mut args_binds = Vec::with_capacity(num_binds);
    let mut indicators = Vec::with_capacity(num_binds);
    let mut data_sizes = Vec::with_capacity(num_binds);
    if num_binds > 0 {
        let bind_names          = vec![ptr::null_mut::<u8>(); num_binds];
        let mut bind_name_lens  = vec![0u8; num_binds];
        let ind_names           = vec![ptr::null_mut::<u8>(); num_binds];
        let mut ind_name_lens   = vec![0u8; num_binds];
        let mut dups            = vec![0u8; num_binds];
        let mut oci_binds       = vec![ptr::null_mut::<OCIBind>(); num_binds];
        let mut found: i32      = 0;
        catch!{err =>
            OCIStmtGetBindInfo(
                stmt, err,
                num_binds as u32, 1, &mut found,
                bind_names.as_ptr(), bind_name_lens.as_mut_ptr(),
                ind_names.as_ptr(), ind_name_lens.as_mut_ptr(),
                dups.as_mut_ptr(),
                oci_binds.as_mut_ptr()
            )
        }
        for i in 0..found as usize {
            if dups[i] == 0 {
                let name = unsafe { std::slice::from_raw_parts(bind_names[i], bind_name_lens[i] as usize) };
                let name = String::from_utf8_lossy(name).to_string();
                param_idxs.insert(name, i);
            }
            args_binds.push(Ptr::new(oci_binds[i]));
            indicators.push(Cell::new(OCI_IND_NOTNULL));
            data_sizes.push(Cell::new(0u32));
        }
    }
    Ok((param_idxs, args_binds, indicators, data_sizes))
}

impl<'a> Statement<'a> {

    fn get_attr<V: attr::AttrGet>(&self, attr_type: u32) -> Result<V> {
        attr::get::<V>(attr_type, OCI_HTYPE_STMT, self.stmt_ptr() as *const c_void, self.err_ptr())
    }

    fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V) -> Result<()> {
        attr::set::<V>(attr_type, attr_val, OCI_HTYPE_STMT, self.stmt_ptr() as *mut c_void, self.err_ptr())
    }

    /// Binds the argument to a parameter placeholder at the specified position in the SQL statement
    fn bind_by_pos(&self, idx: usize, sql_type: u16, data: *mut c_void, buff_size: usize, data_size: *mut u32, null_ind: *mut i16) -> Result<()> {
        let pos = idx + 1;
        catch!{self.err_ptr() =>
            OCIBindByPos2(
                self.stmt_ptr(), self.args_binds[idx].as_ptr(), self.err_ptr(),
                pos as u32,
                data, buff_size as i64, sql_type,
                null_ind as *mut c_void,  // Pointer to an indicator variable or array
                data_size,                // Pointer to an array of actual lengths of array elements
                ptr::null_mut::<u16>(),   // Pointer to an array of column-level return codes
                0,                        // Maximum array length
                ptr::null_mut::<u32>(),   // Pointer to the actual number of elements in the array
                OCI_DEFAULT
            )
        }
        Ok(())
    }

    /// Returns index of the parameter placeholder.
    fn get_parameter_index(&self, name: &str) -> Result<usize> {
        // Try uppercase version of the parameter name first.
        // Explicitly convert to uppercase only if as-is search fails.
        if let Some(&ix) = self.param_idxs.get(&name[1..]) {
            Ok(ix)
        } else if let Some(&ix) = self.param_idxs.get(name[1..].to_uppercase().as_str()) {
            Ok(ix)
        } else {
            Err(Error::new(&format!("Statement does not define {} parameter placeholder", name)))
        }
    }

    /// Binds provided arguments to SQL parameter placeholders. Returns indexes of parameter placeholders for the OUT args.
    fn bind_args(&self, in_args: &[&dyn SqlInArg], out_args: &mut [&mut dyn SqlOutArg]) -> Result<Option<Vec<usize>>> {
        let mut args_idxs : HashSet<_> = self.param_idxs.values().cloned().collect();

        let mut idx = 0;
        for arg in in_args {
            let param_idx = if let Some( name ) = arg.name() { self.get_parameter_index(name)? } else { idx };
            let (sql_type, data, size) = arg.as_to_sql().to_sql();
            self.bind_by_pos(
                param_idx, sql_type, data as *mut c_void, size,
                ptr::null_mut::<u32>(),
                ptr::null_mut::<i16>()
            )?;
            args_idxs.remove(&param_idx);
            idx += 1;
        }

        let out_idxs = if out_args.is_empty() {
            None
        } else {
            let mut out_param_idxs = Vec::with_capacity(out_args.len());
            for arg in out_args {
                let param_idx = if let Some( name ) = arg.name() { self.get_parameter_index(name)? } else { idx };
                let (sql_type, data, data_buffer_size, in_size) = arg.as_to_sql_out().to_sql_output();
                if data_buffer_size == 0 {
                    let msg = if let Some( name ) = arg.name() {
                        format!("Storage capacity of output variable {} is 0", name)
                    } else {
                        format!("Storage capacity of output variable {} is 0", out_param_idxs.len())
                    };
                    return Err(Error::new(&msg));
                }
                self.data_sizes[param_idx].set(in_size as u32);
                self.bind_by_pos(
                    param_idx, sql_type, data, data_buffer_size,
                    self.data_sizes[param_idx].as_ptr(),
                    self.indicators[param_idx].as_ptr()
                )?;
                args_idxs.remove(&param_idx);
                out_param_idxs.push(param_idx);
                idx += 1;
            }
            Some(out_param_idxs)
        };

        // Check whether all placeholders are bound for this execution.
        // While OCIStmtExecute would see missing binds on the first run, the subsequent
        // execution of the same prepared statement might try to reuse previously bound
        // values, and those might already be gone. Hense the explicit check here.
        if !args_idxs.is_empty() {
            Err(Error::new("Not all parameters are bound"))
        } else {
            Ok(out_idxs)
        }
    }

    /**
        Checks whether the value returned for the output parameter is NULL.
    */
    pub fn is_null(&self, pos: impl Position) -> Result<bool> {
        pos.name()
            .and_then(|name|
                self.param_idxs.get(&name[1..])
                    .or(self.param_idxs.get(name[1..].to_uppercase().as_str()))
            )
            .map(|ix| *ix)
            .or(pos.index())
            .and_then(|ix| self.indicators.get(ix))
            .map(|cell| cell.get() == OCI_IND_NULL)
            .ok_or_else(|| Error::new("Parameter not found."))
    }

    /**
        Retrieves a single implicit result (cursor) in the order in which they were returned
        from the PL/SQL procedure or block. If no more results are available, then `None` is
        returned.

        PL/SQL provides a subprogram RETURN_RESULT in the DBMS_SQL package to return the result
        of an executed statement. Only SELECT query result-sets can be implicitly returned by a
        PL/SQL procedure or block.

        `next_result` can be called iteratively by the application to retrieve each implicit
        result from an executed PL/SQL statement. Applications retrieve each result-set sequentially
        but can fetch rows from any result-set independently.

        # Example
        ```
        use sibyl::Number;
        use std::cmp::Ordering::Equal;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            DECLARE
                c1 SYS_REFCURSOR;
                c2 SYS_REFCURSOR;
            BEGIN
                OPEN c1 FOR
                    SELECT department_name, first_name, last_name, salary
                     FROM (
                           SELECT first_name, last_name, salary, department_id
                                , ROW_NUMBER() OVER (ORDER BY salary) ord
                             FROM hr.employees
                          ) e
                     JOIN hr.departments d
                       ON d.department_id = e.department_id
                    WHERE ord = 1
                ;
                DBMS_SQL.RETURN_RESULT (c1);

                OPEN c2 FOR
                    SELECT department_name, first_name, last_name, salary
                      FROM (
                            SELECT first_name, last_name, salary, department_id
                                 , MEDIAN(salary) OVER () median_salary
                              FROM hr.employees
                           ) e
                      JOIN hr.departments d
                        ON d.department_id = e.department_id
                     WHERE salary = median_salary
                  ORDER BY department_name, last_name, first_name
                ;
                DBMS_SQL.RETURN_RESULT (c2);
            END;
        ")?;
        let expected_lowest_salary = Number::from_int(2100, &conn)?;
        let expected_median_salary = Number::from_int(6200, &conn)?;

        stmt.execute(&[])?;

        let lowest_payed_employee = stmt.next_result()?.unwrap();

        let mut rows = lowest_payed_employee.rows()?;
        let row = rows.next()?.unwrap();

        let department_name : &str = row.get(0)?.unwrap();
        let first_name : &str = row.get(1)?.unwrap();
        let last_name : &str = row.get(2)?.unwrap();
        let salary : Number = row.get(3)?.unwrap();

        assert_eq!(department_name, "Shipping");
        assert_eq!(first_name, "TJ");
        assert_eq!(last_name, "Olson");
        assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

        let row = rows.next()?;
        assert!(row.is_none());

        let median_salary_employees = stmt.next_result()?.unwrap();

        let mut rows = median_salary_employees.rows()?;

        let row = rows.next()?.unwrap();
        let department_name : &str = row.get(0)?.unwrap();
        let first_name : &str = row.get(1)?.unwrap();
        let last_name : &str = row.get(2)?.unwrap();
        let salary : Number = row.get(3)?.unwrap();

        assert_eq!(department_name, "Sales");
        assert_eq!(first_name, "Amit");
        assert_eq!(last_name, "Banda");
        assert_eq!(salary.compare(&expected_median_salary)?, Equal);

        let row = rows.next()?.unwrap();

        let department_name : &str = row.get(0)?.unwrap();
        let first_name : &str = row.get(1)?.unwrap();
        let last_name : &str = row.get(2)?.unwrap();
        let salary : Number = row.get(3)?.unwrap();

        assert_eq!(department_name, "Sales");
        assert_eq!(first_name, "Charles");
        assert_eq!(last_name, "Johnson");
        assert_eq!(salary.compare(&expected_median_salary)?, Equal);

        let row = rows.next()?;
        assert!(row.is_none());

        assert!(stmt.next_result()?.is_none());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn next_result(&'a self) -> Result<Option<Cursor>> {
        let stmt = Ptr::null();
        let mut stmt_type = 0u32;
        let res = unsafe {
            OCIStmtGetNextResult(self.stmt_ptr(), self.err_ptr(), stmt.as_ptr(), &mut stmt_type, OCI_DEFAULT)
        };
        match res {
            OCI_NO_DATA => Ok( None ),
            OCI_SUCCESS => Ok( Some ( Cursor::implicit(stmt, self) ) ),
            _ => Err( Error::oci(self.err_ptr(), res) )
        }
    }

    /**
        Sets the number of top-level rows to be prefetched. The default value is 1 row.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, first_name, last_name
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        stmt.set_prefetch_rows(10)?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_prefetch_rows(&self, num_rows: u32) -> Result<()> {
        self.set_attr(OCI_ATTR_PREFETCH_ROWS, num_rows)
    }

    /**
        Sets the maximum size of data that will be fetched from LONG and LONG RAW.

        By default 32768 bytes are allocated for values from LONG and LONG RAW columns.
        If the actual value is expected to be larger than that, then the "column size"
        has to be changed **before** the `query` is run.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
        #     DECLARE
        #         name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
        #     BEGIN
        #         EXECUTE IMMEDIATE '
        #             CREATE TABLE test_long_and_raw_data (
        #                 id      NUMBER GENERATED ALWAYS AS IDENTITY,
        #                 bin     RAW(100),
        #                 text    LONG
        #             )
        #         ';
        #     EXCEPTION
        #       WHEN name_already_used THEN
        #         EXECUTE IMMEDIATE '
        #             TRUNCATE TABLE test_long_and_raw_data
        #         ';
        #     END;
        # ")?;
        # stmt.execute(&[])?;
        # let stmt = conn.prepare("
        #     INSERT INTO test_long_and_raw_data (text) VALUES (:TEXT)
        #     RETURNING id INTO :ID
        # ")?;
        # let text = "When I have fears that I may cease to be Before my pen has gleaned my teeming brain, Before high-pilèd books, in charactery, Hold like rich garners the full ripened grain; When I behold, upon the night’s starred face, Huge cloudy symbols of a high romance, And think that I may never live to trace Their shadows with the magic hand of chance; And when I feel, fair creature of an hour, That I shall never look upon thee more, Never have relish in the faery power Of unreflecting love—then on the shore Of the wide world I stand alone, and think Till love and fame to nothingness do sink.";
        # let mut id = 0;
        # let count = stmt.execute_into(
        #     &[
        #         &(":TEXT", text)
        #     ], &mut [
        #         &mut (":ID", &mut id),
        #     ]
        # )?;
        let stmt = conn.prepare("
            SELECT text
              FROM test_long_and_raw_data
             WHERE id = :id
        ")?;
        stmt.set_max_long_size(100_000);
        let mut rows = stmt.query(&[ &id ])?;
        let row = rows.next()?.expect("first (and only) row");
        let txt : &str = row.get(0)?.expect("long text");
        # assert_eq!(txt, text);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_max_long_size(&self, size: u32) {
        self.max_long.set(size);
    }

    /**
        Returns he number of columns in the select-list of this statement.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, last_name, first_name
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        let mut _rows = stmt.query(&[ &103 ])?;
        let num_cols = stmt.get_column_count()?;

        assert_eq!(num_cols, 3);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_column_count(&self) -> Result<usize> {
        let num_columns = self.get_attr::<u32>(OCI_ATTR_PARAM_COUNT)? as usize;
        Ok( num_columns )
    }

    /**
        Returns `pos` column meta data handler. `pos` is 0-based. Returns None if
        `pos` is greater than the number of columns in the query or if the prepared
        statement is not a SELECT and has no columns.

        # Example
        ```
        use sibyl::ColumnType;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, last_name, first_name
              FROM hr.employees
             WHERE manager_id = :id
        ")?;
        let mut _rows = stmt.query(&[ &103 ])?;
        let col = stmt.get_column(0).expect("employee_id column info");
        assert_eq!(col.name()?, "EMPLOYEE_ID");
        assert_eq!(col.data_type()?, ColumnType::Number);
        assert_eq!(col.precision()?, 6);
        assert_eq!(col.scale()?, 0);
        assert!(!col.is_null()?);
        assert!(col.is_visible()?);
        assert!(!col.is_identity()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_column(&self, pos: usize) -> Option<ColumnInfo> {
        self.cols.get().and_then(|cols| cols.get_column_info(self, pos))
    }

    /**
        Returns the number of rows processed/seen so far in SELECT statements.

        For INSERT, UPDATE, and DELETE statements, it is the number of rows processed
        by the most recent statement.

        For nonscrollable cursors, it is the total number of rows fetched into user buffers
        since this statement handle was executed. Because they are forward sequential only,
        this also represents the highest row number seen by the application.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, first_name, last_name
              FROM hr.employees
             WHERE manager_id = :id
          ORDER BY employee_id
        ")?;
        stmt.set_prefetch_rows(5)?;
        let mut rows = stmt.query(&[ &103 ])?;
        let mut ids = Vec::new();
        while let Some( row ) = rows.next()? {
            // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
            let id : u32 = row.get(0)?.unwrap();
            ids.push(id);
        }
        assert_eq!(stmt.get_row_count()?, 4);
        assert_eq!(ids.len(), 4);
        assert_eq!(ids.as_slice(), &[104 as u32, 105, 106, 107]);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_row_count(&self) -> Result<usize> {
        let num_rows = self.get_attr::<u64>(OCI_ATTR_UB8_ROW_COUNT)? as usize;
        Ok( num_rows )
    }

    // Indicates the number of rows that were successfully fetched into the user's buffers
    // in the last fetch or execute with nonzero iterations.
    // This is not very useful in this implementation as we set up buffers for 1 row only.
    // pub fn get_rows_fetched(&self) -> Result<usize> {
    //     let num_rows = self.get_attr::<u32>(OCI_ATTR_ROWS_FETCHED)? as usize;
    //     Ok( num_rows )
    // }
}
