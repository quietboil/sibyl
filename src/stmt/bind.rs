//! Binding of parameter placeholders

use super::Position;
use crate::{Result, Error, oci::{self, *}, ToSql};
use std::{ptr, collections::HashMap};
use libc::c_void;

/// Represents statement parameters (a.k.a. parameter placeholders)
pub struct Params {
    /// Parameter placeholder (name) indexes
    idxs: HashMap<&'static str,usize>,
    /// Parameter names
    names: HashMap<usize,&'static str>,
    /// OCI bind handles
    binds: Vec<Ptr<OCIBind>>,
    /// NULL indicators
    nulls: Vec<i16>,
    /// Sizes of returned data
    out_data_lens: Vec<u32>,
    /// Map of arguments indexes (positions) to parameter placeholder indexes
    bind_order: Vec<usize>,
}

impl Params {
    pub(super) fn new(stmt: &OCIStmt, err: &OCIError) -> Result<Option<Self>> {
        let num_binds : u32 = attr::get(OCI_ATTR_BIND_COUNT, OCI_HTYPE_STMT, stmt, err)?;
        if num_binds == 0 {
            Ok(None)
        } else {
            let num_binds = num_binds as usize;
            let mut idxs  = HashMap::with_capacity(num_binds);
            let mut names = HashMap::with_capacity(num_binds);
            let mut binds = Vec::with_capacity(num_binds);

            let mut bind_names      = vec![     ptr::null_mut::<u8>(); num_binds];
            let mut bind_name_lens  = vec![                       0u8; num_binds];
            let mut ind_names       = vec![     ptr::null_mut::<u8>(); num_binds];
            let mut ind_name_lens   = vec![                       0u8; num_binds];
            let mut dups            = vec![                       0u8; num_binds];
            let mut oci_binds       = vec![ptr::null_mut::<OCIBind>(); num_binds];
            let mut found: i32      = 0;

            oci::stmt_get_bind_info(
                stmt, err,
                num_binds as u32, 1, &mut found,
                bind_names.as_mut_ptr(), bind_name_lens.as_mut_ptr(),
                ind_names.as_mut_ptr(),  ind_name_lens.as_mut_ptr(),
                dups.as_mut_ptr(),       oci_binds.as_mut_ptr()
            )?;

            for i in 0..found as usize {
                if dups[i] == 0 {
                    let name = unsafe { std::slice::from_raw_parts(bind_names[i], bind_name_lens[i] as usize) };
                    let name = unsafe { std::str::from_utf8_unchecked(name) };
                    // The `idxs` and `names` hash maps won't outlive `Params` and the latter won't outlive `Statement`.
                    // While `str` for names that we created above will only live as long as the containing `Statement`,
                    // within `Params` they can be seen as static as they will be alive longer.
                    idxs.insert(name, i);
                    names.insert(i, name);
                }
                binds.push(Ptr::new(oci_binds[i]));
            }

            Ok(Some(Self{
                idxs, names, binds,
                nulls: Vec::with_capacity(num_binds),
                out_data_lens: Vec::with_capacity(num_binds),
                bind_order: Vec::with_capacity(num_binds),
            }))
        }
    }

    /// Returns the bind name without an optional leading colon
    fn strip_colon(name: &str) -> &str {
        if name.starts_with(':') {
            &name[1..]
        } else {
            name
        }
    }

    /// Returns index of the parameter placeholder.
    pub(crate) fn index_of(&self, name: &str) -> Result<usize> {
        // Assume `name` is already uppercase and use it as-is first.
        // Explicitly convert to uppercase only if as-is search fails.
        let name = Self::strip_colon(name);
        if let Some(&ix) = self.idxs.get(name) {
            Ok(ix)
        } else if let Some(&ix) = self.idxs.get(name.to_uppercase().as_str()) {
            Ok(ix)
        } else {
            Err(Error::msg(format!("Statement does not define parameter placeholder {}", name)))
        }
    }

    /// Binds an IN argument to a parameter placeholder at the specified position in the SQL statement.
    pub(crate) fn bind(&mut self, idx: usize, sql_type: u16, data_ptr: *mut c_void, data_len: usize, stmt: &OCIStmt, err: &OCIError) -> Result<()> {
        self.bind_order.push(idx);
        oci::bind_by_pos(
            stmt, self.binds[idx].as_mut_ptr(), err,
            (idx + 1) as u32, data_ptr, data_len as i64, sql_type,
            ptr::null_mut(), ptr::null_mut(),
            OCI_DEFAULT
        )
    }

    /// Binds an OUT argument to a parameter placeholder at the specified position in the SQL statement.
    pub(crate) fn bind_out(&mut self, idx: usize, sql_type: u16, data_ptr: *mut c_void, data_len: usize, buff_size: usize, stmt: &OCIStmt, err: &OCIError) -> Result<()> {
        if buff_size == 0 {
            let msg = if let Some(name) = self.names.get(&idx) {
                format!("Storage capacity of output variable {} is 0", name)
            } else {
                format!("Storage capacity of output variable {} is 0", idx)
            };
            return Err(Error::msg(msg));
        }
        self.bind_order.push(idx);
        if data_len != 0 {
            self.nulls[idx] = OCI_IND_NOTNULL;
        }
        self.out_data_lens[idx] = data_len as _;
        oci::bind_by_pos(
            stmt, self.binds[idx].as_mut_ptr(), err,
            (idx + 1) as u32, data_ptr, buff_size as i64, sql_type,
            &mut self.nulls[idx],           // Pointer to an indicator variable or array
            &mut self.out_data_lens[idx],   // Pointer to an array of actual lengths of array elements
            OCI_DEFAULT
        )
    }

