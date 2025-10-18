//! Futures for OCI functions that might return `OCI_STILL_EXECUTING`

use crate::session::SvcCtx;
use super::{*, ptr::Ptr};
use std::{future::Future, ops::Deref, pin::Pin, task::{Context, Poll}, sync::{Arc, atomic::{AtomicI32, Ordering}}};

macro_rules! wait {
    (|$self:ident, $ctx:ident| $oci_call:expr) => {{
        let id = std::ptr::from_ref($self.deref()) as usize;
        if !$self.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let res = unsafe { $oci_call };
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $self.ctx.unlock();
            Poll::Ready(())
        }
    }};
}

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

macro_rules! wait_bool_flag {
    (|$self:ident, $err:expr, $field:expr, $ctx:ident| $oci_call:expr) => {{
        let id = std::ptr::from_ref($self.deref()) as usize;
        if !$self.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let rc = unsafe { $oci_call };
        let res = check_invalid_handle!($err, rc);
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $self.ctx.unlock();
            if res == OCI_SUCCESS {
                Poll::Ready(Ok($field != 0))
            } else {
                Poll::Ready(Err(Error::oci($err, res)))
            }
        }
    }};
}

/// Counter that keeps the number of active async drops.
pub static NUM_ACTIVE_ASYNC_DROPS : AtomicI32 = AtomicI32::new(0);

pub(crate) struct StmtRelease {
    stmt: Ptr<OCIStmt>,
    err:  Handle<OCIError>,
    ctx:  Arc<SvcCtx>,
}

impl StmtRelease {
    pub(crate) fn new(stmt: Ptr<OCIStmt>, err: Handle<OCIError>, ctx: Arc<SvcCtx>) -> Self {
        NUM_ACTIVE_ASYNC_DROPS.fetch_add(1, Ordering::Relaxed);
        Self { stmt, err, ctx }
    }
}

impl Drop for StmtRelease {
    fn drop(&mut self) {
        NUM_ACTIVE_ASYNC_DROPS.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Future for StmtRelease {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait!(|self, cx| OCIStmtRelease(self.stmt.get(), self.err.as_ref(), std::ptr::null(), 0, OCI_DEFAULT))
    }
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

//--
pub(crate) struct LobIsOpen<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
    flag: u8,
}

impl<'a> LobIsOpen<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob, flag: 0 }
    }
}

impl<'a> Future for LobIsOpen<'a> {
    type Output = Result<bool>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_bool_flag!(|self, &err, self.flag, cx| OCILobIsOpen(svc.get(), err.get(), self.lob, &mut self.flag))
    }
}


pub(crate) struct LobIsTemporary<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
    flag: u8,
}

impl<'a> LobIsTemporary<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob, flag: 0 }
    }
}

impl<'a> Future for LobIsTemporary<'a> {
    type Output = Result<bool>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        
        let env: Ptr<OCIEnv> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_bool_flag!(|self, &err, self.flag, cx| OCILobIsTemporary(env.get(), err.get(), self.lob, &mut self.flag))
    }
}


pub(crate) struct LobClose<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
}

impl<'a> LobClose<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob }
    }
}

impl<'a> Future for LobClose<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCILobClose(svc.get(), err.get(), self.lob))
    }
}


pub(crate) struct LobFileClose<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
}

impl<'a> LobFileClose<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob }
    }
}

impl<'a> Future for LobFileClose<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCILobFileClose(svc.get(), err.get(), self.lob))
    }
}


pub(crate) struct LobFreeTemporary<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
}

impl<'a> LobFreeTemporary<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob }
    }
}

impl<'a> Future for LobFreeTemporary<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCILobFreeTemporary(svc.get(), err.get(), self.lob))
    }
}


pub(crate) struct LobLocatorAssign<'a> {
    ctx:  Arc<SvcCtx>,
    src:  &'a OCILobLocator,
    dst:  Ptr<OCILobLocator>,
}

impl<'a> LobLocatorAssign<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, src: &'a OCILobLocator, dst: &'a OCILobLocator) -> Self {
        Self { ctx, src, dst: Ptr::from(dst) }
    }
}

impl<'a> Future for LobLocatorAssign<'a> {
    type Output = Result<Ptr<OCILobLocator>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_val!(|self, &err, self.dst, cx| OCILobLocatorAssign(svc.get(), err.get(), self.src, self.dst.as_mut_ptr()))
    }
}


pub(crate) struct LobGetLength<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
    len:  u64,
}

impl<'a> LobGetLength<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob, len: 0 }
    }
}

impl<'a> Future for LobGetLength<'a> {
    type Output = Result<u64>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_val!(|self, &err, self.len, cx| OCILobGetLength2(svc.get(), err.get(), self.lob, &mut self.len))
    }
}


pub(crate) struct LobOpen<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
    mode: u8,
}

impl<'a> LobOpen<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator, mode: u8) -> Self {
        Self { ctx, lob, mode }
    }
}

impl<'a> Future for LobOpen<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCILobOpen(svc.get(), err.get(), self.lob, self.mode))
    }
}


pub(crate) struct LobLoadFromFile<'a> {
    ctx:  Arc<SvcCtx>,
    dst:  &'a OCILobLocator,
    src:  &'a OCILobLocator,
    dst_off: u64,
    src_off: u64,
    amount:  u64,
}

impl<'a> LobLoadFromFile<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, dst: &'a OCILobLocator, dst_off: usize, src: &'a OCILobLocator, src_off: usize, amount: usize) -> Self {
        Self { ctx, dst, dst_off: (dst_off + 1) as u64 , src, src_off: (src_off + 1) as u64, amount: amount as u64}
    }
}

