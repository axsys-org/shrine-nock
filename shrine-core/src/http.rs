use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use milton::{cue, Atom, Cell, Noun};
use shrine_storage::core::types::slot;
use shrine_storage::store::Namespace;
use shrine_storage::{Care, DiskPail, Gift, Mode, Observe, Tale};
use tokio::sync::mpsc;

use crate::driver::{
    DriverAck, DriverCard, DriverID, DriverNote, DriverOwned, IntoTale, ShrineDriver, Subscription,
};
use crate::supervisor::{ActorHandle, ChildSpec, RestartPolicy};

const HTTP_PATH: &str = "/drv/http";

// ---------------------------------------------------------------------------
// HttpDriver — root supervisor that watches /drv/http
// ---------------------------------------------------------------------------

pub struct HttpDriver {
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
    handle: Option<ActorHandle>,
    active_requests: Vec<String>,
    last_y_version: u32,
}

impl HttpDriver {
    pub fn new(id: DriverID, namespace: Arc<Namespace>) -> Self {
        Self {
            id,
            namespace,
            tx: None,
            handle: None,
            active_requests: Vec::new(),
            last_y_version: 0,
        }
    }

    async fn reconcile_requests(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let epic = match self.namespace.look(Care::Y, HTTP_PATH) {
            Ok(Observe::Found(epic)) => epic,
            _ => return Ok(()),
        };

        let handle = match &self.handle {
            Some(h) => h.clone(),
            None => return Ok(()),
        };

        let root_path = self.namespace.get_path_id("/").unwrap().raw();

        for (path_idx, disk_saga) in epic.iter() {
            let path_str = match self.namespace.path_idx_to_str(path_idx) {
                Ok(Some(s)) => s,
                _ => continue,
            };

            if path_str == HTTP_PATH {
                continue;
            }

            let tale = disk_saga.tale();

            // Already processed
            if tale.get(slot::RES).is_some() {
                continue;
            }

            // No request to process
            if tale.get(slot::REQ).is_none() {
                continue;
            }

            let name = path_str.rsplit('/').next().unwrap_or(&path_str).to_string();

            if self.active_requests.contains(&name) {
                continue;
            }

            let worker_path = path_str.clone();
            let spec = ChildSpec {
                name: name.clone(),
                factory: Arc::new(move |id, ns| {
                    Box::new(HttpWorkerDriver::new(id, ns, worker_path.clone()))
                }),
                restart: RestartPolicy::Temporary,
                subscription: Subscription { path: root_path, care: Care::X },
            };

            println!("[http] spawning worker for '{}'", name);
            let _ = handle.spawn(spec).await;
            self.active_requests.push(name);
        }

        Ok(())
    }
}

#[async_trait]
impl ShrineDriver for HttpDriver {
    fn id(&self) -> DriverID {
        self.id
    }

    fn subscribe(&self) -> Subscription {
        Subscription {
            path: self.namespace.get_path_id("/").unwrap().raw(),
            care: Care::Z,
        }
    }

    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>) {
        self.tx = Some(tx);
    }

    fn set_handle(&mut self, handle: ActorHandle) {
        self.handle = Some(handle);
    }

    async fn on_start(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let tx = self.tx.clone().expect("tx must be set");
        tx.send(vec![DriverCard {
            id: 0,
            note: DriverNote {
                path: HTTP_PATH.to_string(),
                mode: Mode::Add,
                slots: Arc::new(DriverOwned::new(self.id)),
            },
        }])
        .await
        .ok();

        self.reconcile_requests().await?;
        Ok(())
    }

    async fn on_dirty(&mut self, dirty: &[Gift]) -> Result<(), Box<dyn Error + Send>> {
        let y_version = dirty
            .iter()
            .find(|g| g.care == Care::Y)
            .map(|g| g.ever.y.data)
            .unwrap_or(0);

        if y_version != 0 && y_version == self.last_y_version {
            return Ok(());
        }
        self.last_y_version = y_version;

        self.reconcile_requests().await?;
        Ok(())
    }

    async fn on_done(&mut self, _ack: &DriverAck) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// HttpWorkerDriver — per-request child
