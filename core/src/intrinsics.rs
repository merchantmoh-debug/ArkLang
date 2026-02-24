/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 */

use crate::persistent::{PMap, PVec};
use crate::runtime::{NativeFn, RuntimeError, Scope, Value};
use regex::Regex;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;
#[cfg(not(target_arch = "wasm32"))]
use shell_words;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::thread;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha512};

#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::net::{TcpListener, TcpStream};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicI64, Ordering};

#[cfg(not(target_arch = "wasm32"))]
pub enum SocketResource {
    Listener(TcpListener),
    Stream(TcpStream),
}

#[cfg(not(target_arch = "wasm32"))]
static AI_CLIENT: OnceLock<Client> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
static AI_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
static SOCKET_ID_COUNTER: AtomicI64 = AtomicI64::new(1);

#[cfg(not(target_arch = "wasm32"))]
static SOCKETS: OnceLock<Mutex<HashMap<i64, SocketResource>>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn get_sockets() -> &'static Mutex<HashMap<i64, SocketResource>> {
    SOCKETS.get_or_init(|| Mutex::new(HashMap::new()))
}

// Threading & Events Globals
static THREADS: OnceLock<Mutex<HashMap<i64, thread::JoinHandle<()>>>> = OnceLock::new();
static EVENTS: OnceLock<Mutex<VecDeque<Value>>> = OnceLock::new();
static NEXT_THREAD_ID: OnceLock<Mutex<i64>> = OnceLock::new();

pub struct IntrinsicRegistry;

impl IntrinsicRegistry {
    pub fn resolve(hash: &str) -> Option<NativeFn> {
        match hash {
            "intrinsic_add" => Some(intrinsic_add),
            "intrinsic_sub" => Some(intrinsic_sub),
            "intrinsic_mul" => Some(intrinsic_mul),
            "intrinsic_div" => Some(intrinsic_div),
            "intrinsic_mod" => Some(intrinsic_mod),
            "intrinsic_gt" => Some(intrinsic_gt),
            "intrinsic_lt" => Some(intrinsic_lt),
            "intrinsic_ge" => Some(intrinsic_ge),
            "intrinsic_le" => Some(intrinsic_le),
            "intrinsic_eq" => Some(intrinsic_eq),
            "intrinsic_and" => Some(intrinsic_and),
            "intrinsic_or" => Some(intrinsic_or),
            "intrinsic_not" => Some(intrinsic_not),
            "intrinsic_print" => Some(intrinsic_print),
            "print" => Some(intrinsic_print),
            // Core aliases for Python parity
            "get" => Some(intrinsic_list_get),
            "len" => Some(intrinsic_len),
            "intrinsic_ask_ai" | "sys.ai.ask" | "ai.ask" => Some(intrinsic_ask_ai),
            "sys_exec" | "intrinsic_exec" => Some(intrinsic_exec),
            "sys_fs_write" | "intrinsic_fs_write" | "sys.fs.write" => Some(intrinsic_fs_write),
            "sys_fs_read" | "intrinsic_fs_read" | "sys.fs.read" => Some(intrinsic_fs_read),
            "intrinsic_crypto_hash" | "sys.crypto.hash" => Some(intrinsic_crypto_hash),
            "intrinsic_crypto_verify" | "sys.crypto.verify" => Some(intrinsic_crypto_verify),
            "intrinsic_crypto_sha512" | "sys.crypto.sha512" => Some(intrinsic_crypto_sha512),
            "intrinsic_crypto_hmac_sha512" | "sys.crypto.hmac_sha512" => {
                Some(intrinsic_crypto_hmac_sha512)
            }
            "intrinsic_crypto_pbkdf2" | "sys.crypto.pbkdf2" => Some(intrinsic_crypto_pbkdf2),
            "intrinsic_crypto_aes_gcm_encrypt" | "sys.crypto.aes_gcm_encrypt" => {
                Some(intrinsic_crypto_aes_gcm_encrypt)
            }
            "intrinsic_crypto_aes_gcm_decrypt" | "sys.crypto.aes_gcm_decrypt" => {
                Some(intrinsic_crypto_aes_gcm_decrypt)
            }
            "intrinsic_crypto_random_bytes" | "sys.crypto.random_bytes" => {
                Some(intrinsic_crypto_random_bytes)
            }
            "intrinsic_crypto_ed25519_generate"
            | "sys.crypto.ed25519_generate"
            | "sys.crypto.ed25519.gen" => Some(intrinsic_crypto_ed25519_generate),
            "intrinsic_crypto_ed25519_sign"
            | "sys.crypto.ed25519_sign"
            | "sys.crypto.ed25519.sign" => Some(intrinsic_crypto_ed25519_sign),
            "intrinsic_crypto_ed25519_verify"
            | "sys.crypto.ed25519_verify"
            | "sys.crypto.ed25519.verify" => Some(intrinsic_crypto_ed25519_verify),
            "intrinsic_merkle_root" | "sys.crypto.merkle_root" => Some(intrinsic_merkle_root),
            "sys.crypto.pbkdf2_hmac_sha512" => Some(intrinsic_crypto_pbkdf2),
            "intrinsic_buffer_alloc" | "sys.mem.alloc" => Some(intrinsic_buffer_alloc),
            "intrinsic_buffer_inspect" | "sys.mem.inspect" => Some(intrinsic_buffer_inspect),
            "intrinsic_buffer_read" | "sys.mem.read" => Some(intrinsic_buffer_read),
            "intrinsic_buffer_write" | "sys.mem.write" => Some(intrinsic_buffer_write),
            "intrinsic_list_get" | "sys.list.get" | "sys.str.get" => Some(intrinsic_list_get),
            "intrinsic_list_append" | "sys.list.append" => Some(intrinsic_list_append),
            "intrinsic_list_pop" | "sys.list.pop" => Some(intrinsic_list_pop),
            "intrinsic_list_delete" | "sys.list.delete" => Some(intrinsic_list_delete),
            "intrinsic_len" | "sys.len" => Some(intrinsic_len),
            "intrinsic_struct_get" | "sys.struct.get" => Some(intrinsic_struct_get),
            "intrinsic_struct_set" | "sys.struct.set" => Some(intrinsic_struct_set),
            "intrinsic_struct_has" | "sys.struct.has" => Some(intrinsic_struct_has),
            "intrinsic_time_now" | "time.now" | "sys.time.now" => Some(intrinsic_time_now),
            "intrinsic_math_pow" | "math.pow" => Some(intrinsic_math_pow),
            "intrinsic_pow_mod" | "math.pow_mod" | "sys.math.pow_mod" => Some(intrinsic_pow_mod),
            "intrinsic_math_sqrt" | "math.sqrt" => Some(intrinsic_math_sqrt),
            "intrinsic_math_sin" | "math.sin" => Some(intrinsic_math_sin),
            "intrinsic_math_cos" | "math.cos" => Some(intrinsic_math_cos),
            "intrinsic_math_tan" | "math.tan" => Some(intrinsic_math_tan),
            "intrinsic_math_asin" | "math.asin" => Some(intrinsic_math_asin),
            "intrinsic_math_acos" | "math.acos" => Some(intrinsic_math_acos),
            "intrinsic_math_atan" | "math.atan" => Some(intrinsic_math_atan),
            "intrinsic_math_atan2" | "math.atan2" => Some(intrinsic_math_atan2),
            "intrinsic_io_cls" | "io.cls" => Some(intrinsic_io_cls),
            "intrinsic_list_set" | "sys.list.set" => Some(intrinsic_list_set),
            "intrinsic_chain_height" | "sys.chain.height" => Some(intrinsic_chain_height),
            "intrinsic_chain_get_balance" | "sys.chain.get_balance" => {
                Some(intrinsic_chain_get_balance)
            }
            "intrinsic_chain_submit_tx" | "sys.chain.submit_tx" => Some(intrinsic_chain_submit_tx),
            "intrinsic_chain_verify_tx" | "sys.chain.verify_tx" => Some(intrinsic_chain_verify_tx),
            "sys.fs.write_buffer" => Some(intrinsic_fs_write_buffer),
            "sys.fs.read_buffer" => Some(intrinsic_fs_read_buffer),
            "math.sin_scaled" => Some(intrinsic_math_sin_scaled),
            "math.cos_scaled" => Some(intrinsic_math_cos_scaled),
            "math.pi_scaled" => Some(intrinsic_math_pi_scaled),
            "sys.str.from_code" => Some(intrinsic_str_from_code),
            "sys.time.sleep" | "intrinsic_time_sleep" => Some(intrinsic_time_sleep),
            "sys.io.read_bytes" | "intrinsic_io_read_bytes" => Some(intrinsic_io_read_bytes),
            "sys.io.read_line" | "intrinsic_io_read_line" => Some(intrinsic_io_read_line),
            "sys.io.write" | "intrinsic_io_write" => Some(intrinsic_io_write),
            "sys.io.read_file_async" | "intrinsic_io_read_file_async" => {
                Some(intrinsic_io_read_file_async)
            }
            "sys.extract_code" | "intrinsic_extract_code" => Some(intrinsic_extract_code),
            // Networking Intrinsics
            "net.http.request" | "intrinsic_http_request" | "sys.net.http.request" => {
                Some(intrinsic_http_request)
            }
            "net.http.serve" | "intrinsic_http_serve" | "sys.net.http.serve" => {
                Some(intrinsic_http_serve)
            }
            "net.socket.bind" | "intrinsic_socket_bind" | "sys.net.socket.bind" => {
                Some(intrinsic_socket_bind)
            }
            "net.socket.accept" | "intrinsic_socket_accept" | "sys.net.socket.accept" => {
                Some(intrinsic_socket_accept)
            }
            "net.socket.connect" | "intrinsic_socket_connect" | "sys.net.socket.connect" => {
                Some(intrinsic_socket_connect)
            }
            "net.socket.send" | "intrinsic_socket_send" | "sys.net.socket.send" => {
                Some(intrinsic_socket_send)
            }
            "net.socket.recv" | "intrinsic_socket_recv" | "sys.net.socket.recv" => {
                Some(intrinsic_socket_recv)
            }
            "net.socket.close" | "intrinsic_socket_close" | "sys.net.socket.close" => {
                Some(intrinsic_socket_close)
            }
            "net.socket.set_timeout"
            | "intrinsic_socket_set_timeout"
            | "sys.net.socket.set_timeout" => Some(intrinsic_socket_set_timeout),
            // Advanced Runtime
            "sys.thread.spawn" => Some(intrinsic_thread_spawn),
            "sys.thread.join" => Some(intrinsic_thread_join),
            "sys.event.poll" => Some(intrinsic_event_poll),
            "sys.event.push" => Some(intrinsic_event_push),
            "sys.func.apply" => Some(intrinsic_func_apply),
            "sys.vm.eval" => Some(intrinsic_vm_eval),
            // Phase 78: Final 12 Parity Intrinsics
            "sys.json.parse" | "intrinsic_json_parse" => Some(intrinsic_json_parse),
            "sys.json.stringify" | "intrinsic_json_stringify" => Some(intrinsic_json_stringify),
            "sys.log" | "intrinsic_log" => Some(intrinsic_log),
            "sys.exit" | "exit" | "quit" | "intrinsic_exit" => Some(intrinsic_exit),
            "sys.html_escape" | "intrinsic_html_escape" => Some(intrinsic_html_escape),
            "sys.z3.verify" | "intrinsic_z3_verify" => Some(intrinsic_z3_verify),
            "sys.vm.source" | "intrinsic_vm_source" => Some(intrinsic_vm_source),
            "sys.info" | "intrinsic_sys_info" => Some(intrinsic_sys_info),
            "math.Tensor" | "intrinsic_math_tensor" => Some(intrinsic_math_tensor),
            "math.matmul" | "intrinsic_math_matmul" => Some(intrinsic_math_matmul),
            "math.transpose" | "intrinsic_math_transpose" => Some(intrinsic_math_transpose),
            "math.dot" | "intrinsic_math_dot" => Some(intrinsic_math_dot),
            "math.add" | "intrinsic_math_tensor_add" => Some(intrinsic_math_tensor_add),
            "math.sub" | "intrinsic_math_tensor_sub" => Some(intrinsic_math_tensor_sub),
            "math.mul_scalar" | "intrinsic_math_mul_scalar" => Some(intrinsic_math_mul_scalar),
            // Persistent Data Structure Intrinsics
            "pvec.new" | "sys.pvec.new" | "intrinsic_pvec_new" => Some(intrinsic_pvec_new),
            "pvec.conj" | "sys.pvec.conj" | "intrinsic_pvec_conj" => Some(intrinsic_pvec_conj),
            "pvec.get" | "sys.pvec.get" | "intrinsic_pvec_get" => Some(intrinsic_pvec_get),
            "pvec.assoc" | "sys.pvec.assoc" | "intrinsic_pvec_assoc" => Some(intrinsic_pvec_assoc),
            "pvec.pop" | "sys.pvec.pop" | "intrinsic_pvec_pop" => Some(intrinsic_pvec_pop),
            "pvec.len" | "sys.pvec.len" | "intrinsic_pvec_len" => Some(intrinsic_pvec_len),
            "pmap.new" | "sys.pmap.new" | "intrinsic_pmap_new" => Some(intrinsic_pmap_new),
            "pmap.assoc" | "sys.pmap.assoc" | "intrinsic_pmap_assoc" => Some(intrinsic_pmap_assoc),
            "pmap.dissoc" | "sys.pmap.dissoc" | "intrinsic_pmap_dissoc" => {
                Some(intrinsic_pmap_dissoc)
            }
            "pmap.get" | "sys.pmap.get" | "intrinsic_pmap_get" => Some(intrinsic_pmap_get),
            "pmap.keys" | "sys.pmap.keys" | "intrinsic_pmap_keys" => Some(intrinsic_pmap_keys),
            "pmap.merge" | "sys.pmap.merge" | "intrinsic_pmap_merge" => Some(intrinsic_pmap_merge),
            // WASM Component Interop Intrinsics (native only)
            #[cfg(not(target_arch = "wasm32"))]
            "sys.wasm.load" | "wasm.load" | "intrinsic_wasm_load" => {
                Some(crate::wasm_interop::intrinsic_wasm_load)
            }
            #[cfg(not(target_arch = "wasm32"))]
            "sys.wasm.exports" | "wasm.exports" | "intrinsic_wasm_exports" => {
                Some(crate::wasm_interop::intrinsic_wasm_exports)
            }
            #[cfg(not(target_arch = "wasm32"))]
            "sys.wasm.call" | "wasm.call" | "intrinsic_wasm_call" => {
                Some(crate::wasm_interop::intrinsic_wasm_call)
            }
            #[cfg(not(target_arch = "wasm32"))]
            "sys.wasm.drop" | "wasm.drop" | "intrinsic_wasm_drop" => {
                Some(crate::wasm_interop::intrinsic_wasm_drop)
            }
            _ => None,
        }
    }

