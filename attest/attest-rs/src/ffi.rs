#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::id::AttestAgent;
use crate::runtime::policy::PolicyEngine;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::tpm::create_identity_provider;
use crate::traits::HardwareIdentity;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static STRICT_HARDWARE: AtomicBool = AtomicBool::new(false);

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

#[no_mangle]
pub extern "C" fn attest_agent_new() -> *mut AttestAgent {
    let agent = AttestAgent::new();
    Box::into_raw(Box::new(agent))
}

#[no_mangle]
pub extern "C" fn attest_agent_free(ptr: *mut AttestAgent) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn attest_agent_get_id(ptr: *mut AttestAgent) -> *mut c_char {
    let agent = unsafe {
        assert!(!ptr.is_null());
        &*ptr
    };
    let c_str = CString::new(agent.id.clone()).unwrap();
    c_str.into_raw()
}

#[no_mangle]
pub extern "C" fn attest_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn attest_set_strict_hardware(strict: bool) {
    STRICT_HARDWARE.store(strict, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn attest_seal(data: *const u8, data_len: usize, out_len: *mut usize) -> *mut u8 {
    let result = std::panic::catch_unwind(|| {
        let rt = get_runtime();
        let strict = STRICT_HARDWARE.load(Ordering::SeqCst);
        let tpm: Box<dyn HardwareIdentity> = create_identity_provider(!strict);
        let input = unsafe { std::slice::from_raw_parts(data, data_len) };

        match rt.block_on(tpm.seal("default", input)) {
            Ok(sealed) => unsafe {
                *out_len = sealed.len();
                let mut buf = sealed.into_boxed_slice();
                let ptr = buf.as_mut_ptr();
                std::mem::forget(buf);
                ptr
            },
            Err(e) => {
                eprintln!("[FFI] seal error: {}", e);
                std::ptr::null_mut()
            }
        }
    });

    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            eprintln!("[FFI] attest_seal PANICKED");
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn attest_unseal(blob: *const u8, blob_len: usize, out_len: *mut usize) -> *mut u8 {
    let result = std::panic::catch_unwind(|| {
        let rt = get_runtime();
        let strict = STRICT_HARDWARE.load(Ordering::SeqCst);
        let tpm: Box<dyn HardwareIdentity> = create_identity_provider(!strict);
        let input = unsafe { std::slice::from_raw_parts(blob, blob_len) };

        match rt.block_on(tpm.unseal(input)) {
            Ok(unsealed) => unsafe {
                *out_len = unsealed.len();
                let mut buf = unsealed.into_boxed_slice();
                let ptr = buf.as_mut_ptr();
                std::mem::forget(buf);
                ptr
            },
            Err(e) => {
                eprintln!("[FFI] unseal error: {}", e);
                std::ptr::null_mut()
            }
        }
    });

    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            eprintln!("[FFI] attest_unseal PANICKED");
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn attest_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len));
        }
    }
}

#[no_mangle]
pub extern "C" fn attest_policy_engine_new() -> *mut PolicyEngine {
    let engine = PolicyEngine::new();
    Box::into_raw(Box::new(engine))
}

#[no_mangle]
pub extern "C" fn attest_policy_engine_free(ptr: *mut PolicyEngine) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn attest_policy_engine_load_defaults(ptr: *mut PolicyEngine) {
    let engine = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    engine.load_defaults();
}

#[no_mangle]
pub extern "C" fn attest_verify_intent(
    agent_ptr: *mut AttestAgent,
    policy_ptr: *mut PolicyEngine,
    intent_json: *const c_char,
) -> bool {
    let result = std::panic::catch_unwind(|| {
        let _agent = unsafe {
            assert!(!agent_ptr.is_null());
            &*agent_ptr
        };
        let policy = unsafe {
            assert!(!policy_ptr.is_null());
            &*policy_ptr
        };
        let c_str = unsafe {
            assert!(!intent_json.is_null());
            CStr::from_ptr(intent_json)
        };
        let intent_str = c_str.to_str().unwrap();
        let intent: crate::runtime::intent::Intent = serde_json::from_str(intent_str).unwrap();

        let ctx = crate::runtime::policy::ActionContext {
            action_type: "intent".into(),
            target: intent.goal.clone(),
            agent_id: _agent.id.clone(),
            intent_id: intent.id.clone(),
            ..Default::default()
        };

        let (allowed, _) = policy.should_allow(&ctx);
        allowed
    });

    match result {
        Ok(allowed) => allowed,
        Err(_) => {
            eprintln!("[FFI] attest_verify_intent PANICKED");
            false
        }
    }
}

// -----------------------------------------------------------------------------
// CHORA Client FFI
// -----------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn attest_chora_handshake(
    base_url: *const c_char,
    api_key: *const c_char,
    confidence: f64,
) -> *mut c_char {
    let result = std::panic::catch_unwind(|| {
        let rt = get_runtime();

        // Safely extract C strings
        let url_str = unsafe {
            if base_url.is_null() {
                return std::ptr::null_mut();
            }
            CStr::from_ptr(base_url).to_string_lossy().into_owned()
        };
        let key_str = unsafe {
            if api_key.is_null() {
                return std::ptr::null_mut();
            }
            CStr::from_ptr(api_key).to_string_lossy().into_owned()
        };

        let client = crate::cloud::chora::ChoraClient::new(url_str, key_str);

        match rt.block_on(client.handshake(confidence)) {
            Ok(json_val) => {
                let json_str = json_val.to_string();
                CString::new(json_str).unwrap().into_raw()
            }
            Err(e) => {
                eprintln!("[FFI] CHORA handshake error: {}", e);
                std::ptr::null_mut()
            }
        }
    });

    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            eprintln!("[FFI] attest_chora_handshake PANICKED");
            std::ptr::null_mut()
        }
    }
}
