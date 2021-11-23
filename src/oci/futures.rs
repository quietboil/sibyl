//! Futures for OCI functions that might return `OCI_STILL_EXECUTING`

use super::{*, ptr::Ptr};
use std::{future::Future, pin::Pin, task::{Context, Poll}};

pub(crate) struct SessionEnd {
    svc_ptr: *mut OCISvcCtx,
    err_ptr: *mut OCIError,
    usr_ptr: *mut OCISession,
}

impl SessionEnd {
    pub(crate) fn new(svc_ptr: *mut OCISvcCtx, err_ptr: *mut OCIError, usr_ptr: *mut OCISession) -> Self {
        Self { svc_ptr, err_ptr, usr_ptr }
    }
}

impl Future for SessionEnd {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = unsafe {
            OCISessionEnd(self.svc_ptr, self.err_ptr, self.usr_ptr, OCI_DEFAULT)
        };
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub(crate) struct ServerDetach {
    srv_ptr: *mut OCIServer,
    err_ptr: *mut OCIError,
}

impl ServerDetach{
    pub(crate) fn new(srv_ptr: *mut OCIServer, err_ptr: *mut OCIError) -> Self {
        Self { srv_ptr, err_ptr }
    }
}

impl Future for ServerDetach {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = unsafe {
            OCIServerDetach(self.srv_ptr, self.err_ptr, OCI_DEFAULT)
        };
        if res == OCI_STILL_EXECUTING {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