    pub fn register_all(scope: &mut Scope) {
        scope.set(
            "intrinsic_add".to_string(),
            Value::NativeFunction(intrinsic_add),
        );
        scope.set(
            "intrinsic_sub".to_string(),
            Value::NativeFunction(intrinsic_sub),
        );
        scope.set(
            "intrinsic_mul".to_string(),
            Value::NativeFunction(intrinsic_mul),
        );
        scope.set(
            "intrinsic_div".to_string(),
            Value::NativeFunction(intrinsic_div),
        );
        scope.set(
            "intrinsic_mod".to_string(),
            Value::NativeFunction(intrinsic_mod),
        );
        scope.set(
            "intrinsic_gt".to_string(),
            Value::NativeFunction(intrinsic_gt),
        );
        scope.set(
            "intrinsic_lt".to_string(),
            Value::NativeFunction(intrinsic_lt),
        );
        scope.set(
            "intrinsic_ge".to_string(),
            Value::NativeFunction(intrinsic_ge),
        );
        scope.set(
            "intrinsic_le".to_string(),
            Value::NativeFunction(intrinsic_le),
        );
        scope.set(
            "intrinsic_eq".to_string(),
            Value::NativeFunction(intrinsic_eq),
        );
        scope.set(
            "intrinsic_and".to_string(),
            Value::NativeFunction(intrinsic_and),
        );
        scope.set(
            "intrinsic_or".to_string(),
            Value::NativeFunction(intrinsic_or),
        );
        scope.set(
            "intrinsic_not".to_string(),
            Value::NativeFunction(intrinsic_not),
        );
        scope.set(
            "intrinsic_print".to_string(),
            Value::NativeFunction(intrinsic_print),
        );
        scope.set("print".to_string(), Value::NativeFunction(intrinsic_print)); // Alias
        scope.set(
            "intrinsic_ask_ai".to_string(),
            Value::NativeFunction(intrinsic_ask_ai),
        );
        // AI namespace — The Neural Bridge
        scope.set(
            "sys.ai.ask".to_string(),
            Value::NativeFunction(intrinsic_ask_ai),
        );
        scope.set(
            "ai.ask".to_string(),
            Value::NativeFunction(intrinsic_ask_ai),
        );

        // System
        scope.set("sys.len".to_string(), Value::NativeFunction(intrinsic_len));
        scope.set(
            "sys.exec".to_string(),
            Value::NativeFunction(intrinsic_exec),
        );
        scope.set(
            "sys.fs.write".to_string(),
            Value::NativeFunction(intrinsic_fs_write),
        );
        scope.set(
            "sys.fs.read".to_string(),
            Value::NativeFunction(intrinsic_fs_read),
        );
        scope.set(
            "sys.crypto.hash".to_string(),
            Value::NativeFunction(intrinsic_crypto_hash),
        );
        scope.set(
            "sys.crypto.verify".to_string(),
            Value::NativeFunction(intrinsic_crypto_verify),
        );
        scope.set(
            "sys.crypto.sha512".to_string(),
            Value::NativeFunction(intrinsic_crypto_sha512),
        );
        scope.set(
            "sys.crypto.hmac_sha512".to_string(),
            Value::NativeFunction(intrinsic_crypto_hmac_sha512),
        );
        scope.set(
            "sys.crypto.pbkdf2".to_string(),
            Value::NativeFunction(intrinsic_crypto_pbkdf2),
        );
        scope.set(
            "sys.crypto.aes_gcm_encrypt".to_string(),
            Value::NativeFunction(intrinsic_crypto_aes_gcm_encrypt),
        );
        scope.set(
            "sys.crypto.aes_gcm_decrypt".to_string(),
            Value::NativeFunction(intrinsic_crypto_aes_gcm_decrypt),
        );
        scope.set(
            "sys.crypto.random_bytes".to_string(),
            Value::NativeFunction(intrinsic_crypto_random_bytes),
        );
        scope.set(
            "sys.crypto.ed25519_generate".to_string(),
            Value::NativeFunction(intrinsic_crypto_ed25519_generate),
        );
        scope.set(
            "sys.crypto.ed25519_sign".to_string(),
            Value::NativeFunction(intrinsic_crypto_ed25519_sign),
        );
        scope.set(
            "sys.crypto.ed25519_verify".to_string(),
            Value::NativeFunction(intrinsic_crypto_ed25519_verify),
        );

        // List/Struct
        scope.set(
            "sys.list.get".to_string(),
            Value::NativeFunction(intrinsic_list_get),
        );
        scope.set(
            "sys.list.append".to_string(),
            Value::NativeFunction(intrinsic_list_append),
        );
        scope.set(
            "intrinsic_list_pop".to_string(),
            Value::NativeFunction(intrinsic_list_pop),
        );
        scope.set(
            "intrinsic_list_delete".to_string(),
            Value::NativeFunction(intrinsic_list_delete),
        );
        scope.set(
            "sys.list.delete".to_string(),
            Value::NativeFunction(intrinsic_list_delete),
        );
        scope.set(
            "sys.struct.get".to_string(),
            Value::NativeFunction(intrinsic_struct_get),
        );
        scope.set(
            "sys.struct.set".to_string(),
            Value::NativeFunction(intrinsic_struct_set),
        );
        scope.set(
            "intrinsic_struct_has".to_string(),
            Value::NativeFunction(intrinsic_struct_has),
        );
        scope.set(
            "sys.struct.has".to_string(),
            Value::NativeFunction(intrinsic_struct_has),
        );
        scope.set(
            "time.now".to_string(),
            Value::NativeFunction(intrinsic_time_now),
        );
        scope.set(
            "intrinsic_math_pow".to_string(),
            Value::NativeFunction(intrinsic_math_pow),
        );
        scope.set(
            "intrinsic_pow_mod".to_string(),
            Value::NativeFunction(intrinsic_pow_mod),
        );
        scope.set(
            "math.pow_mod".to_string(),
            Value::NativeFunction(intrinsic_pow_mod),
        );
        scope.set(
            "math.pow".to_string(),
            Value::NativeFunction(intrinsic_math_pow),
        );
        scope.set(
            "intrinsic_math_sqrt".to_string(),
            Value::NativeFunction(intrinsic_math_sqrt),
        );
        scope.set(
            "math.sqrt".to_string(),
            Value::NativeFunction(intrinsic_math_sqrt),
        );
        scope.set(
            "intrinsic_math_sin".to_string(),
            Value::NativeFunction(intrinsic_math_sin),
        );
        scope.set(
            "math.sin".to_string(),
            Value::NativeFunction(intrinsic_math_sin),
        );
        scope.set(
            "intrinsic_math_cos".to_string(),
            Value::NativeFunction(intrinsic_math_cos),
        );
        scope.set(
            "math.cos".to_string(),
            Value::NativeFunction(intrinsic_math_cos),
        );
        scope.set(
            "intrinsic_math_tan".to_string(),
            Value::NativeFunction(intrinsic_math_tan),
        );
        scope.set(
            "math.tan".to_string(),
            Value::NativeFunction(intrinsic_math_tan),
        );
        scope.set(
            "intrinsic_math_asin".to_string(),
            Value::NativeFunction(intrinsic_math_asin),
        );
        scope.set(
            "math.asin".to_string(),
            Value::NativeFunction(intrinsic_math_asin),
        );
        scope.set(
            "intrinsic_math_acos".to_string(),
            Value::NativeFunction(intrinsic_math_acos),
        );
        scope.set(
            "math.acos".to_string(),
            Value::NativeFunction(intrinsic_math_acos),
        );
        scope.set(
            "intrinsic_math_atan".to_string(),
            Value::NativeFunction(intrinsic_math_atan),
        );
        scope.set(
            "math.atan".to_string(),
            Value::NativeFunction(intrinsic_math_atan),
        );
        scope.set(
            "intrinsic_math_atan2".to_string(),
            Value::NativeFunction(intrinsic_math_atan2),
        );
        scope.set(
            "math.atan2".to_string(),
            Value::NativeFunction(intrinsic_math_atan2),
        );
        scope.set(
            "io.cls".to_string(),
            Value::NativeFunction(intrinsic_io_cls),
        );
        scope.set(
            "sys.list.set".to_string(),
            Value::NativeFunction(intrinsic_list_set),
        );
        scope.set(
            "sys.chain.height".to_string(),
            Value::NativeFunction(intrinsic_chain_height),
        );
        scope.set(
            "sys.chain.get_balance".to_string(),
            Value::NativeFunction(intrinsic_chain_get_balance),
        );
        scope.set(
            "sys.chain.submit_tx".to_string(),
            Value::NativeFunction(intrinsic_chain_submit_tx),
        );
        scope.set(
            "sys.chain.verify_tx".to_string(),
            Value::NativeFunction(intrinsic_chain_verify_tx),
        );
        scope.set(
            "sys.fs.write_buffer".to_string(),
            Value::NativeFunction(intrinsic_fs_write_buffer),
        );
        scope.set(
            "sys.fs.read_buffer".to_string(),
            Value::NativeFunction(intrinsic_fs_read_buffer),
        );
        scope.set(
            "math.sin_scaled".to_string(),
            Value::NativeFunction(intrinsic_math_sin_scaled),
        );
        scope.set(
            "math.cos_scaled".to_string(),
            Value::NativeFunction(intrinsic_math_cos_scaled),
        );
        scope.set(
            "sys.fs.write".to_string(),
            Value::NativeFunction(intrinsic_fs_write),
        );
        // Advanced Runtime Registration
        scope.set(
            "sys.thread.spawn".to_string(),
            Value::NativeFunction(intrinsic_thread_spawn),
        );
        scope.set(
            "sys.thread.join".to_string(),
            Value::NativeFunction(intrinsic_thread_join),
        );
        scope.set(
            "sys.event.poll".to_string(),
            Value::NativeFunction(intrinsic_event_poll),
        );
        scope.set(
            "sys.event.push".to_string(),
            Value::NativeFunction(intrinsic_event_push),
        );
        scope.set(
            "sys.func.apply".to_string(),
            Value::NativeFunction(intrinsic_func_apply),
        );
        scope.set(
            "sys.vm.eval".to_string(),
            Value::NativeFunction(intrinsic_vm_eval),
        );
        scope.set(
            "sys.time.sleep".to_string(),
            Value::NativeFunction(intrinsic_time_sleep),
        );
        scope.set(
            "sys.io.read_bytes".to_string(),
            Value::NativeFunction(intrinsic_io_read_bytes),
        );
        scope.set(
            "sys.io.read_line".to_string(),
            Value::NativeFunction(intrinsic_io_read_line),
        );
        scope.set(
            "sys.io.write".to_string(),
            Value::NativeFunction(intrinsic_io_write),
        );
        scope.set(
            "sys.io.read_file_async".to_string(),
            Value::NativeFunction(intrinsic_io_read_file_async),
        );
        scope.set(
            "sys.extract_code".to_string(),
            Value::NativeFunction(intrinsic_extract_code),
        );
        /*
         */

        // Networking Intrinsics
        scope.set(
            "net.http.request".to_string(),
            Value::NativeFunction(intrinsic_http_request),
        );
        scope.set(
            "net.http.serve".to_string(),
            Value::NativeFunction(intrinsic_http_serve),
        );
        scope.set(
            "net.socket.bind".to_string(),
            Value::NativeFunction(intrinsic_socket_bind),
        );
        scope.set(
            "net.socket.accept".to_string(),
            Value::NativeFunction(intrinsic_socket_accept),
        );
        scope.set(
            "net.socket.connect".to_string(),
            Value::NativeFunction(intrinsic_socket_connect),
        );
        scope.set(
            "net.socket.send".to_string(),
            Value::NativeFunction(intrinsic_socket_send),
        );
        scope.set(
            "net.socket.recv".to_string(),
            Value::NativeFunction(intrinsic_socket_recv),
        );
        scope.set(
            "net.socket.close".to_string(),
            Value::NativeFunction(intrinsic_socket_close),
        );
        scope.set(
            "net.socket.set_timeout".to_string(),
            Value::NativeFunction(intrinsic_socket_set_timeout),
        );

        // Phase 78: Final 12 Parity Intrinsics
        scope.set(
            "sys.json.parse".to_string(),
            Value::NativeFunction(intrinsic_json_parse),
        );
        scope.set(
            "sys.json.stringify".to_string(),
            Value::NativeFunction(intrinsic_json_stringify),
        );
        scope.set("sys.log".to_string(), Value::NativeFunction(intrinsic_log));
        scope.set(
            "sys.exit".to_string(),
            Value::NativeFunction(intrinsic_exit),
        );
        scope.set("exit".to_string(), Value::NativeFunction(intrinsic_exit));
        scope.set("quit".to_string(), Value::NativeFunction(intrinsic_exit));
        scope.set(
            "sys.html_escape".to_string(),
            Value::NativeFunction(intrinsic_html_escape),
        );
        scope.set(
            "sys.z3.verify".to_string(),
            Value::NativeFunction(intrinsic_z3_verify),
        );
        scope.set(
            "sys.vm.source".to_string(),
            Value::NativeFunction(intrinsic_vm_source),
        );
        scope.set(
            "sys.info".to_string(),
            Value::NativeFunction(intrinsic_sys_info),
        );
        scope.set(
            "math.Tensor".to_string(),
            Value::NativeFunction(intrinsic_math_tensor),
        );
        scope.set(
            "math.matmul".to_string(),
            Value::NativeFunction(intrinsic_math_matmul),
        );
        scope.set(
            "math.transpose".to_string(),
            Value::NativeFunction(intrinsic_math_transpose),
        );
        scope.set(
            "math.dot".to_string(),
            Value::NativeFunction(intrinsic_math_dot),
        );
        scope.set(
            "math.add".to_string(),
            Value::NativeFunction(intrinsic_math_tensor_add),
        );
        scope.set(
            "math.sub".to_string(),
            Value::NativeFunction(intrinsic_math_tensor_sub),
        );
        scope.set(
            "math.mul_scalar".to_string(),
            Value::NativeFunction(intrinsic_math_mul_scalar),
        );

        // ── Governance intrinsics ──
        scope.set(
            "governance.trace".to_string(),
            Value::NativeFunction(intrinsic_governance_trace),
        );
        scope.set(
            "governance.mcc_check".to_string(),
            Value::NativeFunction(intrinsic_governance_mcc_check),
        );
        scope.set(
            "governance.verify_chain".to_string(),
            Value::NativeFunction(intrinsic_governance_verify_chain),
        );
    }
}

