use std::{net::AddrParseError, num::ParseIntError};

use {
    axum::{
        response::{IntoResponse, Response},
        Json,
    },
    envmnt::errors::EnvmntError,
    http::StatusCode,
    icann_rdap_client::{iana::IanaResponseError, RdapClientError},
    icann_rdap_common::{
        prelude::ToResponse,
        response::{RdapResponseError, Rfc9083Error},
    },
    ipnet::PrefixLenError,
    thiserror::Error,
};

/// Errors from the RDAP Server.
#[derive(Debug, Error)]
pub enum RdapServerError {
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    EnvVar(#[from] std::env::VarError),
    #[error(transparent)]
    IntEnvVar(#[from] ParseIntError),
    #[error["configuration error: {0}"]]
    Config(String),
    #[error(transparent)]
    SqlDb(#[from] sqlx::Error),
    #[error("index data for {0} is missing or empty")]
    EmptyIndexData(String),
    #[error("file at {0} is not JSON")]
    NonJsonFile(String),
    #[error("json file at {0} is valid JSON but is not RDAP")]
    NonRdapJsonFile(String),
    #[error(transparent)]
    AddrParse(#[from] AddrParseError),
    #[error(transparent)]
    PrefixLength(#[from] PrefixLenError),
    #[error(transparent)]
    CidrParse(#[from] ipnet::AddrParseError),
    #[error("RDAP objects do not pass checks.")]
    ErrorOnChecks,
    #[error(transparent)]
    Envmnt(#[from] EnvmntError),
    #[error("Argument parsing error: {0}")]
    ArgParse(String),
    #[error("Invalid argument error: {0}")]
    InvalidArg(String),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Response(#[from] RdapResponseError),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Iana(#[from] IanaResponseError),
    #[error("Bootstrap error: {0}")]
    Bootstrap(String),
    #[error(transparent)]
    RdapClientError(#[from] RdapClientError),
}

impl IntoResponse for RdapServerError {
    fn into_response(self) -> Response {
        let response = Rfc9083Error::response_obj()
            .error_code(500)
            .build()
            .to_response();
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", r#"application/rdap"#)],
            Json(response),
        )
            .into_response()
    }
}
