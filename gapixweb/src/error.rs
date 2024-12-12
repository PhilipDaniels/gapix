use axum::{http::StatusCode, response::IntoResponse};

pub struct ApiError {
    code: StatusCode,
    message: Option<String>,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        if let Some(msg) = self.message {
            (self.code, msg).into_response()
        } else {
            self.code.into_response()
        }
        
    }
}

impl From<StatusCode> for ApiError {
    fn from(code: StatusCode) -> Self {
        Self {
            code,
            message: None
        }
    }
}

impl ApiError {
    /// Sets the message on the error.
    pub fn message<S: Into<String>>(mut self, msg: S) -> Self {
        self.message = Some(msg.into());
        self
    }

    pub fn not_found() -> Self {
        Self {
            code: StatusCode::NOT_FOUND,
            message: None
        }
    }

    pub fn server_error() -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: None
        }
    }
}
