// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// FFI: C-compatible foreign function interface for JanusKey
// Allows Ada/SPARK TUI and other language bindings to use JanusKey

use crate::{JanusKey, JanusError};
use crate::content_store::ContentHash;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint};
use std::path::PathBuf;
use std::ptr;

/// Opaque handle to JanusKey instance
pub struct JanusKeyHandle {
    inner: JanusKey,
}

/// Result codes for FFI functions
#[repr(C)]
pub enum JkResult {
    Ok = 0,
    ErrNotInitialized = 1,
    ErrIoError = 2,
    ErrNotFound = 3,
    ErrInvalidPath = 4,
    ErrOperationFailed = 5,
    ErrNullPointer = 6,
    ErrInvalidUtf8 = 7,
}

impl From<&JanusError> for JkResult {
    fn from(err: &JanusError) -> Self {
        match err {
            JanusError::NotInitialized(_) => JkResult::ErrNotInitialized,
            JanusError::IoError(_) | JanusError::Io(_) => JkResult::ErrIoError,
            JanusError::FileNotFound(_) | JanusError::DirectoryNotFound(_) => JkResult::ErrNotFound,
            JanusError::PathExists(_) => JkResult::ErrInvalidPath,
            _ => JkResult::ErrOperationFailed,
        }
    }
}

/// Operation type for history entries
#[repr(C)]
pub enum JkOpType {
    Create = 0,
    Delete = 1,
    Modify = 2,
    Move = 3,
    Copy = 4,
    Chmod = 5,
    Chown = 6,
    Mkdir = 7,
    Rmdir = 8,
    Symlink = 9,
    Append = 10,
    Truncate = 11,
    Touch = 12,
}

/// History entry structure (C-compatible)
#[repr(C)]
pub struct JkHistoryEntry {
    pub id: *mut c_char,
    pub timestamp: i64,
    pub op_type: JkOpType,
    pub path: *mut c_char,
    pub reversible: c_int,
}

/// Status structure (C-compatible)
#[repr(C)]
pub struct JkStatus {
    pub initialized: c_int,
    pub total_operations: c_uint,
    pub reversible_operations: c_uint,
    pub obliterated_count: c_uint,
    pub storage_bytes: u64,
}

// ===========================================================================
// Initialization Functions
// ===========================================================================

/// Initialize JanusKey in a directory
/// Returns handle on success, null on failure
#[no_mangle]
pub extern "C" fn jk_init(path: *const c_char) -> *mut JanusKeyHandle {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match JanusKey::init(&PathBuf::from(path_str)) {
        Ok(jk) => Box::into_raw(Box::new(JanusKeyHandle { inner: jk })),
        Err(_) => ptr::null_mut(),
    }
}

/// Open existing JanusKey directory
/// Returns handle on success, null on failure
#[no_mangle]
pub extern "C" fn jk_open(path: *const c_char) -> *mut JanusKeyHandle {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match JanusKey::open(&PathBuf::from(path_str)) {
        Ok(jk) => Box::into_raw(Box::new(JanusKeyHandle { inner: jk })),
        Err(_) => ptr::null_mut(),
    }
}

