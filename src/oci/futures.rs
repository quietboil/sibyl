//! Futures for OCI functions that might return `OCI_STILL_EXECUTING`

use crate::{session::SvcCtx, lob::{LOB_IS_OPEN, LOB_FILE_IS_OPEN, LOB_IS_TEMP}, pool::session::SPool};
use super::{*, ptr::Ptr};
use std::{future::Future, pin::Pin, task::{Context, Poll}, sync::{Arc, atomic::{AtomicI32, Ordering}}};

macro_rules! wait {
    (|$this:ident, $ctx:ident| $oci_call:expr) => {{
        let id = $this as *mut Self as usize;
        if !$this.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let res = unsafe { $oci_call };
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $this.ctx.unlock();
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
    (|$this:ident, $err:expr, $ctx:ident| $oci_call:expr) => {{
        let id = $this as *mut Self as usize;
        if !$this.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let res = unsafe { $oci_call };
        let res = check_invalid_handle!($err, res);
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $this.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci($err, res)))
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }};
}

macro_rules! wait_oci_result {
    (|$this:ident, $err:expr, $ctx:ident| $oci_call:expr) => {{
        let id = $this as *mut Self as usize;
        if !$this.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let res = unsafe { $oci_call };
        let res = check_invalid_handle!($err, res);
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $this.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci($err, res)))
            } else {
                Poll::Ready(Ok(res))
            }
        }
    }};
}

macro_rules! wait_val {
    (|$this:ident, $err:expr, $field:expr, $ctx:ident| $oci_call:expr) => {{
        let id = $this as *mut Self as usize;
        if !$this.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let res = unsafe { $oci_call };
        let res = check_invalid_handle!($err, res);
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $this.ctx.unlock();
            if res == OCI_SUCCESS {
                $this.ctx.unlock();
                Poll::Ready(Ok($field))
            } else {
                Poll::Ready(Err(Error::oci($err, res)))
            }
        }
    }};
}

macro_rules! wait_bool_flag {
    (|$this:ident, $err:expr, $field:expr, $ctx:ident| $oci_call:expr) => {{
        let id = $this as *mut Self as usize;
        if !$this.ctx.lock(id) {
            $ctx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let rc = unsafe { $oci_call };
        let res = check_invalid_handle!($err, rc);
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            $this.ctx.unlock();
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

enum SessionReleaseSteps {
    TransRollback,
    SessionRelease,
}

pub(crate) struct SessionRelease {
    svc: Ptr<OCISvcCtx>,
    err: Handle<OCIError>,
    env: Arc<Handle<OCIEnv>>,
    spool: Option<Arc<SPool>>,
    step: SessionReleaseSteps,
}

impl SessionRelease {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Handle<OCIError>, env: Arc<Handle<OCIEnv>>, spool: Option<Arc<SPool>>) -> Self {
        NUM_ACTIVE_ASYNC_DROPS.fetch_add(1, Ordering::Relaxed);
        Self { svc, err, env, spool, step: SessionReleaseSteps::TransRollback }
    }
}

impl Drop for SessionRelease {
    fn drop(&mut self) {
        NUM_ACTIVE_ASYNC_DROPS.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Future for SessionRelease {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: &OCISvcCtx = &this.svc;
        let err: &OCIError  = &this.err;
        let res = match this.step {
            SessionReleaseSteps::TransRollback  => unsafe { OCITransRollback(svc, err, OCI_DEFAULT) },
            SessionReleaseSteps::SessionRelease => unsafe { OCISessionRelease(svc, err, std::ptr::null(), 0, OCI_DEFAULT) },
        };
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        match this.step {
            SessionReleaseSteps::TransRollback => {
                this.step = SessionReleaseSteps::SessionRelease;
                cx.waker().wake_by_ref();
                Poll::Pending
            },
            _ => Poll::Ready(())
        }
    }
}


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
        let this = self.get_mut();
        wait!(|this, cx| OCIStmtRelease(this.stmt.get(), this.err.as_ref(), std::ptr::null(), 0, OCI_DEFAULT))
    }
}


pub(crate) struct LobDrop<T> where T: DescriptorType<OCIType=OCILobLocator> {
    loc: Descriptor<T>,
    ctx: Arc<SvcCtx>,
    flags: u32,
}

impl<T> LobDrop<T> where T: DescriptorType<OCIType=OCILobLocator> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, loc: Descriptor<T>, flags: u32) -> Self {
        NUM_ACTIVE_ASYNC_DROPS.fetch_add(1, Ordering::Relaxed);
        Self { ctx, loc, flags }
    }
}

