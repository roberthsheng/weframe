use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use warp::ws::{Message, WebSocket};
use weframe_shared::EditOperation;

// struct to represent single editing session
struct Session {
    clients: HashMap<usize, mpsc::UnboundedSender<Message>>,
}

// manages all active sessions
struct SessionManager {
    sessions: HashMap<String, Arc<RwLock<Session>>>,
}

impl SessionManager {
    fn new() -> Self {
        SessionManager {
            sessions: HashMap::new(),
        }
    }

    async fn get_or_create_session(&mut self, id: &str) -> Arc<RwLock<Session>> {
        self.sessions
            .entry(id.to_string())
            .or_insert_with(|| {
                Arc::new(RwLock::new(Session {
                    clients: HashMap::new(),
                    // initialize video project state here
                }))
            })
            .clone()
    }
}

pub async fn handle_websocket(ws: WebSocket, session_id: String, manager: Arc<RwLock<SessionManager>>) {
    let (ws_sender, mut ws_receiver) = ws.split();
    let (client_sender, client_receiver) = mpsc::unbounded_channel();

    let client_id = rand::random::<usize>();

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
                if letOk(edit_op) = serde_json::from_str::<EditOperation>(&msg.to_string().unwrap_or_default()) {
                    // process edit ops
                    // update video project state
                    // broadcast new state to all clients
                }
            }
            Err(_) => {
                break;
            }
        }
    }

    let mut session = session.write().await;
    session.clients.remove(&client_id);
}

pub async fn run_server() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));

    let routes = warp.path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(warp::any().map(move || session_manager.clone()))
        .map(|ws: warp::ws::Ws, session_id, manager| {
            ws.on_upgrade(move |socket| handle_websocket(socket, session_id, manager))
        });

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}