/// Close JanusKey handle and free memory
#[no_mangle]
pub extern "C" fn jk_close(handle: *mut JanusKeyHandle) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Check if directory is initialized
#[no_mangle]
pub extern "C" fn jk_is_initialized(path: *const c_char) -> c_int {
    if path.is_null() {
        return 0;
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if JanusKey::is_initialized(&PathBuf::from(path_str)) {
        1
    } else {
        0
    }
}

// ===========================================================================
// Status Functions
// ===========================================================================

/// Get JanusKey status
#[no_mangle]
pub extern "C" fn jk_status(handle: *const JanusKeyHandle, status: *mut JkStatus) -> JkResult {
    if handle.is_null() || status.is_null() {
        return JkResult::ErrNullPointer;
    }

    let jk = unsafe { &(*handle).inner };

    // Get operation counts using correct API
    let ops = jk.metadata_store.operations();
    let total = ops.len();
    let reversible = ops.iter().filter(|op| !op.undone).count();

    let obliterated = jk.obliteration_manager.count();

    // Calculate storage size
    let storage_bytes = if let Ok(entries) = std::fs::read_dir(&jk.root.join(".januskey").join("content")) {
        entries.filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    } else {
        0u64
    };

    unsafe {
        (*status).initialized = 1;
        (*status).total_operations = total as c_uint;
        (*status).reversible_operations = reversible as c_uint;
        (*status).obliterated_count = obliterated as c_uint;
        (*status).storage_bytes = storage_bytes;
    }

    JkResult::Ok
}

// ===========================================================================
// Operation Functions
// ===========================================================================

/// Undo the last operation
#[no_mangle]
pub extern "C" fn jk_undo_last(handle: *mut JanusKeyHandle) -> JkResult {
    if handle.is_null() {
        return JkResult::ErrNullPointer;
    }

    let jk = unsafe { &mut (*handle).inner };

    // Find last undoable operation using correct API
    let last_op = match jk.metadata_store.last_undoable() {
        Some(op) => op.clone(),
        None => return JkResult::ErrNotFound,
    };

    // Execute undo operation using correct API
    let mut executor = crate::OperationExecutor::new(
        &jk.content_store,
        &mut jk.metadata_store,
    );

    match executor.undo(&last_op.id) {
        Ok(_) => JkResult::Ok,
        Err(e) => JkResult::from(&e),
    }
}

/// Undo operation by ID
#[no_mangle]
pub extern "C" fn jk_undo_by_id(
    handle: *mut JanusKeyHandle,
    op_id: *const c_char,
) -> JkResult {
    if handle.is_null() || op_id.is_null() {
        return JkResult::ErrNullPointer;
    }

    let id_str = match unsafe { CStr::from_ptr(op_id) }.to_str() {
        Ok(s) => s,
        Err(_) => return JkResult::ErrInvalidUtf8,
    };

    let jk = unsafe { &mut (*handle).inner };
    let mut executor = crate::OperationExecutor::new(
        &jk.content_store,
        &mut jk.metadata_store,
    );

    match executor.undo(id_str) {
        Ok(_) => JkResult::Ok,
        Err(e) => JkResult::from(&e),
    }
}

/// Obliterate content by hash (RMO primitive)
#[no_mangle]
pub extern "C" fn jk_obliterate(
    handle: *mut JanusKeyHandle,
    hash: *const c_char,
    reason: *const c_char,
) -> JkResult {
    if handle.is_null() || hash.is_null() {
        return JkResult::ErrNullPointer;
    }

    let hash_str = match unsafe { CStr::from_ptr(hash) }.to_str() {
        Ok(s) => s,
        Err(_) => return JkResult::ErrInvalidUtf8,
    };

    let reason_opt = if reason.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(reason) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => return JkResult::ErrInvalidUtf8,
        }
    };

    let jk = unsafe { &mut (*handle).inner };

    // Create ContentHash from hex string
    let content_hash = ContentHash::from_hex(hash_str);

    // Use correct obliterate API
    match jk.obliteration_manager.obliterate(
        &jk.content_store,
        &content_hash,
        reason_opt,
        Some("GDPR Article 17".to_string()),
    ) {
        Ok(_) => JkResult::Ok,
        Err(e) => JkResult::from(&e),
    }
}

// ===========================================================================
// History Functions
// ===========================================================================

/// Get the number of history entries
#[no_mangle]
pub extern "C" fn jk_history_count(handle: *const JanusKeyHandle) -> c_uint {
    if handle.is_null() {
        return 0;
    }

    let jk = unsafe { &(*handle).inner };
    jk.metadata_store.operations().len() as c_uint
}

