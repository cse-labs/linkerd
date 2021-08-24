use b3::{ExMetadataMap, InMetadataMap};
use dill::dill::pick_words_server::{PickWords, PickWordsServer};
use dill::dill::sign_words_client::{SignWordsClient};
use dill::dill::{SignRequest, WordsRequest, WordsResponse};
use futures::FutureExt;
use log::{error, info};
use names::Generator;
use opentelemetry::global;
use opentelemetry::trace::noop::NoopTracerProvider;
use rocket::serde::Deserialize;
use std::time::Duration;
use structopt::StructOpt;
use tokio::{signal, sync::oneshot};
use tonic::{transport::{Channel, Server}, Request, Response, Status};
use tower::timeout::Timeout;

#[derive(StructOpt, Deserialize)]
struct Args {

    // address of the SignWords grpc service
    #[structopt(short = "s", long = "sign-svc-addr", default_value = "http://signing-svc:9090")]
    sign_svc_addr: String,

    // pretty print the json or use compact form
    #[structopt(short = "p", long = "port", default_value = "9090")]
    port: u16,
}

#[derive(Default)]
pub struct MyPickWords {
    sign_svc_addr: String,
}

fn generate_words(count: u32) -> Vec<String> {
    let mut words = Vec::new();
    let mut gen = Generator::default();
    match count {
        0 => (),
        1 => words.push(gen.next().unwrap().split('-').collect::<Vec<&str>>()[1].to_string()), // noun
        _ => {
            for _ in 2..count {
                words.push(gen.next().unwrap().split('-').collect::<Vec<&str>>()[0].to_string()) // adjectives
            }
            for w in gen.next().unwrap().split('-') { // adjective and noun
                words.push(w.to_string());
            }
        },
    }
    words
}

#[tonic::async_trait]
impl PickWords for MyPickWords {
    async fn get_words(&self, request: Request<WordsRequest>) -> Result<Response<WordsResponse>, Status> {
        let cx = global::get_text_map_propagator(|propagator| propagator.extract(&ExMetadataMap(request.metadata())));
        let words_request = request.into_inner();
        let count = words_request.count.into();
        let sign = words_request.signed.into();

        let words = generate_words(count);

        match sign {
            true => {
                let addr = self.sign_svc_addr.clone();
                let channel = match Channel::from_shared(addr).unwrap().connect().await {
                    Ok(channel) => channel,
                    Err(e) => {
                        error!("Failed to create SignWords channel: {}", e);
                        return Err(Status::unknown(format!("error creating channel to signing service")))
                    },
                };
            
                let timeout_channel = Timeout::new(channel, Duration::from_millis(500));
                let mut client = SignWordsClient::new(timeout_channel);

                let v = &words;
                let mut req = tonic::Request::new(SignRequest {
                    words: v.to_vec(),
                });

                global::get_text_map_propagator(|propagator| {
                    //let cx = propagator.extract(&ExMetadataMap(request.metadata()));
                    propagator.inject_context(&cx, &mut InMetadataMap(req.metadata_mut()));
                });

                let response = client.sign_words(req).await;
                match response {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        error!("Failed to call SignWords service: {}", e);
                        return Err(Status::unknown(format!("error invoking signing service")))
                    },
                };
            },
            false => {
                let reply = WordsResponse {
                    words: words,
                    ..Default::default()
                };
        
                return Ok(Response::new(reply))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::from_args();

    global::set_text_map_propagator(b3::Propagator::new());
    match opentelemetry_jaeger::new_pipeline()
            .with_service_name("pickle")
            .with_collector_endpoint("http://collector.linkerd-jaeger:55678")
            .build_batch(opentelemetry::runtime::Tokio) {
        Ok(provider) => global::set_tracer_provider(provider),
        Err(e) =>  {
            error!("Failed to setup tracer: {}", e);
             global::set_tracer_provider(NoopTracerProvider::new())
        },
    };
    info!("depa");

    let addr = format!("0.0.0.0:{}", args.port).parse()?;
    let pw = MyPickWords{ sign_svc_addr: args.sign_svc_addr };

    info!("starting server");
    info!("WordsServer listening on {}", addr);
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
        Ok(()) => {},
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        },
    };
    tx.send(()).unwrap();
    server.await.unwrap();
    Ok(())
}

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
        assert_eq!(a.into_iter().filter(|w| w.len() > 0).collect::<Vec<String>>().len(), 3);
    }

    #[test]
    fn generate_words_with_even() {
        let a = generate_words(42);
        assert_eq!(a.len(), 42);
        assert_eq!(a.into_iter().filter(|w| w.len() > 0).collect::<Vec<String>>().len(), 42);
    }

    #[test]
    fn generate_words_with_odd() {
        let a = generate_words(93);
        assert_eq!(a.len(), 93);
        assert_eq!(a.into_iter().filter(|w| w.len() > 0).collect::<Vec<String>>().len(), 93);
    }
}
