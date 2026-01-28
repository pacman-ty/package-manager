use urlencoding::encode;
 
use curl::easy::{Easy2, Handler, WriteError};
use curl::multi::{Easy2Handle, Multi};
use std::collections::HashMap;
use std::str;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use crate::Packages;

struct Collector(Box<String>);
impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        (*self.0).push_str(str::from_utf8(&data.to_vec()).unwrap());
        Ok(data.len())
    }
}

const DEFAULT_SERVER: &str = "ece459.patricklam.ca:4590";
impl Drop for Packages {
    fn drop(&mut self) {
        self.execute()
    }
}

static EASYKEY_COUNTER: AtomicI32 = AtomicI32::new(0);

pub struct AsyncState {
    server: String,
    multi: Multi,
    handles: HashMap<i32, Easy2Handle<Collector>>,
    request_info: HashMap<i32, (String, String, String)>, // (pkg_name, version, local_md5)
}

impl AsyncState {
    pub fn new() -> AsyncState {
        AsyncState {
            server: String::from(DEFAULT_SERVER),
            multi: Multi::new(),
            handles: HashMap::new(),
            request_info: HashMap::new(),
        }
    }
}

impl Packages {
    pub fn set_server(&mut self, new_server: &str) {
        self.async_state.server = String::from(new_server);
    }

    /// Retrieves the version number of pkg and calls enq_verify_with_version with that version number.
    pub fn enq_verify(&mut self, pkg: &str) {
        let version = self.get_available_debver(pkg);
        match version {
            None => {
                println!("Error: package {} not defined.", pkg);
                return;
            }
            Some(v) => {
                let vs = &v.to_string();
                self.enq_verify_with_version(pkg, vs);
            }
        };
    }

    /// Enqueues a request for the provided version/package information.
    /// Stores any needed state to async_state so that execute() can handle the results and print out needed output.
    pub fn enq_verify_with_version(&mut self, pkg: &str, version: &str) {
        let encoded_version = encode(version);
       
        let url = format!(
            "http://{}/rest/v1/checksums/{}/{}",
            self.async_state.server, pkg, encoded_version
        );

        println!("queueing request {}", url);

        let local_md5 = match self.get_md5sum(pkg) {
            Some(md5) => md5.to_string(),
            None => String::new(),
        };

        let mut easy = Easy2::new(Collector(Box::new(String::new())));
        easy.url(&url).unwrap();
        easy.get(true).unwrap();

        let key = EASYKEY_COUNTER.fetch_add(1, Ordering::SeqCst);

        self.async_state.request_info.insert(
            key,
            (pkg.to_string(), version.to_string(), local_md5)
        );

        let handle = self.async_state.multi.add2(easy).unwrap();
        self.async_state.handles.insert(key, handle);
    }

 

    /// Asks curl to perform all enqueued requests.
    /// For requests that succeed with response code 200, compares received MD5sum with local MD5sum.
    /// For requests that fail with 400+, prints error message.
    pub fn execute(&mut self) {
        while self.async_state.multi.perform().unwrap() > 0 {
            let _ = self.async_state.multi.wait(&mut [], Duration::from_secs(1));
        }

        // Drain the handles and process results
        let keys: Vec<i32> = self.async_state.handles.keys().copied().collect();

        for key in keys {
            if let Some(handle) = self.async_state.handles.remove(&key) {
                let mut easy = self.async_state.multi.remove2(handle).unwrap();

                let response_code = easy.response_code().unwrap();

                if let Some((pkg_name, version, local_md5)) = self.async_state.request_info.remove(&key) {
                    if response_code == 200 {

                        let received_md5 = easy.get_ref().0.trim().to_string();
                        let matches = received_md5 == local_md5;
                        println!("verifying {}, matches: {}", pkg_name, matches);

                    } else if response_code >= 400 {

                        println!(
                            "got error {} on request for package {} version {}",
                            response_code, pkg_name, version
                        );
                    }
                }
            }
        }
    }
}
