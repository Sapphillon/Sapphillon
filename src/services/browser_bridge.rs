// Sapphillon
// BrowserBridge: Backend <-> Frontend request relay (UI executes via BrowserOS)

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use floorp_grpc::browser_bridge as pb;
use pb::browser_bridge_server::{BrowserBridge, BrowserBridgeServer};
use pb::{BridgeRequest, CompleteRequest, CompleteResponse, SubscribeRequestsRequest};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use std::pin::Pin;
use futures_util::stream::Stream;

// Hub: holds the current UI subscriber and pending map
#[derive(Default)]
pub struct BridgeHub {
    sender: Mutex<Option<mpsc::Sender<BridgeRequest>>>,
    pending: Mutex<HashMap<String, oneshot::Sender<CompleteRequest>>>,
    waiters: Mutex<VecDeque<oneshot::Sender<BridgeRequest>>>,
    queue: Mutex<VecDeque<BridgeRequest>>, // requests waiting for delivery when no subscriber
}

impl BridgeHub {
    fn new() -> Self {
        Self {
            sender: Mutex::new(None),
            pending: Mutex::new(HashMap::new()),
            waiters: Mutex::new(VecDeque::new()),
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn shared() -> &'static Arc<BridgeHub> {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<Arc<BridgeHub>> = OnceLock::new();
        INSTANCE.get_or_init(|| Arc::new(BridgeHub::new()))
    }

    async fn set_sender(&self, tx: mpsc::Sender<BridgeRequest>) {
        let mut guard = self.sender.lock().await;
        *guard = Some(tx.clone());
        // Flush any queued requests to the new sender
        let mut drained: Vec<BridgeRequest> = Vec::new();
        {
            let mut q = self.queue.lock().await;
            while let Some(req) = q.pop_front() {
                drained.push(req);
            }
        }
        for r in drained {
            let _ = tx.send(r).await;
        }
    }

    #[allow(dead_code)]
    async fn clear_sender(&self) {
        let mut guard = self.sender.lock().await;
        *guard = None;
    }

    pub async fn request(
        &self,
        method: &str,
        args_json: Option<String>,
        timeout: Duration,
    ) -> Result<CompleteRequest, tonic::Status> {
        // Prepare correlation and oneshot
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id.clone(), tx);
        }

        // No-op: delivery to UI is decided below (streaming sender or long-poll waiter)

        let deadline_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
            + timeout.as_millis() as i64;

        let req = BridgeRequest {
            id: id.clone(),
            method: method.to_string(),
            args_json: args_json.unwrap_or_else(|| "{}".to_string()),
            deadline_unix_ms: deadline_ms,
        };

        log::debug!(
            "[Bridge] enqueue id={} method={} args_len={} timeout_ms={}",
            id,
            method,
            req.args_json.len(),
            timeout.as_millis()
        );

        // Delivery strategy:
        // 1) If streaming sender exists, send immediately.
        // 2) Else if a long-poll waiter exists, deliver to the waiter.
        // 3) Else queue until a subscriber/waiter appears (within overall timeout).
        if let Some(s) = self.sender.lock().await.clone() {
            let _ = s.send(req.clone()).await;
        } else if let Some(w) = { self.waiters.lock().await.pop_front() } {
            let _ = w.send(req.clone());
        } else {
            // Queue it for later pickup by a subscriber or waiter
            let mut q = self.queue.lock().await;
            q.push_back(req.clone());
        }

        // Await completion with timeout
        match tokio::time::timeout(timeout, rx).await {
            Err(_) => {
                let mut pending = self.pending.lock().await;
                pending.remove(&id);
                Err(tonic::Status::deadline_exceeded("BrowserBridge timeout"))
            }
            Ok(Err(_canceled)) => {
                let mut pending = self.pending.lock().await;
                pending.remove(&id);
                Err(tonic::Status::cancelled("BrowserBridge canceled"))
            }
            Ok(Ok(done)) => {
                log::debug!(
                    "[Bridge] complete id={} success={} has_json={}",
                    done.id,
                    done.success,
                    done.result_json.as_ref().map(|s| s.len()).is_some()
                );
                Ok(done)
            }
        }
    }

    async fn complete(&self, msg: CompleteRequest) -> bool {
        let tx = {
            let mut pending = self.pending.lock().await;
            pending.remove(&msg.id)
        };
        if let Some(tx) = tx { tx.send(msg).is_ok() } else { false }
    }
}

pub struct BrowserBridgeServiceImpl {
    hub: Arc<BridgeHub>,
}

impl BrowserBridgeServiceImpl {
    pub fn new() -> Self { Self { hub: BridgeHub::shared().clone() } }
    pub fn into_server(self) -> BrowserBridgeServer<Self> { BrowserBridgeServer::new(self) }
}

#[tonic::async_trait]
impl BrowserBridge for BrowserBridgeServiceImpl {
    type SubscribeRequestsStream = Pin<Box<dyn Stream<Item = Result<BridgeRequest, tonic::Status>> + Send + 'static>>;

    async fn subscribe_requests(
        &self,
        _request: tonic::Request<SubscribeRequestsRequest>,
    ) -> Result<tonic::Response<Self::SubscribeRequestsStream>, tonic::Status> {
        log::debug!("[Bridge] UI subscribed to requests stream");
        let (tx, rx) = mpsc::channel::<BridgeRequest>(32);
        self.hub.set_sender(tx).await;

        // Map mpsc Receiver into Result<BridgeRequest, Status>
        let stream = ReceiverStream::new(rx).map(Ok);

        // Note: When the client disconnects, the receiver drops automatically.
        Ok(tonic::Response::new(Box::pin(stream)))
    }

    async fn wait_for_request(
        &self,
        _request: tonic::Request<SubscribeRequestsRequest>,
    ) -> Result<tonic::Response<BridgeRequest>, tonic::Status> {
        // First, check if a queued request already exists
        if let Some(req) = { self.hub.queue.lock().await.pop_front() } {
            return Ok(tonic::Response::new(req));
        }
        // Otherwise register a one-time waiter and await the next request.
        let (tx, rx) = oneshot::channel();
        {
            let mut waiters = self.hub.waiters.lock().await;
            waiters.push_back(tx);
        }
        let req = rx
            .await
            .map_err(|_| tonic::Status::cancelled("Waiter cancelled"))?;
        Ok(tonic::Response::new(req))
    }

    async fn complete(
        &self,
        request: tonic::Request<CompleteRequest>,
    ) -> Result<tonic::Response<CompleteResponse>, tonic::Status> {
        let msg = request.into_inner();
        let accepted = self.hub.complete(msg).await;
        Ok(tonic::Response::new(CompleteResponse { accepted, message: None }))
    }
}

// Re-export for server wiring
// Public re-export removed (unused)
