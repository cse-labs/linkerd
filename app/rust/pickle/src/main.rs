//
// Pickle is an example service that implements an RPC web api in front of grpc
// services. It uses Rocket as its web framework and tonic for grpc support.
//

#[macro_use]
extern crate rocket;

use b3::{HeaderExtractor, InMetadataMap, RocketHttpHeaderMap};
use dill::dill::{
    SignRequest, WordsRequest, WordsResponse,
    pick_words_client::PickWordsClient,
    sign_words_client::SignWordsClient,
};
use log::error;
use once_cell::sync::OnceCell;
use opentelemetry::{
    Context,
    global,
    trace::{
        Span, Tracer, TraceContextExt,
        noop::NoopTracerProvider,
    }
};
use rocket::{
    get,
    response::content::Html,
    serde::{Deserialize, Serialize, json::Json}
};
use rocket_okapi::{
    openapi, routes_with_openapi, JsonSchema,
    swagger_ui::{make_swagger_ui, SwaggerUIConfig},
};
use std::{
    panic,
    time::Duration,
};
use tonic::transport::Channel;

// App-specific config provided using Rocket config
#[derive(Debug)]
struct Config {
    words_svc_addr: String,
    sign_svc_addr: String,
    tracing_service_name: String,
    trace_collector_endpoint: String,
}
static CONFIG: OnceCell<Config> = OnceCell::new();

static WORDS_CHANNEL: OnceCell<Channel> = OnceCell::new();
static SIGN_CHANNEL: OnceCell<Channel> = OnceCell::new();

// json return value
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
    Html(r#"<html>
        <body>
            <a href="swagger/index.html">Swagger docs</a><br/>
            <a href="api/v1.0/openapi.json">OpenAPI docs</a>
        </body>
    <html>"#)
}

#[openapi]
#[get("/words?<count>&<signed>")]
async fn words(
    header_map: RocketHttpHeaderMap<'_>,
    count: Option<u8>,
    signed: bool,
) -> Option<Json<Words>> {
    let cx = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(header_map.0))
    });
    let mut span = global::tracer("pickle web").start_with_context("words", cx.clone());

    let cnt = match count {
        Some(cnt) => cnt,
        None => 3,
    };
    
    let mut client = PickWordsClient::new(WORDS_CHANNEL.get().unwrap().clone());
    let mut request = tonic::Request::new(WordsRequest {
        count: u32::from(cnt),
        signed: signed,
    });

    let grpc_cx = &Context::new().with_remote_span_context(span.span_context().clone());
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&grpc_cx, &mut InMetadataMap(request.metadata_mut()));
    });

    let response = match client.get_words(request).await {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to call GetWords service: {}", e);
            span.record_exception(&e);
            return None;
        }
    };

    span.end();

    Some(Json(Words::from(response.into_inner())))
}

#[openapi]
#[post("/sign", data = "<words>")]
async fn sign_words(
    header_map: RocketHttpHeaderMap<'_>,
    words: Json<Words>,
) -> Option<Json<Words>> {
    let cx = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(header_map.0))
    });
    let mut span = global::tracer("pickle web").start_with_context("sign_words", cx.clone());

    let v = &words.words;

    let mut client = SignWordsClient::new(SIGN_CHANNEL.get().unwrap().clone());
    let mut request = tonic::Request::new(SignRequest { words: v.to_vec() });

    let grpc_cx = &Context::new().with_remote_span_context(span.span_context().clone());
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&grpc_cx, &mut InMetadataMap(request.metadata_mut()));
    });

    let response = match client.sign_words(request).await {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to call GetWords service: {}", e);
            span.record_exception(&e);
            return None;
        }
    };

    span.end();

    Some(Json(Words::from(response.into_inner())))
}

fn get_docs() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "../api/v1.0/openapi.json".to_owned(),
        ..Default::default()
    }
}

#[launch]
async fn rocket() -> _ {
    let rocket = rocket::build()
        .mount("/", routes_with_openapi![index])
        .mount("/api/v1.0", routes_with_openapi![sign_words, words])
        .mount("/swagger", make_swagger_ui(&get_docs()));
    let figment = rocket.figment();
    let config = Config {
        words_svc_addr: figment
            .find_value("words-svc-addr")
            .unwrap()
            .into_string()
            .unwrap(),
        sign_svc_addr: figment
            .find_value("sign-svc-addr")
            .unwrap()
            .into_string()
            .unwrap(),
        tracing_service_name: figment
            .find_value("tracing-service-name")
            .unwrap()
            .into_string()
            .unwrap(),
        trace_collector_endpoint: figment
            .find_value("trace-collector-endpoint")
            .unwrap()
            .into_string()
            .unwrap(),
    };
    CONFIG.set(config).unwrap();

    global::set_text_map_propagator(b3::Propagator::new());
    match opentelemetry_jaeger::new_pipeline()
        .with_service_name(&CONFIG.get().unwrap().tracing_service_name)
        .with_collector_endpoint(&CONFIG.get().unwrap().trace_collector_endpoint)
        .build_batch(opentelemetry::runtime::Tokio)
    {
        Ok(provider) => {
            global::set_tracer_provider(provider);
            info!("Tracing to collector {}", &CONFIG.get().unwrap().trace_collector_endpoint);
        }
        Err(e) => {
            warn!("Failed to setup tracer: {}", e);
            global::set_tracer_provider(NoopTracerProvider::new());
        }
    };

    let sign_addr = &CONFIG.get().unwrap().sign_svc_addr;
    let sign_channel = match Channel::from_static(sign_addr)
        .timeout(Duration::from_millis(500))
        .connect()
        .await
    {
            Ok(channel) => channel,
            Err(e) => {
                panic!("Failed to create Signs channel: {}", e);
            },
    };
    SIGN_CHANNEL.set(sign_channel).unwrap();

    let words_addr = &CONFIG.get().unwrap().words_svc_addr;
    let words_channel = match Channel::from_static(words_addr)
        .timeout(Duration::from_millis(500))
        .connect()
        .await
    {
            Ok(channel) => channel,
            Err(e) => {
                panic!("Failed to create Words channel: {}", e);
            },
    };
    WORDS_CHANNEL.set(words_channel).unwrap();

    rocket
}

// Convenience functions for working with Words and WordsResponses

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
        (self.words.len() == other.words.len())
            && self.words.iter().zip(&other.words).all(|(a, b)| a == b)
            && self.timestamp == self.timestamp
            && self.signature == self.signature
    }
}

// Unit tests

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn words_from_wordsresponse() {
        let p = WordsResponse {
            words: vec![
                String::from("happy"),
                String::from("hungry"),
                String::from("hare"),
            ],
            ..Default::default()
        };
        let s = Words::from(p);
        assert_eq!(
            s,
            Words {
                words: vec![
                    String::from("happy"),
                    String::from("hungry"),
                    String::from("hare")
                ],
                timestamp: None,
                signature: None,
            }
        )
    }
}
