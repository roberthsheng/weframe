use futures::{SinkExt, StreamExt};
use rand::random;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use warp::ws::{Message, WebSocket};
use warp::Filter;
use weframe_shared::{OTOperation, VideoProject};

pub struct SessionManager {
    sessions: HashMap<String, Arc<RwLock<Session>>>,
}

pub struct Session {
    clients: HashMap<usize, mpsc::UnboundedSender<Message>>,
    project: VideoProject,
    server_version: usize,
    last_activity: Instant,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: HashMap::new(),
        }
    }

    pub async fn get_or_create_session(&mut self, id: &str) -> Arc<RwLock<Session>> {
        self.sessions
            .entry(id.to_string())
            .or_insert_with(|| {
                Arc::new(RwLock::new(Session {
                    clients: HashMap::new(),
                    project: VideoProject {
                        clips: Vec::new(),
                        duration: Duration::from_secs(300),
                    },
                    server_version: 0,
                    last_activity: Instant::now(),
                }))
            })
            .clone()
    }

    pub async fn cleanup_inactive_sessions(&mut self) {
        let now = Instant::now();
        self.sessions.retain(|_, session| {
            let last_activity = session.blocking_read().last_activity;
            now.duration_since(last_activity) < Duration::from_secs(24 * 60 * 60)
            // 24 hours
        });
    }
}

pub async fn handle_websocket(
    ws: WebSocket,
    session_id: String,
    manager: Arc<RwLock<SessionManager>>,
) {
    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (client_sender, mut client_receiver) = mpsc::unbounded_channel();

    let client_id = random::<usize>();

    // get or create session
    let session = {
        let mut manager = manager.write().await;
        manager.get_or_create_session(&session_id).await
    };

    // add client to session
    {
        let mut session = session.write().await;
        session.clients.insert(client_id, client_sender);
        session.last_activity = Instant::now();
    }

    // handle incoming messages
    loop {
        tokio::select! {
            Some(result) = ws_receiver.next() => {
                match result {
                    Ok(msg) => {
                        if let Ok(client_op) = serde_json::from_str::<OTOperation>(&msg.to_str().unwrap_or_default()) {
                            let mut session = session.write().await;
                            session.last_activity = Instant::now();

                            let server_op = OTOperation {
                                client_id,
                                client_version: client_op.client_version,
                                server_version: session.server_version,
                                operation: client_op.operation.clone(),
                            };

                            let transformed_op = session.project.transform_operation(&server_op, &client_op);
                            session.project.apply_operation(&transformed_op.operation);
                            session.server_version += 1;

                            // broadcast the transformed operation to all clients
                            let update_msg = serde_json::to_string(&transformed_op).unwrap();
                            for (_, sender) in &session.clients {
                                let _ = sender.send(Message::text(update_msg.clone()));
                            }
                        }
                    }
                    Err(_) => break,
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
    session.clients.remove(&client_id);
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

    let routes = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(warp::any().map(move || session_manager.clone()))
        .map(
            |ws: warp::ws::Ws, session_id: String, manager: Arc<RwLock<SessionManager>>| {
                ws.on_upgrade(move |socket| handle_websocket(socket, session_id, manager))
            },
        );

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
