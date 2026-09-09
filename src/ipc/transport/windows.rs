//! Windows transport: named-pipe server + client (TASK-006 M2).
//!
//! Sözleşme `unix.rs` ile birebir: bağlantı başına tek request →
//! handler → tek response → kapat. Wire format v1 değişmez.
//!
//! Zamanlama paritesi: sunucu tarafı okuma SÜRESİZ (unix `accept`
//! socket'inde timeout yok); istemci tarafı `timeout_secs` ile sınırlı
//! (`wait` op'u sunucuda bloklanır, istemci `timeout+15s` verir).
//! I/O, overlapped `ReadFile`/`WriteFile` + auto-reset event +
//! `WaitForSingleObject` ile yapılır — takılan okuma `CancelIo` ile
//! iptal edilir, sarkan thread/handle kalmaz.

use serde_json::Value;
use std::ffi::c_void;
use std::io::{self, BufReader, Read, Write};
use std::ptr;
use std::sync::Arc;

use super::super::codec::{read_message, write_message};
use super::super::protocol::error_response;
use crate::platform::paths::{sock_display, SockAddr};
use crate::platform::winffi::{
    self, BOOL, DWORD, ERROR_BROKEN_PIPE, ERROR_IO_PENDING, ERROR_PIPE_BUSY,
    ERROR_PIPE_CONNECTED, FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE,
    HANDLE, OPEN_EXISTING, WAIT_OBJECT_0, WAIT_TIMEOUT,
};

const PIPE_ACCESS_DUPLEX: DWORD = 3;
const FILE_FLAG_OVERLAPPED: DWORD = 0x4000_0000;
const PIPE_TYPE_BYTE: DWORD = 0;
const PIPE_READMODE_BYTE: DWORD = 0;
const PIPE_WAIT: DWORD = 0;
const PIPE_UNLIMITED_INSTANCES: DWORD = 255;
const INFINITE: DWORD = 0xFFFF_FFFF;

#[repr(C)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: HANDLE,
}

extern "system" {
    fn CreateNamedPipeW(
        name: *const u16,
        open_mode: DWORD,
        pipe_mode: DWORD,
        max_instances: DWORD,
        out_buf: DWORD,
        in_buf: DWORD,
        timeout: DWORD,
        sec: *mut c_void,
    ) -> HANDLE;
    fn ConnectNamedPipe(h: HANDLE, ov: *mut Overlapped) -> BOOL;
    fn DisconnectNamedPipe(h: HANDLE) -> BOOL;
    fn CreateFileW(
        name: *const u16,
        access: DWORD,
        share: DWORD,
        sec: *mut c_void,
        disp: DWORD,
        flags: DWORD,
        tmpl: HANDLE,
    ) -> HANDLE;
    fn WaitNamedPipeW(name: *const u16, timeout: DWORD) -> BOOL;
    fn ReadFile(
        h: HANDLE,
        buf: *mut u8,
        n: DWORD,
        read: *mut DWORD,
        ov: *mut Overlapped,
    ) -> BOOL;
    fn WriteFile(
        h: HANDLE,
        buf: *const u8,
        n: DWORD,
        written: *mut DWORD,
        ov: *mut Overlapped,
    ) -> BOOL;
    fn CreateEventW(sec: *mut c_void, manual: BOOL, initial: BOOL, name: *const u16) -> HANDLE;
    fn WaitForSingleObject(h: HANDLE, ms: DWORD) -> DWORD;
    fn GetOverlappedResult(h: HANDLE, ov: *mut Overlapped, n: *mut DWORD, wait: BOOL) -> BOOL;
    fn CancelIo(h: HANDLE) -> BOOL;
}

/// Timeout'lu pipe stream. `Drop` event + handle'ı kapatır;
/// sunucu thread ayrıca `DisconnectNamedPipe` çağırır.
struct PipeStream {
    handle: HANDLE,
    event: HANDLE,
    timeout_ms: DWORD,
}

// Kernel handle'ları münhasır sahiplenmede thread'ler arası taşınabilir:
// PipeStream bir anda tek thread'indir (serve'de move, client'ta local).
unsafe impl Send for PipeStream {}

