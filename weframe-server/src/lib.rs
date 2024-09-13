// weframe-server/src/lib.rs
use futures::{SinkExt, StreamExt};
use rand::random;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, RwLock};
use warp::ws::{Message, WebSocket};
use warp::Filter;
use weframe_shared::{Collaborator, CursorPosition, OTOperation, VideoProject};

pub struct SessionManager {
    sessions: HashMap<String, Arc<RwLock<VideoSession>>>,
}

pub struct VideoSession {
    metadata: Metadata,
    project: VideoProject,
    clients: HashMap<String, mpsc::UnboundedSender<Message>>,
    server_version: usize,
    last_activity: SystemTime,
    broadcast: broadcast::Sender<OTOperation>,
    update_tx: mpsc::Sender<ServerMessage>,
    update_rx: mpsc::Receiver<ServerMessage>,
}

#[derive(Clone)]
pub struct Metadata {
    name: String,
    created_at: SystemTime,
    max_duration: Duration,
}

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    ClientOperation(OTOperation),
    NewClient { client_id: String, name: String },
    ClientDisconnected(String),
    ProjectUpdate(VideoProject),
    ChatMessage { client_id: String, message: String },
    Error { client_id: String, message: String },
    Ping(u64),
    Pong(u64),
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: HashMap::new(),
        }
    }

    pub async fn get_or_create_session(&mut self, id: &str) -> Arc<RwLock<VideoSession>> {
        self.sessions
            .entry(id.to_string())
            .or_insert_with(|| {
                Arc::new(RwLock::new(VideoSession::new(Metadata {
                    name: id.to_string(),
                    created_at: SystemTime::now(),
                    max_duration: Duration::from_secs(3600), // 1 hour max session duration
                })))
            })
            .clone()
    }

    pub async fn cleanup_inactive_sessions(&mut self) {
        let now = SystemTime::now();
        self.sessions.retain(|_, session| {
            let last_activity = session.blocking_read().last_activity;
            now.duration_since(last_activity)
                .unwrap_or(Duration::from_secs(0))
                < Duration::from_secs(24 * 60 * 60)
        });
    }
}

impl VideoSession {
    pub fn new(metadata: Metadata) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        let (update_tx, update_rx) = mpsc::channel(100);
        VideoSession {
            metadata: metadata.clone(),
            project: VideoProject::new(
                uuid::Uuid::new_v4().to_string(),
                metadata.name,
                "server".to_string(),
                "Server".to_string(),
            ),
            clients: HashMap::new(),
            server_version: 0,
            last_activity: SystemTime::now(),
            broadcast: broadcast_tx,
            update_tx,
            update_rx,
        }
    }

    pub fn apply_operation(&mut self, operation: &OTOperation) {
        self.project.apply_operation(&operation.operation);
        self.server_version += 1;
        self.broadcast.send(operation.clone()).ok();
    }

    pub fn add_client(&mut self, client_id: String, client_sender: mpsc::UnboundedSender<Message>) {
        self.clients.insert(client_id.clone(), client_sender);
        self.project.collaborators.push(Collaborator {
            id: client_id.clone(),
            name: format!("User {}", client_id),
            cursor_position: CursorPosition {
                track: 0,
                time: Duration::from_secs(0),
            },
        });
        self.last_activity = SystemTime::now();
    }

    pub fn remove_client(&mut self, client_id: &str) {
        self.clients.remove(client_id);
        self.project.collaborators.retain(|c| c.id != client_id);
    }

    pub fn broadcast_message(&self, message: &ServerMessage) {
        let msg = serde_json::to_string(message).unwrap();
        for sender in self.clients.values() {
            sender.send(Message::text(msg.clone())).ok();
        }
    }

    pub fn send_ping(&self) -> ServerMessage {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        ServerMessage::Ping(now)
    }

    pub fn send_pong(&self, received_time: u64) -> ServerMessage {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let duration = now - received_time;
        ServerMessage::Pong(duration)
    }
}

pub async fn handle_websocket(
    ws: WebSocket,
    session_id: String,
    manager: Arc<RwLock<SessionManager>>,
) {
    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (client_sender, mut client_receiver) = mpsc::unbounded_channel();

    let client_id = format!("user-{}", random::<u32>());

    let session = {
        let mut manager = manager.write().await;
        manager.get_or_create_session(&session_id).await
    };

    {
        let mut session = session.write().await;
        if !session.clients.contains_key(&client_id) {
            session.add_client(client_id.clone(), client_sender);
            session.broadcast_message(&ServerMessage::NewClient {
                client_id: client_id.clone(),
                name: format!("User {}", client_id),
            });
        }
    }

    let mut broadcast_rx = {
        let session = session.read().await;
        session.broadcast.subscribe()
    };

    loop {
        tokio::select! {
            Some(result) = ws_receiver.next() => {
                match result {
                    Ok(msg) => {
                        if let Ok(client_op) = serde_json::from_str::<OTOperation>(&msg.to_str().unwrap_or_default()) {
                            let mut session = session.write().await;
                            session.last_activity = SystemTime::now();

                            let transformed_op = session.project.transform_operation(&client_op, session.server_version);
                            session.apply_operation(&transformed_op);
                            println!("Applied operation: {:?}", transformed_op);
                            let server_message = ServerMessage::ClientOperation(transformed_op);
                            let msg = serde_json::to_string(&server_message).unwrap();
                            for (_, sender) in &session.clients {
                                let _ = sender.send(Message::text(msg.clone()));
                            }
                        } else if let Ok(ServerMessage::Ping(timestamp)) = serde_json::from_str(&msg.to_str().unwrap_or_default()) {
                            let pong = session.read().await.send_pong(timestamp);
                            ws_sender.send(Message::text(serde_json::to_string(&pong).unwrap())).await.ok();
                        }
                    }
                    Err(_) => break,
                }
            }
            Ok(operation) = broadcast_rx.recv() => {
                let msg = serde_json::to_string(&operation).unwrap();
                if ws_sender.send(Message::text(msg)).await.is_err() {
                    break;
                }
            }
            Some(msg) = client_receiver.recv() => {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
            else => break,
        }
    }

    let mut session = session.write().await;
    session.remove_client(&client_id);
    session.broadcast_message(&ServerMessage::ClientDisconnected(client_id));
}

pub async fn run_server() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));

    // cleanup inactive sessions
    let cleanup_manager = session_manager.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await; // once an hour
            cleanup_manager
                .write()
                .await
                .cleanup_inactive_sessions()
                .await;
        }
    });

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec!["Content-Type"]);

    let routes = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(warp::any().map(move || session_manager.clone()))
        .map(
            |ws: warp::ws::Ws, session_id: String, manager: Arc<RwLock<SessionManager>>| {
                ws.on_upgrade(move |socket| handle_websocket(socket, session_id, manager))
            },
        )
        .with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
