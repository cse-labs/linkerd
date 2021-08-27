//
// Signer is an example simple grpc service. It uses tonic for grpc support and
// opentelemetry-jaeger to record tracing events.
//

use b3::ExMetadataMap;
use base64::encode;
use bytes::BytesMut;
use dill::dill::{
    sign_words_server::{SignWords, SignWordsServer},
    {SignRequest, WordsResponse},
};
use futures::FutureExt;
use log::{error, info, warn};
use opentelemetry::{
    global,
    global::shutdown_tracer_provider,
    trace::{noop::NoopTracerProvider, Span, Tracer},
};
use rand::SystemRandom;
use ring::{
    rand,
    signature::{
        RsaKeyPair,
        RSA_PSS_SHA256,
    },
};
use rocket::serde::Deserialize;
use simple_error::SimpleError;
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

    // service name
    #[structopt(
        short = "n",
        long = "tracing-service-name",
        default_value = "signing-svc"
    )]
    service_name: String,

    // jaeger collector endpoint
    #[structopt(
        short = "t",
        long = "trace-collector-endpoint",
        default_value = "http://collector.linkerd-jaeger:14268/api/traces"
    )]
    trace_collector_endpoint: String,
}

pub struct MySignWords {
    rsa_key_pair: RsaKeyPair,
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

        // Prepare the message buffer for signing
        let words = request.into_inner().words;
        let mut buffer = BytesMut::new();
        for w in &words {
            buffer.extend_from_slice(w.as_bytes());
        }
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let timestamp = u64::try_from(millis).unwrap();
        buffer.extend_from_slice(&timestamp.to_ne_bytes());
        let message = buffer.freeze();

        // Sign the message
        let key_pair = &self.rsa_key_pair;
        let rng = SystemRandom::new();
        let mut signature = vec![0; key_pair.public_modulus_len()];
        key_pair.sign(
            &RSA_PSS_SHA256,
            &rng,
            &message,
            &mut signature
        ).map_err(|_| {
            warn!("OOM signing message");
            span.record_exception(&SimpleError::new("OOM while signing message"));
        }).unwrap();
        let signature = encode(signature);
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

    info!("Service {}", args.service_name);

    // Setup tracing
    global::set_text_map_propagator(b3::Propagator::new());
    match opentelemetry_jaeger::new_pipeline()
        .with_service_name(args.service_name)
        .with_collector_endpoint(args.trace_collector_endpoint.clone())
        .build_batch(opentelemetry::runtime::Tokio)
    {
        Ok(provider) => {
            global::set_tracer_provider(provider);
            info!("Tracing to collector {}", args.trace_collector_endpoint);
        }
        Err(e) => {
            warn!("Failed to setup tracer: {}", e);
            global::set_tracer_provider(NoopTracerProvider::new());
        }
    };

    // Setup signing key
    let mut bytes: [u8; 1192] = [0; 1192];
    let mut file = File::open("keys/pickle_key.der")?;
    file.read(&mut bytes[..])?;
    let key_pair = RsaKeyPair::from_der(&bytes).unwrap();
    let sw = MySignWords {
        rsa_key_pair: key_pair,
    };
    drop(file);

    // Start service
    let addr = format!("0.0.0.0:{}", args.port).parse()?;

    info!("starting server on {}", addr);
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
