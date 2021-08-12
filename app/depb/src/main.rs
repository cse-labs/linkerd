pub mod dill_rpc {
    tonic::include_proto!("dill");
}

use base64::encode;
use dill_rpc::sign_words_server::{SignWords, SignWordsServer};
use dill_rpc::{SignRequest, SignResponse};
use openssl::sign::Signer;
use openssl::rsa::Rsa;
use openssl::pkey::{PKey, Private};
use openssl::hash::MessageDigest;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{transport::Server, Request, Response, Status};

pub struct MySignWords {
    keypair: PKey<Private>,
}

#[tonic::async_trait]
impl SignWords for MySignWords {
    async fn sign_words(&self, request: Request<SignRequest>) -> Result<Response<SignResponse>, Status> {
        
        let mut signer = Signer::new(MessageDigest::sha256(), &self.keypair).unwrap();
        let words = request.into_inner().words;
        for w in &words {
            signer.update(w.as_bytes()).unwrap();
        }
        let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let timestamp = u64::try_from(millis).unwrap();
        signer.update(&timestamp.to_ne_bytes()).unwrap();
        let signature = encode(signer.sign_to_vec().unwrap());

        let reply = SignResponse {
            words: words,
            signature: signature,
            timestamp: timestamp,
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    let mut bytes: [u8; 8192] = [0; 8192];
    let mut file = File::open("keys/pickle.key")?;
    file.read(&mut bytes[..])?;
    let rsakey = Rsa::private_key_from_pem(&bytes).unwrap();
    let sw = MySignWords{ keypair: PKey::from_rsa(rsakey).unwrap() };

    Server::builder()
        .add_service(SignWordsServer::new(sw))
        .serve(addr)
        .await?;

    Ok(())
}
