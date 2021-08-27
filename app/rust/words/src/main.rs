//
// Words is an example simple grpc service. It uses tonic for grpc support.
//

use b3::{ExMetadataMap, InMetadataMap};
use dill::dill::{
    pick_words_server::{PickWords, PickWordsServer},
    sign_words_client::SignWordsClient,
    SignRequest, WordsRequest, WordsResponse,
};
use futures::FutureExt;
use log::{error, info, warn};
use names::Generator;
use opentelemetry::{
    global,
    global::shutdown_tracer_provider,
    trace::{noop::NoopTracerProvider, Span, TraceContextExt, Tracer},
    Context,
};
use rocket::serde::Deserialize;
use std::time::Duration;
use structopt::StructOpt;
use tokio::{signal, sync::oneshot};
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

#[derive(StructOpt, Deserialize)]
struct Args {
    // port for the grpc service to listen on
    #[structopt(short = "p", long = "port", default_value = "9090")]
    port: u16,

    // address of the SignWords grpc service
    #[structopt(
        short = "s",
        long = "sign-svc-addr",
        default_value = "http://signing-svc:9090"
    )]
    sign_svc_addr: String,

    // service name
    #[structopt(
        short = "n",
        long = "tracing-service-name",
        default_value = "words-svc"
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

// grpc service
pub struct MyPickWords {
    sign_words_channel: Channel,
}

/// Returns a list of adjectives followed by a noun
///
/// # Arguments
///
/// * `count` - Number of words to return
fn generate_words(count: u32) -> Vec<String> {
    let mut words = Vec::new();
    let mut gen = Generator::default();
    match count {
        0 => (),
        1 => words.push(gen.next().unwrap().split('-').collect::<Vec<&str>>()[1].to_string()), // noun
        _ => {
            for _ in 2..count {
                words.push(gen.next().unwrap().split('-').collect::<Vec<&str>>()[0].to_string())
                // adjectives
            }
            for w in gen.next().unwrap().split('-') {
                // adjective and noun
                words.push(w.to_string());
            }
        }
    }
    words
}

#[tonic::async_trait]
impl PickWords for MyPickWords {
    async fn get_words(
        &self,
        request: Request<WordsRequest>,
    ) -> Result<Response<WordsResponse>, Status> {
        let cx = global::get_text_map_propagator(|propagator| {
            propagator.extract(&ExMetadataMap(request.metadata()))
        });
        let words_request = request.into_inner();
        let count = words_request.count.into();
        let sign = words_request.signed.into();

        let mut w_span = global::tracer("words").start_with_context("generating words", cx.clone());
        let words = generate_words(count);
        w_span.end();

        match sign {
            false => {
                let reply = WordsResponse {
                    words: words,
                    ..Default::default()
                };
                return Ok(Response::new(reply));
            }
            true => {
                let mut s_span =
                    global::tracer("words").start_with_context("requesting signature", cx);

                let v = &words;
                let mut req = tonic::Request::new(SignRequest { words: v.to_vec() });

                let grpc_cx =
                    &Context::new().with_remote_span_context(s_span.span_context().clone());
                global::get_text_map_propagator(|propagator| {
                    propagator.inject_context(grpc_cx, &mut InMetadataMap(req.metadata_mut()));
                });

                let response = SignWordsClient::new(self.sign_words_channel.clone())
                    .sign_words(req)
                    .await;
                match response {
                    Ok(response) => {
                        s_span.end();
                        return Ok(response);
                    }
                    Err(e) => {
                        error!("Failed to call SignWords service: {}", e);
                        s_span.record_exception(&e);
                        s_span.end();
                        return Err(Status::unknown(format!("error invoking signing service")));
                    }
                };
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::from_args();

    info!("Service {}", args.service_name);

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

    let channel = Channel::from_shared(args.sign_svc_addr.clone())
        .unwrap()
        .timeout(Duration::from_millis(500))
        .connect()
        .await?;
    let pw = MyPickWords {
        sign_words_channel: channel,
    };
    let addr = format!("0.0.0.0:{}", args.port).parse()?;

    info!("starting server on {}", addr);
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(PickWordsServer::new(pw))
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

// Unit tests

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn generate_words_with_one() {
        let a = generate_words(1);
        assert_eq!(a.len(), 1);
        assert!(a[0].len() > 0);
    }

    #[test]
    fn generate_words_with_two() {
        let a = generate_words(2);
        assert_eq!(a.len(), 2);
        assert!(a[0].len() > 0);
        assert!(a[1].len() > 0);
    }

    #[test]
    fn generate_words_with_zero() {
        let a = generate_words(0);
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn generate_words_with_three() {
        let a = generate_words(3);
        assert_eq!(a.len(), 3);
        assert_eq!(
            a.into_iter()
                .filter(|w| w.len() > 0)
                .collect::<Vec<String>>()
                .len(),
            3
        );
    }

    #[test]
    fn generate_words_with_even() {
        let a = generate_words(42);
        assert_eq!(a.len(), 42);
        assert_eq!(
            a.into_iter()
                .filter(|w| w.len() > 0)
                .collect::<Vec<String>>()
                .len(),
            42
        );
    }

    #[test]
    fn generate_words_with_odd() {
        let a = generate_words(93);
        assert_eq!(a.len(), 93);
        assert_eq!(
            a.into_iter()
                .filter(|w| w.len() > 0)
                .collect::<Vec<String>>()
                .len(),
            93
        );
    }
}
