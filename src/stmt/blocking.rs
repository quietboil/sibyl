//! Blocking SQL statement methods

use super::{
    Statement, Stmt, Params, SqlInArg, SqlOutArg, Columns, Rows,
    cols::DEFAULT_LONG_BUFFER_SIZE,
};
use crate::{Connection, Error, Result, env::Env, oci::{self, *}};
use libc::c_void;
use parking_lot::RwLock;
use std::ptr;
use once_cell::sync::OnceCell;

impl<'a> Statement<'a> {
    /// Creates a new statement
    pub(crate) fn new(sql: &str, conn: &'a Connection<'a>) -> Result<Self> {
        let err = Handle::<OCIError>::new(conn.env_ptr())?;
        let mut stmt = Ptr::null();
        oci::stmt_prepare(
            conn.svc_ptr(), stmt.as_mut_ptr(), err.get(),
            sql.as_ptr(), sql.len() as u32,
            ptr::null(), 0,
            OCI_NTV_SYNTAX, OCI_DEFAULT
        )?;
        let params = Params::new(stmt.get(), err.get())?.map(|params| RwLock::new(params));
        Ok(Self {conn, stmt, params, cols: OnceCell::new(), err, max_long: DEFAULT_LONG_BUFFER_SIZE})
    }

    /// Binds provided arguments to SQL parameter placeholders. Returns indexes of parameter placeholders for the OUT args.
    fn bind_args(&self, in_args: &[&dyn SqlInArg], out_args: &mut [&mut dyn SqlOutArg]) -> Result<Option<Vec<usize>>> {
        self.params.as_ref()
            .map(|params| params.write().bind_args(self.stmt_ptr(), self.err_ptr(), in_args, out_args))
            .unwrap_or_else(|| 
                if in_args.len() == 0 && out_args.len() == 0 {
                    Ok(None)
                } else {
                    Err(Error::new("Statement has no parameters"))
                }
            )
    }

    /// Executes the prepared statement. Returns the OCI result code from OCIStmtExecute.
    fn exec(&self, stmt_type: u16, in_args: &[&dyn SqlInArg], out_args: &mut [&mut dyn SqlOutArg]) -> Result<i32>{
        let out_idxs = self.bind_args(in_args, out_args)?;

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
                    if let Some(params) = self.params.as_ref() {
                        let params = params.read();
                        for (out_arg_ix, out_param_ix) in idxs.into_iter().enumerate() {
                            out_args[out_arg_ix].as_to_sql_out().set_len(params.out_data_len(out_param_ix));
                        }
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
        let mut rows = stmt.query(&[ &103 ])?; // Alexander Hunold
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
        assert_eq!(stmt.get_row_count()?, 4);
        assert_eq!(subs.len(), 4);
        assert!(subs.contains_key(&104), "Bruce Ernst");
        assert!(subs.contains_key(&105), "David Austin");
        assert!(subs.contains_key(&106), "Valli Pataballa");
        assert!(subs.contains_key(&107), "Diana Lorentz");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn query(&'a self, args: &[&dyn SqlInArg]) -> Result<Rows> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Use `execute` or `execute_into` to execute statements other than SELECT") );
        }
        let res = self.exec(stmt_type, args, &mut [])?;

        if self.cols.get().is_none() {
            let cols = Columns::new(self, self.max_long)?;
            self.cols.get_or_init(|| RwLock::new(cols));
        };
        match res {
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO | OCI_NO_DATA => {
                Ok( Rows::from_query(res, self) )
            }
            _ => Err( Error::oci(self.err_ptr(), res) )
        }
    }
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
