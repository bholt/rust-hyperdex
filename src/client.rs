use std::io::net::ip::SocketAddr;
use std::os::{num_cpus, errno};
use std::comm::{Empty, Disconnected};
use std::ptr::null_mut;
use std::collections::HashMap;
use std::c_str::CString;
use std::vec::raw::from_buf;
use std::path::BytesContainer;

use sync::deque::{BufferPool, Stealer, Worker};
use sync::{Arc, Mutex};

use libc::*;

use hyperdex::*;
use hyperdex_client::*;
use hyperdex_datastructures::*;

enum HyperValue {
    HyperString(Vec<u8>),
    HyperInt(i64),
    HyperFloat(f64),
}

type Attributes = HashMap<String, HyperValue>;

struct HyperError {
    status: u32,
    message: String,
    location: String,
}

#[deriving(Clone)]
struct SearchState {
    status: Enum_hyperdex_client_returncode,
    attrs: *const Struct_hyperdex_client_attribute,
    attrs_sz: size_t,
    val_tx: Sender<Attributes>,
    err_tx: Sender<HyperError>,
}

#[deriving(Clone)]
enum HyperState {
    HyperStateOp(Sender<HyperError>),  // for calls that don't return values
    HyperStateSearch(SearchState),  // for calls that do return values
}

struct Request {
    id: int64_t,
    confirm_tx: Sender<bool>,
}

struct InnerClient {
    ptr: *mut Struct_hyperdex_client,
    ops: Arc<Mutex<HashMap<int64_t, HyperState>>>,
    err_tx: Sender<HyperError>,
}

unsafe fn to_bytes(ptr: *const ::libc::c_char) -> Vec<u8> {
    CString::new(ptr, true).container_into_owned_bytes()
}

unsafe fn to_string(ptr: *const ::libc::c_char) -> String {
    String::from_utf8(to_bytes(ptr)).unwrap()  // TODO: better error handling
}

unsafe fn build_attrs(c_attrs: *const Struct_hyperdex_client_attribute, c_attrs_sz: size_t) -> Result<Attributes, String> {
    let mut attrs = HashMap::new();

    for i in range(0, c_attrs_sz) {
        let attr = *c_attrs.offset(i as int);
        let name = to_string(attr.attr);
        match attr.datatype {
            HYPERDATATYPE_STRING => {
                attrs.insert(name,
                             HyperString(from_buf(attr.value as *const u8, attr.value_sz as uint)));
            },
            HYPERDATATYPE_INT64 => {
                let mut cint = 0i64;
                if hyperdex_ds_unpack_int(attr.value as *const i8, attr.value_sz, &mut cint) < 0 {
                    return Err("Server sent a malformed int".into_string());
                }
                attrs.insert(name, HyperInt(cint));
            },
            HYPERDATATYPE_FLOAT => {
                let mut cdouble = 0f64;
                if hyperdex_ds_unpack_float(attr.value as *const i8,
                                            attr.value_sz, &mut cdouble) < 0 {
                    return Err("Server sent a malformed float".into_string());
                }
                attrs.insert(name, HyperFloat(cdouble));
            },
            _ => return Err(format!("Wrong datatype: {}", attr.datatype)),
        }
    }

    return Ok(attrs);
}

impl InnerClient {

    fn new(ptr: *mut Struct_hyperdex_client, err_tx: Sender<HyperError>) -> InnerClient {
        InnerClient {
            ptr: ptr,
            ops: Arc::new(Mutex::new(HashMap::new())),
            err_tx: err_tx,
        }
    }

    fn run_forever(&mut self) {
        unsafe {
            loop {
                hyperdex_client_block(self.ptr, 250);  // prevent busy spinning
                let mut loop_status = 0u32;
                let reqid = hyperdex_client_loop(self.ptr, 0, &mut loop_status);
                if reqid < 0 && loop_status == HYPERDEX_CLIENT_TIMEOUT {
                    // pass
                } else if reqid < 0 && loop_status == HYPERDEX_CLIENT_NONEPENDING {
                    // pass
                } else if reqid < 0 {
                    let e = HyperError {
                        status: loop_status,
                        message: to_string(hyperdex_client_error_message(self.ptr)),
                        location: to_string(hyperdex_client_error_location(self.ptr)),
                    };
                    self.err_tx.send(e);
                } else {
                    let mut ops = &mut*self.ops.lock();
                    match ops.find_copy(&reqid) {
                        None => {},  // TODO: this seems to be an error case... might want to do something
                        Some(HyperStateOp(op_tx)) => {
                            op_tx.send(HyperError {
                                status: loop_status,
                                message: to_string(hyperdex_client_error_message(self.ptr)),
                                location: to_string(hyperdex_client_error_location(self.ptr)),
                            });
                            ops.remove(&reqid);
                        },
                        Some(HyperStateSearch(state)) => {
                        },
                    }
                }
            }
        }
    }
}

pub struct Client {
    ops_maps: Vec<Arc<Mutex<HashMap<int64_t, HyperState>>>>
}

impl Client {

    pub fn new(coordinator: SocketAddr) -> Result<Client, String> {
        let ip = format!("{}", coordinator.ip).to_c_str().as_ptr();
        let port = coordinator.port;

        let (err_tx, err_rx) = channel();

        let mut ops_maps = Vec::new();
        for _ in range(0, num_cpus()) {
            let ptr = unsafe { hyperdex_client_create(ip, port) };
            if ptr.is_null() {
                return Err(format!("Unable to create client.  errno is: {}", errno()));
            } else {
                let ops = Arc::new(Mutex::new(HashMap::new()));
                let mut inner_client = InnerClient {
                    ptr: ptr,
                    ops: ops.clone(),
                    err_tx: err_tx.clone(),
                };
                spawn(proc() {
                    inner_client.run_forever();
                });
                ops_maps.push(ops);
            }
        };

        Ok(Client {
            ops_maps: ops_maps
        })
    }

    // pub fn new_from_conn_str(conn: String) -> Result<Client, String> {
        // let conn_str = conn.to_c_str().as_ptr();
        // let ptr = unsafe { hyperdex_client_create_conn_str(conn_str) };
        // if ptr.is_null() {
            // Err(format!("Unable to create client.  errno is: {}", errno()))
        // } else {
            // unsafe {
                // Ok(Client {
                    // ptr: ptr
                // })
            // }
        // }
    // }

}

// impl Drop for Client {
    // fn drop(&mut self) {
        // unsafe {
            // hyperdex_client_destroy(self.ptr);
        // }
    // }
// }

