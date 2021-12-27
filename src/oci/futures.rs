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

macro_rules! wait_bool_flag {
    (|$this:ident, $ctx:ident, $field:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        if res == OCI_STILL_EXECUTING {
            $ctx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci($this.err, res)))
        } else {
            Poll::Ready(Ok($this.$field != 0))
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
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci(this.err, res)))
        } else if res == OCI_NO_DATA {
            Poll::Ready(Ok(None))
        } else {
            Poll::Ready(Ok(Some(this.cursor)))
        }
    }
}


pub(crate) struct LobIsOpen<'a> {
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    lob:  &'a OCILobLocator,
    flag: u8,
}

impl<'a> LobIsOpen<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob, flag: 0 }
    }
}

impl<'a> Future for LobIsOpen<'a> {
    type Output = Result<bool>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        wait_bool_flag!(|this, cx, flag| OCILobIsOpen(this.svc, this.err, this.lob, &mut this.flag))
    }
}


pub(crate) struct LobIsTemporary<'a> {
    lob:  &'a OCILobLocator,
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    flag: u8,
}

impl<'a> LobIsTemporary<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob, flag: 0 }
    }
}

impl<'a> Future for LobIsTemporary<'a> {
    type Output = Result<bool>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        wait_bool_flag!(|this, cx, flag| OCILobIsTemporary(this.svc, this.err, this.lob, &mut this.flag))
    }
}


pub(crate) struct LobClose<'a> {
    lob:  &'a OCILobLocator,
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
}

impl<'a> LobClose<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob }
    }
}

impl<'a> Future for LobClose<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobClose(self.svc, self.err, self.lob))
    }
}


pub(crate) struct LobFreeTemporary<'a> {
    lob:  &'a OCILobLocator,
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
}

impl<'a> LobFreeTemporary<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob }
    }
}

impl<'a> Future for LobFreeTemporary<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobFreeTemporary(self.svc, self.err, self.lob))
    }
}


pub(crate) struct LobLocatorAssign<'a> {
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    src:  &'a OCILobLocator,
    dst:  Ptr<OCILobLocator>,
}

impl<'a> LobLocatorAssign<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, src: &'a OCILobLocator, dst: &'a OCILobLocator) -> Self {
        Self { svc, err, src, dst: Ptr::from(dst) }
    }
}

impl<'a> Future for LobLocatorAssign<'a> {
    type Output = Result<Ptr<OCILobLocator>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        wait_val!(|this, cx, dst| OCILobLocatorAssign(this.svc, this.err, this.src, this.dst.as_mut_ptr()))
    }
}

pub(crate) struct LobGetLength<'a> {
    lob:  &'a OCILobLocator,
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    len:  u64,
}

impl<'a> LobGetLength<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob, len: 0 }
    }
}

impl<'a> Future for LobGetLength<'a> {
    type Output = Result<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        wait_val!(|this, cx, len| OCILobGetLength2(this.svc, this.err, this.lob, &mut this.len))
    }
}

pub(crate) struct LobOpen<'a> {
    lob:  &'a OCILobLocator,
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    mode: u8,
}

impl<'a> LobOpen<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator, mode: u8) -> Self {
        Self { lob, svc, err, mode }
    }
}

impl<'a> Future for LobOpen<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobOpen(self.svc, self.err, self.lob, self.mode))
    }
}

pub(crate) struct LobRead<'a> {
    lob:        &'a OCILobLocator,
    svc:        &'a OCISvcCtx,
    err:        &'a OCIError,
    buf:        &'a mut Vec<u8>,
    buf_ptr:    *mut u8,
    buf_len:    u64,
    offset:     u64,
    num_bytes:  u64,
    num_chars:  u64,
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobRead<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator, piece: u8, piece_size: usize, offset: usize, byte_len: usize, char_len: usize, char_form: u8, buf: &'a mut Vec<u8>) -> Self {
        let buf_ptr = unsafe { buf.as_mut_ptr().add(buf.len()) };
        let buf_len = std::cmp::min(piece_size, buf.capacity() - buf.len()) as u64;
        Self { 
            lob, svc, err, buf, buf_ptr, buf_len, offset: (offset + 1) as u64, num_bytes: byte_len as u64, num_chars: char_len as u64, char_form, piece,
        }
    }
}

impl<'a> Future for LobRead<'a> {
    type Output = Result<(i32,usize, usize)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let res = unsafe {
            OCILobRead2(
                this.svc, this.err, this.lob,
                &mut this.num_bytes, &mut this.num_chars, this.offset,
                this.buf_ptr, this.buf_len, this.piece,
                std::ptr::null_mut(), std::ptr::null(),
                AL32UTF8, this.char_form
            )
        };        
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci(this.err, res)))
        } else {
            Poll::Ready(Ok((res, this.num_bytes as usize, this.num_chars as usize)))
        }
    }
}