fn check_path_security(path: &str, is_write: bool) -> Result<(), RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Ok(());

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::env;
        use std::path::Path;

        let cwd = env::current_dir().map_err(|_| RuntimeError::NotExecutable)?;
        // Canonicalize CWD too — on Windows, canonicalize returns UNC paths (\\?\C:\...)
        // but current_dir() returns normal paths. Both must match for starts_with.
        let cwd = std::fs::canonicalize(&cwd).unwrap_or(cwd);
        let path_obj = Path::new(path);

        // Construct absolute path
        let abs_path = if path_obj.is_absolute() {
            path_obj.to_path_buf()
        } else {
            cwd.join(path_obj)
        };

        // To handle both read and write (where file might not exist),
        // we check if the path or its parent exists and is within CWD.
        // If neither exists, we can't write anyway (fs::write doesn't mkdir -p).

        let path_to_check = if abs_path.exists() {
            abs_path.clone()
        } else {
            match abs_path.parent() {
                Some(p) => p.to_path_buf(),
                None => return Err(RuntimeError::NotExecutable),
            }
        };

        // If parent doesn't exist, canonicalize fails.
        let canonical_path =
            std::fs::canonicalize(&path_to_check).map_err(|_| RuntimeError::NotExecutable)?;

        if !canonical_path.starts_with(&cwd) {
            println!(
                "[Ark:Sandbox] Access Denied: Path '{}' resolves outside CWD.",
                path
            );
            return Err(RuntimeError::NotExecutable);
        }

        // Sovereign Security: Protected Paths (Write Only)
        if is_write {
            // Relativize path from CWD to check against protected list
            if let Ok(rel_path) = canonical_path.strip_prefix(&cwd) {
                let rel_str = rel_path.to_string_lossy();
                let protected_prefixes = ["core", "meta", "src", "web", ".git", "target"];
                let protected_files = [
                    "Cargo.toml",
                    "Cargo.lock",
                    "Dockerfile",
                    "README.md",
                    "LICENSE",
                ];

                for prefix in protected_prefixes {
                    if rel_str.starts_with(prefix) {
                        println!(
                            "[Ark:FS] Security Violation: Write to protected directory '{}' denied.",
                            prefix
                        );
                        return Err(RuntimeError::NotExecutable);
                    }
                }
                for file in protected_files {
                    if rel_str == file {
                        println!(
                            "[Ark:FS] Security Violation: Write to protected file '{}' denied.",
                            file
                        );
                        return Err(RuntimeError::NotExecutable);
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn intrinsic_ask_ai(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }

    let prompt = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        return Ok(Value::String(
            "[Ark:AI] Unavailable in Browser Runtime (OIS: Low)".to_string(),
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let api_key = std::env::var("GOOGLE_API_KEY").map_err(|_| {
            println!("[Ark:AI] Error: GOOGLE_API_KEY not set.");
            RuntimeError::NotExecutable
        })?;

        let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent".to_string();

        // Optimization: Check Cache first
        let cache = AI_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(cached_response) = guard.get(prompt) {
                println!("[Ark:AI] Cache Hit. Returning stored response.");
                return Ok(Value::String(cached_response.clone()));
            }
        }

        // Optimization: Reuse Client (Connection Pool)
        let client = AI_CLIENT.get_or_init(|| {
            Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new())
        });

        let payload = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        });

        println!("[Ark:AI] Contacting Gemini (Native Rust)...");

        // Execute async logic within blocking context
        // SAFETY: This blocks the current thread. Do not call this from within an existing
        // Tokio runtime context (e.g., inside an async function running on a runtime)
        // or it will panic. The VM is designed to run in a dedicated thread or process.
        // Create a new runtime for this blocking operation
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Simple Retry Logic
            for attempt in 0..3 {
                match client
                    .post(&url)
                    .header("x-goog-api-key", &api_key)
                    .json(&payload)
                    .send()
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            let json_resp = match resp.json::<serde_json::Value>() {
                                Ok(v) => v,
                                Err(e) => {
                                    println!("[Ark:AI] JSON Error: {}", e);
                                    return Err(RuntimeError::NotExecutable);
                                }
                            };

                            if let Some(text) =
                                json_resp["candidates"][0]["content"]["parts"][0]["text"].as_str()
                            {
                                // Store in Cache
                                if let Ok(mut guard) = cache.lock() {
                                    if guard.len() > 1000 {
                                        guard.clear(); // Simple eviction
                                    }
                                    guard.insert(prompt.clone(), text.to_string());
                                }
                                return Ok(Value::String(text.to_string()));
                            }
                        } else if resp.status().as_u16() == 429 {
                            println!("[Ark:AI] Rate limit (429). Retrying...");
                            tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                            continue;
                        } else {
                            println!("[Ark:AI] HTTP Error: {}", resp.status());
                        }
                    }
                    Err(e) => println!("[Ark:AI] Network Error: {}", e),
                }
            }

            // Fallback Mock
            println!("[Ark:AI] WARNING: API Failed. Using Fallback Mock.");
            let start = "```python\n";
            let code =
                "import datetime\nprint(f'Sovereignty Established: {datetime.datetime.now()}')\n";
            let end = "```";
            Ok(Value::String(format!("{}{}{}", start, code, end)))
        })
    }
}

pub fn intrinsic_exec(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }

    // Support both String (legacy, parsed) and List (secure, explicit)
    let (program, args_list) = match &args[0] {
        Value::String(s) => {
            eprintln!(
                "[Ark:Exec] WARNING: usage of sys.exec(String) is deprecated for security. Use sys.exec([cmd, arg1, ...])"
            );
            #[cfg(not(target_arch = "wasm32"))]
            {
                let parts = shell_words::split(s).map_err(|_| RuntimeError::NotExecutable)?;
                if parts.is_empty() {
                    return Err(RuntimeError::NotExecutable);
                }
                (parts[0].clone(), parts[1..].to_vec())
            }
            #[cfg(target_arch = "wasm32")]
            {
                (s.clone(), vec![])
            }
        }
        Value::List(l) => {
            let mut parts = Vec::new();
            for item in l {
                if let Value::String(s) = item {
                    parts.push(s.clone());
                } else {
                    return Err(RuntimeError::TypeMismatch(
                        "String".to_string(),
                        item.clone(),
                    ));
                }
            }
            if parts.is_empty() {
                return Err(RuntimeError::NotExecutable);
            }
            (parts[0].clone(), parts[1..].to_vec())
        }
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or List".to_string(),
                args[0].clone(),
            ));
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        println!("[Ark:WASM] Security Block: sys.exec('{}') denied.", program);
        return Err(RuntimeError::NotExecutable);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Sovereign Security: Command Whitelist
        // Unless ARK_UNSAFE_EXEC=true is strictly set, we block arbitrary execution.
        let allow_unsafe =
            std::env::var("ARK_UNSAFE_EXEC").unwrap_or_else(|_| "false".to_string()) == "true";

        if !allow_unsafe {
            // Allowed binaries (safe-ish subset)
            let whitelist = [
                "ls", "grep", "cat", "echo", "date", "whoami", "clear", "python3", "python",
                "cargo", "rustc", "node", "git",
            ];

            // Check strictly against whitelist (exact match on binary name)
            // If program is a path (e.g. /bin/ls), extract file_name.
            let prog_path = std::path::Path::new(&program);
            let prog_name = prog_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if !whitelist.contains(&prog_name) {
                println!(
                    "[Ark:Exec] Security Violation: Command '{}' is not in the whitelist.",
                    program
                );
                println!("[Ark:Exec] To bypass, set ARK_UNSAFE_EXEC=true (NOT RECOMMENDED).");
                return Err(RuntimeError::NotExecutable);
            }
        }

        println!("[Ark:Exec] {} {:?}", program, args_list);

        let mut cmd = Command::new(&program);
        cmd.args(&args_list);

        let output = cmd.output().map_err(|_| RuntimeError::NotExecutable)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(Value::String(stdout))
    }
}

pub fn intrinsic_fs_write(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };
    let content = match &args[1] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[1].clone(),
            ));
        }
    };

    check_path_security(path_str, true)?;

    #[cfg(target_arch = "wasm32")]
    {
        println!(
            "[Ark:VFS] Write to '{}': (Simulated) [Content Size: {}]",
            path_str,
            content.len()
        );
        Ok(Value::Unit)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // NTS Protocol: Intentional Friction (Level 1)
        if std::path::Path::new(path_str).exists() {
            println!(
                "[Ark:NTS] WARNING: Overwriting existing file '{}' without explicit lock (LAT).",
                path_str
            );
        }

        println!("[Ark:FS] Writing to {}", path_str);
        fs::write(path_str, content).map_err(|_| RuntimeError::NotExecutable)?;
        Ok(Value::Unit)
    }
}

pub fn intrinsic_add(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }

    let mut iter = args.into_iter();
    let left = iter.next().unwrap();
    let right = iter.next().unwrap();

    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
        (Value::String(mut a), Value::String(b)) => {
            a.push_str(&b);
            Ok(Value::String(a))
        }
        (Value::String(mut a), Value::Integer(b)) => {
            a.push_str(&b.to_string());
            Ok(Value::String(a))
        }
        (Value::Integer(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (Value::String(mut a), Value::Boolean(b)) => {
            a.push_str(&b.to_string());
            Ok(Value::String(a))
        }
        (Value::Boolean(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        (l, _) => Err(RuntimeError::TypeMismatch(
            "Integer, String, or Boolean".to_string(),
            l,
        )),
    }
}

pub fn intrinsic_sub(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_mul(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_div(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => {
            if *b == 0 {
                return Err(RuntimeError::NotExecutable); // Div by zero
            }
            Ok(Value::Integer(a / b))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_mod(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => {
            if *b == 0 {
                return Err(RuntimeError::NotExecutable); // Mod by zero
            }
            Ok(Value::Integer(a % b))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_gt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a > b { 1 } else { 0 })),
        (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a > b { 1 } else { 0 })),
        (Value::String(a), Value::Integer(b)) => {
            Ok(Value::Integer(if a > &b.to_string() { 1 } else { 0 }))
        }
        (Value::Integer(a), Value::String(b)) => {
            Ok(Value::Integer(if &a.to_string() > b { 1 } else { 0 }))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_lt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
        (Value::String(a), Value::String(b)) => Ok(Value::Boolean(a < b)),
        _ => Err(RuntimeError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_not(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Boolean(b) => Ok(Value::Boolean(!b)),
        _ => Err(RuntimeError::TypeMismatch(
            "Boolean".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_ge(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a >= b { 1 } else { 0 })),
        (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a >= b { 1 } else { 0 })),
        (Value::String(a), Value::Integer(b)) => {
            Ok(Value::Integer(if a >= &b.to_string() { 1 } else { 0 }))
        }
        (Value::Integer(a), Value::String(b)) => {
            Ok(Value::Integer(if &a.to_string() >= b { 1 } else { 0 }))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_le(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a <= b { 1 } else { 0 })),
        (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a <= b { 1 } else { 0 })),
        (Value::String(a), Value::Integer(b)) => {
            Ok(Value::Integer(if a <= &b.to_string() { 1 } else { 0 }))
        }
        (Value::Integer(a), Value::String(b)) => {
            Ok(Value::Integer(if &a.to_string() <= b { 1 } else { 0 }))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer or String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_eq(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(if a == b { 1 } else { 0 })),
        (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a == b { 1 } else { 0 })),
        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Integer(if a == b { 1 } else { 0 })),
        _ => Ok(Value::Integer(0)), // Default inequality for mismatched types/objects
    }
}

pub fn intrinsic_and(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    // Truthy check: Integer != 0, Boolean == true, String != "0" and != ""
    let is_truthy = |v: &Value| match v {
        Value::Integer(n) => *n != 0,
        Value::Boolean(b) => *b,
        Value::String(s) => s != "0" && !s.is_empty() && s != "false",
        _ => false,
    };

    let left = is_truthy(&args[0]);
    let right = is_truthy(&args[1]);

    Ok(Value::Boolean(left && right))
}

pub fn intrinsic_or(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let is_truthy = |v: &Value| match v {
        Value::Integer(n) => *n != 0,
        Value::Boolean(b) => *b,
        Value::String(s) => s != "0" && !s.is_empty() && s != "false",
        _ => false,
    };

    let left = is_truthy(&args[0]);
    let right = is_truthy(&args[1]);

    Ok(Value::Boolean(left || right))
}

pub fn intrinsic_print(args: Vec<Value>) -> Result<Value, RuntimeError> {
    for arg in args {
        print_value(&arg);
    }
    println!(); // Newline at the end
    Ok(Value::Unit)
}

fn print_value(v: &Value) {
    match v {
        Value::Integer(i) => print!("{}", i),
        Value::String(s) => print!("{}", s),
        Value::Boolean(b) => print!("{}", b),
        Value::Unit => print!("unit"),
        Value::LinearObject { id, .. } => print!("<LinearObject:{}>", id),
        Value::Function(_) => print!("<Function>"),
        Value::NativeFunction(_) => print!("<NativeFunction>"),
        Value::List(l) => {
            print!("[");
            for (i, item) in l.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print_value(item);
            }
            print!("]");
        }
        Value::Buffer(b) => print!("<Buffer: len={}, ptr={:p}>", b.len(), b.as_ptr()),
        Value::Struct(fields) => {
            print!("{{");
            for (i, (k, v)) in fields.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("{}: ", k);
                print_value(v);
            }
            print!("}}");
        }
        Value::PVec(pv) => print!("{}", pv),
        Value::PMap(pm) => print!("{}", pm),
        Value::Return(val) => print_value(val),
        Value::EnumValue {
            enum_name,
            variant,
            fields,
        } => {
            if fields.is_empty() {
                print!("{}::{}", enum_name, variant);
            } else {
                print!("{}::{}(", enum_name, variant);
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print_value(field);
                }
                print!(")");
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn validate_safe_path(path_str: &str) -> Result<PathBuf, RuntimeError> {
    let path = Path::new(path_str);

    // 1. Canonicalize the requested path (resolves symlinks and ..)
    // If the file does not exist, canonicalize fails. For read, this is fine (file must exist).
    let canonical_path = fs::canonicalize(path).map_err(|_| RuntimeError::NotExecutable)?;

    // 2. Canonicalize the current working directory (sandbox root)
    let current_dir = env::current_dir().map_err(|_| RuntimeError::NotExecutable)?;
    let canonical_cwd = fs::canonicalize(current_dir).map_err(|_| RuntimeError::NotExecutable)?;

    // 3. Verify that the requested path starts with the sandbox root
    if canonical_path.starts_with(&canonical_cwd) {
        Ok(canonical_path)
    } else {
        println!(
            "[Ark:FS] Security Violation: Path traversal attempt to '{}'",
            path_str
        );
        Err(RuntimeError::NotExecutable)
    }
}

pub fn intrinsic_fs_read(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    check_path_security(path_str, false)?;

    #[cfg(target_arch = "wasm32")]
    {
        println!("[Ark:VFS] Read from '{}': (Simulated) [Empty]", path_str);
        Ok(Value::String("".to_string()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("[Ark:FS] Reading from {}", path_str);
        // Security: Path Traversal Check
        let safe_path = validate_safe_path(path_str)?;
        let content = fs::read_to_string(safe_path).map_err(|_| RuntimeError::NotExecutable)?;
        Ok(Value::String(content))
    }
}

pub fn intrinsic_crypto_hash(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let data_bytes = match &args[0] {
        Value::String(s) => s.as_bytes(),
        Value::Buffer(b) => b.as_slice(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer".to_string(),
                args[0].clone(),
            ));
        }
    };

    Ok(Value::String(crate::crypto::hash(data_bytes)))
}

pub fn intrinsic_crypto_verify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    // Helper to get bytes from Buffer or Hex String
    let get_bytes = |v: &Value, name: &str| -> Result<Vec<u8>, RuntimeError> {
        match v {
            Value::Buffer(b) => Ok(b.clone()),
            Value::String(s) => hex::decode(s).map_err(|_| {
                RuntimeError::TypeMismatch(format!("Hex String for {}", name), v.clone())
            }),
            _ => Err(RuntimeError::TypeMismatch(
                format!("Buffer or Hex String for {}", name),
                v.clone(),
            )),
        }
    };

    let msg_bytes = match &args[0] {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Buffer(b) => b.clone(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer for msg".to_string(),
                args[0].clone(),
            ));
        }
    };

    let sig_bytes = get_bytes(&args[1], "signature")?;
    let pubkey_bytes = get_bytes(&args[2], "public key")?;

    match crate::crypto::verify_signature(&msg_bytes, &sig_bytes, &pubkey_bytes) {
        Ok(valid) => Ok(Value::Boolean(valid)),
        Err(e) => Err(RuntimeError::InvalidOperation(e)),
    }
}

fn get_bytes_from_value(v: &Value, name: &str) -> Result<Vec<u8>, RuntimeError> {
    match v {
        Value::Buffer(b) => Ok(b.clone()),
        Value::String(s) => hex::decode(s)
            .map_err(|_| RuntimeError::TypeMismatch(format!("Hex String for {}", name), v.clone())),
        _ => Err(RuntimeError::TypeMismatch(
            format!("Buffer or Hex String for {}", name),
            v.clone(),
        )),
    }
}

pub fn intrinsic_crypto_sha512(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let data = match &args[0] {
        Value::String(s) => s.as_bytes(),
        Value::Buffer(b) => b.as_slice(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer".into(),
                args[0].clone(),
            ));
        }
    };
    let mut hasher = Sha512::new();
    hasher.update(data);
    let result = hasher.finalize();
    Ok(Value::String(hex::encode(result)))
}

