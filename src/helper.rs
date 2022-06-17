use aws_sdk_s3::{error::GetObjectError, types::SdkError};
use lambda_http::http::StatusCode;
use tracing::error;

pub fn handle_get_object_error(error: SdkError<GetObjectError>) -> (StatusCode, String) {
    match error {
        SdkError::ServiceError { err, .. } => {
            if err.is_no_such_key() {
                error!("no such s3_key");
                (StatusCode::NOT_FOUND, "no suck s3_key".into())
            } else {
                error!("got get object error:{err:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("got get object error:{err:?}"),
                )
            }
        }
        e => {
            error!("got sdk error:{:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("got sdk error:{e:?}"),
            )
        }
    }
}