impl PipeStream {
    fn new(handle: HANDLE, timeout_ms: DWORD) -> Option<Self> {
        let event = unsafe { CreateEventW(ptr::null_mut(), 0, 0, ptr::null()) };
        if !winffi::valid(event) {
            return None;
        }
        Some(Self {
            handle,
            event,
            timeout_ms,
        })
    }

    fn handle(&self) -> HANDLE {
        self.handle
    }

    /// Overlapped I/O çekirdeği: `op` tetiklenir, event beklenir.
    fn overlapped_op(&mut self, op: &dyn Fn(*mut Overlapped, *mut DWORD) -> BOOL) -> io::Result<DWORD> {
        let mut ov = Overlapped {
            internal: 0,
            internal_high: 0,
            offset: 0,
            offset_high: 0,
            event: self.event,
        };
        let mut n: DWORD = 0;
        let rc = op(&mut ov as *mut Overlapped, &mut n as *mut DWORD);
        if rc != 0 {
            return Ok(n);
        }
        let err = unsafe { winffi::GetLastError() };
        if err == ERROR_BROKEN_PIPE {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed by peer"));
        }
        if err != ERROR_IO_PENDING {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("pipe io start failed: {}", err),
            ));
        }
        match unsafe { WaitForSingleObject(self.event, self.timeout_ms) } {
            WAIT_OBJECT_0 => {
                let mut done: DWORD = 0;
                let ok = unsafe { GetOverlappedResult(self.handle, &mut ov, &mut done, 0) };
                if ok != 0 {
                    Ok(done)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("pipe io failed: {}", unsafe { winffi::GetLastError() }),
                    ))
                }
            }
            WAIT_TIMEOUT => {
                unsafe {
                    CancelIo(self.handle);
                }
                Err(io::Error::new(io::ErrorKind::TimedOut, "pipe io timed out"))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "pipe wait failed".to_string(),
            )),
        }
    }
}

impl Drop for PipeStream {
    fn drop(&mut self) {
        unsafe {
            if winffi::valid(self.event) {
                winffi::CloseHandle(self.event);
            }
            if winffi::valid(self.handle) {
                winffi::CloseHandle(self.handle);
            }
        }
    }
}

impl Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let h = self.handle;
        let len = buf.len().min(u32::MAX as usize) as DWORD;
        let ptr = buf.as_mut_ptr();
        let n = self.overlapped_op(&|ov, out| unsafe { ReadFile(h, ptr, len, out, ov) })?;
        Ok(n as usize)
    }
}

impl Write for PipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let h = self.handle;
        let len = buf.len().min(u32::MAX as usize) as DWORD;
        let ptr = buf.as_ptr();
        let n = self.overlapped_op(&|ov, out| unsafe { WriteFile(h, ptr, len, out, ov) })?;
        Ok(n as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Blocking per-connection server: accept -> read one request ->
/// handler -> write one response -> close (RFC: no persistent conns).
/// Her bağlantı kendi pipe instance'ını alır (unix `accept` eşdeğeri).
pub fn serve<F>(addr: &SockAddr, handler: Arc<F>) -> Result<(), String>
where
    F: Fn(Value) -> Value + Send + Sync + 'static,
{
    let name = winffi::wide_nul(addr.pipe_name());
    loop {
        let h = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                65536,
                65536,
                0,
                ptr::null_mut(),
            )
        };
        if !winffi::valid(h) {
            return Err(format!(
                "pipe bind {}: {}",
                sock_display(addr),
                unsafe { winffi::GetLastError() }
            ));
        }
        let rc = unsafe { ConnectNamedPipe(h, ptr::null_mut()) };
        if rc == 0 && unsafe { winffi::GetLastError() } != ERROR_PIPE_CONNECTED {
            // İstemci create-connect arasına giremedi (race) ya da koptu;
            // instance'ı kapat, yenisini aç.
            unsafe {
                winffi::CloseHandle(h);
            }
            continue;
        }
        let hh = handler.clone();
        match PipeStream::new(h, INFINITE) {
            Some(s) => {
                std::thread::spawn(move || {
                    let mut s = s;
                    handle_one(&mut s, &*hh);
                    unsafe {
                        DisconnectNamedPipe(s.handle());
                    }
                });
            }
            None => {
                unsafe {
                    DisconnectNamedPipe(h);
                    winffi::CloseHandle(h);
                }
            }
        }
    }
}

