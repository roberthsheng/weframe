use weframe_server::run_server;

#[tokio::main]
async fn main() {
    println!("Starting weframe server...");
    run_server().await;
}