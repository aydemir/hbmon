//! Ham Win32 FFI ortakları (bağımlılık yok — `macos.rs` libproc emsali).
//!
//! Sadece kararlı kernel32/psapi giriş noktaları; her çağrı dönüş kodu
//! kontrol eder, asla paniklemez. M2: named-pipe transport.
//! M3: process inspection (Toolhelp + psapi) + Job Object.

use std::ffi::c_void;

pub type HANDLE = *mut c_void;
pub type DWORD = u32;
pub type BOOL = i32;

pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

pub const GENERIC_READ: DWORD = 0x8000_0000;
pub const GENERIC_WRITE: DWORD = 0x4000_0000;
pub const OPEN_EXISTING: DWORD = 3;
pub const FILE_SHARE_READ: DWORD = 1;
pub const FILE_SHARE_WRITE: DWORD = 2;

pub const ERROR_PIPE_BUSY: DWORD = 231;
pub const ERROR_BROKEN_PIPE: DWORD = 109;
pub const ERROR_PIPE_CONNECTED: DWORD = 535;
pub const ERROR_IO_PENDING: DWORD = 997;

pub const WAIT_OBJECT_0: DWORD = 0;
pub const WAIT_TIMEOUT: DWORD = 258;

// ---- process inspection (M3) ----

pub const TH32CS_SNAPPROCESS: DWORD = 2;
pub const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;
pub const PROCESS_VM_READ: DWORD = 0x10;
pub const PROCESS_TERMINATE: DWORD = 0x1;
pub const STILL_ACTIVE: DWORD = 259;

#[repr(C)]
pub struct FileTime {
    pub low: u32,
    pub high: u32,
}

impl FileTime {
    pub fn as_u64(&self) -> u64 {
        ((self.high as u64) << 32) | self.low as u64
    }
}

#[repr(C)]
pub struct ProcessEntry {
    pub size: u32,
    pub usage: u32,
    pub pid: u32,
    pub default_heap: usize,
    pub module_id: u32,
    pub threads: u32,
    pub ppid: u32,
    pub pri_base: i32,
    pub flags: u32,
    pub exe: [u16; 260],
}

#[repr(C)]
pub struct ProcessMemoryCounters {
    pub cb: u32,
    pub page_faults: u32,
    pub peak_working_set: usize,
    pub working_set: usize,
    pub quota_peak_paged: usize,
    pub quota_paged: usize,
    pub quota_peak_nonpaged: usize,
    pub quota_nonpaged: usize,
    pub pagefile_usage: usize,
    pub peak_pagefile_usage: usize,
}

#[repr(C)]
pub struct IoCounters {
    pub read_ops: u64,
    pub write_ops: u64,
    pub other_ops: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub other_bytes: u64,
}

