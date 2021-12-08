//! Futures for OCI functions that might return `OCI_STILL_EXECUTING`

use crate::conn::Session;

use super::{*, ptr::Ptr};
use std::{future::Future, pin::Pin, task::{Context, Poll}, sync::Arc, ptr, mem, slice};


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
        wait!(|cx| OCISessionPoolDestroy(self.pool.get(), self.err.get(), OCI_SPD_FORCE))
    }
}

pub(crate) struct ConnectionPoolDestroy {
    pool: Handle<OCICPool>,
    err:  Handle<OCIError>,
    env:  Arc<Handle<OCIEnv>>,
}

impl ConnectionPoolDestroy {
    pub(crate) fn new(pool: Handle<OCICPool>, err:  Handle<OCIError>, env: Arc<Handle<OCIEnv>>) -> Self {
        Self { pool, err, env }
    }
}

impl Future for ConnectionPoolDestroy {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait!(|cx| OCIConnectionPoolDestroy(self.pool.get(), self.err.get(), OCI_SPD_FORCE))
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
        wait!(|cx| OCISessionRelease(self.svc.get(), self.err.get(), ptr::null(), 0, OCI_DEFAULT))
    }
}

pub(crate) struct StmtRelease {
    session:  Arc<Session>,
    stmt:     Ptr<OCIStmt>,
    err:      Handle<OCIError>,
}

impl StmtRelease {
    pub(crate) fn new(stmt: Ptr<OCIStmt>, err: Handle<OCIError>, session: Arc<Session>) -> Self {
        Self { stmt, err, session }
    }
}

impl Future for StmtRelease {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait!(|cx| OCIStmtRelease(self.stmt.get(), self.err.get(), ptr::null(), 0, OCI_DEFAULT))
    }
}


macro_rules! wait_result {
    (|$this:ident, $ctx:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        match res {
            OCI_STILL_EXECUTING => {
                $ctx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                Poll::Ready(Ok(()))
            }
            _ => {
                Poll::Ready(Err(Error::oci($this.err.get(), res)))
            },
        }
    }};
}

macro_rules! wait_oci_result {
    (|$this:ident, $ctx:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        match res {
            OCI_STILL_EXECUTING => {
                $ctx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO | OCI_NO_DATA => {
                Poll::Ready(Ok(res))
            }
            _ => {
                Poll::Ready(Err(Error::oci($this.err.get(), res)))
            },
        }
    }};
}


pub(crate) struct Ping {
    svc: Ptr<OCISvcCtx>,
    err: Ptr<OCIError>,
}

impl Ping {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>) -> Self {
        Self { svc, err }
    }
}

impl Future for Ping {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCIPing(self.svc.get(), self.err.get(), OCI_DEFAULT))
    }
}


pub(crate) struct StmtPrepare {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    sql:  String,
}

impl StmtPrepare {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, sql: String) -> Self {
        Self { svc, err, sql }
    }
}

impl Future for StmtPrepare {
    type Output = Result<Ptr<OCIStmt>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut stmt = Ptr::null();
        let res = unsafe {
            OCIStmtPrepare2(
                self.svc.get(), stmt.as_mut_ptr(), self.err.get(),
                self.sql.as_ptr(), self.sql.len() as u32,
                ptr::null(), 0,
                OCI_NTV_SYNTAX, OCI_DEFAULT
            )
        };
        match res {
            OCI_STILL_EXECUTING => {
                cx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                Poll::Ready(Ok(stmt))
            }
            _ => {
                Poll::Ready(Err(Error::oci(self.err.get(), res)))
            },
        }
    }
}

pub(crate) struct StmtExecute {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    stmt: Ptr<OCIStmt>,
    iter: u32,
}

impl StmtExecute {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, stmt: Ptr<OCIStmt>, typ: u16) -> Self {
        let iter: u32 = if typ == OCI_STMT_SELECT { 0 } else { 1 };
        Self { svc, err , stmt, iter}
    }
}

impl Future for StmtExecute {
    type Output = Result<i32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_oci_result!( |self, cx|
            OCIStmtExecute(
                self.svc.get(), self.stmt.get(), self.err.get(),
                self.iter, 0,
                ptr::null::<c_void>(), ptr::null_mut::<c_void>(),
                OCI_DEFAULT
            )
        )
    }
}

