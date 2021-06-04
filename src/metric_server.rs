use open_metrics_client::encoding::text::encode;
use open_metrics_client::registry::Registry;

use std::sync::{Arc, Mutex};

pub async fn run(registry: Registry, path: String) -> std::result::Result<(), std::io::Error> {
    // tide::log::start();

    let mut app = tide::with_state(State {
        registry: Arc::new(Mutex::new(registry)),
    });

    app.at(&path).get(|req: tide::Request<State>| async move {
        let mut encoded = Vec::new();
        encode(&mut encoded, &req.state().registry.lock().unwrap()).unwrap();
        Ok(String::from_utf8(encoded).unwrap())
    });

    let listen_addr = "0.0.0.0:8888";
    println!("Listening for metric requests on {}{}", listen_addr, path);
    app.listen(listen_addr).await?;

    Ok(())
}

#[derive(Clone)]
struct State {
    registry: Arc<Mutex<Registry>>,
}
