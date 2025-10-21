//! A "nullable value" type that wraps a non-null value. `Nvl` is bound to a SQL parameer
//! placeholder as null.
//! 
//! `Nvl` exists because several Oracle data types do not have an internal representatation
//! that can be treated as a null value. `Nvl` provides an explicit mechanism to bind them
//! as null and expecting either null or non-null value as the output.
//! 
//! # Examples
//! 
//! ```
//! # use sibyl::*;
//! # #[cfg(feature="blocking")]
//! # fn main() -> sibyl::Result<()> {
//! # let session = sibyl::test_env::get_session()?;
//! let stmt = session.prepare("
//! BEGIN
//!   IF :VAL IS NULL THEN
//!     :VAL := To_Timestamp('1969-07-24 16:50:35','YYYY-MM-DD HH24:MI:SS') 
//!           - To_Timestamp('1969-07-16 13:32:00','YYYY-MM-DD HH24:MI:SS');
//!   ELSE
//!     :VAL := NULL;
//!   END IF;
//! END;
//! ")?;
//! let mut interval = Nvl::new(IntervalDS::new(&session)?);
//! stmt.execute(&mut interval)?;
//! 
//! assert!(interval.as_ref().is_some());
//! let expected_interval = IntervalDS::from_string("+8 03:18:35.00", &session)?;
//! assert_eq!(interval.as_ref().unwrap().compare(&expected_interval)?, std::cmp::Ordering::Equal);
//! # Ok(())
//! # }
//! # #[cfg(feature="nonblocking")]
//! # fn main() {}
//! ``` 

use crate::{Result, ToSql};
use crate::oci::{SqlType, OCIStmt, OCIError};
use crate::stmt::Params;

/// A Nullable Value.
///
/// This type represents a value that is bound to an SQL parameter placeholder as NULL.
pub struct Nvl<T>(Option<T>) where T: ToSql + SqlType;

impl<T> Nvl<T> where T: ToSql + SqlType {
    /// Creates a new `Nvl`
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use sibyl::Nvl;
    ///
    /// let mut arg = Nvl::new(42);
    /// ```
    pub const fn new(value: T) -> Self {
        Self(Some(value))
    }

    /// Returns the contained value as an `Option<&T>`.
    ///
    /// A newly created `Nvl` always returns `Some` even though an SQL statement will bind it
    /// as NULL. After an SQL statement execution, where it was used as an INOUT parameter,
    /// it could be either `Some` if the statement has set to a non-null value, or `None` if
    /// the output valus was null.
    ///
    /// # Examples
    ///
    /// ```
    /// use sibyl::Nvl;
    ///
    /// let mut arg = Nvl::new(42);
    /// let val = arg.as_ref();
    ///
    /// assert_eq!(val, Some(&42));
    /// ```
    pub const fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }

    /// Returns the contained value as an `Option<&mut T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sibyl::Nvl;
    ///
    /// let mut arg = Nvl::new(42);
    /// let mut val = arg.as_mut();
    ///
    /// assert_eq!(val, Some(&mut 42));
    /// ```
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.0.as_mut()
    }

    /// Inserts a new value, then returns a mutable reference to it.
    ///
    /// If `Nvl` already contains a value, the old value is dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use sibyl::Nvl;
    ///
    /// let mut arg = Nvl::new(0);
    /// let val = arg.insert(42);
    ///
    /// assert_eq!(*val, 42);
    /// ```
    pub fn insert(&mut self, value: T) -> &mut T {
        self.0.insert(value)
    }

    /// Replaces the current value the value given in parameter, returning the old value if present,
    /// without deinitializing either one.
    ///
    /// # Examples
    ///
    /// ```
    /// use sibyl::Nvl;
    ///
    /// let mut arg = Nvl::new(0);
    /// let val = arg.replace(42);
    ///
    /// assert_eq!(val, Some(0));
    /// ```
    pub fn replace(&mut self, value: T) -> Option<T> {
        self.0.replace(value)
    }
}

impl<T> ToSql for Nvl<T> where T: ToSql + SqlType {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_null(pos, T::sql_null_type(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl<T> ToSql for &Nvl<T> where T: ToSql + SqlType {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_null(pos, T::sql_null_type(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl<T> ToSql for &mut Nvl<T> where T: ToSql + SqlType {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self.as_mut() {
            let next_pos = val.bind_to(pos, params, stmt, err)?;
            params.mark_as_null(pos);
            Ok(next_pos)
        } else {
            params.bind_null(pos, T::sql_null_type(), stmt, err)?;
            Ok(pos + 1)
        }
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) -> Result<usize> {
        if params.is_null(pos)? {
            self.0.take();
            Ok(pos + 1)
        } else if let Some(val) = self.as_mut() {
            val.update_from_bind(pos, params)
        } else {
            Ok(pos + 1)
        }
    }
}