impl<T> Drop for LobDrop<T> where T: DescriptorType<OCIType=OCILobLocator> {
    fn drop(&mut self) {
        NUM_ACTIVE_ASYNC_DROPS.fetch_sub(1, Ordering::Relaxed);
    }
}

macro_rules! lob_drop_step {
    ($this:ident, $flag:ident => $op:ident) => {{
        let svc: Ptr<OCISvcCtx> = Ptr::from($this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from($this.ctx.as_ref().as_ref());
        let loc: &OCILobLocator = $this.loc.as_ref();
        if $this.flags & $flag != 0 {
            let res = unsafe { $op(svc.get(), err.get(), loc) };
            let res = check_invalid_handle!(err, res);
            if res != OCI_STILL_EXECUTING {
                $this.flags &= !$flag;
                if $this.flags == 0 {
                    $this.ctx.unlock();
                    return Poll::Ready(());
                }
            }
        }
    }};
}

impl<T> Future for LobDrop<T> where T: DescriptorType<OCIType=OCILobLocator> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if this.flags == 0 {
            return Poll::Ready(());
        }

        let id = this as *mut Self as usize;
        if !this.ctx.lock(id) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        lob_drop_step!(this, LOB_FILE_IS_OPEN => OCILobFileClose);
        lob_drop_step!(this, LOB_IS_OPEN => OCILobClose);
        lob_drop_step!(this, LOB_IS_TEMP => OCILobFreeTemporary);

        cx.waker().wake_by_ref();
        Poll::Pending
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCIPing(svc.get(), err.get(), OCI_DEFAULT))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCITransCommit(svc.get(), err.get(), OCI_DEFAULT))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCITransRollback(svc.get(), err.get(), OCI_DEFAULT))
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        wait_val!(|this, this.err, this.stmt, cx|
            OCIStmtPrepare2(
                svc.get(), this.stmt.as_mut_ptr(), this.err,
                this.sql.as_ptr(), this.sql.len() as u32,
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        wait_oci_result!(|this, this.err, cx|
            OCIStmtExecute(svc.get(), this.stmt, this.err, this.iter, 0, std::ptr::null(), std::ptr::null(), OCI_DEFAULT)
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
        let this = self.get_mut();
        wait_oci_result!(|this, this.err, cx|
            OCIStmtFetch2(this.stmt, this.err, 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        let id = this as *mut Self as usize;
        if !this.ctx.lock(id) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let res = unsafe {
            OCIStmtGetNextResult(this.stmt, this.err, this.cursor.as_mut_ptr(), &mut this.stmt_type, OCI_DEFAULT)
        };
        let res = check_invalid_handle!(this.err, res);
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            this.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci(this.err, res)))
            } else if res == OCI_NO_DATA {
                Poll::Ready(Ok(None))
            } else {
                Poll::Ready(Ok(Some(this.cursor)))
            }
        }
    }
}


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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_bool_flag!(|this, &err, this.flag, cx| OCILobIsOpen(svc.get(), err.get(), this.lob, &mut this.flag))
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_bool_flag!(|this, &err, this.flag, cx| OCILobIsTemporary(svc.get(), err.get(), this.lob, &mut this.flag))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobClose(svc.get(), err.get(), this.lob))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobFileClose(svc.get(), err.get(), this.lob))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobFreeTemporary(svc.get(), err.get(), this.lob))
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_val!(|this, &err, this.dst, cx| OCILobLocatorAssign(svc.get(), err.get(), this.src, this.dst.as_mut_ptr()))
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_val!(|this, &err, this.len, cx| OCILobGetLength2(svc.get(), err.get(), this.lob, &mut this.len))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobOpen(svc.get(), err.get(), this.lob, this.mode))
    }
}

