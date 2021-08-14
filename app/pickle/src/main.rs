pub mod dill_rpc {
    tonic::include_proto!("dill");
}

#[macro_use] extern crate rocket;

use dill_rpc::pick_words_client::{PickWordsClient};
use dill_rpc::sign_words_client::{SignWordsClient};
use dill_rpc::{SignRequest, WordsRequest, WordsResponse};
use rocket::form::{FromForm};
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::{json, Json, Value};

#[derive(Deserialize, Serialize)]
struct Words {
    words: Vec<String>,
    timestamp: u64,
    signature: String,
}

#[derive(FromForm)]
struct Params {
    count: u8,
    signed: bool,
}

#[get("/")]
fn index() -> &'static str {
    "words"
}

#[get("/words?<params>")]
async fn words(params: Params) -> Option<Value> {
    let client = PickWordsClient::connect("http://pickle-depa:9090").await;

    let mut client = match client {
        Ok(client) => client,
        Err(_e) => return None
    };

    let request = tonic::Request::new(WordsRequest {
        count: u32::from(params.count),
        signed: params.signed,
    });

    let response = client.get_words(request).await;

    let response = match response {
        Ok(response) => response,
        Err(_e) => return None
    };

    Some(json!(Words::from(response.into_inner())))
}

#[post("/words", data = "<words>")]
async fn sign_words(words: Json<Words>) -> Option<Value> {
    let client = SignWordsClient::connect("http://pickle-depb:9090").await;

    let mut client = match client {
        Ok(client) => client,
        Err(_e) => return None
    };

    let v = &words.words;

    let request = tonic::Request::new(SignRequest {
        words: v.to_vec(),
    });

    let response = client.sign_words(request).await;

    let response = match response {
        Ok(response) => response,
        Err(_e) => return None
    };

    Some(json!(Words::from(response.into_inner())))
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, words, sign_words])
}

impl Words {

    fn from(proto: WordsResponse) -> Words {
        Words {
            words: proto.words,
            timestamp: proto.timestamp.unwrap(),
            signature: proto.signature.unwrap(),
        }
    }
}
