pub mod dill_rpc {
    tonic::include_proto!("dill");
}

#[macro_use] extern crate rocket;

use dill_rpc::pick_words_client::{PickWordsClient};
use dill_rpc::sign_words_client::{SignWordsClient};
use dill_rpc::{SignRequest, WordsRequest, WordsResponse};
use log::{error};
use rocket::form::{FromForm};
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::{json, Json, Value};

#[derive(Debug, Deserialize, Serialize)]
struct Words {
    words: Vec<String>,
    timestamp: Option<u64>,
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
async fn words(opt: Options) -> Option<Value> {
    let client = PickWordsClient::connect("http://words-svc:9090").await;
    let mut client = match client {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create GetWords client: {}", e);
            return None
        },
    };

    let request = tonic::Request::new(WordsRequest {
        count: u32::from(opt.count),
        signed: opt.signed,
    });

    let response = client.get_words(request).await;
    let response = match response {
        Ok(response) => response,
        Err(e) =>  {
            error!("Failed to call GetWords service: {}", e);
            return None
        },
    };

    Some(json!(Words::from(response.into_inner())))
}

#[get("/words")]
async fn words_default() -> Option<Value> {
    let opt = Options{ count: 8, signed: false };
    words(opt).await
}

#[post("/sign", data = "<words>")]
async fn sign_words(words: Json<Words>) -> Option<Value> {
    let client = SignWordsClient::connect("http://signing-svc:9090").await;
    let mut client = match client {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create SignWords client: {}", e);
            return None
        },
    };

    let v = &words.words;
    let request = tonic::Request::new(SignRequest {
        words: v.to_vec(),
    });

    let response = client.sign_words(request).await;
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to call SignWords service: {}", e);
            return None
        },
    };

    Some(json!(Words::from(response.into_inner())))
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![sign_words, words_default, words, index])
}

impl Words {

    fn from(proto: WordsResponse) -> Words {
        Words {
            words: proto.words,
            timestamp: Some(proto.timestamp.unwrap()),
            signature: Some(proto.signature.unwrap()),
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