extern "system" {
    pub fn CloseHandle(h: HANDLE) -> BOOL;
    pub fn GetLastError() -> DWORD;

    pub fn CreateToolhelp32Snapshot(flags: DWORD, pid: DWORD) -> HANDLE;
    pub fn Process32FirstW(h: HANDLE, e: *mut ProcessEntry) -> BOOL;
    pub fn Process32NextW(h: HANDLE, e: *mut ProcessEntry) -> BOOL;
    pub fn OpenProcess(access: DWORD, inherit: BOOL, pid: DWORD) -> HANDLE;
    pub fn GetExitCodeProcess(h: HANDLE, code: *mut DWORD) -> BOOL;
    pub fn GetProcessTimes(
        h: HANDLE,
        creation: *mut FileTime,
        exit: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> BOOL;
    pub fn QueryFullProcessImageNameW(h: HANDLE, flags: DWORD, buf: *mut u16, size: *mut DWORD)
        -> BOOL;
    pub fn GetProcessHandleCount(h: HANDLE, n: *mut DWORD) -> BOOL;
    pub fn GetProcessIoCounters(h: HANDLE, io: *mut IoCounters) -> BOOL;
    pub fn TerminateProcess(h: HANDLE, code: DWORD) -> BOOL;
}

#[link(name = "psapi")]
extern "system" {
    pub fn GetProcessMemoryInfo(h: HANDLE, mem: *mut ProcessMemoryCounters, size: DWORD) -> BOOL;
}

// ---- Job Object: build ağacını grupça öldürme (M3) ----

pub const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x2000;
/// `JOBOBJECTINFOCLASS::JobObjectBasicLimitInformation`
pub const JOB_OBJECT_BASIC_LIMIT_INFO: i32 = 2;

#[repr(C)]
pub struct JobBasicLimits {
    pub per_process_time: i64,
    pub per_job_time: i64,
    pub limit_flags: DWORD,
    pub min_working_set: usize,
    pub max_working_set: usize,
    pub active_process_limit: DWORD,
    pub affinity_mask: usize,
    pub priority_class: DWORD,
    pub scheduling_class: DWORD,
}

impl JobBasicLimits {
    pub fn kill_on_close() -> Self {
        Self {
            per_process_time: 0,
            per_job_time: 0,
            limit_flags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            min_working_set: 0,
            max_working_set: 0,
            active_process_limit: 0,
            affinity_mask: 0,
            priority_class: 0,
            scheduling_class: 0,
        }
    }
}

extern "system" {
    pub fn CreateJobObjectW(sec: *mut c_void, name: *const u16) -> HANDLE;
    pub fn AssignProcessToJobObject(job: HANDLE, proc: HANDLE) -> BOOL;
    pub fn SetInformationJobObject(
        job: HANDLE,
        class: i32,
        info: *const c_void,
        len: DWORD,
    ) -> BOOL;
    pub fn TerminateJobObject(job: HANDLE, code: DWORD) -> BOOL;
}

// ---- Detached spawn: bInheritHandles=FALSE zorunlu (M4) ----
//
// std::Command null-stdio + creation_flags ile bile capture pipe'larını
// devralır (bInheritHandles proses-genelidir; NUL handle'ları inheritable
// olunca pipe'lar da biner). `.output()`-tarzı bekleyen caller (test,
// harness) daemon ölene dek EOF göremez. Ham CreateProcessW + FALSE ile
// detached child HIÇBIR handle devralmaz — unix close(0,1,2) eşdeğeri.

pub const STARTF_USESTDHANDLES: DWORD = 0x1;
pub const CREATE_BREAKAWAY_FROM_JOB: DWORD = 0x0100_0000;
pub const DETACHED_PROCESS: DWORD = 0x0800_0000;
pub const CREATE_NEW_PROCESS_GROUP: DWORD = 0x0000_0200;

#[repr(C)]
pub struct StartupInfo {
    pub cb: DWORD,
    pub reserved: *mut u16,
    pub desktop: *mut u16,
    pub title: *mut u16,
    pub x: DWORD,
    pub y: DWORD,
    pub xsize: DWORD,
    pub ysize: DWORD,
    pub xcount: DWORD,
    pub ycount: DWORD,
    pub fill_attr: DWORD,
    pub flags: DWORD,
    pub show_window: u16,
    pub reserved2: u16,
    pub reserved2_ptr: *mut u8,
    pub std_input: HANDLE,
    pub std_output: HANDLE,
    pub std_error: HANDLE,
}

#[repr(C)]
pub struct ProcessInfo {
    pub process: HANDLE,
    pub thread: HANDLE,
    pub pid: DWORD,
    pub tid: DWORD,
}

extern "system" {
    pub fn CreateFileW(
        name: *const u16,
        access: DWORD,
        share: DWORD,
        sec: *mut c_void,
        disp: DWORD,
        flags: DWORD,
        tmpl: HANDLE,
    ) -> HANDLE;
    pub fn CreateProcessW(
        app: *const u16,
        cmdline: *mut u16,
        proc_sec: *mut c_void,
        thread_sec: *mut c_void,
        inherit: BOOL,
        flags: DWORD,
        env: *mut c_void,
        cwd: *const u16,
        si: *const StartupInfo,
        pi: *mut ProcessInfo,
    ) -> BOOL;
}

/// Rust str → NUL-terminated UTF-16 (tüm `*W` API'leri için).
pub fn wide_nul(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// `true` = geçerli handle.
pub fn valid(h: HANDLE) -> bool {
    !h.is_null() && h != INVALID_HANDLE_VALUE
}