    /// Checks whether previously bound placeholders are rebound.
    /// Returns `true` if they are.
    fn prior_binds_are_rebound(&self, mut prior_binds: Vec<usize>) -> bool {
        prior_binds.retain(|ix| !self.bind_order.contains(ix));
        prior_binds.len() == 0
    }

    /// Binds provided arguments to SQL parameter placeholders.
    pub(crate) fn bind_args(&mut self, stmt: &OCIStmt, err: &OCIError, args: &mut impl ToSql) -> Result<()> {
        let prior_binds = self.bind_order.clone();
        self.bind_order.clear();

        self.nulls.clear();
        self.nulls.resize(self.nulls.capacity(), OCI_IND_NULL);
        self.out_data_lens.clear();
        self.out_data_lens.resize(self.out_data_lens.capacity(), 0);

        args.bind_to(0, self, stmt, err)?;

        if prior_binds.len() > 0 && !self.prior_binds_are_rebound(prior_binds) {
            Err(Error::new("not all existing binds have been updated"))
        } else {
            Ok(())
        }
    }

    pub(crate) fn set_out_to_null(&mut self) {
        self.nulls.fill(OCI_IND_NULL);
        self.out_data_lens.fill(0);
    }

    pub(crate) fn set_out_data_len(&self, args: &mut impl ToSql) {
        args.set_len_from_bind(0, self);
    }

    /// Checks whether the value returned for the output parameter is NULL.
    pub(super) fn is_null(&self, pos: impl Position) -> Result<bool> {        
        pos.name()
            .and_then(|name| {
                let name = Self::strip_colon(name);
                self.idxs
                    .get(name)
                    .or(self.idxs.get(name.to_uppercase().as_str()))
            })
            .map(|ix| *ix)
            .or(pos.index())
            .map(|ix|
                self.nulls.get(ix)
                    .map(|&ind| ind == OCI_IND_NULL)
                    .unwrap_or(true)
            )
            .ok_or_else(|| Error::new("Parameter not found."))
    }

    /// Returns the size of the returned data for the OUT parameter at the specified argument position
    pub(super) fn out_data_len(&self, pos: usize) -> usize {
        self.bind_order
            .get(pos)
            .map(|&ix| self.out_data_lens[ix] as _)
            .unwrap_or_default()
    }
}

#[cfg(all(test, feature="blocking"))]
mod tests {
    use crate::{Result, Environment};

    #[test]
    fn dup_args() -> Result<()> {
        let oracle = Environment::new()?;
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("
            INSERT INTO hr.locations (location_id, state_province, city, postal_code, street_address)
            VALUES (:id, :na, :na, :code, :na)
        ")?;
        assert!(stmt.params.is_some());
        let stmt_params = stmt.params.as_ref().unwrap();
        let params = stmt_params.read();
        assert_eq!(params.binds.len(), 5);
        assert_eq!(params.index_of(":ID")?, 0);
        assert_eq!(params.index_of(":NA")?, 1);
        assert_eq!(params.index_of(":CODE")?, 3);

        let stmt = session.prepare("
          BEGIN
            INSERT INTO hr.locations (location_id, state_province, city, postal_code, street_address)
            VALUES (:id, :na, :na, :code, :na);
          END;
        ")?;
        assert!(stmt.params.is_some());
        let stmt_params = stmt.params.as_ref().unwrap();
        let params = stmt_params.read();
        assert_eq!(params.binds.len(), 3);
        assert_eq!(params.index_of(":ID")?, 0);
        assert_eq!(params.index_of(":NA")?, 1);
        assert_eq!(params.index_of(":CODE")?, 2);

        Ok(())
    }

    #[test]
    fn no_colon_arg_names() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let oracle = Environment::new()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("
            UPDATE hr.employees
               SET salary = Round(salary * :rate, -2)
             WHERE employee_id = :id
            RETURN salary INTO :new_salary
        ")?;
        let mut new_salary = 0u16;
        let num_updated = stmt.execute((
            ("ID",         107             ),
            ("RATE",       1.07            ),
            ("NEW_SALARY", &mut new_salary ),
        ))?;

        assert_eq!(num_updated, 1);
        assert!(!stmt.is_null("NEW_SALARY")?);
        assert_eq!(new_salary, 4500);

        let num_updated = stmt.execute((
            ("ID",         99              ),
            ("RATE",       1.03            ),
            ("NEW_SALARY", &mut new_salary ),
        ))?;

        assert_eq!(num_updated, 0);
        assert!(stmt.is_null("NEW_SALARY")?);

        session.rollback()?;
        Ok(())
    }
}