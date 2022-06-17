mod helper;
use gcp_auth::{AuthenticationManager, CustomServiceAccount};
use lambda_http::{
    http::StatusCode, run, service_fn, Error, IntoResponse, Request, RequestExt, Response,
};
use serde_json::Value;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tracing::{error, info};

use crate::helper::handle_get_object_error;

async fn function_handler(event: Request) -> Result<impl IntoResponse, Error> {
    info!("collect env: S3_BUCKET");
    let bucket = std::env::var("S3_BUCKET").map_err(Box::new)?;
    info!("extract from incoming request parameter:s3_key");
    let service_account_key = match event.query_string_parameters().first("s3_key") {
        Some(s3_key) => s3_key.to_owned(),
        None => {
            error!("no parameter 's3_key'");
            return Ok(Response::builder()
                .status(400)
                .body("parameters is not applied".to_string())
                .map_err(Box::new)?);
        }
    };
    info!("search for key:{}", service_account_key);
    let config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&config);
    let service_account_json = match s3_client
        .get_object()
        .bucket(bucket)
        .key(service_account_key)
        .send()
        .await
    {
        Ok(output) => {
            let mut buf = Vec::new();
            BufReader::new(output.body.into_async_read())
                .read_to_end(&mut buf)
                .await
                .map_err(Box::new)?;
            serde_json::from_slice::<Value>(&buf)
                .map_err(Box::new)?
                .to_string()
        }
        Err(e) => {
            let (status, message) = handle_get_object_error(e);
            return Ok(Response::builder()
                .status(status)
                .body(message)
                .map_err(Box::new)?);
        }
    };
    info!("construct service account struct");
    let service_account = match CustomServiceAccount::from_json(&service_account_json) {
        Ok(service_account) => service_account,
        Err(e) => {
            error!("got construct service account error:{e:?}");
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("got  construct service account error:{e:?}"))
                .map_err(Box::new)?);
        }
    };
    let auth_manager = AuthenticationManager::from(service_account);
    info!("request token");
    let token = match auth_manager
        .get_token(&["https://www.googleapis.com/auth/cloud-platform"])
        .await
    {
        Ok(token) => token,
        Err(e) => {
            error!("got gcp auth error:{e:?}");
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("got gcp auth error:{e:?}"))
                .map_err(Box::new)?);
        }
    };
    info!("done!");
    let resp = Response::builder()
        .status(200)
        .body(token.as_str().to_owned())
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