pub(crate) struct StmtFetch {
    stmt: Ptr<OCIStmt>,
    err:  Ptr<OCIError>,
}

impl StmtFetch {
    pub(crate) fn new(stmt: Ptr<OCIStmt>, err: Ptr<OCIError>) -> Self {
        Self { stmt, err }
    }
}

impl Future for StmtFetch {
    type Output = Result<i32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_oci_result!( |self, cx|
            OCIStmtFetch2(self.stmt.get(), self.err.get(), 1, OCI_FETCH_NEXT, 0, OCI_DEFAULT)
        )
    }
}

pub(crate) struct TransCommit {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl TransCommit {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>) -> Self {
        Self { svc, err }
    }
}

impl Future for TransCommit {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCITransCommit(self.svc.get(), self.err.get(), OCI_DEFAULT))
    }
}

pub(crate) struct TransRollback {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl TransRollback {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>) -> Self {
        Self { svc, err }
    }
}

impl Future for TransRollback {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCITransRollback(self.svc.get(), self.err.get(), OCI_DEFAULT))
    }
}

pub(crate) struct StmtGetNextResult {
    stmt: Ptr<OCIStmt>,
    err:  Ptr<OCIError>,
}

impl StmtGetNextResult {
    pub(crate) fn new(stmt: Ptr<OCIStmt>, err: Ptr<OCIError>) -> Self {
        Self { stmt, err }
    }
}

impl Future for StmtGetNextResult {
    type Output = Result<Option<Ptr<OCIStmt>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut cursor = Ptr::null();
        let mut stmt_type = 0u32;
        let res = unsafe {
            OCIStmtGetNextResult(self.stmt.get(), self.err.get(), cursor.as_mut_ptr(), &mut stmt_type, OCI_DEFAULT)
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
                Poll::Ready(Ok(Some(cursor)))
            }
            _ => {
                Poll::Ready(Err(Error::oci(self.err.get(), res)))
            },
        }
    }
}

macro_rules! wait_bool_flag {
    (|$this:ident, $ctx:ident, $flag:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        match res {
            OCI_STILL_EXECUTING => {
                $ctx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                Poll::Ready(Ok($flag != 0))
            }
            _ => {
                Poll::Ready(Err(Error::oci($this.err.get(), res)))
            },
        }
    }};
}

macro_rules! wait_val {
    (|$this:ident, $ctx:ident, $val:ident| $oci_call:expr) => {{
        let res = unsafe { $oci_call };
        match res {
            OCI_STILL_EXECUTING => {
                $ctx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                Poll::Ready(Ok($val))
            }
            _ => {
                Poll::Ready(Err(Error::oci($this.err.get(), res)))
            },
        }
    }};
}


pub(crate) struct LobIsOpen {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    lob:  Ptr<OCILobLocator>,
}

impl LobIsOpen {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobIsOpen {
    type Output = Result<bool>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut flag = 0u8;
        wait_bool_flag!(|self, cx, flag| OCILobIsOpen(self.svc.get(), self.err.get(), self.lob.get(), &mut flag))
    }
}

pub(crate) struct LobIsTemporary {
    lob:  Ptr<OCILobLocator>,
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl LobIsTemporary {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobIsTemporary {
    type Output = Result<bool>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut flag = 0u8;
        wait_bool_flag!(|self, cx, flag| OCILobIsTemporary(self.svc.get(), self.err.get(), self.lob.get(), &mut flag))
    }
}

pub(crate) struct LobClose {
    lob:  Ptr<OCILobLocator>,
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl LobClose {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobClose {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobClose(self.svc.get(), self.err.get(), self.lob.get()))
    }
}

pub(crate) struct LobFreeTemporary {
    lob:  Ptr<OCILobLocator>,
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl LobFreeTemporary {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobFreeTemporary {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobFreeTemporary(self.svc.get(), self.err.get(), self.lob.get()))
    }
}

pub(crate) struct LobLocatorAssign {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    src:  Ptr<OCILobLocator>,
    dst:  Ptr<OCILobLocator>,
}

impl LobLocatorAssign {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, src: Ptr<OCILobLocator>, dst: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, src, dst }
    }
}

