#[macro_use] extern crate rocket;

use b3::{HeaderExtractor, InMetadataMap, RocketHttpHeaderMap};
use dill::dill::pick_words_client::{PickWordsClient};
use dill::dill::sign_words_client::{SignWordsClient};
use dill::dill::{SignRequest, WordsRequest, WordsResponse};
use log::error;
use once_cell::sync::OnceCell;
use opentelemetry::global;
use opentelemetry::trace::noop::NoopTracerProvider;
use rocket::get;
use rocket::response::content::Html;
use rocket::serde::{Deserialize, Serialize, json::Json};
use rocket_okapi::{openapi, routes_with_openapi, JsonSchema};
use rocket_okapi::swagger_ui::{make_swagger_ui, SwaggerUIConfig};
use std::time::Duration;
use tonic::transport::Channel;
use tower::timeout::Timeout;

#[derive(Debug)]
struct Config {
    words_svc_addr: String,
    sign_svc_addr: String,
}
static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct Words {
    words: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

#[openapi]
#[get("/")]
fn index() -> Html<&'static str> {
    Html(r#"<html><body><a href="api/v1.0/openapi.json">OpenAPI docs</a></body><html>"#)
}

#[openapi]
#[get("/words?<count>&<signed>")]
async fn words(header_map: RocketHttpHeaderMap<'_>, count: Option<u8>, signed: bool) -> Option<Json<Words>>  {
    let channel = match Channel::from_static(&CONFIG.get().unwrap().words_svc_addr).connect().await {
        Ok(channel) => channel,
        Err(e) => {
            error!("Failed to create GetWords channel: {}", e);
            return None
        },
    };

    let cnt = match count {
        Some(cnt) => cnt,
        None => 3,
    };

    let timeout_channel = Timeout::new(channel, Duration::from_millis(500));
    let mut client = PickWordsClient::new(timeout_channel);

    let mut request = tonic::Request::new(WordsRequest {
        count: u32::from(cnt),
        signed: signed,
    });

    global::get_text_map_propagator(|propagator| {
        let cx = propagator.extract(&HeaderExtractor(header_map.0));
        propagator.inject_context(&cx, &mut InMetadataMap(request.metadata_mut()));
    });

    let response = match client.get_words(request).await {
        Ok(response) => response,
        Err(e) =>  {
            error!("Failed to call GetWords service: {}", e);
            return None
        },
    };

    Some(Json(Words::from(response.into_inner())))
}

#[openapi]
#[post("/sign", data = "<words>")]
async fn sign_words(header_map: RocketHttpHeaderMap<'_>, words: Json<Words>) -> Option<Json<Words>> {
    let channel = match Channel::from_static(&CONFIG.get().unwrap().sign_svc_addr).connect().await {
            Ok(channel) => channel,
        Err(e) => {
            error!("Failed to create GetWords channel: {}", e);
            return None
        },
    };

    let timeout_channel = Timeout::new(channel, Duration::from_millis(500));
    let mut client = SignWordsClient::new(timeout_channel);

    let v = &words.words;
    let mut request = tonic::Request::new(SignRequest {
        words: v.to_vec(),
    });

    global::get_text_map_propagator(|propagator| {
        let cx = propagator.extract(&HeaderExtractor(header_map.0));
        propagator.inject_context(&cx, &mut InMetadataMap(request.metadata_mut()));
    });

    let response = match client.sign_words(request).await {
        Ok(response) => response,
        Err(e) =>  {
            error!("Failed to call GetWords service: {}", e);
            return None
        },
    };

    Some(Json(Words::from(response.into_inner())))
}

fn get_docs() -> SwaggerUIConfig {
    SwaggerUIConfig {
        ..Default::default()
    }
}

#[launch]
fn rocket() -> _ {
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

    let rocket = rocket::build()
        .mount("/", routes_with_openapi![index])
        .mount("/api/v1.0", routes_with_openapi![sign_words, words])
        .mount("/swagger", make_swagger_ui(&get_docs()));
    let figment = rocket.figment();
    let config = Config {
        words_svc_addr: figment.extract_inner("words-svc-addr").expect("words-svc-addr"),
        sign_svc_addr: figment.extract_inner("sign-svc-addr").expect("signs-svc-addr"),
    };
    CONFIG.set(config).unwrap();
    rocket
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
