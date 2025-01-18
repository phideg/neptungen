use axum::Router;
use std::net::SocketAddr;
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::config::Config;

fn create_route(conf: &Config) -> Router {
    // serve generated content
    Router::new().fallback_service(
        ServeDir::new(
            conf.output_dir
                .as_ref()
                .expect("Internal error [create_route(..)]: no output path!"),
        ),
    )
}

#[tokio::main]
pub async fn serve(conf: &Config) {
    let app = create_route(conf);
    let port = portpicker::pick_unused_port()
        .expect("Could not find a free tcp port to start the http server :(.");
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Internal error: could not bind http listener port!");
    log::info!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .expect("Internal error: Could not start the http server. PANIC!");
}
