use actix_web::{
    get,
    web::{self, Data, Json, Query},
    Error, HttpMessage, HttpRequest,
};
use mongodb::bson::oid::ObjectId;

use crate::{
    dtos::auth::get_user_dto::{
        self, FilterSearchUserDto, FilterUserDto, SearchUserQuery, SearchUserResponseDto,
        UserResponseDto,
    },
    services::db::Database,
};

// Initialize routes
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_user).service(search_users);
}

#[get("/get-me")]
pub async fn get_user(
    req: HttpRequest,
    db: Data<Database>,
) -> Result<Json<UserResponseDto>, Error> {
    // Extract user_id from request extensions
    let user_id = req.extensions().get::<ObjectId>().cloned();

    // Handle the case where the user_id is not found
    let user_id = match user_id {
        Some(id) => id,
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User ID not found"));
        }
    };

    let user = match db
        .get_user_by_id(mongodb::bson::Bson::ObjectId(user_id))
        .await
    {
        Ok(user) => user,
        Err(e) => {
            return Err(actix_web::error::ErrorUnauthorized(format!(
                "User not found: {}",
                e.to_string()
            )));
        }
    };

    let filtered_user: FilterUserDto = FilterUserDto::filter_user(&user);

    Ok(Json(UserResponseDto {
        status: 200.to_string(),
        data: {
            get_user_dto::UserData {
                user: filtered_user,
            }
        },
    }))
}

#[get("/filter-user")]
pub async fn search_users(
    db: Data<Database>,
    query: Query<SearchUserQuery>,
) -> Result<Json<SearchUserResponseDto>, Error> {
    let query = query.into_inner();
    let users = match db.search_user(query.email_text.clone().to_string()).await {
        Ok(users) => users,
        Err(e) => {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "Failed to fetch users: {}",
                e.to_string()
            )));
        }
    };

    let mut filtered_users: Vec<FilterSearchUserDto> = Vec::new();
    for user in users {
        filtered_users.push(FilterSearchUserDto::filter_user(&user));
    }

    Ok(Json(SearchUserResponseDto {
        status: 200.to_string(),
        users: filtered_users,
    }))
}
