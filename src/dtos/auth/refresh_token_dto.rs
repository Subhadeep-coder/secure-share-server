use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Validate, Default, Clone, Deserialize,Serialize)]
pub struct RefreshTokenDto {
    #[validate(length(min = 1, message = "Access Token is required"))]
    pub refresh_token: String,
}