/// Get history entry at index
/// Caller must free returned strings with jk_free_string
#[no_mangle]
pub extern "C" fn jk_history_get(
    handle: *const JanusKeyHandle,
    index: c_uint,
    entry: *mut JkHistoryEntry,
) -> JkResult {
    if handle.is_null() || entry.is_null() {
        return JkResult::ErrNullPointer;
    }

    let jk = unsafe { &(*handle).inner };
    let ops = jk.metadata_store.operations();

    let idx = index as usize;
    if idx >= ops.len() {
        return JkResult::ErrNotFound;
    }

    let op = &ops[idx];

    // Allocate C strings
    let id_cstring = match CString::new(op.id.clone()) {
        Ok(s) => s,
        Err(_) => return JkResult::ErrInvalidUtf8,
    };
    let path_cstring = match CString::new(op.path.display().to_string()) {
        Ok(s) => s,
        Err(_) => return JkResult::ErrInvalidUtf8,
    };

    // Map operation type to FFI enum
    let op_type = match op.op_type {
        crate::OperationType::Create => JkOpType::Create,
        crate::OperationType::Delete => JkOpType::Delete,
        crate::OperationType::Modify => JkOpType::Modify,
        crate::OperationType::Move => JkOpType::Move,
        crate::OperationType::Copy => JkOpType::Copy,
        crate::OperationType::Chmod => JkOpType::Chmod,
        crate::OperationType::Chown => JkOpType::Chown,
        crate::OperationType::Mkdir => JkOpType::Mkdir,
        crate::OperationType::Rmdir => JkOpType::Rmdir,
        crate::OperationType::Symlink => JkOpType::Symlink,
        crate::OperationType::Append => JkOpType::Append,
        crate::OperationType::Truncate => JkOpType::Truncate,
        crate::OperationType::Touch => JkOpType::Touch,
    };

    let reversible = !op.undone;

    unsafe {
        (*entry).id = id_cstring.into_raw();
        (*entry).timestamp = op.timestamp.timestamp();
        (*entry).op_type = op_type;
        (*entry).path = path_cstring.into_raw();
        (*entry).reversible = if reversible { 1 } else { 0 };
    }

    JkResult::Ok
}

// ===========================================================================
// Memory Management
// ===========================================================================

/// Free a C string allocated by JanusKey FFI
#[no_mangle]
pub extern "C" fn jk_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Free a history entry's allocated strings
#[no_mangle]
pub extern "C" fn jk_free_history_entry(entry: *mut JkHistoryEntry) {
    if !entry.is_null() {
        unsafe {
            if !(*entry).id.is_null() {
                jk_free_string((*entry).id);
            }
            if !(*entry).path.is_null() {
                jk_free_string((*entry).path);
            }
        }
    }
}

// ===========================================================================
// Version Info
// ===========================================================================

/// Get JanusKey version string
/// Returns statically allocated string, do not free
#[no_mangle]
pub extern "C" fn jk_version() -> *const c_char {
    static VERSION: &[u8] = b"1.0.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Get JanusKey library name
/// Returns statically allocated string, do not free
#[no_mangle]
pub extern "C" fn jk_name() -> *const c_char {
    static NAME: &[u8] = b"JanusKey\0";
    NAME.as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use tempfile::TempDir;

    #[test]
    fn test_version() {
        let version = jk_version();
        let version_str = unsafe { CStr::from_ptr(version) }.to_str().unwrap();
        assert_eq!(version_str, "1.0.0");
    }

    #[test]
    fn test_init_and_open() {
        let tmp = TempDir::new().unwrap();
        let path = CString::new(tmp.path().to_str().unwrap()).unwrap();

        // Initialize
        let handle = jk_init(path.as_ptr());
        assert!(!handle.is_null());
        jk_close(handle);

        // Check initialized
        assert_eq!(jk_is_initialized(path.as_ptr()), 1);

        // Open
        let handle2 = jk_open(path.as_ptr());
        assert!(!handle2.is_null());
        jk_close(handle2);
    }

    #[test]
    fn test_status() {
        let tmp = TempDir::new().unwrap();
        let path = CString::new(tmp.path().to_str().unwrap()).unwrap();

        let handle = jk_init(path.as_ptr());
        assert!(!handle.is_null());

        let mut status = JkStatus {
            initialized: 0,
            total_operations: 0,
            reversible_operations: 0,
            obliterated_count: 0,
            storage_bytes: 0,
        };

        let result = jk_status(handle, &mut status);
        assert!(matches!(result, JkResult::Ok));
        assert_eq!(status.initialized, 1);

        jk_close(handle);
    }

    #[test]
    fn test_null_handling() {
        assert!(jk_init(ptr::null()).is_null());
        assert!(jk_open(ptr::null()).is_null());
        assert_eq!(jk_is_initialized(ptr::null()), 0);
        jk_close(ptr::null_mut()); // Should not crash
    }
}
