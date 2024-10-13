use actix_web::{
    post,
    web::{self, Data, Json},
};
use mongodb::bson::oid::ObjectId;
use validator::Validate;

use crate::{
    config::Config,
    dtos::auth::{
        login_user_dto::LoginUserDto,
        refresh_token_dto::RefreshTokenDto,
        register_user_dto::{RegisterUserDto, RegisterUserResponse},
    },
    services::db::Database,
    utils::{
        keys::generate_key,
        password::{compare, hash},
        token::{self, create_token},
    },
};

// Initialize routes
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(register).service(login).service(refresh);
}

#[post("/auth/register")]
pub async fn register(
    body: Json<RegisterUserDto>,
    db: Data<Database>,
    config: Data<Config>,
) -> Json<RegisterUserResponse> {
    let _ = body
        .validate()
        .map_err(|e: validator::ValidationErrors| format!("Validation failed: {}", e));
    let body: RegisterUserDto = body.into_inner();
    match db.get_user(body.email.clone()).await {
        Ok(_) => {
            return Json(RegisterUserResponse {
                status_code: 400,
                access_token: None,
                refresh_token: None,
                message: "User already exists".to_string(),
            })
        }
        Err(e) => {
            print!("{}", e.to_string());
        }
    };

    let hash_password: String = match hash(&body.password) {
        Ok(hash) => hash,
        Err(e) => {
            return Json(RegisterUserResponse {
                status_code: 400,
                access_token: None,
                refresh_token: None,
                message: e.to_string(),
            })
        }
    };

    match db.create_user(body.name, body.email, hash_password).await {
        Ok(user) => {
            let _key_result = match generate_key(db.clone(), user.inserted_id.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    return Json(RegisterUserResponse {
                        status_code: 400,
                        access_token: None,
                        refresh_token: None,
                        message: e.to_string(),
                    })
                }
            };

            let access_token: String = match create_token(
                &user.inserted_id.to_string(),
                &config.jwt_secret.as_bytes(),
                config.access_token_maxage,
            )
            .map_err(|e| format!("Error occured while creating access token: {}", e))
            {
                Ok(token) => token,
                Err(e) => {
                    return Json(RegisterUserResponse {
                        status_code: 400,
                        access_token: None,
                        refresh_token: None,
                        message: e.to_string(),
                    })
                }
            };
            let refresh_token: String = match create_token(
                &user.inserted_id.to_string(),
                &config.jwt_secret.as_bytes(),
                config.refresh_token_maxage,
            )
            .map_err(|e| format!("Error occured while creating refresh token: {}", e))
            {
                Ok(token) => token,
                Err(e) => {
                    return Json(RegisterUserResponse {
                        status_code: 400,
                        access_token: None,
                        refresh_token: None,
                        message: e.to_string(),
                    })
                }
            };
            return Json(RegisterUserResponse {
                status_code: 201,
                message: "Registration successful".to_string(),
                access_token: Some(access_token.to_string()),
                refresh_token: Some(refresh_token.to_string()),
            });
        }
        Err(e) => {
            return Json(RegisterUserResponse {
                status_code: 401,
                message: e.to_string(),
                access_token: None,
                refresh_token: None,
            })
        }
    };
}

