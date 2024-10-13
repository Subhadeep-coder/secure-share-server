use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Validate, Default, Clone, Deserialize)]
pub struct RegisterUserDto {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,

    #[validate(
        length(min = 1, message = "Email is required"),
        email(message = "Email is invalid")
    )]
    pub email: String,

    #[validate(
        length(min = 1, message = "Password is required"),
        length(min = 6, message = "Password must be at least 6 characters")
    )]
    pub password: String,

    #[validate(
        length(min = 1, message = "Confirm Password is required"),
        must_match(other = "password", message = "passwords do not match")
    )]
    #[serde(rename = "passwordConfirm")]
    pub password_confirm: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterUserResponse {
    pub status_code: i32,
    pub message: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}