pub(crate) struct LobWrite<'a> {
    lob:        &'a OCILobLocator,
    svc:        &'a OCISvcCtx,
    err:        &'a OCIError,
    data:       &'a [u8],
    offset:     u64,
    num_bytes:  u64,
    num_chars:  u64,
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobWrite<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator, piece: u8, char_form: u8, offset: usize, data: &'a [u8]) -> Self {
        let num_bytes = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        Self { lob, svc, err, data, offset: (offset + 1) as u64, char_form, piece, num_chars: 0, num_bytes }
    }
}

impl<'a> Future for LobWrite<'a> {
    type Output = Result<(usize,usize)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let res = unsafe {
            OCILobWrite2(
                this.svc, this.err, this.lob,
                &mut this.num_bytes, &mut this.num_chars, this.offset,
                this.data.as_ptr(), this.data.len() as u64, this.piece,
                std::ptr::null(), std::ptr::null(),
                AL32UTF8, this.char_form
            )
        };        
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci(this.err, res)))
        } else {
            Poll::Ready(Ok((this.num_bytes as usize, this.num_chars as usize)))
        }
    }
}

pub(crate) struct LobWriteAppend<'a> {
    lob:        &'a OCILobLocator,
    svc:        &'a OCISvcCtx,
    err:        &'a OCIError,
    data:       &'a [u8],
    num_bytes:  u64,
    num_chars:  u64,
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobWriteAppend<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator, piece: u8, char_form: u8, data: &'a [u8]) -> Self {
        let num_bytes = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        Self { lob, svc, err, data, char_form, piece, num_chars: 0, num_bytes }
    }
}

impl<'a> Future for LobWriteAppend<'a> {
    type Output = Result<(usize,usize)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let res = unsafe {
            OCILobWriteAppend2(
                this.svc, this.err, this.lob,
                &mut this.num_bytes, &mut this.num_chars,
                this.data.as_ptr(), this.data.len() as u64, this.piece,
                std::ptr::null(), std::ptr::null(),
                AL32UTF8, this.char_form
            )
        };        
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci(this.err, res)))
        } else {
            Poll::Ready(Ok((this.num_bytes as usize, this.num_chars as usize)))
        }
    }
}


pub(crate) struct LobAppend<'a> {
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    lob:  &'a OCILobLocator,
    src:  &'a OCILobLocator,
}

impl<'a> LobAppend<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator, src: &'a OCILobLocator) -> Self {
        Self { lob, src, svc, err }
    }
}

impl<'a> Future for LobAppend<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobAppend(self.svc, self.err, self.lob, self.src))
    }
}


pub(crate) struct LobCopy<'a> {
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    dst:  &'a OCILobLocator,
    src:  &'a OCILobLocator,
    dst_off: u64,
    src_off: u64,
    amount:  u64,
}

impl<'a> LobCopy<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, dst: &'a OCILobLocator, dst_off: usize, src: &'a OCILobLocator, src_off: usize, amount: usize) -> Self {
        Self { svc, err, dst, dst_off: (dst_off + 1) as u64, src, src_off: (src_off + 1) as u64, amount: amount as u64 }
    }
}

impl<'a> Future for LobCopy<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobCopy2(self.svc, self.err, self.dst, self.src, self.amount, self.dst_off, self.src_off))
    }
}


pub(crate) struct LobLoadFromFile<'a> {
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    dst:  &'a OCILobLocator,
    src:  &'a OCILobLocator,
    dst_off: u64,
    src_off: u64,
    amount:  u64,
}

impl<'a> LobLoadFromFile<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, dst: &'a OCILobLocator, dst_off: usize, src: &'a OCILobLocator, src_off: usize, amount: usize) -> Self {
        Self { svc, err, dst, dst_off: (dst_off + 1) as u64 , src, src_off: (src_off + 1) as u64, amount: amount as u64}
    }
}

impl<'a> Future for LobLoadFromFile<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobLoadFromFile2(self.svc, self.err, self.dst, self.src, self.amount, self.dst_off, self.src_off))
    }
}


pub(crate) struct LobErase<'a> {
    svc: &'a OCISvcCtx,
    err: &'a OCIError,
    lob: &'a OCILobLocator,
    off: u64,
    len: u64,
}

impl<'a> LobErase<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator, off: usize, len: usize) -> Self {
        Self { lob, off: (off + 1) as u64, len: len as u64, svc, err }
    }
}

impl<'a> Future for LobErase<'a> {
    type Output = Result<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        wait_val!(|this, cx, len| OCILobErase2(this.svc, this.err, this.lob, &mut this.len, this.off))
    }
}


