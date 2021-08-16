pub mod dill_rpc {
    tonic::include_proto!("dill");
}

use dill_rpc::pick_words_server::{PickWords, PickWordsServer};
use dill_rpc::sign_words_client::{SignWordsClient};
use dill_rpc::{SignRequest, WordsRequest, WordsResponse};
use log::{error, info};
use names::Generator;
use tonic::{transport::Server, Request, Response, Status};

#[derive(Default)]
pub struct MyPickWords {}

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
        let words_request = request.into_inner();
        let count = words_request.count.into();
        let sign = words_request.signed.into();

        let words = generate_words(count);

        match sign {
            true => {
                let client = SignWordsClient::connect("http://signing-svc:9090").await;
                let mut client = match client {
                    Ok(client) => client,
                    Err(e) => {
                        error!("Failed to create SignWords client: {}", e);
                        return Err(Status::unknown(format!("error creating signing client")))
                    },
                };

                let v = &words;
                let request = tonic::Request::new(SignRequest {
                    words: v.to_vec(),
                });

                let response = client.sign_words(request).await;
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
    info!("depa");

    let addr = "0.0.0.0:9090".parse()?;
    let pw = MyPickWords::default();

    info!("WordsServer listening on {}", addr);

    info!("starting server");
    Server::builder()
        .add_service(PickWordsServer::new(pw))
        .serve(addr)
        .await?;

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