impl<'a> Future for LobLoadFromFile<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCILobLoadFromFile2(svc.get(), err.get(), self.dst, self.src, self.amount, self.dst_off, self.src_off))
    }
}


pub(crate) struct LobErase<'a> {
    ctx:  Arc<SvcCtx>,
    lob: &'a OCILobLocator,
    off: u64,
    len: u64,
}

impl<'a> LobErase<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator, off: usize, len: usize) -> Self {
        Self { ctx, lob, off: (off + 1) as u64, len: len as u64 }
    }
}

impl<'a> Future for LobErase<'a> {
    type Output = Result<u64>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_val!(|self, &err, self.len, cx| OCILobErase2(svc.get(), err.get(), self.lob, &mut self.len, self.off))
    }
}


pub(crate) struct LobGetChunkSize<'a> {
    ctx: Arc<SvcCtx>,
    lob: &'a OCILobLocator,
    len: u32,
}

impl<'a> LobGetChunkSize<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob, len: 0 }
    }
}

impl<'a> Future for LobGetChunkSize<'a> {
    type Output = Result<u32>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_val!(|self, &err, self.len, cx| OCILobGetChunkSize(svc.get(), err.get(), self.lob, &mut self.len))
    }
}


pub(crate) struct LobGetContentType<'a> {
    ctx: Arc<SvcCtx>,
    lob: &'a OCILobLocator,
    buf: Vec<u8>,
    len: u32,
}

impl<'a> LobGetContentType<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        let buf = Vec::with_capacity(OCI_LOB_CONTENTTYPE_MAXSIZE);
        Self { ctx, lob, len: buf.capacity() as u32, buf }
    }
}

impl<'a> Future for LobGetContentType<'a> {
    type Output = Result<String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let id = std::ptr::from_ref(self.deref()) as usize;
        if !self.ctx.lock(id) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        let env: Ptr<OCIEnv>    = Ptr::from(self.ctx.as_ref().as_ref());
        let res = unsafe {
            OCILobGetContentType(env.get(), svc.get(), err.get(), self.lob, self.buf.as_mut_ptr(), &mut self.len, 0)
        };
        let res = check_invalid_handle!(err, res);
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            self.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci(&err, res)))
            } else {
                let new_len = self.len as usize;
                let txt = unsafe {
                    self.buf.set_len(new_len);
                    String::from_utf8_unchecked(self.buf.as_slice().to_vec())
                };
                Poll::Ready(Ok(txt))
            }
        }
    }
}


pub(crate) struct LobSetContentType<'a> {
    ctx: Arc<SvcCtx>,
    lob: &'a OCILobLocator,
    ctx_type: &'a str,
}

impl<'a> LobSetContentType<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator, ctx_type: &'a str) -> Self {
        Self { ctx, lob, ctx_type }
    }
}

impl<'a> Future for LobSetContentType<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        let env: Ptr<OCIEnv>    = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCILobSetContentType(env.get(), svc.get(), err.get(), self.lob, self.ctx_type.as_ptr(), self.ctx_type.len() as u32, 0))
    }
}


pub(crate) struct LobTrim<'a> {
    ctx: Arc<SvcCtx>,
    lob: &'a OCILobLocator,
    len: u64,
}

impl<'a> LobTrim<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator, len: usize) -> Self {
        Self { ctx, lob, len: len as u64 }
    }
}

impl<'a> Future for LobTrim<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCILobTrim2(svc.get(), err.get(), self.lob, self.len))
    }
}


pub(crate) struct LobCreateTemporary<'a> {
    ctx: Arc<SvcCtx>,
    lob: &'a OCILobLocator,
    lobtype: u8,
    csform:  u8,
    cache:   u8,
}

impl<'a> LobCreateTemporary<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator, lobtype: u8, csform: u8, cache: u8) -> Self {
        Self { ctx, lob, lobtype, csform, cache}
    }
}

impl<'a> Future for LobCreateTemporary<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx|
            OCILobCreateTemporary(svc.get(), err.get(), self.lob, OCI_DEFAULT as u16, self.csform, self.lobtype, self.cache, OCI_DURATION_SESSION)
        )
    }
}


pub(crate) struct LobFileExists<'a> {
    ctx: Arc<SvcCtx>,
    lob: &'a OCILobLocator,
    flag: u8,
}

impl<'a> LobFileExists<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob, flag: 0 }
    }
}

impl<'a> Future for LobFileExists<'a> {
    type Output = Result<bool>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_bool_flag!(|self, &err, self.flag, cx| OCILobFileExists(svc.get(), err.get(), self.lob, &mut self.flag))
    }
}


pub(crate) struct LobFileIsOpen<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
    flag: u8,
}

impl<'a> LobFileIsOpen<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob, flag: 0 }
    }
}

impl<'a> Future for LobFileIsOpen<'a> {
    type Output = Result<bool>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_bool_flag!(|self, &err, self.flag, cx| OCILobFileIsOpen(svc.get(), err.get(), self.lob, &mut self.flag))
    }
}


pub(crate) struct LobFileOpen<'a> {
    ctx: Arc<SvcCtx>,
    lob: &'a OCILobLocator,
}

impl<'a> LobFileOpen<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator) -> Self {
        Self { ctx, lob }
    }
}

impl<'a> Future for LobFileOpen<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let svc: Ptr<OCISvcCtx> = Ptr::from(self.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(self.ctx.as_ref().as_ref());
        wait_result!(|self, &err, cx| OCILobFileOpen(svc.get(), err.get(), self.lob, OCI_FILE_READONLY))
    }
}
