//! Blocking mode OCI environment methods.

use super::Environment;
use crate::{Result, Connection};

impl Environment {
    /**
        Creates and begins a user session for a given server.
        # Example
        ```
        let oracle = sibyl::env()?;
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        assert!(!conn.is_async()?);
        assert!(conn.is_connected()?);
        assert!(conn.ping().is_ok());

        let stmt = conn.prepare("
            SELECT DISTINCT client_driver
              FROM v$session_connect_info
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let mut rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_driver : &str = row.get(0)?.unwrap();
        assert_eq!(client_driver, "sibyl");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn connect(&self, dbname: &str, username: &str, password: &str) -> Result<Connection> {
        Connection::new(self, dbname, username, password)
    }
}

#[cfg(test)]
mod tests {
    use crate::{env, Result};
    #[test]
    fn connect() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("schema name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        assert!(!conn.is_async()?);
        assert!(conn.is_connected()?);
        assert!(conn.ping().is_ok());

        let stmt = conn.prepare("
            SELECT DISTINCT client_driver
              FROM v$session_connect_info
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let mut rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_driver : &str = row.get(0)?.unwrap();
        assert_eq!(client_driver, "sibyl");

        Ok(())
    }
}