pub(crate) struct LobGetChunkSize<'a> {
    lob: &'a OCILobLocator,
    svc: &'a OCISvcCtx,
    err: &'a OCIError,
    len: u32,
}

impl<'a> LobGetChunkSize<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob, len: 0 }
    }
}

impl<'a> Future for LobGetChunkSize<'a> {
    type Output = Result<u32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        wait_val!(|this, cx, len| OCILobGetChunkSize(this.svc, this.err, this.lob, &mut this.len))
    }
}


pub(crate) struct LobGetContentType<'a> {
    svc: &'a OCISvcCtx,
    err: &'a OCIError,
    env: &'a OCIEnv,
    lob: &'a OCILobLocator,
    buf: Vec<u8>,
    len: u32,
}

impl<'a> LobGetContentType<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, env: &'a OCIEnv, lob: &'a OCILobLocator) -> Self {
        let buf = Vec::with_capacity(OCI_LOB_CONTENTTYPE_MAXSIZE);
        Self { lob, env, svc, err, len: buf.capacity() as u32, buf }
    }
}

impl<'a> Future for LobGetContentType<'a> {
    type Output = Result<String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let res = unsafe {
            OCILobGetContentType(this.env, this.svc, this.err, this.lob, this.buf.as_mut_ptr(), &mut this.len, 0)
        };
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else if res < 0 {
            Poll::Ready(Err(Error::oci(this.err, res)))
        } else {
            let txt = unsafe { 
                this.buf.set_len(this.len as usize);
                String::from_utf8_unchecked(this.buf.as_slice().to_vec())
            };
            Poll::Ready(Ok(txt))
        }
    }
}


pub(crate) struct LobSetContentType<'a> {
    svc: &'a OCISvcCtx,
    err: &'a OCIError,
    env: &'a OCIEnv,
    lob: &'a OCILobLocator,
    ctx_type: &'a str,
}

impl<'a> LobSetContentType<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, env: &'a OCIEnv, lob: &'a OCILobLocator, ctx_type: &'a str) -> Self {
        Self { lob, ctx_type, env, svc, err }
    }
}

impl<'a> Future for LobSetContentType<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobSetContentType(self.env, self.svc, self.err, self.lob, self.ctx_type.as_ptr(), self.ctx_type.len() as u32, 0))
    }
}


pub(crate) struct LobTrim<'a> {
    svc: &'a OCISvcCtx,
    err: &'a OCIError,
    lob: &'a OCILobLocator,
    len: u64,
}

impl<'a> LobTrim<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator, len: usize) -> Self {
        Self { lob, len: len as u64, svc, err }
    }
}

impl<'a> Future for LobTrim<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobTrim2(self.svc, self.err, self.lob, self.len))
    }
}


pub(crate) struct LobCreateTemporary<'a> {
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    lob:  &'a OCILobLocator,
    lobtype: u8,
    csform:  u8,
    cache:   u8,
}

impl<'a> LobCreateTemporary<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator, lobtype: u8, csform: u8, cache: u8) -> Self {
        Self { svc, err, lob, lobtype, csform, cache}
    }
}

impl<'a> Future for LobCreateTemporary<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx|
            OCILobCreateTemporary(
                self.svc, self.err, self.lob,
                OCI_DEFAULT as u16, self.csform, self.lobtype, self.cache, OCI_DURATION_SESSION
            )
        )
    }
}


pub(crate) struct LobFileExists<'a> {
    lob:  &'a OCILobLocator,
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    flag: u8,
}

impl<'a> LobFileExists<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob, flag: 0 }
    }
}

impl<'a> Future for LobFileExists<'a> {
    type Output = Result<bool>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        wait_bool_flag!(|this, cx, flag| OCILobFileExists(this.svc, this.err, this.lob, &mut this.flag))
    }
}


pub(crate) struct LobFileIsOpen<'a> {
    lob:  &'a OCILobLocator,
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
    flag: u8,
}

impl<'a> LobFileIsOpen<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob, flag: 0 }
    }
}

impl<'a> Future for LobFileIsOpen<'a> {
    type Output = Result<bool>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        wait_bool_flag!(|this, cx, flag| OCILobFileIsOpen(this.svc, this.err, this.lob, &mut this.flag))
    }
}


pub(crate) struct LobFileOpen<'a> {
    lob:  &'a OCILobLocator,
    svc:  &'a OCISvcCtx,
    err:  &'a OCIError,
}

impl<'a> LobFileOpen<'a> {
    pub(crate) fn new(svc: &'a OCISvcCtx, err: &'a OCIError, lob: &'a OCILobLocator) -> Self {
        Self { svc, err, lob }
    }
}

impl<'a> Future for LobFileOpen<'a> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobFileOpen(self.svc, self.err, self.lob, OCI_FILE_READONLY))
    }
}
