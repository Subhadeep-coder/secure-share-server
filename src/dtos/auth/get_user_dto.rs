use serde::{Deserialize, Serialize};

use crate::models::user_model::User;

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterUserDto {
    pub id: String,
    pub name: String,
    pub email: String,
    pub public_key: String,
}

impl FilterUserDto {
    pub fn filter_user(user: &User) -> Self {
        FilterUserDto {
            id: user._id.to_string(),
            name: user.username.to_owned(),
            email: user.email.to_owned(),
            public_key: user.public_key.to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterSearchUserDto {
    pub id: String,
    pub name: String,
    pub email: String,
}

impl FilterSearchUserDto {
    pub fn filter_user(user: &User) -> Self {
        FilterSearchUserDto {
            id: user._id.to_string(),
            name: user.username.to_owned(),
            email: user.email.to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchUserQuery {
    pub email_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    pub user: FilterUserDto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponseDto {
    pub status: String,
    pub data: UserData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchUserResponseDto {
    pub status: String,
    pub users: Vec<FilterSearchUserDto>,
}
