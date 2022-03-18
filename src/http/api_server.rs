use hyper::{service::{make_service_fn, service_fn}, Body, Method, Request, Response, Server, StatusCode};

use crate::streams::ChannelAuthor;

use std::{net::SocketAddr, sync::{Arc}};
use std::future::ready;
use futures::StreamExt;
use hyper::server::accept;
use hyper::server::conn::AddrIncoming;
use tls_listener::TlsListener;
use tokio::sync::Mutex;
use tokio_rustls::TlsAcceptor;
use crate::http::*;

static NOTFOUND: &[u8] = b"Not Found";
type GenericError = Box<dyn std::error::Error + Send + Sync>;

pub async fn start(
    port: u16,
    author: Arc<Mutex<ChannelAuthor>>,
    sql: mysql::Pool
) -> Result<(), GenericError> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let service = make_service_fn(move |_| {
        let author = author.clone();
        let sql = sql.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |req| {
                responder(
                    req,
                    author.clone(),
                    sql.clone()
                )
            }))
        }
    });

    /*use tokio_rustls::rustls::{PrivateKey, Certificate, ServerConfig};

    let cert = Certificate(include_bytes!("certs/molina.crt").to_vec());
    let key = PrivateKey(include_bytes!("certs/newmolina.key").to_vec());

    let tls_acceptor = TlsAcceptor::from(
        Arc::new(
            ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(vec![cert], key)
                .unwrap(),
        )
    );

    let incoming = TlsListener::new(tls_acceptor, AddrIncoming::bind(&addr)?)
        .filter(|conn| {
            if let Err(err) = conn {
                println!("Error making listener: {}", err);
                ready(false)
            } else {
                ready(true)
            }
        });

    let server = Server::builder(accept::from_stream(incoming)).serve(service);*/
    let server = Server::bind(&addr).serve(service);

    println!("API listening on http://{}", addr);

    server.await?;

    Ok(())
}

async fn responder(
    req: Request<Body>,
    author: Arc<Mutex<ChannelAuthor>>,
    sql: mysql::Pool
) -> Result<Response<Body>, GenericError> {
    match req.method() {
        &Method::OPTIONS => preflight_response().await,
        _ => match (req.method(), req.uri().path()) {
            (&Method::POST, "/subscribe") => subscribe_response(req, author).await,
            (&Method::GET, "/get_channel_address") => {
                channel_address_response(author).await
            }
            (&Method::GET, "/get_announcement_id") => {
                announcement_id_response(author).await
            }
            (&Method::GET, "/get_readings") => {
                readings_response(sql).await
            }
            (&Method::GET, "/get_annotations") => {
                annotations_response(sql).await
            }
            (&Method::POST, "/remove") => {
                removal_response(req, sql).await
            }
            _ => {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(NOTFOUND.into())
                    .unwrap())
            }
        }
    }
}
