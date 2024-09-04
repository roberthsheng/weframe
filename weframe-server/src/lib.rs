use futures::StreamExt;
use rand::random;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use warp::ws::{Message, WebSocket};
use warp::Filter;
use weframe_shared::{EditOperation, VideoProject};

pub struct SessionManager {
    sessions: HashMap<String, Arc<RwLock<Session>>>,
}

pub struct Session {
    clients: HashMap<usize, mpsc::UnboundedSender<Message>>,
    project: VideoProject,
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
                }))
            })
            .clone()
    }
}

pub async fn handle_websocket(
    ws: WebSocket,
    session_id: String,
    manager: Arc<RwLock<SessionManager>>,
) {
    let (_ws_sender, mut ws_receiver) = ws.split();
    let (client_sender, _client_receiver) = mpsc::unbounded_channel();

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
    }

    // handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(msg) => {
                if let Ok(edit_op) =
                    serde_json::from_str::<EditOperation>(&msg.to_str().unwrap_or_default())
                {
                    let mut session = session.write().await;
                    session.project.apply_operation(&edit_op);

                    // broadcast changes to all clients
                    let update_msg = serde_json::to_string(&edit_op).unwrap();
                    for (_, sender) in &session.clients {
                        let _ = sender.send(Message::text(update_msg.clone()));
                    }
                }
            }
            Err(_) => break,
        }
    }

    let mut session = session.write().await;
    session.clients.remove(&client_id);
}

pub async fn run_server() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));

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