fn handle_one(stream: &mut PipeStream, handler: &dyn Fn(Value) -> Value) {
    let req: Value = {
        let mut reader = BufReader::new(&mut *stream);
        match read_message(&mut reader) {
            Ok(v) => v,
            Err(e) => {
                let resp = error_response("unknown", "INVALID_REQUEST", &e);
                drop(reader);
                let _ = write_message(&mut *stream, &resp);
                return;
            }
        }
    };
    let resp = handler(req);
    let _ = write_message(&mut *stream, &resp);
}

/// Client: connect, send one request, read one response.
pub fn send_request(addr: &SockAddr, req: &Value, timeout_secs: u64) -> Result<Value, String> {
    let name = winffi::wide_nul(addr.pipe_name());
    let ms = timeout_secs
        .max(2)
        .saturating_mul(1000)
        .min((INFINITE - 1) as u64) as DWORD;
    if unsafe { WaitNamedPipeW(name.as_ptr(), ms) } == 0 {
        return Err(format!(
            "pipe wait {}: {}",
            sock_display(addr),
            unsafe { winffi::GetLastError() }
        ));
    }
    let h = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            ptr::null_mut(),
        )
    };
    if !winffi::valid(h) {
        return Err(format!(
            "pipe connect {}: {}",
            sock_display(addr),
            unsafe { winffi::GetLastError() }
        ));
    }
    let mut s = match PipeStream::new(h, ms) {
        Some(s) => s,
        None => {
            unsafe {
                winffi::CloseHandle(h);
            }
            return Err("pipe event failed".to_string());
        }
    };
    write_message(&mut s, req).map_err(|e| format!("pipe write: {}", e))?;
    let resp = {
        let mut reader = BufReader::new(&mut s);
        read_message(&mut reader).map_err(|e| format!("pipe read: {}", e))?
    };
    Ok(resp)
}

/// Bu adreste canlı monitor dinliyor mu? (double-spawn guard, cleanup)
/// `ERROR_PIPE_BUSY` bile "var" demektir (tüm instance'lar dolu).
pub fn can_connect(addr: &SockAddr) -> bool {
    let name = winffi::wide_nul(addr.pipe_name());
    let h = unsafe {
        CreateFileW(
            name.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null_mut(),
            OPEN_EXISTING,
            0,
            ptr::null_mut(),
        )
    };
    if winffi::valid(h) {
        unsafe {
            winffi::CloseHandle(h);
        }
        return true;
    }
    unsafe { winffi::GetLastError() == ERROR_PIPE_BUSY }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::paths;

    fn test_addr(tag: &str) -> SockAddr {
        paths::default_sock(&format!("ut-{}-{}", std::process::id(), tag))
    }

    #[test]
    fn pipe_roundtrip_status() {
        let addr = test_addr("roundtrip");
        let srv = addr.clone();
        std::thread::spawn(move || {
            let handler = Arc::new(|req: Value| {
                let mut m = serde_json::Map::new();
                m.insert("echo".to_string(), req);
                crate::ipc::protocol::ok_response("t-1", m)
            });
            let _ = serve(&srv, handler);
        });
        std::thread::sleep(std::time::Duration::from_millis(300));
        let req = serde_json::json!({"v":1,"op":"status","id":"t-1"});
        let resp = send_request(&addr, &req, 10).expect("roundtrip");
        assert_eq!(resp["ok"], true);
        assert_eq!(resp["echo"], req);
    }

    #[test]
    fn pipe_no_server_is_error() {
        let addr = test_addr("nonexistent-xyz");
        let req = serde_json::json!({"v":1,"op":"status","id":"t-9"});
        assert!(send_request(&addr, &req, 2).is_err());
        assert!(!can_connect(&addr));
    }
}
