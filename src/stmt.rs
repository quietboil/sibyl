//! SQL or PL/SQL statement handling

use crate::*;
use crate::types::*;
use crate::env::Env;
use crate::conn::Conn;
use crate::desc::Descriptor;
use crate::column::Column;
use crate::rows::Rows;
use crate::cursor::Cursor;
use libc::c_void;
use std::{
    ptr,
    cell::{
        Cell,
        RefCell,
        Ref
    },
    collections::HashMap
};

// Parsing Syntax Types
const OCI_NTV_SYNTAX   : u32 = 1;

// Statement Types
// const OCI_STMT_UNKNOWN : u16 = 0;
const OCI_STMT_SELECT  : u16 = 1;
// const OCI_STMT_UPDATE  : u16 = 2;
// const OCI_STMT_DELETE  : u16 = 3;
// const OCI_STMT_INSERT  : u16 = 4;
// const OCI_STMT_CREATE  : u16 = 5;
// const OCI_STMT_DROP    : u16 = 6;
// const OCI_STMT_ALTER   : u16 = 7;
// const OCI_STMT_BEGIN   : u16 = 8;
// const OCI_STMT_DECLARE : u16 = 9;
// const OCI_STMT_CALL    : u16 = 10;
// const OCI_STMT_MERGE   : u16 = 16;

// Attributes common to Columns and Stored Procs
const OCI_ATTR_DATA_SIZE         : u32 =  1; // maximum size of the data
const OCI_ATTR_DATA_TYPE         : u32 =  2; // the SQL type of the column/argument
// const OCI_ATTR_DISP_SIZE         : u32 =  3; // the display size
const OCI_ATTR_NAME              : u32 =  4; // the name of the column/argument
const OCI_ATTR_PRECISION         : u32 =  5; // precision if number type
const OCI_ATTR_SCALE             : u32 =  6; // scale if number type
const OCI_ATTR_IS_NULL           : u32 =  7; // is it null ?
const OCI_ATTR_TYPE_NAME         : u32 =  8; // name of the named data type or a package name for package private types
const OCI_ATTR_SCHEMA_NAME       : u32 =  9; // the schema name
// const OCI_ATTR_SUB_NAME          : u32 = 10; // type name if package private type
// const OCI_ATTR_POSITION          : u32 = 11; // relative position of col/arg in the list of cols/args
// const OCI_ATTR_PACKAGE_NAME      : u32 = 12; // package name of package type
const OCI_ATTR_CHARSET_FORM      : u32 = 32;
const OCI_ATTR_COL_PROPERTIES    : u32 = 104;
const OCI_ATTR_CHAR_SIZE         : u32 = 286;