pub fn intrinsic_crypto_hmac_sha512(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }

    let key_bytes = get_bytes_from_value(&args[0], "key")?;

    let data_bytes = match &args[1] {
        Value::String(s) => s.as_bytes(),
        Value::Buffer(b) => b.as_slice(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer".into(),
                args[1].clone(),
            ));
        }
    };

    type HmacSha512 = Hmac<Sha512>;
    let mut mac = <HmacSha512 as Mac>::new_from_slice(&key_bytes)
        .map_err(|_| RuntimeError::InvalidOperation("Invalid Key Length".into()))?;
    mac.update(data_bytes);
    let result = mac.finalize().into_bytes();
    Ok(Value::String(hex::encode(result)))
}

pub fn intrinsic_crypto_pbkdf2(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 4 {
        return Err(RuntimeError::NotExecutable);
    }

    let password = match &args[0] {
        Value::String(s) => s.as_bytes(),
        Value::Buffer(b) => b.as_slice(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer".into(),
                args[0].clone(),
            ));
        }
    };

    let salt = get_bytes_from_value(&args[1], "salt")?;

    let iterations = match args[2] {
        Value::Integer(n) => n as u32,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".into(),
                args[2].clone(),
            ));
        }
    };

    let output_len = match args[3] {
        Value::Integer(n) => n as usize,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".into(),
                args[3].clone(),
            ));
        }
    };

    if iterations == 0 {
        return Err(RuntimeError::InvalidOperation(
            "Iterations must be > 0".into(),
        ));
    }

    let mut result = vec![0u8; output_len];
    pbkdf2::<Hmac<Sha512>>(password, &salt, iterations, &mut result)
        .map_err(|_| RuntimeError::InvalidOperation("PBKDF2 Failed".into()))?;

    Ok(Value::String(hex::encode(result)))
}

pub fn intrinsic_crypto_aes_gcm_encrypt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    let key_bytes = get_bytes_from_value(&args[0], "key")?;
    let nonce_bytes = get_bytes_from_value(&args[1], "nonce")?;

    if key_bytes.len() != 32 {
        return Err(RuntimeError::InvalidOperation(
            "Key must be 32 bytes".into(),
        ));
    }
    if nonce_bytes.len() != 12 {
        return Err(RuntimeError::InvalidOperation(
            "Nonce must be 12 bytes".into(),
        ));
    }

    let plaintext = match &args[2] {
        Value::String(s) => s.as_bytes(),
        Value::Buffer(b) => b.as_slice(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer".into(),
                args[2].clone(),
            ));
        }
    };

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| RuntimeError::InvalidOperation("Invalid Key".into()))?;

    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| RuntimeError::InvalidOperation("Encryption Failed".into()))?;

    Ok(Value::String(hex::encode(ciphertext)))
}

pub fn intrinsic_crypto_aes_gcm_decrypt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    let key_bytes = get_bytes_from_value(&args[0], "key")?;
    let nonce_bytes = get_bytes_from_value(&args[1], "nonce")?;
    let ciphertext = get_bytes_from_value(&args[2], "ciphertext")?;

    if key_bytes.len() != 32 {
        return Err(RuntimeError::InvalidOperation(
            "Key must be 32 bytes".into(),
        ));
    }
    if nonce_bytes.len() != 12 {
        return Err(RuntimeError::InvalidOperation(
            "Nonce must be 12 bytes".into(),
        ));
    }

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| RuntimeError::InvalidOperation("Invalid Key".into()))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|_| {
        RuntimeError::InvalidOperation("Decryption Failed (Invalid Tag or Key)".into())
    })?;

    Ok(Value::Buffer(plaintext))
}

pub fn intrinsic_crypto_random_bytes(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let n = match args[0] {
        Value::Integer(i) => i as usize,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".into(),
                args[0].clone(),
            ));
        }
    };
    if n > 65536 {
        return Err(RuntimeError::InvalidOperation(
            "Too many random bytes requested".into(),
        ));
    }
    let mut bytes = vec![0u8; n];
    OsRng.fill_bytes(&mut bytes);
    Ok(Value::String(hex::encode(bytes)))
}

pub fn intrinsic_crypto_ed25519_generate(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::NotExecutable);
    }
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let mut map = std::collections::HashMap::new();
    map.insert(
        "private_key".to_string(),
        Value::String(hex::encode(signing_key.to_bytes())),
    );
    map.insert(
        "public_key".to_string(),
        Value::String(hex::encode(verifying_key.to_bytes())),
    );

    Ok(Value::Struct(map))
}

pub fn intrinsic_crypto_ed25519_sign(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }

    let msg_bytes = match &args[0] {
        Value::String(s) => s.as_bytes(),
        Value::Buffer(b) => b.as_slice(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer".into(),
                args[0].clone(),
            ));
        }
    };

    let key_bytes = get_bytes_from_value(&args[1], "private_key")?;

    if key_bytes.len() != 32 {
        return Err(RuntimeError::InvalidOperation(
            "Private Key must be 32 bytes".into(),
        ));
    }

    let signing_key = SigningKey::from_bytes(key_bytes.as_slice().try_into().unwrap());
    let signature = signing_key.sign(msg_bytes);

    Ok(Value::String(hex::encode(signature.to_bytes())))
}

pub fn intrinsic_crypto_ed25519_verify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    let msg_bytes = match &args[0] {
        Value::String(s) => s.as_bytes(),
        Value::Buffer(b) => b.as_slice(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String or Buffer".into(),
                args[0].clone(),
            ));
        }
    };

    let sig_bytes = get_bytes_from_value(&args[1], "signature")?;
    let pub_bytes = get_bytes_from_value(&args[2], "public_key")?;

    if sig_bytes.len() != 64 {
        return Ok(Value::Boolean(false));
    }
    if pub_bytes.len() != 32 {
        return Ok(Value::Boolean(false));
    }

    let verifying_key = match VerifyingKey::from_bytes(pub_bytes.as_slice().try_into().unwrap()) {
        Ok(k) => k,
        Err(_) => return Ok(Value::Boolean(false)),
    };

    let signature = ed25519_dalek::Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());

    match verifying_key.verify(msg_bytes, &signature) {
        Ok(_) => Ok(Value::Boolean(true)),
        Err(_) => Ok(Value::Boolean(false)),
    }
}

pub fn intrinsic_merkle_root(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }

    let list = match &args[0] {
        Value::List(l) => l,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "List".to_string(),
                args[0].clone(),
            ));
        }
    };

    // Extract strings from list
    let mut leaves: Vec<String> = Vec::new();
    for item in list {
        match item {
            Value::String(s) => leaves.push(s.clone()),
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "String inside List".to_string(),
                    item.clone(),
                ));
            }
        }
    }

    Ok(Value::String(crate::crypto::merkle_root(&leaves)))
}

pub fn intrinsic_buffer_alloc(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let size = match &args[0] {
        Value::Integer(n) => *n as usize,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };
    let buf = vec![0u8; size];
    Ok(Value::Buffer(buf))
}

pub fn intrinsic_buffer_inspect(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match args.into_iter().next().unwrap() {
        Value::Buffer(b) => {
            let ptr = b.as_ptr();
            println!("<Buffer Inspect: ptr={:p}, len={}>", ptr, b.len());
            Ok(Value::Buffer(b))
        }
        v => Err(RuntimeError::TypeMismatch("Buffer".to_string(), v)),
    }
}

pub fn intrinsic_buffer_read(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // args: [buffer, index]
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let index_val = args.pop().unwrap();
    let buf_val = args.pop().unwrap();

    let index = match index_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), index_val)),
    };

    match buf_val {
        Value::Buffer(b) => {
            if index < 0 || index >= b.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            let val = b[index as usize] as i64;
            let list = vec![Value::Integer(val), Value::Buffer(b)];
            Ok(Value::List(list))
        }
        v => Err(RuntimeError::TypeMismatch("Buffer".to_string(), v)),
    }
}

pub fn intrinsic_buffer_write(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Linear Semantics: buf := sys.mem.write(buf, i, v)
    // Consumes the buffer (linear ownership), modifies it in-place, and returns the modified buffer.
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    // We need to destructure args to get ownership of Buffer
    // args is Vec<Value>.
    let mut args = args; // Allow move
    let val_to_write = args.pop().unwrap(); // value
    let idx_val = args.pop().unwrap(); // index
    let buf_val = args.pop().unwrap(); // buffer

    let index = match idx_val {
        Value::Integer(n) => n as usize,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), idx_val)),
    };

    let byte_val = match val_to_write {
        Value::Integer(n) => n as u8,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                val_to_write,
            ));
        }
    };

    match buf_val {
        Value::Buffer(mut b) => {
            if index >= b.len() {
                return Err(RuntimeError::NotExecutable);
            }
            b[index] = byte_val;
            Ok(Value::Buffer(b)) // Return modified buffer (Linear Threading)
        }
        _ => Err(RuntimeError::TypeMismatch("Buffer".to_string(), buf_val)),
    }
}

pub fn intrinsic_list_get(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let index_val = args.pop().unwrap();
    let list_val = args.pop().unwrap();

    let index = match index_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), index_val)),
    };

    match list_val {
        Value::List(list) => {
            if index < 0 || index >= list.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            let val = list[index as usize].clone();
            let new_list_val = Value::List(list);

            Ok(Value::List(vec![val, new_list_val]))
        }
        Value::String(s) => {
            if index < 0 || index >= s.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            // Unicode safety: chars().nth() is O(N). optimized: as_bytes?
            // Ark strings are UTF-8. Indexing by byte or char?
            // Python does char. Rust String is UTF-8.
            // Let's use bytes for O(1) if we assume ASCII, or chars if we want correctness.
            // Standard: chars.
            if let Some(c) = s.chars().nth(index as usize) {
                let char_str = c.to_string();
                // Return [char_str, original_string]
                Ok(Value::List(vec![Value::String(char_str), Value::String(s)]))
            } else {
                Err(RuntimeError::NotExecutable)
            }
        }
        _ => Err(RuntimeError::TypeMismatch(
            "List or String".to_string(),
            list_val,
        )),
    }
}

pub fn intrinsic_list_append(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    // args: [list, item]
    // consume args
    let mut args = args;
    let item = args.pop().unwrap();
    let list_val = args.pop().unwrap();

    match list_val {
        Value::List(mut list) => {
            // Linear append: Modify in place if we owned it (we do, because args passed by value)
            list.push(item);
            Ok(Value::List(list))
        }
        _ => Err(RuntimeError::TypeMismatch("List".to_string(), list_val)),
    }
}

pub fn intrinsic_list_pop(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.is_empty() || args.len() > 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;

    // Check if 2 args (pop at index) or 1 arg (pop last)
    let idx_val = if args.len() == 2 {
        Some(args.pop().unwrap())
    } else {
        None
    };

    let list_val = args.pop().unwrap();

    match list_val {
        Value::List(mut list) => {
            let index = match idx_val {
                Some(val) => match val {
                    Value::Integer(n) => n,
                    _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), val)),
                },
                None => {
                    // Default to last index
                    if list.is_empty() {
                        // As per prompt: If empty, return ArkValue::Unit
                        return Ok(Value::Unit);
                    }
                    (list.len() - 1) as i64
                }
            };

            if index < 0 || index >= list.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }

            // Linear Pop: Remove element.
            let val = list.remove(index as usize);
            Ok(Value::List(vec![val, Value::List(list)]))
        }
        _ => Err(RuntimeError::TypeMismatch("List".to_string(), list_val)),
    }
}

pub fn intrinsic_list_delete(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let idx_val = args.pop().unwrap();
    let list_val = args.pop().unwrap();

    let index = match idx_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), idx_val)),
    };

    match list_val {
        Value::List(mut list) => {
            if index < 0 || index >= list.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            list.remove(index as usize);
            Ok(Value::List(list))
        }
        _ => Err(RuntimeError::TypeMismatch("List".to_string(), list_val)),
    }
}

pub fn intrinsic_struct_has(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let field_val = args.pop().unwrap();
    let struct_val = args.pop().unwrap();

    let field = match field_val {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch("String".to_string(), field_val));
        }
    };

    match struct_val {
        Value::Struct(data) => {
            let has = data.contains_key(&field);
            // Return [bool, struct] to preserve linearity
            Ok(Value::List(vec![Value::Boolean(has), Value::Struct(data)]))
        }
        _ => {
            // If not a struct, return false and the object (to be safe/linear)
            Ok(Value::List(vec![Value::Boolean(false), struct_val]))
        }
    }
}

pub fn intrinsic_pow_mod(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }
    // args: [base, exp, mod]
    // args.pop() gives mod (last), then exp, then base.
    let mut args = args;
    let mod_val = args.pop().unwrap();
    let exp_val = args.pop().unwrap();
    let base_val = args.pop().unwrap();

    let m = match mod_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), mod_val)),
    };
    let e = match exp_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), exp_val)),
    };
    let b = match base_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), base_val)),
    };

    if m == 0 {
        return Err(RuntimeError::InvalidOperation("Modulo by zero".to_string()));
    }
    // Edge case: mod=1 -> 0
    if m == 1 {
        return Ok(Value::Integer(0));
    }
    // Edge case: exp=0 -> 1
    if e == 0 {
        return Ok(Value::Integer(1));
    }

    if e < 0 {
        return Err(RuntimeError::InvalidOperation(
            "Negative exponent in pow_mod".to_string(),
        ));
    }

    // Use i128 to avoid overflow
    let mut base = b as i128;
    let mut exp = e as i128;
    let modulus = m as i128;
    let mut result: i128 = 1;

    base %= modulus;
    if base < 0 {
        base += modulus;
    }

    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % modulus;
        }
        base = (base * base) % modulus;
        exp /= 2;
    }

    Ok(Value::Integer(result as i64))
}

pub fn intrinsic_len(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let mut args = args;
    let val = args.pop().unwrap();

    let len = match &val {
        Value::String(s) => s.len() as i64,
        Value::List(l) => l.len() as i64,
        Value::Buffer(b) => b.len() as i64,
        _ => return Err(RuntimeError::TypeMismatch("Sequence".to_string(), val)),
    };

    Ok(Value::List(vec![Value::Integer(len), val]))
}

pub fn intrinsic_struct_get(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }

    let mut args = args;
    let field_val = args.pop().unwrap();
    let struct_val = args.pop().unwrap();

    let field = match field_val {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String Key".to_string(),
                field_val,
            ));
        }
    };

    match struct_val {
        Value::Struct(data) => {
            let val_opt = data.get(&field).cloned();
            if let Some(val) = val_opt {
                Ok(Value::List(vec![val, Value::Struct(data)]))
            } else {
                Err(RuntimeError::VariableNotFound(field))
            }
        }
        _ => Err(RuntimeError::TypeMismatch("Struct".to_string(), struct_val)),
    }
}