impl Future for LobLocatorAssign {
    type Output = Result<Ptr<OCILobLocator>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut dst_ptr = self.dst.clone();
        wait_val!(|self, cx, dst_ptr| OCILobLocatorAssign(self.svc.get(), self.err.get(), self.src.get(), dst_ptr.as_mut_ptr()))
    }
}

pub(crate) struct LobGetLength {
    lob:  Ptr<OCILobLocator>,
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl LobGetLength {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobGetLength {
    type Output = Result<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut len = 0u64;
        wait_val!(|self, cx, len| OCILobGetLength2(self.svc.get(), self.err.get(), self.lob.get(), &mut len))
    }
}

pub(crate) struct LobOpen {
    lob:  Ptr<OCILobLocator>,
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    mode: u8,
}

impl LobOpen {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>, mode: u8) -> Self {
        Self { lob, svc, err, mode }
    }
}

impl Future for LobOpen {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobOpen(self.svc.get(), self.err.get(), self.lob.get(), self.mode))
    }
}

pub(crate) struct LobRead<'a> {
    lob:        Ptr<OCILobLocator>,
    svc:        Ptr<OCISvcCtx>,
    err:        Ptr<OCIError>,
    buf:        &'a mut Vec<u8>,
    piece_size: u64,
    offset:     u64,
    byte_len:   u64,
    char_len:   u64,
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobRead<'a> {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>, piece: u8, piece_size: usize, offset: usize, byte_len: usize, char_len: usize, char_form: u8, buf: &'a mut Vec<u8>) -> Self {
        Self { lob, svc, err, piece, piece_size: piece_size as u64, offset: offset as u64 + 1, byte_len: byte_len as u64, char_len: char_len as u64, char_form, buf }
    }
}

impl Future for LobRead<'_> {
    type Output = Result<(i32,usize)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut num_bytes = this.byte_len;
        let mut num_chars = this.char_len;
        let res = unsafe {
            OCILobRead2(
                this.svc.get(), this.err.get(), this.lob.get(),
                &mut num_bytes, &mut num_chars, this.offset,
                this.buf.as_mut_ptr().add(this.buf.len()), this.piece_size, this.piece,
                ptr::null_mut::<c_void>(), ptr::null::<c_void>(),
                AL32UTF8, this.char_form
            )
        };
        match res {
            OCI_STILL_EXECUTING => {
                cx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO | OCI_NEED_DATA => {
                Poll::Ready(Ok((res,num_bytes as usize)))
            }
            _ => {
                Poll::Ready(Err(Error::oci(this.err.get(), res)))
            },
        }
    }
}

pub(crate) struct LobWrite<'a> {
    lob:        Ptr<OCILobLocator>,
    svc:        Ptr<OCISvcCtx>,
    err:        Ptr<OCIError>,
    data:       &'a [u8],
    offset:     u64,
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobWrite<'a> {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>, piece: u8, char_form: u8, offset: usize, data: &'a [u8]) -> Self {
        Self { lob, svc, err, data, offset: offset as u64 + 1, char_form, piece }
    }
}

impl Future for LobWrite<'_> {
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut num_bytes = if self.piece == OCI_ONE_PIECE { self.data.len() as u64 } else { 0u64 };
        let mut num_chars = 0u64;
        let res = unsafe {
            OCILobWrite2(
                self.svc.get(), self.err.get(), self.lob.get(),
                &mut num_bytes, &mut num_chars, self.offset,
                self.data.as_ptr(), self.data.len() as u64, self.piece,
                ptr::null_mut::<c_void>(), ptr::null::<c_void>(),
                AL32UTF8, self.char_form
            )
        };
        match res {
            OCI_STILL_EXECUTING => {
                cx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO | OCI_NEED_DATA => {
                Poll::Ready(Ok(num_bytes as usize))
            }
            _ => {
                Poll::Ready(Err(Error::oci(self.err.get(), res)))
            },
        }
    }
}

pub(crate) struct LobWriteAppend<'a> {
    lob:        Ptr<OCILobLocator>,
    svc:        Ptr<OCISvcCtx>,
    err:        Ptr<OCIError>,
    data:       &'a [u8],
    char_form:  u8,
    piece:      u8,
}

impl<'a> LobWriteAppend<'a> {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>, piece: u8, char_form: u8, data: &'a [u8]) -> Self {
        Self { lob, svc, err, data, char_form, piece }
    }
}

