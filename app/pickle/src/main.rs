pub mod dill_rpc {
    tonic::include_proto!("dill");
}

#[macro_use] extern crate rocket;

use dill_rpc::pick_words_client::{PickWordsClient};
use dill_rpc::sign_words_client::{SignWordsClient};
use dill_rpc::{SignRequest, WordsRequest, WordsResponse};
use log::error;
use rocket::form::{FromForm};
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::Json;
use std::time::Duration;
use tonic::transport::Channel;
use tower::timeout::Timeout;

#[derive(Debug, Deserialize, Serialize)]
struct Words {
    words: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

#[derive(FromForm)]
struct Options {
    count: u8,
    signed: bool,
}

#[get("/")]
fn index() -> &'static str {
    "GET /words?count=4&signed or POST /sign with json words: list body"
}

#[get("/words?<opt..>")]
async fn words(opt: Options) -> Option<String> {
    let channel = match Channel::from_static("http://words-svc:9090").connect().await {
        Ok(channel) => channel,
        Err(e) => {
            error!("Failed to create GetWords channel: {}", e);
            return None
        },
    };

    let timeout_channel = Timeout::new(channel, Duration::from_millis(500));
    let mut client = PickWordsClient::new(timeout_channel);

    let request = tonic::Request::new(WordsRequest {
        count: u32::from(opt.count),
        signed: opt.signed,
    });

    let response = match client.get_words(request).await {
        Ok(response) => response,
        Err(e) =>  {
            error!("Failed to call GetWords service: {}", e);
            return None
        },
    };

    Some(serde_json::to_string_pretty(&Words::from(response.into_inner())).unwrap())
}

#[get("/words")]
async fn words_default() -> Option<String> {
    let opt = Options{ count: 4, signed: false };
    words(opt).await
}

#[post("/sign", data = "<words>")]
async fn sign_words(words: Json<Words>) -> Option<String> {
    let channel = match Channel::from_static("http://signing-svc:9090").connect().await {
        Ok(channel) => channel,
        Err(e) => {
            error!("Failed to create SignWords channel: {}", e);
            return None
        },
    };

    let timeout_channel = Timeout::new(channel, Duration::from_millis(500));
    let mut client = SignWordsClient::new(timeout_channel);

    let v = &words.words;
    let request = tonic::Request::new(SignRequest {
        words: v.to_vec(),
    });

    let response = match client.sign_words(request).await {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to call SignWords service: {}", e);
            return None
        },
    };

    Some(serde_json::to_string_pretty(&Words::from(response.into_inner())).unwrap())
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![sign_words, words_default, words, index])
}

impl Words {

    fn from(proto: WordsResponse) -> Words {
        Words {
            words: proto.words,
            timestamp: match proto.timestamp {
                Some(time) => Some(time),
                _ => None,
            },
            signature: match proto.signature {
                Some(signature) => Some(signature),
                _ => None,
            },
        }
    }
}

impl PartialEq for Words {

    fn eq(&self, other: &Self) -> bool {
        (self.words.len() == other.words.len()) &&
            self.words.iter()
            .zip(&other.words)
            .all(|(a,b)| a == b) &&
        self.timestamp == self.timestamp &&
        self.signature == self.signature
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn words_from_wordsresponse() {
        let p = WordsResponse {
            words: vec![String::from("happy"), String::from("hungry"), String::from("hare")],
            ..Default::default()
        };
        let s = Words::from(p);
        assert_eq!(s, Words {
            words: vec![String::from("happy"), String::from("hungry"), String::from("hare")],
            timestamp: None,
            signature: None,
        })
    }
}
