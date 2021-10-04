//! SQL or PL/SQL statement handling

mod defs;
pub mod args;
pub mod cols;
pub mod cursor;
pub mod rows;

use self::defs::*;
use self::args::*;
use self::cols::{Columns, ColumnInfo};
use self::cursor::Cursor;
use self::rows::{Rows, ResultSetProvider};
use crate::*;
use crate::types::Ctx;
use libc::c_void;
use std::{cell::{Cell, RefCell}, collections::{HashMap, HashSet}, ptr};

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-E6C1DC67-D464-4D2A-9F19-737423D31779
    fn OCIStmtPrepare2(
        svchp:      *mut OCISvcCtx,
        stmthp:     *mut *mut OCIStmt,
        errhp:      *mut OCIError,
        stmttext:   *const u8,
        stmt_len:   u32,
        key:        *const u8,
        keylen:     u32,
        language:   u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-256034CE-2ADB-4BE5-BC8D-748307F2EA8E
    fn OCIStmtRelease(
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        key:        *const u8,
        keylen:     u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-87D50C09-F18D-45BB-A8AF-1E6AFEC6FE2E
    fn OCIStmtGetBindInfo(
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        size:       u32,
        startloc:   u32,
        found:      *mut i32,
        bvnp:       *const *mut u8,
        bvnl:       *mut u8,
        invp:       *const *mut u8,
        invl:       *mut u8,
        dupl:       *mut u8,
        hndl:       *const *mut OCIBind
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-CD63DF78-2178-4727-A896-B9673C4A37F0
    // fn OCIBindByName2(
    //     stmtp:      *mut OCIStmt,
    //     bindpp:     *mut *mut OCIBind,
    //     errhp:      *mut OCIError,
    //     namep:      *const u8,
    //     name_len:   i32,
    //     valuep:     *mut c_void,
    //     value_sz:   i64,
    //     dty:        u16,
    //     indp:       *mut c_void,
    //     alenp:      *mut u32,
    //     rcodep:     *mut u16,
    //     maxarr_len: u32,
    //     curelep:    *mut u32,
    //     mode:       u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-D28DF5A7-3C75-4E52-82F7-A5D6D5714E69
    fn OCIBindByPos2(
        stmtp:      *mut OCIStmt,
        bindpp:     *mut *mut OCIBind,
        errhp:      *mut OCIError,
        position:   u32,
        valuep:     *mut c_void,
        value_sz:   i64,
        dty:        u16,
        indp:       *mut c_void,
        alenp:      *mut u32,
        rcodep:     *mut u16,
        maxarr_len: u32,
        curelep:    *mut u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-030270CB-346A-412E-B3B3-556DD6947BE2
    // fn OCIBindDynamic(
    //     bindp:      *mut OCIBind,
    //     errhp:      *mut OCIError,
    //     ictxp:      *mut c_void,
    //     icbfp:      OCICallbackInBind,
    //     octxp:      *mut c_void,
    //     ocbfp:      OCICallbackOutBind
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-98B26708-3E02-45C0-8258-5D5544F32BE9
    fn OCIStmtExecute(
        svchp:      *mut OCISvcCtx,
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        iters:      u32,
        rowoff:     u32,
        snap_in:    *const c_void,  // *const OCISnapshot
        snap_out:   *mut c_void,    // *mut OCISnapshot
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-60B998F9-F213-43BA-AB84-76F1EC6A6687
    fn OCIStmtGetNextResult(
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        result:     *mut *mut OCIStmt,
        rtype:      *mut u32,
        mode:       u32
    ) -> i32;
}

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
    stmt:        *mut OCIStmt,
    param_idxs:  HashMap<String,usize>,
    args_binds:  Vec<Cell<*mut OCIBind>>,
    indicators:  Vec<Cell<i16>>,
    data_sizes:  Vec<Cell<u32>>,
    cols:        RefCell<Columns>,
}

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        self.cols.borrow_mut().drop_output_buffers(self.env_ptr(), self.err_ptr());
        if !self.stmt.is_null() {
            unsafe {
                OCIStmtRelease(self.stmt, self.err_ptr(), ptr::null(), 0, OCI_DEFAULT);
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
        self.stmt
    }
}

impl Ctx for Statement<'_> {
    fn as_ptr(&self) -> *mut c_void {
        self.conn.usr_ptr() as *mut c_void
    }
}

impl ResultSetProvider for Statement<'_> {
    fn get_cols(&self) -> &RefCell<Columns> {
        &self.cols
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

fn define_binds(stmt: *mut OCIStmt, err: *mut OCIError) -> Result<(HashMap<String,usize>, Vec<Cell<*mut OCIBind>>, Vec<Cell<i16>>, Vec<Cell<u32>>)> {
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
            args_binds.push(Cell::new(oci_binds[i]));
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

    /// Binds the argument to a named placeholder in the SQL statement
    // fn bind_by_name(&self, name: &str, sql_type: u16, data: *mut c_void, buff_size: usize, data_size: *mut u32, null_ind: *mut i16) -> Result<()> {
    //     let arg_idx;
    //     if let Some( &idx ) = self.param_idxs.get(&name[1..]) {
    //         arg_idx = idx;
    //     } else if let Some( &idx ) = self.param_idxs.get(name[1..0].to_uppercase().as_str()) {
    //         arg_idx = idx;
    //     } else {
    //         return Err( Error::new(&format!("Statement does not define {} parameter placeholder", name)) );
    //     }
    //     catch!{self.err_ptr() =>
    //         OCIBindByName2(
    //             self.stmt_ptr(), self.args_binds[arg_idx].as_ptr(), self.err_ptr(),
    //             name.as_ptr(), name.len() as i32,
    //             data, buff_size as i64, sql_type,
    //             null_ind as *mut c_void,
    //             data_size,
    //             ptr::null_mut::<u16>(),
    //             0,
    //             ptr::null_mut::<u32>(),
    //             OCI_DEFAULT
    //         )
    //     }
    //     Ok(())
    // }

    /// Executes the prepared statement. Returns the OCI result code from OCIStmtExecute.
    fn exec(&self, stmt_type: u16, in_args: &[&dyn SqlInArg], out_args: &mut [&mut dyn SqlOutArg]) -> Result<i32>{
        let mut args_idxs : HashSet<_> = self.param_idxs.values().cloned().collect();

        let mut idx = 0;
        for arg in in_args {
            let (sql_type, data, size) = arg.as_to_sql().to_sql();
            if let Some( name ) = arg.name() {
                if let Some(&ix) = self.param_idxs.get(&name[1..]) {
                    idx = ix;
                } else if let Some(&ix) = self.param_idxs.get(name[1..].to_uppercase().as_str()) {
                    idx = ix;
                } else {
                    return Err( Error::new(&format!("Statement does not define {} parameter placeholder", name)) );
                }
            }
            self.bind_by_pos(idx, sql_type, data as *mut c_void, size, ptr::null_mut::<u32>(), ptr::null_mut::<i16>())?;
            args_idxs.remove(&idx);
            idx += 1;
        }

        let out_idxs = if out_args.is_empty() {
            None
        } else {
            let mut idxs = Vec::with_capacity(out_args.len());
            for arg in out_args {
                let mut out_idx = idx;
                if let Some( name ) = arg.name() {
                    if let Some( &param_idx ) = self.param_idxs.get(&name[1..]) {
                        out_idx = param_idx;
                    } else if let Some( &param_idx ) = self.param_idxs.get(name[1..].to_uppercase().as_str()) {
                        out_idx = param_idx;
                    } else {
                        return Err(Error::new(&format!("Statement does not define {} parameter placeholder", name)));
                    }
                } else {
                    idx += 1;
                }
                let (sql_type, data, data_buffer_size, in_size) = arg.as_to_sql_out().to_sql_output();
                if data_buffer_size == 0 {
                    let msg = if let Some( name ) = arg.name() {
                        format!("Storage capacity of output variable {} is 0", name)
                    } else {
                        format!("Storage capacity of output variable {} is 0", out_idx)
                    };
                    return Err(Error::new(&msg));
                }
                self.data_sizes[out_idx].set(in_size as u32);
                self.bind_by_pos(
                    out_idx, sql_type, data as *mut c_void, data_buffer_size,
                    self.data_sizes[out_idx].as_ptr(),
                    self.indicators[out_idx].as_ptr()
                )?;
                args_idxs.remove(&out_idx);
                idxs.push((arg, out_idx));
            }
            Some(idxs)
        };

        // Check whether all placeholders are bound for this execution.
        // While OCIStmtExecute would see missing binds on the first run, the subsequent
        // execution of the same prepared statement might try to reuse previously bound
        // values, and those might already be gone. Hense the explicit check here.
        if !args_idxs.is_empty() {
            return Err( Error::new("Not all parameters are bound") );
        }

        let iters: u32 = if stmt_type == OCI_STMT_SELECT { 0 } else { 1 };
        let res = unsafe {
            OCIStmtExecute(
                self.conn.svc_ptr(), self.stmt_ptr(), self.err_ptr(),
                iters, 0,
                ptr::null::<c_void>(), ptr::null_mut::<c_void>(),
                OCI_DEFAULT
            )
        };
        match res {
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                if let Some(idxs) = out_idxs {
                    for (arg,ix) in idxs {
                        arg.as_to_sql_out().set_len(self.data_sizes[ix].get() as usize);
                    }
                }
                Ok(res)
            },
            OCI_ERROR | OCI_INVALID_HANDLE => {
                Err( Error::oci(self.err_ptr(), res) )
            }
            _ => Ok(res)
        }
    }
}

impl<'a> Statement<'a> {
    pub(crate) fn new(sql: &str, conn: &'a Connection<'a>) -> Result<Self> {
        let mut stmt = ptr::null_mut::<OCIStmt>();
        catch!{conn.err_ptr() =>
            OCIStmtPrepare2(
                conn.svc_ptr(), &mut stmt, conn.err_ptr(),
                sql.as_ptr(), sql.len() as u32,
                ptr::null(), 0,
                OCI_NTV_SYNTAX, OCI_DEFAULT
            )
        }
        let (param_idxs, args_binds, indicators, data_sizes) = define_binds(stmt, conn.err_ptr())?;
        Ok(Self {
            conn, stmt, param_idxs, args_binds, indicators, data_sizes,
            cols: RefCell::new(Columns::new()),
        })
    }
}

impl<'a> Statement<'a> {
    /**
        Executes the prepared statement. Returns the number of rows affected.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            UPDATE hr.departments
               SET manager_id = :manager_id
             WHERE department_id = :department_id
        ")?;
        let num_updated_rows = stmt.execute(&[
            &( ":DEPARTMENT_ID", 120 ),
            &( ":MANAGER_ID",    101 ),
        ])?;
        assert_eq!(num_updated_rows, 1);
        # conn.rollback()?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn execute(&self, args: &[&dyn SqlInArg]) -> Result<usize> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type == OCI_STMT_SELECT {
            return Err( Error::new("Use `query` to execute SELECT") );
        }
        let is_returning: u8 = self.get_attr(OCI_ATTR_STMT_IS_RETURNING)?;
        if is_returning != 0 {
            return Err( Error::new("Use `execute_into` with output arguments to execute a RETURNING statement") );
        }
        self.exec(stmt_type, args, &mut [])?;
        self.get_row_count()
    }

    /**
        Executes a prepared RETURNING statement. Returns the number of rows affected.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            INSERT INTO hr.departments
                   ( department_id, department_name, manager_id, location_id )
            VALUES ( hr.departments_seq.nextval, :department_name, :manager_id, :location_id )
         RETURNING department_id
              INTO :department_id
        ")?;
        let mut department_id : usize = 0;
        // In this case (no duplicates in the statement parameters and the OUT parameter follows
        // the IN parameters) we could have used positional arguments. However, there are many
        // cases when positional is too difficult to use correcty with `execute_into`. For example,
        // OUT is used as an IN-OUT parameter, OUT precedes or in the middle of the IN parameter
        // list, parameter list is very long, etc. This example shows the call with the named
        // arguments as this might be a more typical use case for it.
        let num_rows = stmt.execute_into(&[
            &( ":DEPARTMENT_NAME", "Security" ),
            &( ":MANAGER_ID",      ""         ),
            &( ":LOCATION_ID",     1700       ),
        ], &mut [
            &mut ( ":DEPARTMENT_ID", &mut department_id )
        ])?;
        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":DEPARTMENT_ID")?);
        assert!(department_id > 0);
        # conn.rollback()?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn execute_into(&self, in_args: &[&dyn SqlInArg], out_args: &mut [&mut dyn SqlOutArg]) -> Result<usize> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type == OCI_STMT_SELECT {
            return Err( Error::new("Use `query` to execute SELECT") );
        }
        self.exec(stmt_type, in_args, out_args)?;
        self.get_row_count()
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
        let mut stmt = ptr::null_mut::<OCIStmt>();
        let mut stmt_type = std::mem::MaybeUninit::<u32>::uninit();
        let res = unsafe {
            OCIStmtGetNextResult(self.stmt_ptr(), self.err_ptr(), &mut stmt, stmt_type.as_mut_ptr(), OCI_DEFAULT)
        };
        match res {
            OCI_NO_DATA => Ok( None ),
            OCI_SUCCESS => Ok( Some ( Cursor::implicit(stmt, self) ) ),
            _ => Err( Error::oci(self.err_ptr(), res) )
        }
    }

    /**
        Sets the buffer size for fetching LONG and LONG RAW via the data interface.

        By default 32768 bytes are allocated for values from LONG and LONG RAW columns.
        If the actual value is expected to be larger than that, then the "column size"
        has to be changed before `query` is run.
    */
    #[deprecated="Use set_column_size"]
    pub fn set_max_column_fetch_size(&self, size: usize) {
        self.cols.borrow_mut().set_max_long_fetch_size(size as u32);
    }

    /**
        Sets the buffer size for fetching LONG and LONG RAW via the data interface.

        By default 32768 bytes are allocated for values from LONG and LONG RAW columns.
        If the actual value is expected to be larger than that, then the "column size"
        has to be changed before `query` is run.

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
        stmt.set_column_size(0, 100_000);
        let mut rows = stmt.query(&[ &id ])?;
        let row = rows.next()?.expect("first (and only) row");
        let txt : &str = row.get(0)?.expect("long text");
        # assert_eq!(txt, text);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_column_size(&self, pos: usize, size: usize) {
        self.cols.borrow_mut().set_long_column_size(pos, size as u32);
    }

    /**
        Executes the prepared statement. Returns "streaming iterator" over the returned rows.

        # Example
        ```
        # use std::collections::HashMap;
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id, last_name, first_name
              FROM hr.employees
             WHERE manager_id = :id
          ORDER BY employee_id
        ")?;
        stmt.set_prefetch_rows(5)?;
        let mut rows = stmt.query(&[ &103 ])?;
        let mut subs = HashMap::new();
        while let Some( row ) = rows.next()? {
            // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
            let id : u32 = row.get(0)?.unwrap();
            // Same for the LAST_NAME.
            // Note that `last_name` is retrieved as a slice. This is fast as it
            // borrows directly from the column buffer, but it can only live until
            // the end of the current scope, i.e. only during the lifetime of the
            // current row.
            let last_name : &str = row.get(1)?.unwrap();
            // FIRST_NAME is NULL-able...
            let first_name : Option<&str> = row.get(2)?;
            let name = first_name.map_or(last_name.to_string(),
                |first_name| format!("{}, {}", last_name, first_name)
            );
            subs.insert(id, name);
        }
        assert_eq!(stmt.get_row_count()?, 3);
        assert_eq!(subs.len(), 3);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn query(&'a self, args: &[&dyn SqlInArg]) -> Result<Rows> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Use `execute` or `execute_into` to execute statements other than SELECT") );
        }
        let res = self.exec(stmt_type, args, &mut [])?;
        self.cols.borrow_mut().setup(self)?;
        match res {
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                Ok( Rows::new(res, self) )
            }
            OCI_NO_DATA => {
                Ok( Rows::new(res, self) )
            }
            _ => Err( Error::oci(self.err_ptr(), res) )
        }
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
        self.cols.borrow().get_column_info(self, pos)
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
        assert_eq!(stmt.get_row_count()?, 3);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids.as_slice(), &[104 as u32, 105, 106]);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_row_count(&self) -> Result<usize> {
        let num_rows = self.get_attr::<u64>(OCI_ATTR_UB8_ROW_COUNT)? as usize;
        Ok( num_rows )
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

    // Indicates the number of rows that were successfully fetched into the user's buffers
    // in the last fetch or execute with nonzero iterations.
    // This is not very useful in this implementation as we set up buffers for 1 row only.
    // pub fn get_rows_fetched(&self) -> Result<usize> {
    //     let num_rows = self.get_attr::<u32>(OCI_ATTR_ROWS_FETCHED)? as usize;
    //     Ok( num_rows )
    // }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn stmt_args() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let oracle = env()?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            INSERT INTO hr.departments
                   ( department_id, department_name, manager_id, location_id )
            VALUES ( 9, :department_name, :manager_id, :location_id )
         RETURNING department_id
              INTO :department_id
        ")?;
        let mut department_id : i32 = 0;
        let num_rows = stmt.execute_into(&[
            &( ":department_name", "Security" ),
            &( ":manager_id",      ""         ),
            &( ":location_id",     1700       ),
        ], &mut [
            &mut ( ":department_id", &mut department_id )
        ])?;
        assert_eq!(num_rows, 1);
        assert!(!stmt.is_null(":department_id")?);
        assert_eq!(department_id, 9);
        conn.rollback()?;
        Ok(())
    }
}