impl Future for LobWriteAppend<'_> {
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut num_bytes = if self.piece == OCI_ONE_PIECE { self.data.len() as u64 } else { 0u64 };
        let mut num_chars = 0u64;
        let res = unsafe {
            OCILobWriteAppend2(
                self.svc.get(), self.err.get(), self.lob.get(),
                &mut num_bytes, &mut num_chars,
                self.data.as_ptr(), self.data.len() as u64, self.piece,
                ptr::null_mut::<c_void>(), ptr::null::<c_void>(),
                AL32UTF8, self.char_form
            )
        };
        match res {
            OCI_STILL_EXECUTING => {
                cx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO | OCI_NEED_DATA => {
                Poll::Ready(Ok(num_bytes as usize))
            }
            _ => {
                Poll::Ready(Err(Error::oci(self.err.get(), res)))
            },
        }
    }
}

// OCILobWriteAppend2(svchp, errhp, loc, byte_cnt, char_cnt, buf, buf_len, piece, ctx, write_cb, csid, csfrm)

pub(crate) struct LobAppend {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    lob:  Ptr<OCILobLocator>,
    src:  Ptr<OCILobLocator>,
}

impl LobAppend {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>, src: Ptr<OCILobLocator>) -> Self {
        Self { lob, src, svc, err }
    }
}

impl Future for LobAppend {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobAppend(self.svc.get(), self.err.get(), self.lob.get(), self.src.get()))
    }
}

pub(crate) struct LobCopy {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    src:  Ptr<OCILobLocator>,
    dst:  Ptr<OCILobLocator>,
    src_off: u64,
    dst_off: u64,
    amount:  u64,
}

impl LobCopy {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, dst: Ptr<OCILobLocator>, src: Ptr<OCILobLocator>, dst_off: usize, src_off: usize, amount: usize) -> Self {
        Self { dst, src, dst_off: dst_off as u64 + 1, src_off: src_off as u64 + 1, amount: amount as u64, svc, err }
    }
}

impl Future for LobCopy {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobCopy2(self.svc.get(), self.err.get(), self.dst.get(), self.src.get(), self.amount, self.dst_off, self.src_off))
    }
}

pub(crate) struct LobLoadFromFile {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    src:  Ptr<OCILobLocator>,
    dst:  Ptr<OCILobLocator>,
    src_off: u64,
    amount:  u64,
    dst_off: u64,
}

impl LobLoadFromFile {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, dst: Ptr<OCILobLocator>, src: Ptr<OCILobLocator>, src_off: usize, amount: usize, dst_off: usize) -> Self {
        Self { svc, err , src, src_off: src_off as u64 + 1, amount: amount as u64, dst, dst_off: dst_off as u64 + 1 }
    }
}

impl Future for LobLoadFromFile {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobLoadFromFile2(self.svc.get(), self.err.get(), self.dst.get(), self.src.get(), self.amount, self.dst_off, self.src_off))
    }
}

pub(crate) struct LobErase {
    svc: Ptr<OCISvcCtx>,
    err: Ptr<OCIError>,
    lob: Ptr<OCILobLocator>,
    off: u64,
    len: u64,
}

impl LobErase {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>, off: usize, len: usize) -> Self {
        Self { lob, off: off as u64 + 1, len: len as u64, svc, err }
    }
}

impl Future for LobErase {
    type Output = Result<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut len = self.len;
        wait_val!(|self, cx, len| OCILobErase2(self.svc.get(), self.err.get(), self.lob.get(), &mut len, self.off))
    }
}

pub(crate) struct LobGetChunkSize {
    lob: Ptr<OCILobLocator>,
    svc: Ptr<OCISvcCtx>,
    err: Ptr<OCIError>,
}

impl LobGetChunkSize {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobGetChunkSize {
    type Output = Result<u32>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut size = 0u32;
        wait_val!(|self, cx, size| OCILobGetChunkSize(self.svc.get(), self.err.get(), self.lob.get(), &mut size))
    }
}

pub(crate) struct LobGetContentType {
    svc: Ptr<OCISvcCtx>,
    err: Ptr<OCIError>,
    env: Ptr<OCIEnv>,
    lob: Ptr<OCILobLocator>,
}

impl LobGetContentType {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, env: Ptr<OCIEnv>, lob: Ptr<OCILobLocator>) -> Self {
        Self { lob, env, svc, err }
    }
}

