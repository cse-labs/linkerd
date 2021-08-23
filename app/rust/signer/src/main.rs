use base64::encode;
use dill::dill::sign_words_server::{SignWords, SignWordsServer};
use dill::dill::{SignRequest, WordsResponse};
use futures::FutureExt;
use log::{error, info};
use openssl::sign::Signer;
use openssl::rsa::Rsa;
use openssl::pkey::{PKey, Private};
use openssl::hash::MessageDigest;
use opentelemetry::global;
use opentelemetry::trace::noop::NoopTracerProvider;
use rocket::serde::Deserialize;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
use structopt::StructOpt;
use tokio::{signal, sync::oneshot};
use tonic::{transport::Server, Request, Response, Status};
use tracing::*;

#[derive(StructOpt, Deserialize)]
struct Args {

    // pretty print the json or use compact form
    #[structopt(short = "p", long = "port", default_value = "9090")]
    port: u16,
}

#[derive(Debug)]
pub struct MySignWords {
    keypair: PKey<Private>,
}

#[tonic::async_trait]
impl SignWords for MySignWords {

    #[instrument]
    async fn sign_words(&self, request: Request<SignRequest>) -> Result<Response<WordsResponse>, Status> {
        
        let mut signer = Signer::new(MessageDigest::sha256(), &self.keypair).unwrap();
        let words = request.into_inner().words;
        for w in &words {
            signer.update(w.as_bytes()).unwrap();
        }
        let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let timestamp = u64::try_from(millis).unwrap();
        signer.update(&timestamp.to_ne_bytes()).unwrap();
        let signature = encode(signer.sign_to_vec().unwrap());

        let reply = WordsResponse {
            words: words,
            timestamp: Some(timestamp),
            signature: Some(signature),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::from_args();

    match opentelemetry_jaeger::new_pipeline()
            .with_service_name("words")
            .with_collector_endpoint("http://collector.linkerd-jaeger:55678")
            .build_simple() {
            //.build_batch(opentelemetry::runtime::Tokio) {
        Ok(provider) => global::set_tracer_provider(provider),
        Err(e) =>  {
            error!("Failed to setup tracer: {}", e);
             global::set_tracer_provider(NoopTracerProvider::new())
        },
    };

    info!("depb");

    let addr = format!("0.0.0.0:{}", args.port).parse()?;

    let mut bytes: [u8; 8192] = [0; 8192];
    let mut file = File::open("keys/pickle.key")?;
    file.read(&mut bytes[..])?;
    let rsakey = Rsa::private_key_from_pem(&bytes).unwrap();
    let sw = MySignWords{ keypair: PKey::from_rsa(rsakey).unwrap() };
    drop(file);

    info!("starting server");
    info!("SignServer listening on {}", addr);
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(SignWordsServer::new(sw))
            .serve_with_shutdown(addr, rx.map(drop))
            .await
            .unwrap();
    });

    // graceful shutdown on ctrl-c
    match signal::ctrl_c().await {
        Ok(()) => {},
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        },
    };
    tx.send(()).unwrap();
    server.await.unwrap();
    global::shutdown_tracer_provider();
    Ok(())
}