// ---------------------------------------------------------------------------

struct HttpWorkerDriver {
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
    path: String,
}

impl HttpWorkerDriver {
    fn new(id: DriverID, namespace: Arc<Namespace>, path: String) -> Self {
        Self {
            id,
            namespace,
            tx: None,
            path,
        }
    }
}

#[async_trait]
impl ShrineDriver for HttpWorkerDriver {
    fn id(&self) -> DriverID {
        self.id
    }

    fn subscribe(&self) -> Subscription {
        Subscription {
            path: self.namespace.get_path_id(&self.path).unwrap().raw(),
            care: Care::X,
        }
    }

    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>) {
        self.tx = Some(tx);
    }

    async fn on_start(&mut self) -> Result<(), Box<dyn Error + Send>> {
        let saga = match self.namespace.grab(&self.path) {
            Ok(Some(s)) => s,
            _ => {
                println!("[http:worker] no saga at {}", self.path);
                return Ok(());
            }
        };

        let req = match saga.tale().get(slot::REQ) {
            Some(p) => p.clone(),
            None => {
                println!("[http:worker] no req slot at {}", self.path);
                return Ok(());
            }
        };

        let tx = self.tx.clone().expect("tx must be set");

        match req {
            DiskPail::Jam { data, .. } => {
                let note = DriverNote::poke(&self.path, HttpExecTale { jam_bytes: data });
                tx.send(vec![DriverCard { id: self.id, note }]).await.ok();
            }
            _ => {
                println!("[http:worker] unexpected req type at {}", self.path);
            }
        }

        Ok(())
    }

    async fn on_dirty(&mut self, _dirty: &[Gift]) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    async fn on_done(&mut self, _ack: &DriverAck) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// HttpExecTale — runs on main thread (Noun is !Send)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct HttpExecTale {
    jam_bytes: Vec<u8>,
}

impl IntoTale for HttpExecTale {
    fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
        let mut tale = Tale::empty();

        // Cue the request noun: (url (method headers) body)
        let Some(req_noun) = cue(&self.jam_bytes) else {
            println!("[http:exec] failed to cue request");
            return tale;
        };

        // Destructure: (url . rest)
        let Some(req_cell) = req_noun.as_cell() else {
            println!("[http:exec] request is not a cell");
            return tale;
        };

        let url_ref = req_cell.head();
        let rest = req_cell.tail();

        // Extract URL string
        let Some(url_atom) = url_ref.as_atom() else {
            println!("[http:exec] url is not an atom");
            return tale;
        };
        let url = url_atom.to_owned().to_string();
        if url.is_empty() {
            println!("[http:exec] url is empty");
            return tale;
        }

        // Destructure rest: ((method headers) body)
        let Some(rest_cell) = rest.as_cell() else {
            println!("[http:exec] rest is not a cell");
            return tale;
        };

        let method_headers_ref = rest_cell.head();
        let body_ref = rest_cell.tail();

        // Destructure (method headers)
        let (method, headers) = if let Some(mh_cell) = method_headers_ref.as_cell() {
            let method_ref = mh_cell.head();
            let headers_ref = mh_cell.tail();

            let method_str = if let Some(m_atom) = method_ref.as_atom() {
                let s = m_atom.to_owned().to_string();
                if s.is_empty() { "GET".to_string() } else { s }
            } else {
                "GET".to_string()
            };

            // Parse headers list: (key . value) pairs, null-terminated
            let mut hdrs: Vec<(String, String)> = Vec::new();
            let mut cur_noun = headers_ref.to_owned();
            loop {
                if cur_noun.is_atom() {
                    // Null terminator (0) or end of list
                    break;
                }
                if let Some(pair_cell) = cur_noun.as_cell() {
                    let head = pair_cell.head();
                    let tail_owned = pair_cell.tail().to_owned();

                    if let Some(kv_cell) = head.as_cell() {
                        let k = kv_cell.head();
                        let v = kv_cell.tail();
                        if let (Some(ka), Some(va)) = (k.as_atom(), v.as_atom()) {
                            hdrs.push((
                                ka.to_owned().to_string(),
                                va.to_owned().to_string(),
                            ));
                        }
                    }
                    cur_noun = tail_owned;
                } else {
                    break;
                }
            }

            (method_str, hdrs)
        } else {
            ("GET".to_string(), Vec::new())
        };