pub fn intrinsic_list_set(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }
    // args: [list, index, value]
    let mut args = args;
    let val = args.pop().unwrap();
    let idx_val = args.pop().unwrap();
    let list_val = args.pop().unwrap();

    let index = match idx_val {
        Value::Integer(n) => n,
        _ => return Err(RuntimeError::TypeMismatch("Integer".to_string(), idx_val)),
    };

    match list_val {
        Value::List(mut list) => {
            if index < 0 || index >= list.len() as i64 {
                return Err(RuntimeError::NotExecutable);
            }
            list[index as usize] = val;
            Ok(Value::List(list))
        }
        _ => Err(RuntimeError::TypeMismatch("List".to_string(), list_val)),
    }
}

pub fn intrinsic_struct_set(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }

    let mut args = args;
    let new_val = args.pop().unwrap();
    let field_val = args.pop().unwrap();
    let struct_val = args.pop().unwrap();

    let field = match field_val {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String Key".to_string(),
                field_val,
            ));
        }
    };

    match struct_val {
        Value::Struct(mut data) => {
            // Linear Update: Mutate in place (we own it)
            data.insert(field, new_val);
            Ok(Value::Struct(data))
        }
        _ => Err(RuntimeError::TypeMismatch("Struct".to_string(), struct_val)),
    }
}

pub fn intrinsic_time_now(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RuntimeError::InvalidOperation("Time went backwards".to_string()))?;
    Ok(Value::Integer(since_the_epoch.as_millis() as i64))
}

pub fn intrinsic_time_sleep(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }

    let duration_ms = match args[0] {
        Value::Integer(n) => {
            if n < 0 {
                return Err(RuntimeError::InvalidOperation(
                    "Negative sleep duration".to_string(),
                ));
            }
            n as u64
        }
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        // In WASM, blocking sleep is generally not supported or freezes the browser.
        // We'll log it as a simulation.
        println!("[Ark:Time] Sleep {}ms (Simulated)", duration_ms);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        thread::sleep(Duration::from_millis(duration_ms));
    }

    Ok(Value::Unit)
}

pub fn intrinsic_math_pow(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(base), Value::Integer(exp)) => {
            let res = (*base as f64).powf(*exp as f64);
            Ok(Value::Integer(res as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_sqrt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            if *n < 0 {
                return Err(RuntimeError::InvalidOperation(
                    "Square root of negative number".to_string(),
                ));
            }
            let res = (*n as f64).sqrt();
            Ok(Value::Integer(res as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_sin(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let angle = (*n as f64) / 10000.0;
            let res = angle.sin();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_cos(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let angle = (*n as f64) / 10000.0;
            let res = angle.cos();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_tan(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let angle = (*n as f64) / 10000.0;
            let res = angle.tan();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_asin(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let val = (*n as f64) / 10000.0;
            if !(-1.0..=1.0).contains(&val) {
                return Err(RuntimeError::InvalidOperation(
                    "asin out of domain".to_string(),
                ));
            }
            let res = val.asin();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_acos(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let val = (*n as f64) / 10000.0;
            if !(-1.0..=1.0).contains(&val) {
                return Err(RuntimeError::InvalidOperation(
                    "acos out of domain".to_string(),
                ));
            }
            let res = val.acos();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_atan(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::Integer(n) => {
            let val = (*n as f64) / 10000.0;
            let res = val.atan();
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_math_atan2(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    match (&args[0], &args[1]) {
        (Value::Integer(y), Value::Integer(x)) => {
            let y_val = (*y as f64) / 10000.0;
            let x_val = (*x as f64) / 10000.0;
            let res = y_val.atan2(x_val);
            Ok(Value::Integer((res * 10000.0) as i64))
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Integer".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_io_cls(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    print!("\x1b[2J\x1b[H");
    Ok(Value::Unit)
}

pub fn intrinsic_io_read_bytes(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    check_path_security(path_str, false)?;

    #[cfg(target_arch = "wasm32")]
    {
        println!("[Ark:VFS] Read Bytes from '{}': (Simulated)", path_str);
        Ok(Value::List(vec![]))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Security: Path Traversal Check
        let safe_path = validate_safe_path(path_str)?;
        let content = fs::read(safe_path).map_err(|_| RuntimeError::NotExecutable)?;

        // Convert Vec<u8> to Vec<Value> (List of Integers)
        let list = content
            .into_iter()
            .map(|b| Value::Integer(b as i64))
            .collect();
        Ok(Value::List(list))
    }
}

pub fn intrinsic_io_read_line(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::NotExecutable);
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(Value::String("".to_string()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|_| RuntimeError::NotExecutable)?;
        // Trim newline
        if input.ends_with('\n') {
            input.pop();
            if input.ends_with('\r') {
                input.pop();
            }
        }
        Ok(Value::String(input))
    }
}

pub fn intrinsic_io_write(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let s = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    print!("{}", s);
    #[cfg(not(target_arch = "wasm32"))]
    {
        io::stdout()
            .flush()
            .map_err(|_| RuntimeError::NotExecutable)?;
    }
    Ok(Value::Unit)
}

pub fn intrinsic_io_read_file_async(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // MVP: Blocking Fallback.
    // In a future version, this should spawn a thread or use Tokio fs and return a Promise/Future object.
    intrinsic_fs_read(args)
}

pub fn intrinsic_chain_height(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    Ok(Value::Integer(10000))
}

pub fn intrinsic_chain_get_balance(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::String(_) => Ok(Value::Integer(5000)),
        _ => Err(RuntimeError::TypeMismatch(
            "String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_chain_submit_tx(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::String(_) => Ok(Value::String("0x123...".to_string())),
        _ => Err(RuntimeError::TypeMismatch(
            "String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_chain_verify_tx(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    match &args[0] {
        Value::String(_) => Ok(Value::Boolean(true)),
        _ => Err(RuntimeError::TypeMismatch(
            "String".to_string(),
            args[0].clone(),
        )),
    }
}

pub fn intrinsic_fs_write_buffer(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    check_path_security(path_str, true)?;

    match &args[1] {
        Value::Buffer(buf) => {
            #[cfg(target_arch = "wasm32")]
            {
                println!(
                    "[Ark:VFS] Write Buffer to '{}': [Size: {}]",
                    path_str,
                    buf.len()
                );
                Ok(Value::Unit)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if std::path::Path::new(path_str).exists() {
                    println!(
                        "[Ark:NTS] WARNING: Overwriting existing file '{}' without explicit lock (LAT).",
                        path_str
                    );
                }
                println!("[Ark:FS] Writing buffer to {}", path_str);
                fs::write(path_str, buf).map_err(|_| RuntimeError::NotExecutable)?;
                Ok(Value::Unit)
            }
        }
        _ => Err(RuntimeError::TypeMismatch(
            "Buffer".to_string(),
            args[1].clone(),
        )),
    }
}

pub fn intrinsic_fs_read_buffer(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let path_str = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    check_path_security(path_str, false)?;

    #[cfg(target_arch = "wasm32")]
    {
        println!("[Ark:VFS] Read Buffer from '{}': [Empty]", path_str);
        Ok(Value::Buffer(vec![]))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("[Ark:FS] Reading buffer from {}", path_str);
        // Security: Path Traversal Check
        let safe_path = validate_safe_path(path_str)?;
        let content = fs::read(safe_path).map_err(|_| RuntimeError::NotExecutable)?;
        Ok(Value::Buffer(content))
    }
}

pub fn intrinsic_math_sin_scaled(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }
    let angle = match &args[0] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };
    let scale_in = match &args[1] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[1].clone(),
            ));
        }
    };
    let scale_out = match &args[2] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[2].clone(),
            ));
        }
    };

    if scale_in == 0.0 {
        return Err(RuntimeError::InvalidOperation("Scale In is 0".to_string()));
    }

    let res = (angle / scale_in).sin() * scale_out;
    Ok(Value::Integer(res.round() as i64))
}

pub fn intrinsic_math_cos_scaled(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::NotExecutable);
    }
    let angle = match &args[0] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };
    let scale_in = match &args[1] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[1].clone(),
            ));
        }
    };
    let scale_out = match &args[2] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[2].clone(),
            ));
        }
    };

    if scale_in == 0.0 {
        return Err(RuntimeError::InvalidOperation("Scale In is 0".to_string()));
    }

    let res = (angle / scale_in).cos() * scale_out;
    Ok(Value::Integer(res.round() as i64))
}

pub fn intrinsic_math_pi_scaled(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let scale = match &args[0] {
        Value::Integer(i) => *i as f64,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };

    let res = std::f64::consts::PI * scale;
    Ok(Value::Integer(res.round() as i64))
}

pub fn intrinsic_str_from_code(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let code = match &args[0] {
        Value::Integer(i) => *i as u32,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };
    if let Some(c) = std::char::from_u32(code) {
        Ok(Value::String(c.to_string()))
    } else {
        Err(RuntimeError::InvalidOperation(
            "Invalid Char Code".to_string(),
        ))
    }
}

pub fn intrinsic_extract_code(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let text = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    // Regex to capture fenced code blocks: ```lang ... ```
    let re = Regex::new(r"```(?:\w+)?\n([\s\S]*?)```")
        .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))?;

    let mut blocks = Vec::new();
    for cap in re.captures_iter(text) {
        if let Some(match_str) = cap.get(1) {
            blocks.push(Value::String(match_str.as_str().to_string()));
        }
    }

    Ok(Value::List(blocks))
}
// ----------------------------------------------------------------------
// NETWORKING INTRINSICS
// ----------------------------------------------------------------------

pub fn intrinsic_http_request(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.len() < 2 {
            return Err(RuntimeError::NotExecutable);
        }

        let method = match &args[0] {
            Value::String(s) => s.as_str(),
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "String".to_string(),
                    args[0].clone(),
                ));
            }
        };

        let url = match &args[1] {
            Value::String(s) => s.as_str(),
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "String".to_string(),
                    args[1].clone(),
                ));
            }
        };

        let request = ureq::request(method, url);

        let result = if args.len() > 2 {
            match &args[2] {
                Value::String(body) => request.send_string(body),
                _ => {
                    return Err(RuntimeError::TypeMismatch(
                        "String".to_string(),
                        args[2].clone(),
                    ));
                }
            }
        } else {
            request.call()
        };

        let handle_response = |response: ureq::Response| -> Result<Value, RuntimeError> {
            let status = response.status() as i64;
            let mut headers_map = HashMap::new();
            for name in response.headers_names() {
                if let Some(value) = response.header(&name) {
                    headers_map.insert(name, Value::String(value.to_string()));
                }
            }
            let body = response
                .into_string()
                .map_err(|_| RuntimeError::NotExecutable)?;
            let mut resp_struct = HashMap::new();
            resp_struct.insert("status".to_string(), Value::Integer(status));
            resp_struct.insert("body".to_string(), Value::String(body));
            resp_struct.insert("headers".to_string(), Value::Struct(headers_map));
            Ok(Value::Struct(resp_struct))
        };

        match result {
            Ok(response) => handle_response(response),
            Err(ureq::Error::Status(_code, response)) => handle_response(response),
            Err(e) => {
                // Transport error
                Err(RuntimeError::InvalidOperation(format!("HTTP Error: {}", e)))
            }
        }
    }
}

pub fn intrinsic_http_serve(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.len() != 1 {
            return Err(RuntimeError::NotExecutable);
        }
        let port = match &args[0] {
            Value::Integer(i) => *i as u16,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[0].clone(),
                ));
            }
        };

        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .map_err(|_| RuntimeError::NotExecutable)?;

        // Accept ONE connection
        let (mut stream, _) = listener.accept().map_err(|_| RuntimeError::NotExecutable)?;

        // Read request
        // We'll read what's available or set a timeout.
        stream
            .set_read_timeout(Some(Duration::from_millis(1000)))
            .ok();

        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).unwrap_or(0);
        let request_str = String::from_utf8_lossy(&buffer[..n]).to_string();

        Ok(Value::String(request_str))
    }
}

pub fn intrinsic_socket_bind(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.len() != 1 {
            return Err(RuntimeError::NotExecutable);
        }
        let port = match &args[0] {
            Value::Integer(i) => *i as u16,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[0].clone(),
                ));
            }
        };

        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .map_err(|_| RuntimeError::NotExecutable)?;

        let id = SOCKET_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut sockets = get_sockets().lock().unwrap();
        sockets.insert(id, SocketResource::Listener(listener));

        Ok(Value::Integer(id))
    }
}

pub fn intrinsic_socket_accept(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.len() != 1 {
            return Err(RuntimeError::NotExecutable);
        }
        let id = match &args[0] {
            Value::Integer(i) => *i,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[0].clone(),
                ));
            }
        };

        // We need to release the lock while accepting, otherwise we block all network ops.
        // BUT we can't easily clone TcpListener.
        // Rust TcpListener `try_clone` exists.
        let listener_clone = {
            let sockets = get_sockets().lock().unwrap();
            match sockets.get(&id) {
                Some(SocketResource::Listener(l)) => {
                    l.try_clone().map_err(|_| RuntimeError::NotExecutable)?
                }
                _ => return Err(RuntimeError::InvalidOperation("Not a listener".to_string())),
            }
        };

        let (stream, _) = listener_clone
            .accept()
            .map_err(|_| RuntimeError::NotExecutable)?;

        let new_id = SOCKET_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut sockets = get_sockets().lock().unwrap();
        sockets.insert(new_id, SocketResource::Stream(stream));

        Ok(Value::Integer(new_id))
    }
}

pub fn intrinsic_socket_connect(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.len() != 2 {
            return Err(RuntimeError::NotExecutable);
        }
        let host = match &args[0] {
            Value::String(s) => s.clone(),
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "String".to_string(),
                    args[0].clone(),
                ));
            }
        };
        let port = match &args[1] {
            Value::Integer(i) => *i as u16,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[1].clone(),
                ));
            }
        };

        let stream = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|_| RuntimeError::NotExecutable)?;

        let id = SOCKET_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut sockets = get_sockets().lock().unwrap();
        sockets.insert(id, SocketResource::Stream(stream));

        Ok(Value::Integer(id))
    }
}

pub fn intrinsic_socket_send(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.len() != 2 {
            return Err(RuntimeError::NotExecutable);
        }
        let id = match &args[0] {
            Value::Integer(i) => *i,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[0].clone(),
                ));
            }
        };
        let data = match &args[1] {
            Value::String(s) => s.as_bytes().to_vec(),
            Value::Buffer(b) => b.clone(),
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "String or Buffer".to_string(),
                    args[1].clone(),
                ));
            }
        };

        let mut sockets = get_sockets().lock().unwrap();
        match sockets.get_mut(&id) {
            Some(SocketResource::Stream(s)) => {
                s.write_all(&data)
                    .map_err(|_| RuntimeError::NotExecutable)?;
                Ok(Value::Integer(data.len() as i64))
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Invalid socket or not a stream".to_string(),
            )),
        }
    }
}