pub(crate) struct LobRead<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
    buf:  &'a mut Vec<u8>,
    buf_ptr:    *mut u8,
    buf_len:    u64,
    offset:     u64,
    num_bytes:  u64,
    num_chars:  u64,
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobRead<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator, piece: u8, piece_size: usize, offset: usize, byte_len: usize, char_len: usize, char_form: u8, buf: &'a mut Vec<u8>) -> Self {
        let buf_ptr = unsafe { buf.as_mut_ptr().add(buf.len()) };
        let buf_len = std::cmp::min(piece_size, buf.capacity() - buf.len()) as u64;
        Self {
            ctx, lob, buf, buf_ptr, buf_len, offset: (offset + 1) as u64, num_bytes: byte_len as u64, num_chars: char_len as u64, char_form, piece,
        }
    }
}

impl<'a> Future for LobRead<'a> {
    type Output = Result<(i32,usize, usize)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        let id = this as *mut Self as usize;
        if !this.ctx.lock(id) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        let res = unsafe {
            OCILobRead2(
                svc.get(), err.get(), this.lob,
                &mut this.num_bytes, &mut this.num_chars, this.offset,
                this.buf_ptr, this.buf_len, this.piece,
                std::ptr::null_mut(), std::ptr::null(),
                AL32UTF8, this.char_form
            )
        };
        let res = check_invalid_handle!(err, res);
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            this.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci(&err, res)))
            } else {
                Poll::Ready(Ok((res, this.num_bytes as usize, this.num_chars as usize)))
            }
        }
    }
}


pub(crate) struct LobWrite<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
    data: &'a [u8],
    offset:     u64,
    num_bytes:  u64,
    num_chars:  u64,
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobWrite<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator, piece: u8, char_form: u8, offset: usize, data: &'a [u8]) -> Self {
        let num_bytes = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        Self { ctx, lob, data, offset: (offset + 1) as u64, char_form, piece, num_chars: 0, num_bytes }
    }
}

impl<'a> Future for LobWrite<'a> {
    type Output = Result<(usize,usize)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        let id = this as *mut Self as usize;
        if !this.ctx.lock(id) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        let res = unsafe {
            OCILobWrite2(
                svc.get(), err.get(), this.lob,
                &mut this.num_bytes, &mut this.num_chars, this.offset,
                this.data.as_ptr(), this.data.len() as u64, this.piece,
                std::ptr::null(), std::ptr::null(),
                AL32UTF8, this.char_form
            )
        };
        let res = check_invalid_handle!(err, res);
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            this.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci(&err, res)))
            } else {
                Poll::Ready(Ok((this.num_bytes as usize, this.num_chars as usize)))
            }
        }
    }
}

pub(crate) struct LobWriteAppend<'a> {
    ctx:  Arc<SvcCtx>,
    lob:  &'a OCILobLocator,
    data: &'a [u8],
    num_bytes:  u64,
    num_chars:  u64,
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobWriteAppend<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, lob: &'a OCILobLocator, piece: u8, char_form: u8, data: &'a [u8]) -> Self {
        let num_bytes = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        Self { ctx, lob, data, char_form, piece, num_chars: 0, num_bytes }
    }
}