#[post("/auth/login")]
pub async fn login(
    body: Json<LoginUserDto>,
    db: Data<Database>,
    config: Data<Config>,
) -> Json<RegisterUserResponse> {
    let _ = body
        .validate()
        .map_err(|e: validator::ValidationErrors| format!("Validation failed: {}", e));
    let body: LoginUserDto = body.into_inner();
    let user = match db.get_user(body.email.clone()).await {
        Ok(user) => user,
        Err(e) => {
            return Json(RegisterUserResponse {
                status_code: 400,
                access_token: None,
                refresh_token: None,
                message: format!("Wrong credentials: {}", e),
            })
        }
    };

    let password_matched = match compare(&body.password, &user.password)
        .map_err(|e| format!("Error comparing the password: {}", e))
    {
        Ok(value) => value,
        Err(e) => {
            return Json(RegisterUserResponse {
                status_code: 401,
                message: format!("Invalid credentials: {}", e),
                access_token: None,
                refresh_token: None,
            })
        }
    };

    if password_matched {
        let access_token = match create_token(
            &user._id.to_string(),
            &config.jwt_secret.as_bytes(),
            config.access_token_maxage,
        )
        .map_err(|e| format!("Error occured while creating access token: {}", e))
        {
            Ok(token) => token,
            Err(e) => {
                return Json(RegisterUserResponse {
                    status_code: 400,
                    access_token: None,
                    refresh_token: None,
                    message: e.to_string(),
                })
            }
        };
        let refresh_token = match create_token(
            &user._id.to_string(),
            &config.jwt_secret.as_bytes(),
            config.refresh_token_maxage,
        )
        .map_err(|e| format!("Error occured while creating refresh token: {}", e))
        {
            Ok(token) => token,
            Err(e) => {
                return Json(RegisterUserResponse {
                    status_code: 400,
                    access_token: None,
                    refresh_token: None,
                    message: e.to_string(),
                })
            }
        };

        return Json(RegisterUserResponse {
            status_code: 201,
            message: "Login successful".to_string(),
            access_token: Some(access_token.to_string()),
            refresh_token: Some(refresh_token.to_string()),
        });
    } else {
        return Json(RegisterUserResponse {
            status_code: 400,
            access_token: None,
            refresh_token: None,
            message: "Wrong credentials".to_string(),
        });
    }
}

#[post("/auth/refresh")]
pub async fn refresh(
    body: Json<RefreshTokenDto>,
    config: Data<Config>,
) -> Json<RegisterUserResponse> {
    let _ = body
        .validate()
        .map_err(|e: validator::ValidationErrors| format!("Validation failed: {}", e));
    let body: RefreshTokenDto = body.into_inner();

    let refresh_token = body.refresh_token;

    // Decode JWT token
    let token_details = match token::decode_token(&refresh_token, config.jwt_secret.as_bytes()) {
        Ok(details) => details,
        Err(e) => {
            return Json(RegisterUserResponse {
                status_code: 400,
                access_token: None,
                refresh_token: None,
                message: format!("Error while decoding the token: {}", e),
            });
        }
    };

    // Convert token 'sub' to ObjectId
    let user_id = match ObjectId::parse_str(&token_details.to_string()) {
        Ok(id) => id,
        Err(e) => {
            return Json(RegisterUserResponse {
                status_code: 400,
                access_token: None,
                refresh_token: None,
                message: format!(
                    "Error while converting userId to objectId: {}",
                    e.to_string()
                ),
            });
        }
    };

    let access_token = match create_token(
        &user_id.to_string(),
        &config.jwt_secret.as_bytes(),
        config.access_token_maxage,
    )
    .map_err(|e| format!("Error occured while creating access token: {}", e))
    {
        Ok(token) => token,
        Err(e) => {
            return Json(RegisterUserResponse {
                status_code: 400,
                access_token: None,
                refresh_token: None,
                message: e.to_string(),
            })
        }
    };
    let refresh_token = match create_token(
        &user_id.to_string(),
        &config.jwt_secret.as_bytes(),
        config.refresh_token_maxage,
    )
    .map_err(|e| format!("Error occured while creating refresh token: {}", e))
    {
        Ok(token) => token,
        Err(e) => {
            return Json(RegisterUserResponse {
                status_code: 400,
                access_token: None,
                refresh_token: None,
                message: e.to_string(),
            })
        }
    };

    return Json(RegisterUserResponse {
        status_code: 201,
        message: "Token refreshed successfully".to_string(),
        access_token: Some(access_token.to_string()),
        refresh_token: Some(refresh_token.to_string()),
    });
}