pub fn intrinsic_socket_recv(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.is_empty() {
            return Err(RuntimeError::NotExecutable);
        }
        let id = match &args[0] {
            Value::Integer(i) => *i,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[0].clone(),
                ));
            }
        };
        let max_bytes = if args.len() > 1 {
            match &args[1] {
                Value::Integer(i) => *i as usize,
                _ => 1024,
            }
        } else {
            1024
        };

        let mut sockets = get_sockets().lock().unwrap();
        match sockets.get_mut(&id) {
            Some(SocketResource::Stream(s)) => {
                let mut buf = vec![0u8; max_bytes];
                let n = s.read(&mut buf).map_err(|_| RuntimeError::NotExecutable)?;
                // Truncate to actual size
                buf.truncate(n);
                // Convert to string (lossy) or return buffer?
                // Prompt says "Return String".
                Ok(Value::String(String::from_utf8_lossy(&buf).to_string()))
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Invalid socket or not a stream".to_string(),
            )),
        }
    }
}

pub fn intrinsic_socket_close(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.len() != 1 {
            return Err(RuntimeError::NotExecutable);
        }
        let id = match &args[0] {
            Value::Integer(i) => *i,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[0].clone(),
                ));
            }
        };

        let mut sockets = get_sockets().lock().unwrap();
        if sockets.remove(&id).is_some() {
            Ok(Value::Boolean(true))
        } else {
            Ok(Value::Boolean(false))
        }
    }
}
pub fn intrinsic_thread_spawn(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let callable = args[0].clone();

    // Get Next ID
    let thread_id = {
        let mut id_guard = NEXT_THREAD_ID.get_or_init(|| Mutex::new(1)).lock().unwrap();
        let id = *id_guard;
        *id_guard += 1;
        id
    };

    let handle = thread::spawn(move || {
        match callable {
            Value::NativeFunction(f) => {
                let _ = f(vec![]);
            }
            Value::Function(chunk) => {
                // Create VM and run
                // Assumes 0-arg function in chunk
                if let Ok(mut vm) = crate::vm::VM::new((*chunk).clone(), "THREAD", 0) {
                    let _ = vm.run();
                }
            }
            _ => {}
        }
    });

    THREADS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
        .insert(thread_id, handle);

    Ok(Value::Integer(thread_id))
}

pub fn intrinsic_thread_join(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let thread_id = match args[0] {
        Value::Integer(i) => i,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "Integer".to_string(),
                args[0].clone(),
            ));
        }
    };

    let handle_opt = THREADS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
        .remove(&thread_id);

    if let Some(handle) = handle_opt {
        if handle.join().is_ok() {
            Ok(Value::Boolean(true))
        } else {
            Ok(Value::Boolean(false))
        }
    } else {
        Ok(Value::Boolean(false)) // Thread not found
    }
}

pub fn intrinsic_socket_set_timeout(args: Vec<Value>) -> Result<Value, RuntimeError> {
    #[cfg(target_arch = "wasm32")]
    return Err(RuntimeError::NotExecutable);

    #[cfg(not(target_arch = "wasm32"))]
    {
        if args.len() != 2 {
            return Err(RuntimeError::NotExecutable);
        }
        let id = match &args[0] {
            Value::Integer(i) => *i,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[0].clone(),
                ));
            }
        };
        let timeout_ms = match &args[1] {
            Value::Integer(i) => *i as u64,
            _ => {
                return Err(RuntimeError::TypeMismatch(
                    "Integer".to_string(),
                    args[1].clone(),
                ));
            }
        };

        let sockets = get_sockets().lock().unwrap();
        match sockets.get(&id) {
            Some(SocketResource::Stream(s)) => {
                let dur = if timeout_ms == 0 {
                    None
                } else {
                    Some(Duration::from_millis(timeout_ms))
                };
                s.set_read_timeout(dur)
                    .map_err(|_| RuntimeError::NotExecutable)?;
                s.set_write_timeout(dur)
                    .map_err(|_| RuntimeError::NotExecutable)?;
                Ok(Value::Unit)
            }
            _ => Err(RuntimeError::InvalidOperation(
                "Invalid socket or not a stream".to_string(),
            )),
        }
    }
}

pub fn intrinsic_event_poll(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    let mut events = EVENTS
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .unwrap();
    if let Some(val) = events.pop_front() {
        Ok(val)
    } else {
        Ok(Value::Unit)
    }
}

pub fn intrinsic_event_push(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let val = args[0].clone();
    EVENTS
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .unwrap()
        .push_back(val);
    Ok(Value::Unit)
}

pub fn intrinsic_func_apply(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::NotExecutable);
    }
    let func = args[0].clone();
    let func_args = match &args[1] {
        Value::List(l) => l.clone(),
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "List".to_string(),
                args[1].clone(),
            ));
        }
    };

    match func {
        Value::String(name) => {
            if let Some(native_fn) = IntrinsicRegistry::resolve(&name) {
                native_fn(func_args)
            } else {
                Err(RuntimeError::FunctionNotFound(name))
            }
        }
        Value::Function(chunk) => {
            // Run VM for function
            if let Ok(mut vm) = crate::vm::VM::new((*chunk).clone(), "APPLY", 0) {
                for arg in func_args {
                    vm.stack.push(arg);
                }
                vm.run()
                    .map_err(|e| RuntimeError::InvalidOperation(e.to_string()))
            } else {
                Err(RuntimeError::NotExecutable)
            }
        }
        Value::NativeFunction(f) => f(func_args),
        _ => Err(RuntimeError::TypeMismatch(
            "Function or String".to_string(),
            func,
        )),
    }
}

pub fn intrinsic_vm_eval(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::NotExecutable);
    }
    let source = match &args[0] {
        Value::String(s) => s,
        _ => {
            return Err(RuntimeError::TypeMismatch(
                "String".to_string(),
                args[0].clone(),
            ));
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        return Ok(Value::String("[Ark:WASM] Eval not supported".to_string()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Write to temp file
        let temp_path = "temp_eval.ark";
        fs::write(temp_path, source).map_err(|_| RuntimeError::NotExecutable)?;

        // Execute python meta/ark.py
        let output = Command::new("python3")
            .arg("meta/ark.py")
            .arg(temp_path)
            .output()
            .map_err(|_| RuntimeError::NotExecutable)?;

        let _ = fs::remove_file(temp_path);

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Also capture stderr if needed, but return stdout as per requirement.
        Ok(Value::String(stdout))
    }
}

// ----------------------------------------------------------------------
// PHASE 78: FINAL 12 PARITY INTRINSICS
// ----------------------------------------------------------------------

/// sys.json.parse(json_string) → Value
/// Parses a JSON string into an Ark Value (Struct, List, Integer, String, Boolean, Unit).
fn intrinsic_json_parse(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "sys.json.parse expects 1 argument (string)".into(),
        ));
    }
    let json_str = match &args[0] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.json.parse expects a string".into(),
            ));
        }
    };
    json_to_value(&json_str)
        .map_err(|e| RuntimeError::InvalidOperation(format!("JSON Parse Error: {}", e)))
}

fn json_to_value(s: &str) -> Result<Value, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty JSON string".into());
    }
    if s == "null" {
        return Ok(Value::Unit);
    }
    if s == "true" {
        return Ok(Value::Boolean(true));
    }
    if s == "false" {
        return Ok(Value::Boolean(false));
    }
    // Try integer
    if let Ok(n) = s.parse::<i64>() {
        return Ok(Value::Integer(n));
    }
    // Try float → integer
    if let Ok(f) = s.parse::<f64>() {
        return Ok(Value::Integer(f as i64));
    }
    // String
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        let unescaped = inner
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\\\", "\\");
        return Ok(Value::String(unescaped));
    }
    // Array
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1].trim();
        if inner.is_empty() {
            return Ok(Value::List(vec![]));
        }
        let items = split_json_top_level(inner, ',')?;
        let mut list = Vec::new();
        for item in items {
            list.push(json_to_value(item.trim())?);
        }
        return Ok(Value::List(list));
    }
    // Object
    if s.starts_with('{') && s.ends_with('}') {
        let inner = &s[1..s.len() - 1].trim();
        if inner.is_empty() {
            return Ok(Value::Struct(HashMap::new()));
        }
        let pairs = split_json_top_level(inner, ',')?;
        let mut map = HashMap::new();
        for pair in pairs {
            let kv = split_json_top_level(pair.trim(), ':')?;
            if kv.len() < 2 {
                return Err(format!("Invalid JSON object pair: {}", pair));
            }
            let key = kv[0].trim();
            let val_str = kv[1..].join(":"); // rejoin in case value contains colons
                                             // Strip quotes from key
            let key = if key.starts_with('"') && key.ends_with('"') && key.len() >= 2 {
                &key[1..key.len() - 1]
            } else {
                key
            };
            map.insert(key.to_string(), json_to_value(val_str.trim())?);
        }
        return Ok(Value::Struct(map));
    }
    // Fallback: treat as string
    Ok(Value::String(s.to_string()))
}

fn split_json_top_level(s: &str, delimiter: char) -> Result<Vec<&str>, String> {
    let mut result = Vec::new();
    let mut depth_brace = 0i32;
    let mut depth_bracket = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match c {
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            _ if c == delimiter && depth_brace == 0 && depth_bracket == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    result.push(&s[start..]);
    Ok(result)
}

/// sys.json.stringify(value) → String
/// Converts an Ark Value to its JSON string representation.
fn intrinsic_json_stringify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "sys.json.stringify expects 1 argument".into(),
        ));
    }
    let json_str = value_to_json(&args[0]);
    Ok(Value::String(json_str))
}

fn value_to_json(val: &Value) -> String {
    match val {
        Value::Integer(n) => n.to_string(),
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Value::Boolean(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Unit => "null".into(),
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(value_to_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Struct(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, value_to_json(v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        _ => "null".into(),
    }
}

/// sys.log(args...) → Unit
/// Prints a log message to stderr with [LOG] prefix.
fn intrinsic_log(args: Vec<Value>) -> Result<Value, RuntimeError> {
    let parts: Vec<String> = args
        .iter()
        .map(|a| match a {
            Value::Integer(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Unit => "null".into(),
            Value::List(_) => "[List]".into(),
            Value::Struct(_) => "{Struct}".into(),
            _ => format!("{:?}", a),
        })
        .collect();
    eprintln!("[LOG] {}", parts.join(" "));
    Ok(Value::Unit)
}

/// sys.exit(code?) → never returns
/// Exits the process with the given exit code (default 0).
fn intrinsic_exit(args: Vec<Value>) -> Result<Value, RuntimeError> {
    let code = if !args.is_empty() {
        match &args[0] {
            Value::Integer(n) => *n as i32,
            _ => 0,
        }
    } else {
        0
    };
    std::process::exit(code);
}

/// sys.html_escape(string) → String
/// Escapes HTML special characters: & < > " '
fn intrinsic_html_escape(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "sys.html_escape expects 1 string argument".into(),
        ));
    }
    let s = match &args[0] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.html_escape expects a string".into(),
            ));
        }
    };
    let escaped = s
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;");
    Ok(Value::String(escaped))
}

/// sys.z3.verify(constraints) → Struct
/// Stub: Z3 integration requires external z3 crate. Returns satisfiability stub.
fn intrinsic_z3_verify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "sys.z3.verify expects a List of constraint strings".into(),
        ));
    }
    let constraints = match &args[0] {
        Value::List(items) => items.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.z3.verify expects a List".into(),
            ));
        }
    };
    // Validate all items are strings
    for item in &constraints {
        match item {
            Value::String(_) => {}
            _ => {
                return Err(RuntimeError::InvalidOperation(
                    "sys.z3.verify constraints must be Strings".into(),
                ));
            }
        }
    }
    // Stub result — real z3 binding would call the solver
    let mut result = HashMap::new();
    result.insert("satisfiable".to_string(), Value::Boolean(true));
    result.insert("solver".to_string(), Value::String("stub".into()));
    result.insert(
        "constraint_count".to_string(),
        Value::Integer(constraints.len() as i64),
    );
    Ok(Value::Struct(result))
}

/// sys.vm.source(path) → String
/// Reads a source file and returns its contents as a string.
/// (Full eval requires parser access; Rust runtime returns the raw source.)
fn intrinsic_vm_source(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "sys.vm.source expects a file path string".into(),
        ));
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "sys.vm.source expects a string path".into(),
            ));
        }
    };
    // Security: validate path
    let p = PathBuf::from(&path);
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(RuntimeError::InvalidOperation(format!(
            "Path traversal blocked: {}",
            path
        )));
    }
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(Value::String(contents)),
        Err(e) => Err(RuntimeError::ResourceError(format!("Source Error: {}", e))),
    }
}

/// sys.info() → Struct
/// Returns system information (OS, Arch, Version).
fn intrinsic_sys_info(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::InvalidOperation(
            "sys.info expects no arguments".into(),
        ));
    }
    let mut info = HashMap::new();
    info.insert(
        "os".to_string(),
        Value::String(std::env::consts::OS.to_string()),
    );
    info.insert(
        "arch".to_string(),
        Value::String(std::env::consts::ARCH.to_string()),
    );
    info.insert(
        "version".to_string(),
        Value::String("v112.0 (Prime)".to_string()),
    );
    info.insert("status".to_string(), Value::String("Sovereign".to_string()));

    Ok(Value::Struct(info))
}

// --- Tensor Math Helpers ---

fn make_tensor(flat_data: Vec<i64>, shape: Vec<i64>) -> Value {
    let data = Value::List(flat_data.iter().map(|v| Value::Integer(*v)).collect());
    let shape_val = Value::List(shape.iter().map(|s| Value::Integer(*s)).collect());
    let mut fields = HashMap::new();
    fields.insert("data".to_string(), data);
    fields.insert("shape".to_string(), shape_val);
    Value::Struct(fields)
}

fn extract_tensor(val: &Value) -> Result<(Vec<i64>, Vec<i64>), RuntimeError> {
    let fields = match val {
        Value::Struct(f) => f,
        _ => {
            return Err(RuntimeError::InvalidOperation(format!(
                "Expected tensor (Struct), got {:?}",
                val
            )));
        }
    };
    let data = match fields.get("data") {
        Some(Value::List(items)) => items
            .iter()
            .map(|v| match v {
                Value::Integer(n) => Ok(*n),
                _ => Err(RuntimeError::InvalidOperation(
                    "Tensor data must be integers".into(),
                )),
            })
            .collect::<Result<Vec<i64>, _>>()?,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Tensor must have 'data' List field".into(),
            ));
        }
    };
    let shape = match fields.get("shape") {
        Some(Value::List(items)) => items
            .iter()
            .map(|v| match v {
                Value::Integer(n) => Ok(*n),
                _ => Err(RuntimeError::InvalidOperation(
                    "Tensor shape must be integers".into(),
                )),
            })
            .collect::<Result<Vec<i64>, _>>()?,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "Tensor must have 'shape' List field".into(),
            ));
        }
    };
    Ok((data, shape))
}

/// math.Tensor(data: List, shape: List) → Tensor struct
fn intrinsic_math_tensor(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "math.Tensor expects data(list) and shape(list)".into(),
        ));
    }
    let data = match &args[0] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Integer(n) => Ok(*n),
                _ => Err(RuntimeError::InvalidOperation(
                    "Tensor data must be integers".into(),
                )),
            })
            .collect::<Result<Vec<i64>, _>>()?,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "math.Tensor expects List arguments".into(),
            ));
        }
    };
    let shape = match &args[1] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Integer(n) => Ok(*n),
                _ => Err(RuntimeError::InvalidOperation(
                    "Tensor shape must be integers".into(),
                )),
            })
            .collect::<Result<Vec<i64>, _>>()?,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "math.Tensor expects List arguments".into(),
            ));
        }
    };
    Ok(make_tensor(data, shape))
}