// Flags coresponding to the column properties
const OCI_ATTR_COL_PROPERTY_IS_IDENTITY             : u8 = 0x01;
const OCI_ATTR_COL_PROPERTY_IS_GEN_ALWAYS           : u8 = 0x02;
const OCI_ATTR_COL_PROPERTY_IS_GEN_BY_DEF_ON_NULL   : u8 = 0x04;

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
    fn OCIBindByName2(
        stmtp:      *mut OCIStmt,
        bindpp:     *mut *mut OCIBind,
        errhp:      *mut OCIError,
        namep:      *const u8,
        name_len:   i32,
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

/// A trait for types that can be used as SQL statement IN arguments
pub trait SqlInArg {
    fn name(&self) -> Option<&str>;
    fn as_to_sql(&self) -> &dyn ToSql;
}

impl<T: ToSql> SqlInArg for T {
    fn name(&self) -> Option<&str>      { None }
    fn as_to_sql(&self) -> &dyn ToSql   { self }
}

impl<T: ToSql> SqlInArg for (&str, T) {
    fn name(&self) -> Option<&str>      { Some( self.0 ) }
    fn as_to_sql(&self) -> &dyn ToSql   { &self.1        }
}

/// A trait for types that can be used as SQL statement OUT arguments
pub trait SqlOutArg {
    fn name(&self) -> Option<&str>;
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut;
}

impl<T: ToSqlOut> SqlOutArg for T {
    fn name(&self) -> Option<&str>                      { None }
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut    { self }
}

impl<T: ToSqlOut> SqlOutArg for (&str, T) {
    fn name(&self) -> Option<&str>                      { Some( self.0 ) }
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut    { &mut self.1    }
}

struct Binds {
    list: Vec<Cell<*mut OCIBind>>,
    idxs: HashMap<String,usize>
}

impl Binds {
    fn new() -> Self {
        Self {
            list: Vec::new(),
            idxs: HashMap::new()
        }
    }

    fn init(&mut self, stmt: *mut OCIStmt, err: *mut OCIError) -> Result<()> {
        let num_binds = attr::get::<u32>(OCI_ATTR_BIND_COUNT, OCI_HTYPE_STMT, stmt as *const c_void, err)? as usize;
        self.list.reserve(num_binds);
        self.idxs.reserve(num_binds);
        if num_binds > 0 {
            let bind_names          = vec![ptr::null_mut::<u8>(); num_binds];
            let mut bind_name_lens  = vec![0u8; num_binds];
            let ind_names           = vec![ptr::null_mut::<u8>(); num_binds];
            let mut ind_name_lens   = vec![0u8; num_binds];
            let mut dups            = vec![0u8; num_binds];
            let mut binds           = vec![ptr::null_mut::<OCIBind>(); num_binds];
            let mut found: i32      = 0;
            catch!{err =>
                OCIStmtGetBindInfo(
                    stmt, err,
                    num_binds as u32, 1, &mut found,
                    bind_names.as_ptr(), bind_name_lens.as_mut_ptr(),
                    ind_names.as_ptr(), ind_name_lens.as_mut_ptr(),
                    dups.as_mut_ptr(),
                    binds.as_mut_ptr()
                )
            }
            for i in 0..found as usize {
                let name = unsafe { std::slice::from_raw_parts(bind_names[i], bind_name_lens[i] as usize) };
                let name = String::from_utf8_lossy(name).to_string();
                if dups[i] == 0 {
                    self.idxs.insert(name, i);
                }
                self.list.push(Cell::new(binds[i]));
            }
        }
        Ok(())
    }

    fn bind_by_pos(&self, idx: usize, stmt: &Statement, sql_type: u16, data: *mut c_void, buff_size: usize, null_ind: *mut i16, data_size: *mut u32) -> Result<()> {
        catch!{stmt.err_ptr() =>
            OCIBindByPos2(
                stmt.stmt_ptr(), self.list[idx].as_ptr(), stmt.err_ptr(),
                (idx + 1) as u32,
                data, buff_size as i64, sql_type,
                null_ind as *mut c_void,    // Pointer to an indicator variable or array
                data_size,                  // Pointer to an array of actual lengths of array elements
                ptr::null_mut::<u16>(),     // Pointer to an array of column-level return codes
                0,                          // Maximum array length
                ptr::null_mut::<u32>(),     // Pointer to the actual number of elements in the array
                OCI_DEFAULT
            )
        }
        Ok(())
    }

    /// Binds the argument to a named placeholder in the SQL statement
    fn bind_by_name(&self, name: &str, stmt: &Statement, sql_type: u16, data: *mut c_void, buff_size: usize, null_ind: *mut i16, data_size: *mut u32) -> Result<()> {
        let name = name[1..].to_uppercase();
        if let Some( &idx ) = self.idxs.get(&name) {
            catch!{stmt.err_ptr() =>
                OCIBindByName2(
                    stmt.stmt_ptr(), self.list[idx].as_ptr(), stmt.err_ptr(),
                    name.as_ptr(), name.len() as i32,
                    data, buff_size as i64, sql_type,
                    null_ind as *mut c_void,    // Pointer to an indicator variable or array
                    data_size,                  // Pointer to an array of actual lengths of array elements
                    ptr::null_mut::<u16>(),     // Pointer to an array of column-level return codes
                    0,                          // Maximum array length
                    ptr::null_mut::<u32>(),     // Pointer to the actual number of elements in the array
                    OCI_DEFAULT
                )
            }
            Ok(())
        } else {
            Err( Error::new(&format!("Statement does not define {} placeholder", name)) )
        }
    }
}

/// Represents a prepared for execution SQL or PL/SQL statement
pub struct Statement<'a> {
    stmt: *mut OCIStmt,
    max_col_size: Cell<usize>,
    cols: RefCell<Vec<Column>>,
    binds: Binds,
    conn: &'a dyn Conn,
}

impl Env for Statement<'_> {
    fn env_ptr(&self) -> *mut OCIEnv      { self.conn.env_ptr() }
    fn err_ptr(&self) -> *mut OCIError    { self.conn.err_ptr() }
}

impl Conn for Statement<'_> {
    fn srv_ptr(&self) -> *mut OCIServer   { self.conn.srv_ptr() }
    fn svc_ptr(&self) -> *mut OCISvcCtx   { self.conn.svc_ptr() }
    fn usr_ptr(&self) -> *mut OCISession  { self.conn.usr_ptr() }
}

impl UsrEnv for Statement<'_> {
    fn as_ptr(&self) -> *mut c_void         { self.conn.usr_ptr() as *mut c_void }
    fn as_conn(&self) -> Option<&dyn Conn>  { Some( self.conn ) }
}

/// A trait for types that can provide access to the `OCIStmt` handle
pub trait Stmt : Conn {
    fn stmt_ptr(&self) -> *mut OCIStmt;
    fn conn(&self) -> &dyn Conn;
    fn get_max_col_size(&self) -> usize;
    fn usr_env(&self) -> &dyn UsrEnv;
}