impl<'a> Future for LobWriteAppend<'a> {
    type Output = Result<(usize,usize)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        let id = this as *mut Self as usize;
        if !this.ctx.lock(id) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        let res = unsafe {
            OCILobWriteAppend2(
                svc.get(), err.get(), this.lob,
                &mut this.num_bytes, &mut this.num_chars,
                this.data.as_ptr(), this.data.len() as u64, this.piece,
                std::ptr::null(), std::ptr::null(),
                AL32UTF8, this.char_form
            )
        };
        let res = check_invalid_handle!(err, res);
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            this.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci(&err, res)))
            } else {
                Poll::Ready(Ok((this.num_bytes as usize, this.num_chars as usize)))
            }
        }
    }
}


pub(crate) struct LobAppend<'a> {
    ctx:  Arc<SvcCtx>,
    dst:  &'a OCILobLocator,
    src:  &'a OCILobLocator,
}

impl<'a> LobAppend<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, dst: &'a OCILobLocator, src: &'a OCILobLocator) -> Self {
        Self { ctx, dst, src }
    }
}

impl<'a> Future for LobAppend<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobAppend(svc.get(), err.get(), this.dst, this.src))
    }
}


pub(crate) struct LobCopy<'a> {
    ctx:  Arc<SvcCtx>,
    dst:  &'a OCILobLocator,
    src:  &'a OCILobLocator,
    dst_off: u64,
    src_off: u64,
    amount:  u64,
}

impl<'a> LobCopy<'a> {
    pub(crate) fn new(ctx: Arc<SvcCtx>, dst: &'a OCILobLocator, dst_off: usize, src: &'a OCILobLocator, src_off: usize, amount: usize) -> Self {
        Self { ctx, dst, dst_off: (dst_off + 1) as u64, src, src_off: (src_off + 1) as u64, amount: amount as u64 }
    }
}

impl<'a> Future for LobCopy<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobCopy2(svc.get(), err.get(), this.dst, this.src, this.amount, this.dst_off, this.src_off))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobLoadFromFile2(svc.get(), err.get(), this.dst, this.src, this.amount, this.dst_off, this.src_off))
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_val!(|this, &err, this.len, cx| OCILobErase2(svc.get(), err.get(), this.lob, &mut this.len, this.off))
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_val!(|this, &err, this.len, cx| OCILobGetChunkSize(svc.get(), err.get(), this.lob, &mut this.len))
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        let id = this as *mut Self as usize;
        if !this.ctx.lock(id) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        let env: Ptr<OCIEnv>    = Ptr::from(this.ctx.as_ref().as_ref());
        let res = unsafe {
            OCILobGetContentType(env.get(), svc.get(), err.get(), this.lob, this.buf.as_mut_ptr(), &mut this.len, 0)
        };
        let res = check_invalid_handle!(err, res);
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            this.ctx.unlock();
            if res < 0 {
                Poll::Ready(Err(Error::oci(&err, res)))
            } else {
                let txt = unsafe {
                    this.buf.set_len(this.len as usize);
                    String::from_utf8_unchecked(this.buf.as_slice().to_vec())
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        let env: Ptr<OCIEnv>    = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobSetContentType(env.get(), svc.get(), err.get(), this.lob, this.ctx_type.as_ptr(), this.ctx_type.len() as u32, 0))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobTrim2(svc.get(), err.get(), this.lob, this.len))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx|
            OCILobCreateTemporary(svc.get(), err.get(), this.lob, OCI_DEFAULT as u16, this.csform, this.lobtype, this.cache, OCI_DURATION_SESSION)
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_bool_flag!(|this, &err, this.flag, cx| OCILobFileExists(svc.get(), err.get(), this.lob, &mut this.flag))
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_bool_flag!(|this, &err, this.flag, cx| OCILobFileIsOpen(svc.get(), err.get(), this.lob, &mut this.flag))
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
        let this = self.get_mut();
        let svc: Ptr<OCISvcCtx> = Ptr::from(this.ctx.as_ref().as_ref());
        let err: Ptr<OCIError>  = Ptr::from(this.ctx.as_ref().as_ref());
        wait_result!(|this, &err, cx| OCILobFileOpen(svc.get(), err.get(), this.lob, OCI_FILE_READONLY))
    }
}
