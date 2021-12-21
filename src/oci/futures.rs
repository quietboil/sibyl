//! Futures for OCI functions that might return `OCI_STILL_EXECUTING`

use crate::conn::SvcCtx;
use super::{*, ptr::Ptr};
use std::{future::Future, pin::Pin, task::{Context, Poll}, sync::Arc};


macro_rules! wait {
    (|$ctx:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }};
}

macro_rules! wait_result {
    (|$this:ident, $ctx:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci($this.err, res)))
        } else {
            Poll::Ready(Ok(()))
        }
    }};
}

macro_rules! wait_oci_result {
    (|$this:ident, $ctx:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci($this.err, res)))
        } else {
            Poll::Ready(Ok(res))
        }
    }};
}

macro_rules! wait_val {
    (|$this:ident, $ctx:ident, $field:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci($this.err, res)))
        } else {
            Poll::Ready(Ok($this.$field))
        }
    }};
}

pub(crate) struct SessionPoolDestroy {
    pool: Handle<OCISPool>,
    err:  Handle<OCIError>,
    env:  Arc<Handle<OCIEnv>>,
}

impl SessionPoolDestroy {
    pub(crate) fn new(pool: Handle<OCISPool>, err:  Handle<OCIError>, env: Arc<Handle<OCIEnv>>) -> Self {
        Self { pool, err, env }
    }
}

impl Future for SessionPoolDestroy {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let pool: &OCISPool = &self.pool;
        let err:  &OCIError = &self.err;
        wait!(|cx| OCISessionPoolDestroy(pool, err, OCI_SPD_FORCE))
    }
}


pub(crate) struct SessionRelease {
    env: Arc<Handle<OCIEnv>>,
    err: Handle<OCIError>,
    svc: Ptr<OCISvcCtx>,
}

impl SessionRelease {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Handle<OCIError>, env: Arc<Handle<OCIEnv>>) -> Self {
        Self { svc, err, env }
    }
}

impl Future for SessionRelease {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!(">> SessionRelease");
        let svc: &OCISvcCtx = &self.svc;
        let err: &OCIError  = &self.err;
        wait!(|cx| OCISessionRelease(svc, err, std::ptr::null(), 0, OCI_DEFAULT))
    }
}


pub(crate) struct StmtRelease {
    ctx:  Arc<SvcCtx>,
    stmt: Ptr<OCIStmt>,
    err:  Handle<OCIError>,
}

impl StmtRelease {
    pub(crate) fn new(stmt: Ptr<OCIStmt>, err: Handle<OCIError>, ctx: Arc<SvcCtx>) -> Self {
        Self { stmt, err, ctx }
    }
}

impl Future for StmtRelease {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!(">> StmtRelease");
        let stmt: &OCIStmt  = &self.stmt;
        let err:  &OCIError = &self.err;
        wait!(|cx| OCIStmtRelease(stmt, err, std::ptr::null(), 0, OCI_DEFAULT))
    }
}


pub(crate) struct Ping<'a> {
    svc: &'a OCISvcCtx,
    err: &'a OCIError,
}

impl<'a> Ping<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError) -> Self {
        Self { svc, err }
    }
}

impl<'a> Future for Ping<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCIPing(self.svc, self.err, OCI_DEFAULT))
    }
}


pub(crate) struct TransCommit<'a> {
    svc: &'a OCISvcCtx,
    err: &'a OCIError,
}

impl<'a> TransCommit<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError) -> Self {
        Self { svc, err }
    }
}

impl<'a> Future for TransCommit<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!(">> TransCommit");
        wait_result!(|self, cx| OCITransCommit(self.svc, self.err, OCI_DEFAULT))
    }
}


pub(crate) struct TransRollback<'a> {
    svc: &'a OCISvcCtx,
    err: &'a OCIError,
}

impl<'a> TransRollback<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError) -> Self {
        Self { svc, err }
    }
}

impl<'a> Future for TransRollback<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCITransRollback(self.svc, self.err, OCI_DEFAULT))
    }
}


pub(crate) struct StmtPrepare<'a> {
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    sql:  &'a str,
    stmt: Ptr<OCIStmt>,
}

impl<'a> StmtPrepare<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, sql: &'a str) -> Self {
        Self { svc, err, sql, stmt: Ptr::null() }
    }
}

impl<'a> Future for StmtPrepare<'a> {
    type Output = Result<Ptr<OCIStmt>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!(">> StmtPrepare");
        let this = self.get_mut();
        wait_val!(|this, cx, stmt|
            OCIStmtPrepare2(
                this.svc, this.stmt.as_mut_ptr(), this.err, 
                this.sql.as_ptr(), this.sql.len() as u32, 
                std::ptr::null(), 0, OCI_NTV_SYNTAX, OCI_DEFAULT
            ) 
        )
    }
}


pub(crate) struct StmtExecute<'a> {
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    stmt: &'a OCIStmt,
    iter: u32,
}

impl<'a> StmtExecute<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, stmt: &'a OCIStmt, typ: u16) -> Self {
        let iter: u32 = if typ == OCI_STMT_SELECT { 0 } else { 1 };
        Self { svc, err , stmt, iter}
    }
}

impl<'a> Future for StmtExecute<'a> {
    type Output = Result<i32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!(">> StmtExecute");
        wait_oci_result!( |self, cx| 
            OCIStmtExecute(self.svc, self.stmt, self.err, self.iter, 0, std::ptr::null(), std::ptr::null(), OCI_DEFAULT)
        )
    }
}


pub(crate) struct StmtFetch<'a> {
    stmt: &'a OCIStmt,
    err:  &'a OCIError,
}

impl<'a> StmtFetch<'a> {
    pub(crate) fn new(stmt: &'a OCIStmt, err: &'a OCIError) -> Self {
        Self { stmt, err }
    }
}

impl<'a> Future for StmtFetch<'a> {
    type Output = Result<i32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!(">> StmtFetch");
        wait_oci_result!( |self, cx|
            OCIStmtFetch2(self.stmt, self.err, 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
        )
    }
}


pub(crate) struct StmtGetNextResult<'a> {
    stmt:       &'a OCIStmt,
    err:        &'a OCIError,
    cursor:     Ptr<OCIStmt>,
    stmt_type:  u32,
}

impl<'a> StmtGetNextResult<'a> {
    pub(crate) fn new(stmt: &'a OCIStmt, err: &'a OCIError) -> Self {
        Self { stmt, err, cursor: Ptr::null(), stmt_type: 0 }
    }
}

impl<'a> Future for StmtGetNextResult<'a> {
    type Output = Result<Option<Ptr<OCIStmt>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let res = unsafe {
            OCIStmtGetNextResult(this.stmt, this.err, this.cursor.as_mut_ptr(), &mut this.stmt_type, OCI_DEFAULT)
        };
        match res {
            OCI_STILL_EXECUTING => {
                cx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_NO_DATA => {
                Poll::Ready(Ok(None))
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                Poll::Ready(Ok(Some(this.cursor)))
            }
            _ => {
                Poll::Ready(Err(Error::oci(this.err, res)))
            },
        }
    }
}