impl Stmt for Statement<'_> {
    fn stmt_ptr(&self) -> *mut OCIStmt    { self.stmt }
    fn conn(&self) -> &dyn Conn           { self.conn }
    fn get_max_col_size(&self) -> usize   { self.max_col_size.get() }
    fn usr_env(&self) -> &dyn UsrEnv      { self }
}

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        let env = self.env_ptr();
        let err = self.err_ptr();
        let cols = self.cols.get_mut();
        while let Some( mut col ) = cols.pop() {
            col.drop_output(env, err);
        }
        if !self.stmt.is_null() {
            unsafe {
                OCIStmtRelease(self.stmt, self.err_ptr(), ptr::null(), 0, OCI_DEFAULT);
            }
        }
    }
}

impl<'a> Statement<'a> {
    pub(crate) fn new(sql: &str, conn: &'a dyn Conn) -> Result<Self> {
        let mut oci_stmt = ptr::null_mut::<OCIStmt>();
        catch!{conn.err_ptr() =>
            OCIStmtPrepare2(
                conn.svc_ptr(), &mut oci_stmt, conn.err_ptr(),
                sql.as_ptr(), sql.len() as u32,
                ptr::null(), 0,
                OCI_NTV_SYNTAX, OCI_DEFAULT
            )
        }
        let mut stmt = Self {
            conn,
            stmt: oci_stmt,
            binds: Binds::new(),
            max_col_size: Cell::new(32768),
            cols: RefCell::new(Vec::new())
        };
        stmt.binds.init(oci_stmt, conn.err_ptr())?;
        Ok( stmt )
    }

    pub(crate) fn borrow_columns(&self) -> Result<Ref<Vec<Column>>> {
        let borrow = self.cols.try_borrow();
        if borrow.is_err() {
            Err( Error::new("cannot borrow projection") )
        } else {
            Ok( borrow.unwrap() )
        }
    }

    fn get_attr<V: attr::AttrGet>(&self, attr_type: u32) -> Result<V> {
        attr::get::<V>(attr_type, OCI_HTYPE_STMT, self.stmt_ptr() as *const c_void, self.err_ptr())
    }

    fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V) -> Result<()> {
        attr::set::<V>(attr_type, attr_val, OCI_HTYPE_STMT, self.stmt_ptr() as *mut c_void, self.err_ptr())
    }

    fn get_param(&self, pos: usize) -> Result<Descriptor<OCIParam>> {
        param::get::<OCIParam>(pos as u32, OCI_HTYPE_STMT, self.stmt_ptr() as *const c_void, self.err_ptr())
    }

    /// Executes the prepared statement. Returns the statement execution result code.
    fn exec(&self, stmt_type: u16, in_args: &[&dyn SqlInArg], out_args: &mut [&mut dyn SqlOutArg], null_ind: &mut [i16]) -> Result<i32>{
        if out_args.len() != null_ind.len() {
            return Err( Error::new("sizes of out_args and null_ind must be the same") );
        }

        let mut arg_idx = 0;
        for arg in in_args {
            let (sql_type, data, size) = arg.as_to_sql().to_sql();
            if let Some( name ) = arg.name() {
                self.binds.bind_by_name(name, self, sql_type, data as *mut c_void, size, ptr::null_mut::<i16>(), ptr::null_mut::<u32>())?;
            } else {
                self.binds.bind_by_pos(arg_idx, self, sql_type, data as *mut c_void, size, ptr::null_mut::<i16>(), ptr::null_mut::<u32>())?;
            }
            arg_idx += 1;
        }

        let mut data_sizes = Vec::with_capacity(out_args.len());
        let mut out_idx = 0;
        for arg in out_args.iter_mut() {
            let (sql_type, data, size) = arg.as_to_sql_out().to_sql_output(0);
            data_sizes.push(size as u32);
            if let Some( name ) = arg.name() {
                self.binds.bind_by_name(name, self, sql_type, data as *mut c_void, size, &mut null_ind[out_idx], &mut data_sizes[out_idx])?;
            } else {
                self.binds.bind_by_pos(arg_idx, self, sql_type, data as *mut c_void, size, &mut null_ind[out_idx], &mut data_sizes[out_idx])?;
            }
            out_idx += 1;
            arg_idx += 1;
        }

        let iters: u32 = if stmt_type == OCI_STMT_SELECT { 0 } else { 1 };
        let res = unsafe {
            OCIStmtExecute(
                self.svc_ptr(), self.stmt_ptr(), self.err_ptr(),
                iters, 0,
                ptr::null::<c_void>(), ptr::null_mut::<c_void>(),
                OCI_DEFAULT
            )
        };
        match res {
            OCI_ERROR | OCI_INVALID_HANDLE => {
                Err( Error::oci(self.err_ptr(), res) )
            }
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                out_idx = 0;
                for arg in out_args {
                    let out = arg.as_to_sql_out();
                    out.set_len(data_sizes[out_idx] as usize);
                    out_idx += 1;
                }
                Ok( res )
            }
            _ => Ok( res )
        }
    }

    /// Executes the prepared statement. Returns the number of rows affected.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     UPDATE hr.departments
    ///        SET manager_id = :manager_id
    ///      WHERE department_id = :department_id
    /// ")?;
    /// let num_rows = stmt.execute(&[
    ///     &( ":department_id", 120 ),
    ///     &( ":manager_id",    101 ),
    /// ])?;
    ///
    /// assert_eq!(1, num_rows);
    /// # conn.rollback()?;
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn execute(&self, args: &[&dyn SqlInArg]) -> Result<usize> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type == OCI_STMT_SELECT {
            return Err( Error::new("Use `query` to execute SELECT") );
        }
        let is_returning: u8 = self.get_attr(OCI_ATTR_STMT_IS_RETURNING)?;
        if is_returning != 0 {
            return Err( Error::new("Use `execute_into` with output arguments to execute a RETURNING statement") );
        }
        self.exec(stmt_type, args, &mut [], &mut [])?;
        self.get_row_count()
    }

    /// Executes a prepared RETURNING statement. Returns the optional vector of booleans where each
    /// element corresponds to the provided OUT argument and indicates whether the value returned into
    /// the OUT variable was NULL. Returns `None` when the statement has not created/updated any rows.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     INSERT INTO hr.departments
    ///            ( department_id, department_name, manager_id, location_id )
    ///     VALUES ( hr.departments_seq.nextval, :department_name, :manager_id, :location_id )
    ///  RETURNING department_id
    ///       INTO :department_id
    /// ")?;
    /// let mut department_id : usize = 0;
    /// // In this case (no duplicates in the statement parameters and the OUT parameter follows
    /// // the IN parameters) we could have used positional arguments. However, there are many
    /// // cases when positional is too difficult to use correcty with `execute_into`. For example,
    /// // OUT is used as an IN-OUT parameter, OUT precedes or in the middle of the IN parameter
    /// // list, parameter list is very long, etc. This example shows the call with the named
    /// // arguments as this might be a more typical use case for it.
    /// let res = stmt.execute_into(&[
    ///     &( ":department_name", "Security" ),
    ///     &( ":manager_id",      ""         ),
    ///     &( ":location_id",     1700       ),
    /// ], &mut [
    ///     &mut ( ":department_id", &mut department_id )
    /// ])?;
    ///
    /// let is_null = res.expect("optional vector of 'is null?' flags");
    /// assert_eq!(1, is_null.len());
    /// assert!(!is_null[0]);
    /// assert!(department_id > 0);
    /// # conn.rollback()?;
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn execute_into(&self, in_args: &[&dyn SqlInArg], out_args: &mut [&mut dyn SqlOutArg]) -> Result<Option<Vec<bool>>> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type == OCI_STMT_SELECT {
            return Err( Error::new("Use `query` to execute SELECT") );
        }
        let mut null_ind = vec![OCI_IND_NOTNULL; out_args.len()];
        let null_ind = null_ind.as_mut_slice();
        self.exec(stmt_type, in_args, out_args, null_ind)?;
        let row_count = self.get_row_count()?;
        if row_count == 0 {
            Ok( None )
        } else {
            let nulls: Vec<_> = null_ind.iter().map(|&ind| ind == OCI_IND_NULL).collect();
            Ok( Some(nulls) )
        }
    }

    /// Retrieves a single implicit result (cursor) in the order in which they were returned
    /// from the PL/SQL procedure or block. If no more results are available, then `None` is
    /// returned.
    ///
    /// PL/SQL provides a subprogram RETURN_RESULT in the DBMS_SQL package to return the result
    /// of an executed statement. Only SELECT query result-sets can be implicitly returned by a
    /// PL/SQL procedure or block.
    ///
    /// `next_result` can be called iteratively by the application to retrieve each implicit
    /// result from an executed PL/SQL statement. Applications retrieve each result-set sequentially
    /// but can fetch rows from any result-set independently.
    ///
    /// ## Example
    /// ```
    /// use sibyl::Number;
    /// use std::cmp::Ordering::Equal;
    ///
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     DECLARE
    ///         c1 SYS_REFCURSOR;
    ///         c2 SYS_REFCURSOR;
    ///     BEGIN
    ///         OPEN c1 FOR
    ///             SELECT department_name, first_name, last_name, salary
    ///              FROM (
    ///                    SELECT first_name, last_name, salary, department_id
    ///                         , ROW_NUMBER() OVER (ORDER BY salary) ord
    ///                      FROM hr.employees
    ///                   ) e
    ///              JOIN hr.departments d
    ///                ON d.department_id = e.department_id
    ///             WHERE ord = 1
    ///         ;
    ///         DBMS_SQL.RETURN_RESULT (c1);
    ///
    ///         OPEN c2 FOR
    ///             SELECT department_name, first_name, last_name, salary
    ///               FROM (
    ///                     SELECT first_name, last_name, salary, department_id
    ///                          , MEDIAN(salary) OVER () median_salary
    ///                       FROM hr.employees
    ///                    ) e
    ///               JOIN hr.departments d
    ///                 ON d.department_id = e.department_id
    ///              WHERE salary = median_salary
    ///           ORDER BY department_name, last_name, first_name
    ///         ;
    ///         DBMS_SQL.RETURN_RESULT (c2);
    ///     END;
    /// ")?;
    /// stmt.execute(&[])?;
    ///
    /// // <<< c1 >>>
    /// let res = stmt.next_result()?;
    /// assert!(res.is_some());
    ///
    /// let cursor = res.unwrap();
    /// let rows = cursor.rows()?;
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    ///
    /// let department_name = row.get::<&str>(0)?.unwrap();
    /// assert_eq!(department_name, "Shipping");
    ///
    /// let first_name = row.get::<&str>(1)?;
    /// assert!(first_name.is_some());
    /// let first_name = first_name.unwrap();
    /// assert_eq!(first_name, "TJ");
    ///
    /// let last_name = row.get::<&str>(2)?.unwrap();
    /// assert_eq!(last_name, "Olson");
    ///
    /// let salary = row.get::<Number>(3)?.unwrap();
    /// let expected = Number::from_int(2100, &oracle);
    /// assert!(salary.cmp(&expected)? == Equal);
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_none());
    ///
    /// // <<< c2 >>>
    /// let res = stmt.next_result()?;
    /// assert!(res.is_some());
    ///
    /// let cursor = res.unwrap();
    /// let rows = cursor.rows()?;
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    ///
    /// let department_name = row.get::<&str>(0)?.unwrap();
    /// assert_eq!(department_name, "Sales");
    ///
    /// let first_name = row.get::<&str>(1)?;
    /// assert!(first_name.is_some());
    /// let first_name = first_name.unwrap();
    /// assert_eq!(first_name, "Amit");
    ///
    /// let last_name = row.get::<&str>(2)?.unwrap();
    /// assert_eq!(last_name, "Banda");
    ///
    /// let expected = Number::from_int(6200, &oracle);
    ///
    /// let salary = row.get::<Number>(3)?.unwrap();
    /// assert!(salary.cmp(&expected)? == Equal);
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    ///
    /// let department_name = row.get::<&str>(0)?.unwrap();
    /// assert_eq!(department_name, "Sales");
    ///
    /// let first_name = row.get::<&str>(1)?;
    /// assert!(first_name.is_some());
    /// let first_name = first_name.unwrap();
    /// assert_eq!(first_name, "Charles");
    ///
    /// let last_name = row.get::<&str>(2)?.unwrap();
    /// assert_eq!(last_name, "Johnson");
    ///
    /// let salary = row.get::<Number>(3)?.unwrap();
    /// assert!(salary.cmp(&expected)? == Equal);
    ///
    /// let row = rows.next()?;
    /// assert!(row.is_none());
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn next_result(&self) -> Result<Option<Cursor>> {
        let mut stmt = ptr::null_mut::<OCIStmt>();
        let mut stmt_type: u32;
        let res = unsafe {
            stmt_type = std::mem::uninitialized();
            OCIStmtGetNextResult(self.stmt_ptr(), self.err_ptr(), &mut stmt, &mut stmt_type, OCI_DEFAULT)
        };
        match res {
            OCI_NO_DATA => Ok( None ),
            OCI_SUCCESS => Ok( Some ( Cursor::implicit(stmt, self) ) ),
            _ => Err( Error::oci(self.err_ptr(), res) )
        }
    }

    /// Sets the buffer size for fetching LONG and LONG RAW via the data interface.
    ///
    /// By default 32768 bytes are allocated for values from LONG and LONG RAW columns.
    /// If the actual value is expected to be larger than that, then the "max column
    /// fetch size" has to be changed before `query` is run.
    ///
    /// ## Example
    /// ```rust,ignore
    /// let stmt = conn.prepare("
    ///     SELECT id, long_text
    ///       FROM all_texts
    ///      WHERE id = :id
    /// ")?;
    /// stmt.set_max_column_fetch_size(250_000);
    /// let rset = stmt.query(&[ &42 ])?;
    /// ```
    pub fn set_max_column_fetch_size(&self, size: usize) {
        if size > 128 {
            // 128 is an arbitrary limit, actually it can be anything > 0 to ensure there is a buffer
            self.max_col_size.replace(size);
        }
    }

    /// Executes the prepared statement. Returns "streaming iterator" over the returned rows.
    /// ## Example
    /// ```
    /// # use std::collections::HashMap;
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     SELECT employee_id, last_name, first_name
    ///       FROM hr.employees
    ///      WHERE manager_id = :id
    ///   ORDER BY employee_id
    /// ")?;
    /// stmt.set_prefetch_rows(5)?;
    /// let rows = stmt.query(&[ &103 ])?;
    /// let mut subs = HashMap::new();
    /// while let Some( row ) = rows.next()? {
    ///     // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
    ///     let id = row.get::<usize>(0)?.unwrap();
    ///     // Same for the LAST_NAME.
    ///     // Note that `last_name` is retrieved a slice. This is fast as it
    ///     // borrows directly from the column buffer, but it can only live until
    ///     // the end of the current scope, i.e. only during the lifetime of the
    ///     // current row.
    ///     let last_name = row.get::<&str>(1)?.unwrap();
    ///     let name =
    ///         // FIRST_NAME is NULL-able...
    ///         if let Some( first_name ) = row.get::<&str>(2)? {
    ///             format!("{}, {}", last_name, first_name)
    ///         } else {
    ///             last_name.to_string()
    ///         }
    ///     ;
    ///     subs.insert(id, name);
    /// }
    /// assert_eq!(4, stmt.get_row_count()?);
    /// assert_eq!(4, subs.len());
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn query(&self, args: &[&dyn SqlInArg]) -> Result<Rows> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Use `execute` or `execute_into` to execute statements other than SELECT") );
        }
        let res = self.exec(stmt_type, args, &mut [], &mut [])?;
        self.define_columns()?;
        match res {
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                self.define_column_buffers()?;
                let cols = self.borrow_columns()?;
                Ok( Rows::new(res, cols, self) )
            }
            OCI_NO_DATA => {
                let cols = self.borrow_columns()?;
                Ok( Rows::new(res, cols, self) )
            }
            _ => Err( Error::oci(self.err_ptr(), res) )
        }
    }

    /// Initializes, if necessary, the internal vector of columns
    fn define_columns(&self) -> Result<()> {
        let mut cols = self.cols.borrow_mut();
        if cols.is_empty() {
            let num_columns = self.get_column_count()?;
            cols.reserve_exact(num_columns);
            for pos in 1..=num_columns {
                let col = self.get_param(pos)?;
                let col = Column::new(pos, col)?;
                cols.push(col)
            }
        }
        Ok(())
    }

    /// Ensures each column has an internal value buffer that matches
    /// its type and column output is redirected into that buffer
    fn define_column_buffers(&self) -> Result<()> {
        let mut cols = self.cols.borrow_mut();
        for col in cols.iter_mut() {
            col.define_output_buffer(self)?;
        }
        Ok(())
    }

    /// Returns he number of columns in the select-list of this statement.
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     SELECT employee_id, last_name, first_name
    ///       FROM hr.employees
    ///      WHERE manager_id = :id
    /// ")?;
    /// let _rows = stmt.query(&[ &103 ])?;
    /// let num_cols = stmt.get_column_count()?;
    ///
    /// assert_eq!(3, num_cols);
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_column_count(&self) -> Result<usize> {
        let num_columns = self.get_attr::<u32>(OCI_ATTR_PARAM_COUNT)? as usize;
        Ok( num_columns )
    }

    /// Returns `pos` column meta data handler. `pos` is 0 based. Returns None if
    /// `pos` is greater than the number of columns in the query.
    /// ## Example
    /// ```
    /// # use sibyl::ColumnType;
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     SELECT employee_id, last_name, first_name
    ///       FROM hr.employees
    ///      WHERE manager_id = :id
    /// ")?;
    /// let _rows = stmt.query(&[ &103 ])?;
    /// let col = stmt.get_column(0)?;
    /// assert!(col.is_some());
    ///
    /// let col = col.unwrap();
    /// assert_eq!("EMPLOYEE_ID", col.name()?);
    /// assert_eq!(ColumnType::Number, col.data_type()?);
    /// assert_eq!(6, col.precision()?);
    /// assert_eq!(0, col.scale()?);
    /// assert!(!col.is_null()?);
    /// assert!(col.is_visible()?);
    /// let props = col.generated()?;
    /// assert!(!props.is_identity());
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_column(&self, pos: usize) -> Result<Option<ColumnInfo>> {
        let stmt_type: u16 = self.get_attr(OCI_ATTR_STMT_TYPE)?;
        if stmt_type != OCI_STMT_SELECT {
            return Err( Error::new("Columns are only available in SELECT statements") );
        }
        let opt_col = if let Some( col ) = self.cols.borrow().get(pos) {
            Some(
                ColumnInfo::new(self as &dyn Stmt, col)
            )
        } else {
            None
        };
        Ok( opt_col )
    }

    /// Returns the number of rows processed/seen so far in SELECT statements.
    ///
    /// For INSERT, UPDATE, and DELETE statements, it is the number of rows processed
    /// by the most recent statement.
    ///
    /// For nonscrollable cursors, it is the total number of rows fetched into user buffers
    /// since this statement handle was executed. Because they are forward sequential only,
    /// this also represents the highest row number seen by the application.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     SELECT employee_id, first_name, last_name
    ///       FROM hr.employees
    ///      WHERE manager_id = :id
    ///   ORDER BY employee_id
    /// ")?;
    /// stmt.set_prefetch_rows(5)?;
    /// let rows = stmt.query(&[ &103 ])?;
    /// let mut ids = Vec::new();
    /// while let Some( row ) = rows.next()? {
    ///     // EMPLOYEE_ID is NOT NULL, so we can safely unwrap it
    ///     let id = row.get::<usize>(0)?.unwrap();
    ///     ids.push(id);
    /// }
    ///
    /// assert_eq!(4, stmt.get_row_count()?);
    /// assert_eq!(4, ids.len());
    /// assert_eq!(&[104 as usize, 105, 106, 107], ids.as_slice());
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_row_count(&self) -> Result<usize> {
        let num_rows = self.get_attr::<u64>(OCI_ATTR_UB8_ROW_COUNT)? as usize;
        Ok( num_rows )
    }

    /// Sets the number of top-level rows to be prefetched. The default value is 1 row.
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     SELECT employee_id, first_name, last_name
    ///       FROM hr.employees
    ///      WHERE manager_id = :id
    /// ")?;
    /// stmt.set_prefetch_rows(10)?;
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
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

