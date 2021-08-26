//
// Signer is an example simple grpc service. It uses tonic for grpc support and
// opentelemetry-jaeger to record tracing events.
//

use b3::ExMetadataMap;
use base64::encode;
use dill::dill::{
    {SignRequest, WordsResponse},
    sign_words_server::{SignWords, SignWordsServer},
};
use futures::FutureExt;
use log::{error, info};
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Private},
    rsa::Rsa,
    sign::Signer,
};
use opentelemetry::{
    global,
    global::shutdown_tracer_provider,
    trace::{
        Span, Tracer,
        noop::NoopTracerProvider,
    }
};
use rocket::serde::Deserialize;
use std::{
    convert::TryFrom,
    fs::File,
    io::Read,
    time::{SystemTime, UNIX_EPOCH},
};
use structopt::StructOpt;
use tokio::{signal, sync::oneshot};
use tonic::{transport::Server, Request, Response, Status};

#[derive(StructOpt, Deserialize)]
struct Args {
    // port for grpc service to listen on
    #[structopt(short = "p", long = "port", default_value = "9090")]
    port: u16,
}

pub struct MySignWords {
    keypair: PKey<Private>,
}

#[tonic::async_trait]
impl SignWords for MySignWords {
    async fn sign_words(
        &self,
        request: Request<SignRequest>,
    ) -> Result<Response<WordsResponse>, Status> {
        let cx = global::get_text_map_propagator(|propagator| {
            propagator.extract(&ExMetadataMap(request.metadata()))
        });
        let mut span = global::tracer("signer").start_with_context("signing words", cx);

        let mut signer = Signer::new(MessageDigest::sha256(), &self.keypair).unwrap();
        let words = request.into_inner().words;
        for w in &words {
            signer.update(w.as_bytes()).unwrap();
        }
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let timestamp = u64::try_from(millis).unwrap();
        signer.update(&timestamp.to_ne_bytes()).unwrap();
        let signature = encode(signer.sign_to_vec().unwrap());
        span.add_event("signed words".to_string(), Vec::new());

        span.end();

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

    global::set_text_map_propagator(b3::Propagator::new());
    match opentelemetry_jaeger::new_pipeline()
        .with_service_name("signing-svc")
        .with_collector_endpoint("http://collector.linkerd-jaeger:14268/api/traces")
        .build_batch(opentelemetry::runtime::Tokio)
    {
        Ok(provider) => global::set_tracer_provider(provider),
        Err(e) => {
            error!("Failed to setup tracer: {}", e);
            global::set_tracer_provider(NoopTracerProvider::new())
        }
    };
    info!("depb");

    let addr = format!("0.0.0.0:{}", args.port).parse()?;

    let mut bytes: [u8; 8192] = [0; 8192];
    let mut file = File::open("keys/pickle.key")?;
    file.read(&mut bytes[..])?;
    let rsakey = Rsa::private_key_from_pem(&bytes).unwrap();
    let sw = MySignWords {
        keypair: PKey::from_rsa(rsakey).unwrap(),
    };
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
        Ok(()) => {}
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        }
    };
    tx.send(()).unwrap();
    server.await.unwrap();
    shutdown_tracer_provider();
    Ok(())
}