impl Future for LobGetContentType {
    type Output = Result<String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut ctx_type : [u8;OCI_LOB_CONTENTTYPE_MAXSIZE] = unsafe { mem::MaybeUninit::uninit().assume_init() };
        let mut len = ctx_type.len() as u32;
        let res = unsafe {
            OCILobGetContentType(self.env.get(), self.svc.get(), self.err.get(), self.lob.get(), ctx_type.as_mut_ptr(), &mut len, 0)
        };
        match res {
            OCI_STILL_EXECUTING => {
                cx.waker().wake_by_ref();
                Poll::Pending
            },
            OCI_SUCCESS | OCI_SUCCESS_WITH_INFO => {
                let txt = unsafe{ slice::from_raw_parts(ctx_type.as_ptr(), len as usize) };
                let txt = String::from_utf8_lossy(txt);
                Poll::Ready(Ok(txt.to_string()))
            }
            _ => {
                Poll::Ready(Err(Error::oci(self.err.get(), res)))
            },
        }
    }
}

pub(crate) struct LobSetContentType<'a> {
    svc: Ptr<OCISvcCtx>,
    err: Ptr<OCIError>,
    env: Ptr<OCIEnv>,
    lob: Ptr<OCILobLocator>,
    ctx_type: &'a str,
}

impl<'a> LobSetContentType<'a> {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, env: Ptr<OCIEnv>, lob: Ptr<OCILobLocator>, ctx_type: &'a str) -> Self {
        Self { lob, ctx_type, env, svc, err }
    }
}

impl Future for LobSetContentType<'_> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobSetContentType(self.env.get(), self.svc.get(), self.err.get(), self.lob.get(), self.ctx_type.as_ptr(), self.ctx_type.len() as u32, 0))
    }
}

pub(crate) struct LobTrim {
    svc: Ptr<OCISvcCtx>,
    err: Ptr<OCIError>,
    lob: Ptr<OCILobLocator>,
    len: u64,
}

impl LobTrim {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>, len: usize) -> Self {
        Self { lob, len: len as u64, svc, err }
    }
}

impl Future for LobTrim {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobTrim2(self.svc.get(), self.err.get(), self.lob.get(), self.len))
    }
}

pub(crate) struct LobCreateTemporary {
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
    lob:  Ptr<OCILobLocator>,
    lobtype: u8,
    csform:  u8,
    cache:   u8,
}

impl LobCreateTemporary {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>, lobtype: u8, csform: u8, cache: u8) -> Self {
        Self { svc, err, lob, lobtype, csform, cache}
    }
}

impl Future for LobCreateTemporary {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx|
            OCILobCreateTemporary(
                self.svc.get(), self.err.get(), self.lob.get(),
                OCI_DEFAULT as u16, self.csform, self.lobtype, self.cache, OCI_DURATION_SESSION
            )
        )
    }
}

pub(crate) struct LobFileExists {
    lob:  Ptr<OCILobLocator>,
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl LobFileExists {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobFileExists {
    type Output = Result<bool>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut flag = 0u8;
        wait_bool_flag!(|self, cx, flag| OCILobFileExists(self.svc.get(), self.err.get(), self.lob.get(), &mut flag))
    }
}

pub(crate) struct LobFileIsOpen {
    lob:  Ptr<OCILobLocator>,
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl LobFileIsOpen {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobFileIsOpen {
    type Output = Result<bool>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut flag = 0u8;
        wait_bool_flag!(|self, cx, flag| OCILobFileIsOpen(self.svc.get(), self.err.get(), self.lob.get(), &mut flag))
    }
}

pub(crate) struct LobFileOpen {
    lob:  Ptr<OCILobLocator>,
    svc:  Ptr<OCISvcCtx>,
    err:  Ptr<OCIError>,
}

impl LobFileOpen {
    pub(crate) fn new(svc: Ptr<OCISvcCtx>, err: Ptr<OCIError>, lob: Ptr<OCILobLocator>) -> Self {
        Self { svc, err, lob }
    }
}

impl Future for LobFileOpen {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        wait_result!(|self, cx| OCILobFileOpen(self.svc.get(), self.err.get(), self.lob.get(), OCI_FILE_READONLY))
    }
}