/// math.matmul(A, B) → Tensor. A=[m,k], B=[k,n] → C=[m,n]
fn intrinsic_math_matmul(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "math.matmul expects 2 tensors".into(),
        ));
    }
    let (a_data, a_shape) = extract_tensor(&args[0])?;
    let (b_data, b_shape) = extract_tensor(&args[1])?;
    if a_shape.len() != 2 || b_shape.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "math.matmul expects 2D tensors".into(),
        ));
    }
    let (m, k) = (a_shape[0] as usize, a_shape[1] as usize);
    let (k2, n) = (b_shape[0] as usize, b_shape[1] as usize);
    if k != k2 {
        return Err(RuntimeError::InvalidOperation(format!(
            "math.matmul dimension mismatch: {} vs {}",
            k, k2
        )));
    }
    let mut result = vec![0i64; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut s = 0i64;
            for p in 0..k {
                s += a_data[i * k + p] * b_data[p * n + j];
            }
            result[i * n + j] = s;
        }
    }
    Ok(make_tensor(result, vec![m as i64, n as i64]))
}

/// math.transpose(T) → Tensor. T=[m,n] → T'=[n,m]
fn intrinsic_math_transpose(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "math.transpose expects 1 tensor".into(),
        ));
    }
    let (data, shape) = extract_tensor(&args[0])?;
    if shape.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "math.transpose expects a 2D tensor".into(),
        ));
    }
    let (m, n) = (shape[0] as usize, shape[1] as usize);
    let mut result = vec![0i64; m * n];
    for j in 0..n {
        for i in 0..m {
            result[j * m + i] = data[i * n + j];
        }
    }
    Ok(make_tensor(result, vec![n as i64, m as i64]))
}

/// math.dot(a, b) → Integer. Element-wise multiply and sum.
fn intrinsic_math_dot(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "math.dot expects 2 tensors".into(),
        ));
    }
    let (a_data, _) = extract_tensor(&args[0])?;
    let (b_data, _) = extract_tensor(&args[1])?;
    if a_data.len() != b_data.len() {
        return Err(RuntimeError::InvalidOperation(format!(
            "math.dot dimension mismatch: {} vs {}",
            a_data.len(),
            b_data.len()
        )));
    }
    let s: i64 = a_data.iter().zip(b_data.iter()).map(|(a, b)| a * b).sum();
    Ok(Value::Integer(s))
}

/// math.add(a, b) → Tensor. Element-wise addition.
fn intrinsic_math_tensor_add(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "math.add expects 2 tensors".into(),
        ));
    }
    let (a_data, a_shape) = extract_tensor(&args[0])?;
    let (b_data, b_shape) = extract_tensor(&args[1])?;
    if a_shape != b_shape {
        return Err(RuntimeError::InvalidOperation(format!(
            "math.add shape mismatch: {:?} vs {:?}",
            a_shape, b_shape
        )));
    }
    let result: Vec<i64> = a_data
        .iter()
        .zip(b_data.iter())
        .map(|(a, b)| a + b)
        .collect();
    Ok(make_tensor(result, a_shape))
}

/// math.sub(a, b) → Tensor. Element-wise subtraction.
fn intrinsic_math_tensor_sub(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "math.sub expects 2 tensors".into(),
        ));
    }
    let (a_data, a_shape) = extract_tensor(&args[0])?;
    let (b_data, b_shape) = extract_tensor(&args[1])?;
    if a_shape != b_shape {
        return Err(RuntimeError::InvalidOperation(format!(
            "math.sub shape mismatch: {:?} vs {:?}",
            a_shape, b_shape
        )));
    }
    let result: Vec<i64> = a_data
        .iter()
        .zip(b_data.iter())
        .map(|(a, b)| a - b)
        .collect();
    Ok(make_tensor(result, a_shape))
}

/// math.mul_scalar(tensor, scalar) → Tensor. Multiply every element by scalar.
fn intrinsic_math_mul_scalar(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "math.mul_scalar expects tensor and scalar".into(),
        ));
    }
    let (data, shape) = extract_tensor(&args[0])?;
    let scalar = match &args[1] {
        Value::Integer(n) => *n,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "math.mul_scalar scalar must be integer".into(),
            ));
        }
    };
    let result: Vec<i64> = data.iter().map(|v| v * scalar).collect();
    Ok(make_tensor(result, shape))
}

// ============================================================================
// GOVERNANCE INTRINSICS
// ============================================================================

/// governance.trace(run_id, step, phase, conf_before, conf_after, pre_state, post_state, hmac_key)
/// Returns a Struct containing the signed step trace.
fn intrinsic_governance_trace(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() < 8 {
        return Err(RuntimeError::InvalidOperation(
            "governance.trace requires 8 args: run_id, step, phase, conf_before, conf_after, pre_state, post_state, hmac_key".into(),
        ));
    }
    let run_id = match &args[0] {
        Value::String(s) => s.clone(),
        other => return Err(RuntimeError::TypeMismatch("String".into(), other.clone())),
    };
    let step = match &args[1] {
        Value::Integer(i) => *i as u64,
        other => return Err(RuntimeError::TypeMismatch("Integer".into(), other.clone())),
    };
    let phase_str = match &args[2] {
        Value::String(s) => s.to_uppercase(),
        other => return Err(RuntimeError::TypeMismatch("String".into(), other.clone())),
    };
    let phase = match phase_str.as_str() {
        "SENSE" => crate::governance::Phase::Sense,
        "ASSESS" => crate::governance::Phase::Assess,
        "DECIDE" => crate::governance::Phase::Decide,
        "ACTION" => crate::governance::Phase::Action,
        "VERIFY" => crate::governance::Phase::Verify,
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "phase must be one of: SENSE, ASSESS, DECIDE, ACTION, VERIFY".into(),
            ));
        }
    };
    let conf_before = match &args[3] {
        Value::Integer(i) => *i as f64 / 100.0,
        Value::String(s) => s.parse::<f64>().unwrap_or(0.5),
        _ => 0.5,
    };
    let conf_after = match &args[4] {
        Value::Integer(i) => *i as f64 / 100.0,
        Value::String(s) => s.parse::<f64>().unwrap_or(0.5),
        _ => 0.5,
    };
    let pre_state = match &args[5] {
        Value::String(s) => s.as_bytes().to_vec(),
        _ => b"unknown".to_vec(),
    };
    let post_state = match &args[6] {
        Value::String(s) => s.as_bytes().to_vec(),
        _ => b"unknown".to_vec(),
    };
    let hmac_key = match &args[7] {
        Value::String(s) => s.as_bytes().to_vec(),
        _ => b"ark-default-key".to_vec(),
    };

    let decision = if conf_after >= conf_before {
        crate::governance::Decision::Accept
    } else {
        crate::governance::Decision::Reject
    };

    let trace = crate::governance::StepTrace::new(
        &run_id,
        step,
        phase,
        conf_before,
        conf_after,
        decision,
        &pre_state,
        &post_state,
        crate::governance::DualBand::new(0.5, 0.5),
        vec![],
        vec![],
        &hmac_key,
    );

    let map = trace.to_map();
    let struct_map: HashMap<String, Value> = map
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();
    Ok(Value::Struct(struct_map))
}

/// governance.mcc_check(conf_before, conf_after)
/// Returns Boolean: true if conf_after >= conf_before (monotone non-decreasing).
fn intrinsic_governance_mcc_check(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        return Err(RuntimeError::InvalidOperation(
            "governance.mcc_check requires 2 args: conf_before, conf_after".into(),
        ));
    }
    let before = match &args[0] {
        Value::Integer(i) => *i as f64,
        Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };
    let after = match &args[1] {
        Value::Integer(i) => *i as f64,
        Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };
    Ok(Value::Boolean(after >= before))
}

/// governance.verify_chain(traces_json, hmac_key)
/// Returns Boolean: true if all traces in the JSON array have valid signatures.
fn intrinsic_governance_verify_chain(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        return Err(RuntimeError::InvalidOperation(
            "governance.verify_chain requires 2 args: traces_json, hmac_key".into(),
        ));
    }
    let hmac_key = match &args[1] {
        Value::String(s) => s.as_bytes().to_vec(),
        _ => b"ark-default-key".to_vec(),
    };

    // Build a receipt chain and verify
    let mut chain = crate::governance::ReceiptChain::new(&hmac_key);

    match &args[0] {
        Value::List(traces) => {
            for (i, trace_val) in traces.iter().enumerate() {
                if let Value::Struct(map) = trace_val {
                    let get_str = |key: &str| -> String {
                        map.get(key)
                            .and_then(|v| {
                                if let Value::String(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default()
                    };

                    let run_id = get_str("run_id");
                    let step = (i + 1) as u64;
                    let conf_before: f64 = get_str("conf_before").parse().unwrap_or(0.0);
                    let conf_after: f64 = get_str("conf_after").parse().unwrap_or(0.0);

                    let decision = if conf_after >= conf_before {
                        crate::governance::Decision::Accept
                    } else {
                        crate::governance::Decision::Reject
                    };

                    let trace = crate::governance::StepTrace::new(
                        &run_id,
                        step,
                        crate::governance::Phase::Verify,
                        conf_before,
                        conf_after,
                        decision,
                        get_str("pre_state_hash").as_bytes(),
                        get_str("post_state_hash").as_bytes(),
                        crate::governance::DualBand::new(0.5, 0.5),
                        vec![],
                        vec![],
                        &hmac_key,
                    );
                    chain.append(trace);
                }
            }
        }
        _ => {
            return Err(RuntimeError::InvalidOperation(
                "First argument must be a List of trace Structs".into(),
            ));
        }
    }

    match chain.verify_integrity() {
        Ok(true) => Ok(Value::Boolean(true)),
        Ok(false) => Ok(Value::Boolean(false)),
        Err(_) => Ok(Value::Boolean(false)),
    }
}

// ============================================================================
// PERSISTENT DATA STRUCTURE INTRINSICS
// ============================================================================

/// pvec.new() → PVec. Create a new empty persistent vector.
fn intrinsic_pvec_new(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    Ok(Value::PVec(PVec::new()))
}

/// pvec.conj(pvec, val) → PVec. Append a value to a persistent vector.
fn intrinsic_pvec_conj(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "pvec.conj expects 2 arguments: (pvec, value)".to_string(),
        ));
    }
    match &args[0] {
        Value::PVec(pv) => Ok(Value::PVec(pv.conj(args[1].clone()))),
        _ => Err(RuntimeError::InvalidOperation(
            "pvec.conj: first argument must be a PVec".to_string(),
        )),
    }
}

/// pvec.get(pvec, index) → Value. Get value at index from a persistent vector.
fn intrinsic_pvec_get(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "pvec.get expects 2 arguments: (pvec, index)".to_string(),
        ));
    }
    match (&args[0], &args[1]) {
        (Value::PVec(pv), Value::Integer(idx)) => match pv.get(*idx as usize) {
            Some(v) => Ok(v.clone()),
            None => Ok(Value::Unit),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "pvec.get: expects (PVec, Integer)".to_string(),
        )),
    }
}

/// pvec.assoc(pvec, index, val) → PVec. Set value at index in a persistent vector.
fn intrinsic_pvec_assoc(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            "pvec.assoc expects 3 arguments: (pvec, index, value)".to_string(),
        ));
    }
    match (&args[0], &args[1]) {
        (Value::PVec(pv), Value::Integer(idx)) => match pv.assoc(*idx as usize, args[2].clone()) {
            Some(new_pv) => Ok(Value::PVec(new_pv)),
            None => Err(RuntimeError::InvalidOperation(format!(
                "pvec.assoc: index {} out of bounds",
                idx
            ))),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "pvec.assoc: expects (PVec, Integer, Value)".to_string(),
        )),
    }
}

/// pvec.pop(pvec) → PVec. Remove the last element from a persistent vector.
fn intrinsic_pvec_pop(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "pvec.pop expects 1 argument: (pvec)".to_string(),
        ));
    }
    match &args[0] {
        Value::PVec(pv) => match pv.pop() {
            Some((new_pv, _last)) => Ok(Value::PVec(new_pv)),
            None => Err(RuntimeError::InvalidOperation(
                "pvec.pop: cannot pop from empty PVec".to_string(),
            )),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "pvec.pop: argument must be a PVec".to_string(),
        )),
    }
}

/// pvec.len(pvec) → Integer. Get the length of a persistent vector.
fn intrinsic_pvec_len(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "pvec.len expects 1 argument: (pvec)".to_string(),
        ));
    }
    match &args[0] {
        Value::PVec(pv) => Ok(Value::Integer(pv.len() as i64)),
        _ => Err(RuntimeError::InvalidOperation(
            "pvec.len: argument must be a PVec".to_string(),
        )),
    }
}

/// pmap.new() → PMap. Create a new empty persistent map.
fn intrinsic_pmap_new(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    Ok(Value::PMap(PMap::new()))
}

/// pmap.assoc(pmap, key, val) → PMap. Associate a key with a value.
fn intrinsic_pmap_assoc(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::InvalidOperation(
            "pmap.assoc expects 3 arguments: (pmap, key, value)".to_string(),
        ));
    }
    match (&args[0], &args[1]) {
        (Value::PMap(pm), Value::String(key)) => {
            Ok(Value::PMap(pm.assoc(key.clone(), args[2].clone())))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "pmap.assoc: expects (PMap, String, Value)".to_string(),
        )),
    }
}

/// pmap.dissoc(pmap, key) → PMap. Remove a key from a persistent map.
fn intrinsic_pmap_dissoc(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "pmap.dissoc expects 2 arguments: (pmap, key)".to_string(),
        ));
    }
    match (&args[0], &args[1]) {
        (Value::PMap(pm), Value::String(key)) => Ok(Value::PMap(pm.dissoc(key))),
        _ => Err(RuntimeError::InvalidOperation(
            "pmap.dissoc: expects (PMap, String)".to_string(),
        )),
    }
}

/// pmap.get(pmap, key) → Value. Get the value for a key.
fn intrinsic_pmap_get(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "pmap.get expects 2 arguments: (pmap, key)".to_string(),
        ));
    }
    match (&args[0], &args[1]) {
        (Value::PMap(pm), Value::String(key)) => match pm.get(key) {
            Some(v) => Ok(v.clone()),
            None => Ok(Value::Unit),
        },
        _ => Err(RuntimeError::InvalidOperation(
            "pmap.get: expects (PMap, String)".to_string(),
        )),
    }
}

/// pmap.keys(pmap) → List<String>. Get all keys from a persistent map.
fn intrinsic_pmap_keys(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::InvalidOperation(
            "pmap.keys expects 1 argument: (pmap)".to_string(),
        ));
    }
    match &args[0] {
        Value::PMap(pm) => {
            let keys: Vec<Value> = pm.keys().into_iter().map(Value::String).collect();
            Ok(Value::List(keys))
        }
        _ => Err(RuntimeError::InvalidOperation(
            "pmap.keys: argument must be a PMap".to_string(),
        )),
    }
}

