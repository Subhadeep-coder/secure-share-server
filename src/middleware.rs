use actix_web::{dev::ServiceRequest, http::header, Error, HttpMessage};
use actix_web_httpauth::extractors::{bearer::BearerAuth, AuthenticationError};
use mongodb::bson::oid::ObjectId;

use crate::{config::Config, utils::token};

pub async fn validator(
    req: ServiceRequest,
    _credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {

    // Use the bearer config directly instead of your Config
    let bearer_config = actix_web_httpauth::extractors::bearer::Config::default();
    
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_value| {
            if auth_value.starts_with("Bearer ") {
                Some(auth_value[7..].to_owned())
            } else {
                None
            }
        });

    // Return error if token is missing
    let token = match token {
        Some(t) => t,
        None => {
            return Err((AuthenticationError::from(bearer_config).into(), req));
        }
    };

    let config = Config::init();

    // Decode JWT token
    let token_details = match token::decode_token(&token, config.jwt_secret.as_bytes()) {
        Ok(details) => details,
        Err(_) => {
            return Err((AuthenticationError::from(bearer_config).into(), req));
        }
    };

    // Convert token 'sub' to ObjectId
    let user_id = match ObjectId::parse_str(&token_details.to_string()) {
        Ok(id) => id,
        Err(_) => {
            return Err((AuthenticationError::from(bearer_config).into(), req));
        }
    };

    // Attach user to request extensions
    req.extensions_mut().insert(user_id);
    Ok(req)
}
