//! Futures for OCI functions that might return `OCI_STILL_EXECUTING`

use crate::session::SvcCtx;
use super::{*, ptr::Ptr};
use std::{future::Future, ops::Deref, pin::Pin, task::{Context, Poll}, sync::Arc};

/// Some OCI calls "hide" OCI_STILL_EXECUTING behind OCI_INVALID_HANDLE
macro_rules! check_invalid_handle {
    ($err:expr, $res:ident) => {
        if $res == OCI_INVALID_HANDLE {
            let mut errcode = $res;
            let err: &OCIError = &$err;
            unsafe {
                OCIErrorGet(err as *const OCIError as _, 1, std::ptr::null(), &mut errcode, std::ptr::null_mut(), 0, OCI_HTYPE_ERROR);
            }
            -errcode
        } else {
            $res
        }
    };
}

macro_rules! wait_result {
    (|$self:ident, $err:expr, $ctx:ident| $oci_call:expr) => {{
        let id = std::ptr::from_ref($self.deref()) as usize;
        if !$self.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let res = unsafe { $oci_call };
        let res = check_invalid_handle!($err, res);
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $self.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci($err, res)))
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }};
}

macro_rules! wait_oci_result {
    (|$self:ident, $err:expr, $ctx:ident| $oci_call:expr) => {{
        let id = std::ptr::from_ref($self.deref()) as usize;
        if !$self.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let res = unsafe { $oci_call };
        let res = check_invalid_handle!($err, res);
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $self.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci($err, res)))
            } else {
                Poll::Ready(Ok(res))
            }
        }
    }};
}

macro_rules! wait_val {
    (|$self:ident, $err:expr, $field:expr, $ctx:ident| $oci_call:expr) => {{
        let id = std::ptr::from_ref($self.deref()) as usize;
        if !$self.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let res = unsafe { $oci_call };
        let res = check_invalid_handle!($err, res);
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $self.ctx.unlock();
            if res == OCI_SUCCESS {
                $self.ctx.unlock();
                Poll::Ready(Ok($field))
            } else {
                Poll::Ready(Err(Error::oci($err, res)))
            }
        }
    }};
}

pub(crate) struct Ping {
    ctx: Arc<SvcCtx>,
}

impl Ping {
    pub(crate) fn new(ctx: Arc<SvcCtx>) -> Self {
        Self { ctx }
    }
}

impl Future for Ping {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCIPing(svc.get(), err.get(), OCI_DEFAULT))
    }
}


pub(crate) struct TransCommit {
    ctx: Arc<SvcCtx>,
}

impl TransCommit {
    pub(crate) fn new(ctx: Arc<SvcCtx>) -> Self {
        Self { ctx }
    }
}

impl Future for TransCommit {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCITransCommit(svc.get(), err.get(), OCI_DEFAULT))
    }
}


pub(crate) struct TransRollback {
    ctx: Arc<SvcCtx>,
}

impl TransRollback {
    pub(crate) fn new(ctx: Arc<SvcCtx>) -> Self {
        Self { ctx }
    }
}

impl Future for TransRollback {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCITransRollback(svc.get(), err.get(), OCI_DEFAULT))
    }
}


pub(crate) struct StmtPrepare<'a> {
    ctx: Arc<SvcCtx>,
    err:  &'a OCIError,
    sql:  &'a str,
    stmt: Ptr<OCIStmt>,
}

impl<'a> StmtPrepare<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, err: &'a OCIError, sql: &'a str) -> Self {
        Self { ctx, err, sql, stmt: Ptr::null() }
    }
}

impl<'a> Future for StmtPrepare<'a> {
    type Output = Result<Ptr<OCIStmt>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        wait_val!(|self, self.err, self.stmt, cx|
            OCIStmtPrepare2(
                svc.get(), self.stmt.as_mut_ptr(), self.err,
                self.sql.as_ptr(), self.sql.len() as u32,
                std::ptr::null(), 0, OCI_NTV_SYNTAX, OCI_DEFAULT
            )
        )
    }
}


pub(crate) struct StmtExecute<'a> {
    ctx: Arc<SvcCtx>,
    err:  &'a OCIError,
    stmt: &'a OCIStmt,
    iter: u32,
}

impl<'a> StmtExecute<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, err: &'a OCIError, stmt: &'a OCIStmt, typ: u16) -> Self {
        let iter: u32 = if typ == OCI_STMT_SELECT { 0 } else { 1 };
        Self { ctx, err, stmt, iter}
    }
}

impl<'a> Future for StmtExecute<'a> {
    type Output = Result<i32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        wait_oci_result!(|self, self.err, cx|
            OCIStmtExecute(svc.get(), self.stmt, self.err, self.iter, 0, std::ptr::null(), std::ptr::null(), OCI_DEFAULT)
        )
    }
}


pub(crate) struct StmtFetch<'a> {
    ctx:  Arc<SvcCtx>,
    stmt: &'a OCIStmt,
    err:  &'a OCIError,
}

impl<'a> StmtFetch<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, stmt: &'a OCIStmt, err: &'a OCIError) -> Self {
        Self { ctx, stmt, err }
    }
}

impl<'a> Future for StmtFetch<'a> {
    type Output = Result<i32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_oci_result!(|self, self.err, cx|
            OCIStmtFetch2(self.stmt, self.err, 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
        )
    }
}


pub(crate) struct StmtGetNextResult<'a> {
    ctx:    Arc<SvcCtx>,
    stmt:   &'a OCIStmt,
    err:    &'a OCIError,
    cursor: Ptr<OCIStmt>,
    stmt_type:  u32,
}

impl<'a> StmtGetNextResult<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, stmt: &'a OCIStmt, err: &'a OCIError) -> Self {
        Self { ctx, stmt, err, cursor: Ptr::null(), stmt_type: 0 }
    }
}

impl<'a> Future for StmtGetNextResult<'a> {
    type Output = Result<Option<Ptr<OCIStmt>>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let id = std::ptr::from_ref(self.deref()) as usize;
        if !self.ctx.lock(id) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let res = unsafe {
            OCIStmtGetNextResult(self.stmt, self.err, self.cursor.as_mut_ptr(), &mut self.stmt_type, OCI_DEFAULT)
        };
        let res = check_invalid_handle!(self.err, res);
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            self.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci(self.err, res)))
            } else if res == OCI_NO_DATA {
                Poll::Ready(Ok(None))
            } else {
                Poll::Ready(Ok(Some(self.cursor)))
            }
        }
    }
}