/// Describes an identity clause against a table column defined using a numeric type.
pub struct Identity(u8);

impl Identity {
    /// Returns `true` if column is an identity column.
    pub fn is_identity(&self) -> bool {
        self.0 & OCI_ATTR_COL_PROPERTY_IS_IDENTITY != 0
    }

    /// Returns `true` if column value is GENERATED ALWAYS.
    /// `false` means that the value is GENERATED BY DEFAULT.
    pub fn is_generated_always(&self) -> bool {
        self.0 & OCI_ATTR_COL_PROPERTY_IS_GEN_ALWAYS != 0
    }

    /// Returns true if column was declared as GENERATED BY DEFAULT ON NULL.
    pub fn is_generated_on_null(&self) -> bool {
        self.0 & OCI_ATTR_COL_PROPERTY_IS_GEN_BY_DEF_ON_NULL != 0
    }
}

/// Column data type.
#[derive(Debug,PartialEq)]
pub enum ColumnType {
    /// Less common type for which data type decoder has not been implemented (yet).
    Unknown,
    Char,
    NChar,
    Varchar,
    NVarchar,
    Clob,
    NClob,
    Long,
    Raw,
    LongRaw,
    Blob,
    Number,
    BinaryFloat,
    BinaryDouble,
    Date,
    Timestamp,
    TimestampWithTimeZone,
    TimestampWithLocalTimeZone,
    IntervalYearToMonth,
    IntervalDayToSecond,
    RowID,
    Cursor
}