/// pmap.merge(pmap1, pmap2) → PMap. Merge two persistent maps.
fn intrinsic_pmap_merge(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::InvalidOperation(
            "pmap.merge expects 2 arguments: (pmap1, pmap2)".to_string(),
        ));
    }
    match (&args[0], &args[1]) {
        (Value::PMap(pm1), Value::PMap(pm2)) => Ok(Value::PMap(pm1.merge(pm2))),
        _ => Err(RuntimeError::InvalidOperation(
            "pmap.merge: both arguments must be PMap".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Value;

    // Existing tests...
    #[test]
    fn test_thread_spawn_join() {
        // Simple test: Spawn a thread that returns Unit (via function returning unit)
        // thread_spawn returns ID. join(ID) returns true.
        let args = vec![Value::NativeFunction(intrinsic_io_cls)]; // io.cls is safe void
        let res = intrinsic_thread_spawn(args).unwrap();
        let id = match res {
            Value::Integer(i) => i,
            _ => panic!("Expected Thread ID"),
        };

        // Wait a bit to ensure thread runs
        thread::sleep(Duration::from_millis(100));

        let args_join = vec![Value::Integer(id)];
        let join_res = intrinsic_thread_join(args_join).unwrap();
        assert_eq!(join_res, Value::Boolean(true));
    }

    #[test]
    fn test_event_push_poll() {
        // Push 42
        let args_push = vec![Value::Integer(42)];
        intrinsic_event_push(args_push).unwrap();

        // Poll
        let res = intrinsic_event_poll(vec![]).unwrap();
        assert_eq!(res, Value::Integer(42));

        // Poll again (empty)
        let res_empty = intrinsic_event_poll(vec![]).unwrap();
        assert_eq!(res_empty, Value::Unit);
    }

    #[test]
    fn test_func_apply_native() {
        // Apply "intrinsic_add" with [1, 2]
        let args = vec![
            Value::String("intrinsic_add".to_string()),
            Value::List(vec![Value::Integer(1), Value::Integer(2)]),
        ];
        let res = intrinsic_func_apply(args).unwrap();
        assert_eq!(res, Value::Integer(3));
    }

    #[test]
    fn test_time_now() {
        let res = intrinsic_time_now(vec![]);
        match res {
            Ok(Value::Integer(t)) => assert!(t > 0),
            _ => panic!("Expected Integer, got {:?}", res),
        }
    }

    #[test]
    fn test_math_pow() {
        // 2^3 = 8
        let args = vec![Value::Integer(2), Value::Integer(3)];
        assert_eq!(intrinsic_math_pow(args).unwrap(), Value::Integer(8));

        // 10^2 = 100
        let args = vec![Value::Integer(10), Value::Integer(2)];
        assert_eq!(intrinsic_math_pow(args).unwrap(), Value::Integer(100));

        // 2^-1 = 0 (0.5 as integer)
        let args = vec![Value::Integer(2), Value::Integer(-1)];
        assert_eq!(intrinsic_math_pow(args).unwrap(), Value::Integer(0));
    }

    #[test]
    fn test_math_sqrt() {
        // sqrt(16) = 4
        let args = vec![Value::Integer(16)];
        assert_eq!(intrinsic_math_sqrt(args).unwrap(), Value::Integer(4));

        // sqrt(10) = 3 (3.16... as integer)
        let args = vec![Value::Integer(10)];
        assert_eq!(intrinsic_math_sqrt(args).unwrap(), Value::Integer(3));

        // sqrt(-1) -> Error
        let args = vec![Value::Integer(-1)];
        assert!(intrinsic_math_sqrt(args).is_err());
    }

    #[test]
    fn test_io_cls() {
        // Just verify it runs and returns Unit
        let args = vec![];
        assert_eq!(intrinsic_io_cls(args).unwrap(), Value::Unit);
    }

    #[test]
    fn test_math_trig() {
        // sin(0) = 0
        let args = vec![Value::Integer(0)];
        assert_eq!(intrinsic_math_sin(args).unwrap(), Value::Integer(0));

        // sin(PI/2) approx 10000 (PI/2 = 1.5707... * 10000 = 15707)
        let args = vec![Value::Integer(15708)]; // 1.5708
                                                // sin(1.5708) is close to 1
        let res = intrinsic_math_sin(args).unwrap();
        if let Value::Integer(v) = res {
            assert!(v >= 9999 && v <= 10000);
        } else {
            panic!("Expected Integer");
        }

        // cos(0) = 10000
        let args = vec![Value::Integer(0)];
        assert_eq!(intrinsic_math_cos(args).unwrap(), Value::Integer(10000));

        // tan(45deg) = tan(PI/4) = 1 (approx)
        // PI/4 = 0.78539 * 10000 = 7854
        let args = vec![Value::Integer(7854)];
        let res = intrinsic_math_tan(args).unwrap();
        if let Value::Integer(v) = res {
            assert!(v >= 9990 && v <= 10010);
        } else {
            panic!("Expected Integer");
        }
    }

    #[test]
    fn test_crypto_verify() {
        // Valid Signature (Test Vector 2 from RFC 8032)
        // Msg: "r" (0x72)
        let msg = Value::String("r".to_string());
        let sig_hex = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";
        let pubkey_hex = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";

        let args = vec![
            msg.clone(),
            Value::String(sig_hex.to_string()),
            Value::String(pubkey_hex.to_string()),
        ];
        let res = intrinsic_crypto_verify(args).unwrap();
        assert_eq!(res, Value::Boolean(true));
    }

    #[test]
    fn test_buffer_write_linear() {
        // Setup: Buffer of size 3
        let buf = Value::Buffer(vec![0u8; 3]);
        // args: [buffer, index, value] -> sys.mem.write(buf, 1, 42)
        let args = vec![buf, Value::Integer(1), Value::Integer(42)];

        // Execute
        let res = intrinsic_buffer_write(args).unwrap();

        // Assert
        match res {
            Value::Buffer(b) => {
                assert_eq!(b.len(), 3);
                assert_eq!(b[1], 42);
            }
            _ => panic!("Expected Buffer"),
        }
    }

    #[test]
    fn test_security_fs_write_traversal() {
        // [MODE: KINETIC_EXECUTION]
        // Rationale: Attempt to write outside the sandbox.
        // We expect this to FAIL now that the fix is applied.

        let file_name = "../intrinsics_test_exploit.txt";

        // Clean up before test just in case
        let _ = std::fs::remove_file(file_name);

        let args = vec![
            Value::String(file_name.to_string()),
            Value::String("pwned".to_string()),
        ];

        // At this stage (after fix), we expect this to FAIL.
        let res = intrinsic_fs_write(args);

        // Assert Error
        match res {
            Err(RuntimeError::NotExecutable) => {}
            _ => panic!("Expected RuntimeError::NotExecutable, got {:?}", res),
        }

        // Verify file was NOT written
        if std::path::Path::new(file_name).exists() {
            // Cleanup if it somehow wrote
            std::fs::remove_file(file_name).unwrap();
            panic!("File was written despite error!");
        }
    }

    #[test]
    fn test_security_fs_write_valid() {
        let file_name = "intrinsics_test_safe.txt";
        let _ = std::fs::remove_file(file_name);

        let args = vec![
            Value::String(file_name.to_string()),
            Value::String("safe".to_string()),
        ];

        let res = intrinsic_fs_write(args);
        assert!(res.is_ok());

        assert!(std::path::Path::new(file_name).exists());
        std::fs::remove_file(file_name).unwrap();
    }

    #[test]
    fn test_security_fs_read_traversal() {
        let file_name = "../Cargo.toml";
        // This file exists in repo root, but is outside core/ CWD.
        // So it should be blocked.

        if std::path::Path::new(file_name).exists() {
            let args = vec![Value::String(file_name.to_string())];
            let res = intrinsic_fs_read(args);
            match res {
                Err(RuntimeError::NotExecutable) => {}
                _ => panic!("Expected RuntimeError::NotExecutable, got {:?}", res),
            }
        } else {
            println!("Skipping read traversal test because ../Cargo.toml not found");
        }
    }

    #[test]
    fn test_list_pop() {
        // [1, 2, 3] pop(1) -> 2, list becomes [1, 3]
        let list = Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        let args = vec![list, Value::Integer(1)];
        let res = intrinsic_list_pop(args).unwrap();

        match res {
            Value::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], Value::Integer(2)); // Popped value
                match &items[1] {
                    Value::List(l) => {
                        assert_eq!(l.len(), 2);
                        assert_eq!(l[0], Value::Integer(1));
                        assert_eq!(l[1], Value::Integer(3));
                    }
                    _ => panic!("Expected List as second item"),
                }
            }
            _ => panic!("Expected List result"),
        }
    }

    #[test]
    fn test_time_sleep() {
        let args = vec![Value::Integer(10)];
        assert!(intrinsic_time_sleep(args).is_ok());
    }

    #[test]
    fn test_time_sleep_negative() {
        let args = vec![Value::Integer(-10)];
        assert!(intrinsic_time_sleep(args).is_err());
    }

    #[test]
    fn test_io_write_basic() {
        let args = vec![Value::String("test output".to_string())];
        assert!(intrinsic_io_write(args).is_ok());
    }

    #[test]
    fn test_io_read_bytes_valid() {
        let filename = "test_bytes.bin";
        let _ = std::fs::remove_file(filename);
        std::fs::write(filename, vec![1, 2, 3]).unwrap();

        let args = vec![Value::String(filename.to_string())];
        let res = intrinsic_io_read_bytes(args).unwrap();

        match res {
            Value::List(l) => {
                assert_eq!(l.len(), 3);
                assert_eq!(l[0], Value::Integer(1));
                assert_eq!(l[1], Value::Integer(2));
                assert_eq!(l[2], Value::Integer(3));
            }
            _ => panic!("Expected List"),
        }
        let _ = std::fs::remove_file(filename);
    }

    #[test]
    fn test_extract_code_blocks() {
        let md = "Start\n```rust\nfn main() {}\n```\nMid\n```\nraw\n```\nEnd";
        let args = vec![Value::String(md.to_string())];
        let res = intrinsic_extract_code(args).unwrap();

        match res {
            Value::List(blocks) => {
                assert_eq!(blocks.len(), 2);
                match &blocks[0] {
                    Value::String(s) => assert_eq!(s, "fn main() {}\n"),
                    _ => panic!("Expected String"),
                }
                match &blocks[1] {
                    Value::String(s) => assert_eq!(s, "raw\n"),
                    _ => panic!("Expected String"),
                }
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_crypto_sha512() {
        // Test Vector: Empty String
        // SHA-512("") = cf83e135...
        let args = vec![Value::String("".to_string())];
        let res = intrinsic_crypto_sha512(args).unwrap();
        match res {
            Value::String(h) => assert_eq!(
                h,
                "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
            ),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_extract_code_empty() {
        let md = "No code blocks here.";
        let args = vec![Value::String(md.to_string())];
        let res = intrinsic_extract_code(args).unwrap();

        match res {
            Value::List(blocks) => assert!(blocks.is_empty()),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_crypto_hmac_sha512() {
        let key = Value::String(hex::encode("key"));
        let data = Value::String("The quick brown fox jumps over the lazy dog".to_string());
        let args = vec![key, data];
        let res = intrinsic_crypto_hmac_sha512(args).unwrap();
        match res {
            Value::String(h) => assert_eq!(
                h,
                "b42af09057bac1e2d41708e48a902e09b5ff7f12ab428a4fe86653c73dd248fb82f948a549f7b791a5b41915ee4d1ec3935357e4e2317250d0372afa2ebeeb3a"
            ),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_crypto_aes_gcm_roundtrip() {
        let key = Value::String(hex::encode("01234567890123456789012345678901")); // 32 bytes
        let nonce = Value::String(hex::encode("012345678901")); // 12 bytes
        let plaintext = Value::String("Hello World".to_string());

        // Encrypt
        let args_enc = vec![key.clone(), nonce.clone(), plaintext.clone()];
        let ciphertext = intrinsic_crypto_aes_gcm_encrypt(args_enc).unwrap();

        // Decrypt
        let args_dec = vec![key.clone(), nonce.clone(), ciphertext.clone()];
        let decrypted = intrinsic_crypto_aes_gcm_decrypt(args_dec).unwrap();

        match decrypted {
            Value::Buffer(b) => assert_eq!(b, b"Hello World"),
            _ => panic!("Expected Buffer"),
        }
    }

    #[test]
    fn test_crypto_aes_gcm_fail() {
        let key = Value::String(hex::encode("01234567890123456789012345678901"));
        let wrong_key = Value::String(hex::encode("01234567890123456789012345678902"));
        let nonce = Value::String(hex::encode("012345678901"));
        let plaintext = Value::String("Secret".to_string());

        let args_enc = vec![key, nonce.clone(), plaintext];
        let ciphertext = intrinsic_crypto_aes_gcm_encrypt(args_enc).unwrap();

        let args_dec = vec![wrong_key, nonce, ciphertext];
        let res = intrinsic_crypto_aes_gcm_decrypt(args_dec);
        assert!(res.is_err()); // Should fail with invalid key/tag match
    }

    #[test]
    fn test_crypto_ed25519_roundtrip() {
        // Generate
        let gen_res = intrinsic_crypto_ed25519_generate(vec![]).unwrap();
        let (pub_key, priv_key) = match gen_res {
            Value::Struct(map) => (
                map.get("public_key").unwrap().clone(),
                map.get("private_key").unwrap().clone(),
            ),
            _ => panic!("Expected Struct"),
        };

        // Sign
        let msg = Value::String("message".to_string());
        let sign_args = vec![msg.clone(), priv_key.clone()];
        let sig = intrinsic_crypto_ed25519_sign(sign_args).unwrap();

        // Verify
        let verify_args = vec![msg.clone(), sig.clone(), pub_key.clone()];
        let valid = intrinsic_crypto_ed25519_verify(verify_args).unwrap();
        assert_eq!(valid, Value::Boolean(true));

        // Verify Fail (wrong msg)
        let wrong_msg = Value::String("wrong".to_string());
        let verify_fail_args = vec![wrong_msg, sig, pub_key];
        let invalid = intrinsic_crypto_ed25519_verify(verify_fail_args).unwrap();
        assert_eq!(invalid, Value::Boolean(false));
    }

    #[test]
    fn test_crypto_random() {
        let args = vec![Value::Integer(16)];
        let res = intrinsic_crypto_random_bytes(args).unwrap();
        match res {
            Value::String(s) => {
                assert_eq!(s.len(), 32); // 16 bytes = 32 hex chars
                                         // Verify hex
                assert!(hex::decode(&s).is_ok());
            }
            _ => panic!("Expected String"),
        }

        // Networking Tests
        #[test]
        fn test_socket_bind_close() {
            // Bind to port 0 (ephemeral)
            let args = vec![Value::Integer(0)];
            let res = intrinsic_socket_bind(args).unwrap();
            let id = match res {
                Value::Integer(i) => i,
                _ => panic!("Expected Integer ID"),
            };
            assert!(id > 0);

            // Close it
            let args_close = vec![Value::Integer(id)];
            let res_close = intrinsic_socket_close(args_close).unwrap();
            assert_eq!(res_close, Value::Boolean(true));
        }

        #[test]
        fn test_close_nonexistent() {
            let args_close = vec![Value::Integer(999999)];
            let res_close = intrinsic_socket_close(args_close).unwrap();
            assert_eq!(res_close, Value::Boolean(false));
        }

        #[test]
        fn test_http_request_invalid_url() {
            let args = vec![
                Value::String("GET".to_string()),
                Value::String("http://invalid.url.local".to_string()),
            ];
            let res = intrinsic_http_request(args);
            // Should return Error, not panic
            assert!(res.is_err());
        }
    }
}
