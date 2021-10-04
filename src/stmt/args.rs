/// Statement arguments

use crate::*;

/// A trait for types that can be used as SQL statement IN arguments
pub trait SqlInArg {
    /// Returns the parameter name or None for positional arguments.
    fn name(&self) -> Option<&str>;
    /// Returns `ToSql` trait implementation for this argument.
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
    /// Returns the parameter name or None for positional arguments.
    fn name(&self) -> Option<&str>;
    /// Returns `ToSqlOut` trait implementation for this argument.
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut;
}

impl<T: ToSqlOut> SqlOutArg for T {
    fn name(&self) -> Option<&str>                      { None }
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut    { self }
}

impl<T: ToSqlOut> SqlOutArg for (&str, &mut T) {
    fn name(&self) -> Option<&str>                      { Some( self.0 ) }
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut    { self.1 }
}