impl std::fmt::Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ColumnType::Unknown                     => write!(f, "UNKNOWN"),
            ColumnType::Char                        => write!(f, "CHAR"),
            ColumnType::NChar                       => write!(f, "NCHAR"),
            ColumnType::Varchar                     => write!(f, "VARCHAR2"),
            ColumnType::NVarchar                    => write!(f, "NVARCHAR2"),
            ColumnType::Clob                        => write!(f, "CLOB"),
            ColumnType::NClob                       => write!(f, "NCLOB"),
            ColumnType::Long                        => write!(f, "LONG"),
            ColumnType::Raw                         => write!(f, "RAW"),
            ColumnType::LongRaw                     => write!(f, "LONG RAW"),
            ColumnType::Blob                        => write!(f, "BLOB"),
            ColumnType::Number                      => write!(f, "NUMBER"),
            ColumnType::BinaryFloat                 => write!(f, "BINARY_FLOAT"),
            ColumnType::BinaryDouble                => write!(f, "BINARY_DOUBLE"),
            ColumnType::Date                        => write!(f, "DATE"),
            ColumnType::Timestamp                   => write!(f, "TIMESTAMP"),
            ColumnType::TimestampWithTimeZone       => write!(f, "TIMESTAMP WITH TIME ZONE"),
            ColumnType::TimestampWithLocalTimeZone  => write!(f, "TIMESTAMP WITH LOCAL TIME ZONE"),
            ColumnType::IntervalYearToMonth         => write!(f, "INTERVAL YEAR TO MONTH"),
            ColumnType::IntervalDayToSecond         => write!(f, "INTERVAL DAY TO SECOND"),
            ColumnType::RowID                       => write!(f, "ROWID"),
            ColumnType::Cursor                      => write!(f, "CURSOR"),
        }
    }
}

