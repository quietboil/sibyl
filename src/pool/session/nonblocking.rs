//! Session pool nonblocking mode implementation

use super::SessionPool;
use crate::{Connection, Result, oci::{self, *}, Environment, task};
use std::{ptr, slice, str, marker::PhantomData};

impl<'a> SessionPool<'a> {
    pub(crate) async fn new(env: &'a Environment, dblink: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<SessionPool<'a>> {
        let err = Handle::<OCIError>::new(&env)?;
        let pool = Handle::<OCISPool>::new(&env)?;
        let info = Handle::<OCIAuthInfo>::new(&env)?;
        info.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        pool.set_attr(OCI_ATTR_SPOOL_AUTH, info.get_ptr(), &err)?;

        let env_ptr = Ptr::<OCIEnv>::from(env.as_ref());
        let err_ptr = err.get_ptr();
        let pool_ptr = pool.get_ptr();
        let dblink_ptr = Ptr::new(dblink.as_ptr() as *mut u8);
        let dblink_len = dblink.len() as u32;
        let username_ptr = Ptr::new(username.as_ptr() as *mut u8);
        let username_len = username.len() as u32;
        let password_ptr = Ptr::new(password.as_ptr() as *mut u8);
        let password_len = password.len() as u32;

        let name = task::spawn_blocking(move || -> Result<&[u8]> {
            let mut pool_name_ptr = ptr::null::<u8>();
            let mut pool_name_len = 0u32;
            oci::session_pool_create(
                &env_ptr, &err_ptr, &pool_ptr,
                &mut pool_name_ptr, &mut pool_name_len,
                dblink_ptr.as_ref(), dblink_len,
                min as u32, max as u32, inc as u32,
                username_ptr.as_ref(), username_len,
                password_ptr.as_ref(), password_len,
                OCI_SPC_HOMOGENEOUS | OCI_SPC_STMTCACHE
            )?;
            let name = unsafe {
                slice::from_raw_parts(pool_name_ptr, pool_name_len as usize)
            };
            Ok(name)
        }).await??;
        Ok(Self {env: env.get_env(), err, pool, name, phantom_env: PhantomData})
    }

    pub(crate) async fn get_svc_ctx(&self) -> Result<Ptr<OCISvcCtx>> {
        let env_ptr = self.env.get_ptr();
        let err_ptr = self.err.get_ptr();

        let inf = Handle::<OCIAuthInfo>::new(&env_ptr)?;

        let pool_name_ptr = Ptr::new(self.name.as_ptr() as *mut u8);
        let pool_name_len = self.name.len() as u32;

        task::spawn_blocking(move || -> Result<Ptr<OCISvcCtx>> {
            let mut svc = Ptr::<OCISvcCtx>::null();
            let mut found = 0u8;
            oci::session_get(
                &env_ptr, &err_ptr, svc.as_mut_ptr(), &inf,
                pool_name_ptr.as_ref(), pool_name_len, &mut found,
                OCI_SESSGET_SPOOL | OCI_SESSGET_PURITY_SELF
            )?;
            Ok(svc)
        }).await?
    }

    /**
        Returns a new session with a new underlyng connection from this pool.

        # Example

        ```
        use sibyl::{Environment, Connection, Date, Result};
        use once_cell::sync::OnceCell;
        use std::{env, sync::Arc};

        fn main() -> Result<()> {
            sibyl::block_on(async {
                static ORACLE : OnceCell<Environment> = OnceCell::new();
                let oracle = ORACLE.get_or_try_init(|| {
                    Environment::new()
                })?;

                let dbname = env::var("DBNAME").expect("database name");
                let dbuser = env::var("DBUSER").expect("user name");
                let dbpass = env::var("DBPASS").expect("password");

                let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 4).await?;
                let pool = Arc::new(pool);

                let mut workers = Vec::with_capacity(10);
                for _i in 0..workers.capacity() {
                    let pool = pool.clone();
                    let handle = sibyl::spawn(async move {
                        let conn = pool.get_session().await.expect("database session");

                        select_latest_hire(&conn).await.expect("selected employee name")
                    });
                    workers.push(handle);
                }
                for handle in workers {
                    let name = handle.await.expect("select result");
                    assert_eq!(name, "Amit Banda was hired on April 21, 2008");
                }
                Ok(())
            })
        }
        # async fn select_latest_hire(conn: &Connection<'_>) -> Result<String> {
        #     let stmt = conn.prepare("
        #         SELECT first_name, last_name, hire_date
        #           FROM (
        #                 SELECT first_name, last_name, hire_date
        #                      , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
        #                   FROM hr.employees
        #                )
        #          WHERE hire_date_rank = 1
        #     ").await?;
        #     let rows = stmt.query(()).await?;
        #     if let Some( row ) = rows.next().await? {
        #         let first_name : Option<&str> = row.get(0)?;
        #         let last_name : &str = row.get(1)?.expect("last_name");
        #         let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
        #         let hire_date : Date = row.get(2)?.expect("hire_date");
        #         let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;
        #         Ok(format!("{} was hired on {}", name, hire_date))
        #     } else {
        #         Ok("Not found".to_string())
        #     }
        # }
        ```

    */
    pub async fn get_session(&self) -> Result<Connection<'_>> {
        Connection::from_session_pool(self).await
    }
}

#[cfg(test)]
mod tests {
    use crate::{Result, Error, Environment, spawn};

    #[test]
    fn async_session_pool() -> Result<()> {
        crate::block_on(async {
            use std::sync::Arc;
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                Environment::new()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
            let pool = Arc::new(pool);

            let mut workers = Vec::with_capacity(100);
            for _i in 0..workers.capacity() {
                let pool = pool.clone();
                let handle = spawn(async move {
                    let conn = pool.get_session().await.expect("database session");
                    conn.start_call_time_measurements()?;
                    conn.ping().await?;
                    let dt = conn.call_time()?;
                    conn.stop_call_time_measurements()?;
                    Ok::<_,Error>(dt)
                });
                workers.push(handle);
            }
            for handle in workers {
                let dt = handle.await??;
                assert!(dt > 0, "ping time");
            }

            Ok(())
        })
    }
}