        // Extract body bytes
        let body_bytes: Vec<u8> = if let Some(b_atom) = body_ref.as_atom() {
            if b_atom.as_u32() == Some(0) {
                Vec::new()
            } else {
                b_atom.to_owned().to_vec()
            }
        } else {
            Vec::new()
        };

        println!("[http:exec] {} {}", method, url);

        // Execute HTTP request via ureq
        let agent = ureq::agent();
        let method_upper = method.to_uppercase();

        let response = match method_upper.as_str() {
            "POST" | "PUT" | "PATCH" => {
                let mut req = match method_upper.as_str() {
                    "POST" => agent.post(&url),
                    "PUT" => agent.put(&url),
                    _ => agent.patch(&url),
                };
                for (k, v) in &headers {
                    req = req.header(k.as_str(), v.as_str());
                }
                if body_bytes.is_empty() {
                    req.send_empty()
                } else {
                    req.send(&body_bytes[..])
                }
            }
            _ => {
                let mut req = match method_upper.as_str() {
                    "HEAD" => agent.head(&url),
                    "DELETE" => agent.delete(&url),
                    _ => agent.get(&url),
                };
                for (k, v) in &headers {
                    req = req.header(k.as_str(), v.as_str());
                }
                req.call()
            }
        };

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();

                // Collect response headers
                let resp_headers: Vec<(String, String)> = resp.headers()
                    .iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();

                // Read response body
                let resp_body = match resp.into_body().read_to_vec() {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        println!("[http:exec] failed to read response body: {}", e);
                        Vec::new()
                    }
                };

                println!("[http:exec] response: {} ({} bytes)", status, resp_body.len());

                // Build response noun: (status-code (headers) body)
                let status_noun = Atom::from_u32(status as u32).into_noun();

                // Build headers list: ((k . v) (k . v) ... 0)
                let mut hdr_noun = Noun::null();
                for (k, v) in resp_headers.iter().rev() {
                    let k_atom = Atom::from_str(k).into_noun();
                    let v_atom = Atom::from_str(v).into_noun();
                    let pair = Cell::new(k_atom, v_atom).into_noun();
                    hdr_noun = Cell::new(pair, hdr_noun).into_noun();
                }

                let body_noun = if resp_body.is_empty() {
                    Noun::null()
                } else {
                    Atom::from_bytes(&resp_body).into_noun()
                };

                let resp_noun = Cell::trel(status_noun, hdr_noun, body_noun).into_noun();
                let jammed = resp_noun.jam();

                tale.insert(slot::RES, DiskPail::Jam {
                    typ: "http-res".to_string(),
                    data: jammed,
                });
            }
            Err(e) => {
                println!("[http:exec] request failed: {}", e);

                // Return error as response: (0 0 error-message)
                let status_noun = Noun::null();
                let hdr_noun = Noun::null();
                let err_msg = format!("{}", e);
                let body_noun = Atom::from_str(&err_msg).into_noun();
                let resp_noun = Cell::trel(status_noun, hdr_noun, body_noun).into_noun();
                let jammed = resp_noun.jam();

                tale.insert(slot::RES, DiskPail::Jam {
                    typ: "http-res".to_string(),
                    data: jammed,
                });
            }
        }

        tale
    }
}