/// Provides access to the projection column metadata.
pub struct ColumnInfo<'s> {
    stmt: &'s dyn Stmt,
    desc: *mut OCIParam,
}

impl<'s> ColumnInfo<'s> {
    fn new(stmt: &'s dyn Stmt, col: &Column) -> Self {
        Self { stmt, desc: col.as_ptr() }
    }

    fn get_attr<T: attr::AttrGet>(&self, attr: u32) -> Result<T> {
        attr::get(attr, OCI_DTYPE_PARAM, self.desc as *const c_void, self.stmt.err_ptr())
    }

    /// Returns `true` if a column is visible
    pub fn is_visible(&self) -> Result<bool> {
        let invisible: u8 = self.get_attr(OCI_ATTR_INVISIBLE_COL)?;
        Ok( invisible == 0 )
    }

    /// Returns `true` if NULLs are permitted in the column.
    ///
    /// Does not return a correct value for a CUBE or ROLLUP operation.
    pub fn is_null(&self) -> Result<bool> {
        let is_null: u8 = self.get_attr(OCI_ATTR_IS_NULL)?;
        Ok( is_null != 0 )
    }

    /// Returns identity column properties.
    pub fn generated(&self) -> Result<Identity> {
        let col_props: u8 = self.get_attr(OCI_ATTR_COL_PROPERTIES)?;
        Ok( Identity(col_props) )
    }

