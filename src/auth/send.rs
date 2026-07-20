use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use chrono::{Duration, Utc};
use jwt_compact::AlgorithmExt;
use jwt_compact::{alg::Hs256Key, Claims as JwtClaims, Header, TimeOptions, UntrustedToken};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use worker::Env;

use crate::auth::bearer_token_from_header_value;
use crate::error::AppError;

pub const SEND_ACCESS_TOKEN_ISSUER: &str = "warden-worker|send-access";
pub const SEND_ACCESS_TOKEN_TTL_SECS: i64 = 120;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendAccessClaims {
    pub sub: String,
    pub iss: String,
}

fn send_access_time_options() -> TimeOptions {
    TimeOptions::from_leeway(Duration::zero())
}

fn invalid_send_access_token() -> AppError {
    AppError::Unauthorized("Invalid token".to_string())
}

pub(crate) fn create_send_access_token(env: &Env, send_id: &str) -> Result<String, AppError> {
    if Uuid::parse_str(send_id).is_err() {
        return Err(AppError::Internal);
    }

    let now = Utc::now();
    let time_options = send_access_time_options();
    let claims = JwtClaims::new(SendAccessClaims {
        sub: send_id.to_string(),
        iss: SEND_ACCESS_TOKEN_ISSUER.to_string(),
    })
    .set_duration_and_issuance(&time_options, Duration::seconds(SEND_ACCESS_TOKEN_TTL_SECS))
    .set_not_before(now);

    let secret = env.secret("JWT_SECRET")?.to_string();
    let key = Hs256Key::new(secret.as_bytes());
    jwt_compact::alg::Hs256
        .token(&Header::empty(), &claims, &key)
        .map_err(|_| AppError::Crypto("Failed to create Send access token".to_string()))
}

pub(crate) async fn decode_send_access_token(
    env: &Env,
    raw_token: &str,
) -> Result<SendAccessClaims, AppError> {
    let secret = env
        .secret("JWT_SECRET")
        .map_err(|_| invalid_send_access_token())?
        .to_string();
    let key = Hs256Key::new(secret.as_bytes());
    let token = UntrustedToken::new(raw_token).map_err(|_| invalid_send_access_token())?;
    let token = jwt_compact::alg::Hs256
        .validator::<SendAccessClaims>(&key)
        .validate(&token)
        .map_err(|_| invalid_send_access_token())?;
    let time_options = send_access_time_options();
    token
        .claims()
        .validate_expiration(&time_options)
        .map_err(|_| invalid_send_access_token())?;
    token
        .claims()
        .validate_maturity(&time_options)
        .map_err(|_| invalid_send_access_token())?;

    let claims = token.into_parts().1.custom;
    if claims.iss != SEND_ACCESS_TOKEN_ISSUER || Uuid::parse_str(&claims.sub).is_err() {
        return Err(invalid_send_access_token());
    }

    Ok(claims)
}

impl FromRequestParts<Arc<Env>> for SendAccessClaims {
    type Rejection = AppError;

    #[worker::send]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<Env>,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|auth_header| auth_header.to_str().ok())
            .and_then(bearer_token_from_header_value)
            .ok_or_else(invalid_send_access_token)?;

        decode_send_access_token(state.as_ref(), &token).await
    }
}
