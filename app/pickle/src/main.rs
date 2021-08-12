#[macro_use] extern crate rocket;

use dill_rpc::pick_words_client::{PickWordsClient};
use dill_rpc::sign_words_client::{SignWordsClient};
use dill_rpc::{SignRequest, SignResponse, WordsRequest, WordsResponse};
use tonic::{transport::Server, Request, Response, Status};

#[get("/")]
fn words() -> &'static str {
    let mut client = PickWordsClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(WordsRequest {
        count: ,
        signed: ,
    });

    let response = client.get_words(request).await?;
}

#[post("/")]
fn sign_words() -> &'static str {
    let mut client = SignWordsClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(SignRequest {
        
    });

    let response = client.sign_words(request).await?;
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![words, sign_words])
}