    /// Returns the column name
    pub fn name(&self) -> Result<&str> {
        self.get_attr::<&str>(OCI_ATTR_NAME)
    }

    /// Returns the maximum size of the column in bytes.
    /// For example, it returns 22 for NUMBERs.
    pub fn size(&self) -> Result<usize> {
        let size = self.get_attr::<u16>(OCI_ATTR_DATA_SIZE)? as usize;
        Ok( size )
    }

    /// Returns the column character length that is the number of characters allowed in the column.
    ///
    /// It is the counterpart of `size`, which gets the byte length.
    pub fn char_size(&self) -> Result<usize> {
        let size = self.get_attr::<u16>(OCI_ATTR_CHAR_SIZE)? as usize;
        Ok( size )
    }

    /// The precision of numeric columns.
    ///
    /// If the precision is nonzero and scale is -127, then it is a FLOAT; otherwise, it is a NUMBER(precision, scale).
    /// When precision is 0, NUMBER(precision, scale) can be represented simply as NUMBER.
    pub fn precision(&self) -> Result<i16> {
        self.get_attr::<i16>(OCI_ATTR_PRECISION)
    }

    /// The scale of numeric columns.
    ///
    /// If the precision is nonzero and scale is -127, then it is a FLOAT; otherwise, it is a NUMBER(precision, scale).
    /// When precision is 0, NUMBER(precision, scale) can be represented simply as NUMBER.
    pub fn scale(&self) -> Result<i8> {
        self.get_attr::<i8>(OCI_ATTR_SCALE)
    }

    /// Returns column data type.
    pub fn data_type(&self) -> Result<ColumnType> {
        let col_type = match self.get_attr::<u16>(OCI_ATTR_DATA_TYPE)? {
            SQLT_RDD => ColumnType::RowID,
            SQLT_CHR => {
                match self.get_attr::<u8>(OCI_ATTR_CHARSET_FORM)? {
                    SQLCS_NCHAR => ColumnType::NVarchar,
                    _ => ColumnType::Varchar,
                }
            }
            SQLT_AFC => {
                match self.get_attr::<u8>(OCI_ATTR_CHARSET_FORM)? {
                    SQLCS_NCHAR => ColumnType::NChar,
                    _ => ColumnType::Char,
                }
            }
            SQLT_CLOB => {
                match self.get_attr::<u8>(OCI_ATTR_CHARSET_FORM)? {
                    SQLCS_NCHAR => ColumnType::NClob,
                    _ => ColumnType::Clob,
                }
            }
            SQLT_LNG  => ColumnType::Long,
            SQLT_BIN  => ColumnType::Raw,
            SQLT_LBI  => ColumnType::LongRaw,
            SQLT_BLOB => ColumnType::Blob,
            SQLT_NUM  => ColumnType::Number,
            SQLT_DAT  => ColumnType::Date,
            SQLT_TIMESTAMP     => ColumnType::Timestamp,
            SQLT_TIMESTAMP_TZ  => ColumnType::TimestampWithTimeZone,
            SQLT_TIMESTAMP_LTZ => ColumnType::TimestampWithLocalTimeZone,
            SQLT_INTERVAL_YM   => ColumnType::IntervalYearToMonth,
            SQLT_INTERVAL_DS   => ColumnType::IntervalDayToSecond,
            SQLT_IBFLOAT  => ColumnType::BinaryFloat,
            SQLT_IBDOUBLE => ColumnType::BinaryDouble,
            SQLT_RSET => ColumnType::Cursor,
            _ => ColumnType::Unknown
        };
        Ok( col_type )
    }

    /// Returns the column type name:
    /// - If the data type is SQLT_NTY, the name of the named data type's type is returned.
    /// - If the data type is SQLT_REF, the type name of the named data type pointed to by the REF is returned.
    /// - If the data type is anything other than SQLT_NTY or SQLT_REF, an empty string is returned.
    pub fn type_name(&self) -> Result<&str> {
        self.get_attr::<&str>(OCI_ATTR_TYPE_NAME)
    }

    /// Returns the schema name under which the type has been created.
    pub fn schema_name(&self) -> Result<&str> {
        self.get_attr::<&str>(OCI_ATTR_SCHEMA_NAME)
    }
}

