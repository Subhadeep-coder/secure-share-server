use std::{fs, path::PathBuf};

use crate::{
    dtos::file::{
        delete_file::DeleteFileQuery,
        get_files::{FilteredFile, QueryParams},
        retrieve_file::{RetrieveFileDto, RetrieveFileResponse},
        upload_file::{FileUploadDtos, UploadFileResponse},
    },
    services::db::Database,
    utils::{
        file::{decrypt::decrypt_file, encrypt::encrypt_file},
        password,
    },
};
use actix_multipart::Multipart;
use actix_web::{
    delete, get, post,
    web::{self, Data, Json, Query},
    Error, HttpMessage, HttpRequest,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::stream::StreamExt;
use mongodb::bson::{self, oid::ObjectId, Bson};
use rsa::{
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    RsaPrivateKey, RsaPublicKey,
};
use validator::Validate;

// Initialize routes
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_file)
        .service(retrieve_file)
        .service(get_user_files)
        .service(get_recieve_files)
        .service(delete_file);
}

#[post("/upload-file")]
pub async fn upload_file(
    mut payload: Multipart, // Handle multipart payload
    req: HttpRequest,
    db: Data<Database>,
) -> Result<Json<UploadFileResponse>, Error> {
    // Extract user_id from request extensions
    let user_id = req.extensions().get::<ObjectId>().cloned();

    // Handle the case where the user_id is not found
    let user_id = match user_id {
        Some(id) => id,
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User ID not found"));
        }
    };

    let _user = match db
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

    let mut file_data = Vec::new();
    let mut file_name = String::new();
    let mut file_size: i64 = 0;

    let mut form_data = FileUploadDtos {
        recipient_email: String::new(),
        password: String::new(),
        expiration_date: String::new(),
    };

    // Process the file upload
    while let Some(Ok(mut field)) = payload.next().await {
        // Safely handle content_disposition
        let content_disposition = match field.content_disposition() {
            Some(disposition) => disposition,
            None => {
                return Err(actix_web::error::ErrorBadRequest(
                    "Missing content disposition",
                ));
            }
        };

        // Safely handle field name
        let field_name = match field.name() {
            Some(name) => name.to_string(),
            None => {
                return Err(actix_web::error::ErrorBadRequest("Field name not found"));
            }
        };

        match field_name.as_str() {
            // Handle file upload field
            "fileUpload" => {
                file_name = content_disposition
                    .get_filename()
                    .unwrap_or("unknown_file")
                    .to_string();

                // Collect the file bytes into a Vec<u8>
                while let Some(Ok(chunk)) = field.next().await {
                    file_data.extend_from_slice(&chunk);
                }

                // Get the size of the file in bytes
                file_size = file_data.len() as i64;
            }
            // Handle other form fields
            "recipient_email" => {
                if let Some(bytes) = field.next().await {
                    form_data.recipient_email =
                        String::from_utf8(bytes?.to_vec()).unwrap_or_default();
                }
            }
            "password" => {
                if let Some(bytes) = field.next().await {
                    form_data.password = String::from_utf8(bytes?.to_vec()).unwrap_or_default();
                }
            }
            "expiration_date" => {
                if let Some(bytes) = field.next().await {
                    let expiration_value = String::from_utf8(bytes?.to_vec()).unwrap_or_default();
                    form_data.expiration_date = expiration_value;
                } else {
                }
            }
            _ => {}
        }
    }

    form_data.validate().map_err(|e| {
        actix_web::error::ErrorBadRequest(format!("Failed to validate the form: {}", e.to_string()))
    })?;

    let recipient_user = db
        .get_user(form_data.recipient_email.clone())
        .await
        .map_err(|e| {
            actix_web::error::ErrorBadRequest(format!(
                "Failed to get reciepient: {}",
                e.to_string()
            ))
        })?;

    let public_key_str = recipient_user.public_key;

    let public_key_bytes = STANDARD.decode(public_key_str).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!("Failed to get public key: {}", e.to_string()))
    })?;

    let public_key = String::from_utf8(public_key_bytes).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!("Failed to get public key: {}", e.to_string()))
    })?;

    let public_key_pem = RsaPublicKey::from_pkcs1_pem(&public_key).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!("Failed to get public key: {}", e.to_string()))
    })?;

    let (encrypted_aes_key, encrypted_data, iv) = encrypt_file(file_data, &public_key_pem).await?;

    let hash_password = password::hash(&form_data.password).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!("Failed to get public key: {}", e.to_string()))
    })?;

    // Convert chrono DateTime to MongoDB's BSON DateTime
    let mongo_expiration_date = match bson::DateTime::parse_rfc3339_str(&form_data.expiration_date)
    {
        Ok(date) => date,
        Err(e) => {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "Failed to parse date time: {}",
                e.to_string()
            )));
        }
    };

    let result = match db
        .save_file(
            file_name,
            file_size,
            encrypted_data,
            iv,
            encrypted_aes_key,
            recipient_user._id.to_string(),
            user_id,
            hash_password,
            mongo_expiration_date,
        )
        .await
    {
        Ok(res) => res,
        Err(e) => {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "Failed to save encrypted file: {}",
                e.to_string()
            )))
        }
    };

    Ok(Json(UploadFileResponse {
        status: 200,
        message: format!(
            "File uUploaded successully. FileId: {}",
            result.inserted_id.to_string()
        ),
    }))
}

#[post("/retrieve-file")]
pub async fn retrieve_file(
    req: HttpRequest,
    body: Json<RetrieveFileDto>,
    db: Data<Database>,
) -> Result<Json<RetrieveFileResponse>, Error> {
    let _ = body.validate().map_err(|e: validator::ValidationErrors| {
        actix_web::error::ErrorUnauthorized(format!("User ID not found: {}", e.to_string()))
    });
    let body = body.into_inner();

    // Extract user_id from request extensions
    let user_id = req.extensions().get::<ObjectId>().cloned();

    // Handle the case where the user_id is not found
    let user_id = match user_id {
        Some(id) => id,
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User ID not found"));
        }
    };
    // Safely extract the ObjectId from the reciepient_user_id
    let share_id = match ObjectId::parse_str(&body.shared_id) {
        Ok(id) => id,
        Err(e) => {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "Failed to convert to objectid: {}",
                e.to_string()
            )));
        }
    };
    let shared_result = db
        .get_shared(share_id.clone(), user_id)
        .await
        .ok()
        .expect("Failed to fetch shared file doc");

    let matched_password =
        password::compare(&body.password, &shared_result.password).map_err(|e| {
            actix_web::error::ErrorBadRequest(format!(
                "Failed to comapre password: {}",
                e.to_string()
            ))
        })?;

    if !matched_password {
        return Err(actix_web::error::ErrorBadRequest(format!(
            "Password don't match"
        )));
    }

    let file_result = db
        .get_file(Bson::ObjectId(shared_result.file_id.clone()))
        .await
        .ok()
        .expect("Error while fetching file");

    let mut path = PathBuf::from("assets/private_keys");
    path.push(format!("{}.pem", user_id.clone()));

    let private_key = fs::read_to_string(&path).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!(
            "Failed to collect private key: {}",
            e.to_string()
        ))
    })?;

    let private_key_pem = RsaPrivateKey::from_pkcs1_pem(&private_key).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!(
            "Failed to decode rsa private key: {}",
            e.to_string()
        ))
    })?;

    let decrypt_file = decrypt_file(
        file_result.encrypted_aes_key,
        file_result.encrypted_file,
        file_result.iv,
        &private_key_pem,
    )
    .await?;

    // let response = HttpResponse::Ok()
    //     .insert_header((
    //         header::CONTENT_DISPOSITION,
    //         format!("attachment; filename=\"{}\"", file_result.file_name),
    //     )) // Set Content-Disposition header to treat it as a file attachment
    //     .insert_header((header::CONTENT_TYPE, "application/octet-stream")) // Set Content-Type header for binary data
    //     .body(decrypt_file); // Add the file data as the response body

    let response = RetrieveFileResponse { file: decrypt_file };
    Ok(Json(response))
}

#[get("/get-my-files")]
pub async fn get_user_files(
    req: HttpRequest,
    db: Data<Database>,
    query: Query<QueryParams>,
) -> Result<Json<Vec<FilteredFile>>, Error> {
    // Extract user_id from request extensions
    let user_id = req.extensions().get::<ObjectId>().cloned();
    let query = query.into_inner();
    // Handle the case where the user_id is not found
    let user_id = match user_id {
        Some(id) => id,
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User ID not found"));
        }
    };

    let files = db
        .get_sent_files(
            user_id.to_string(),
            query.skip.unwrap_or(1).try_into().unwrap(),
            query.limit.unwrap_or(10),
        )
        .await
        .ok()
        .expect("Failed to fetch files");

    let mut res_files: Vec<FilteredFile> = Vec::new();
    for (file, share_id) in files {
        let user = match db
            .get_recipients_email_by_file_id(file._id.clone().to_string())
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
        res_files.push(FilteredFile::filter_file(&file, user.email, Some(share_id)));
    }

    Ok(Json(res_files))
}

#[get("/get-recieved-files")]
pub async fn get_recieve_files(
    req: HttpRequest,
    db: Data<Database>,
    query: Query<QueryParams>,
) -> Result<Json<Vec<FilteredFile>>, Error> {
    // Extract user_id from request extensions
    let user_id = req.extensions().get::<ObjectId>().cloned();
    let query = query.into_inner();

    // Handle the case where the user_id is not found
    let user_id = match user_id {
        Some(id) => id,
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User ID not found"));
        }
    };

    let files = db
        .get_recieve_files(
            user_id.to_string(),
            query.skip.unwrap_or(1).try_into().unwrap(),
            query.limit.unwrap_or(10),
        )
        .await
        .ok()
        .expect("Failed to fetch files");

    let mut res_files: Vec<FilteredFile> = Vec::new();
    for (file, share_id) in files {
        let user = match db
            .get_user_by_id(mongodb::bson::Bson::ObjectId(file.user_id.clone()))
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
        res_files.push(FilteredFile::filter_file(&file, user.email, Some(share_id)));
    }

    Ok(Json(res_files))
}

#[delete("/delete-file")]
pub async fn delete_file(
    req: HttpRequest,
    db: Data<Database>,
    query: Query<DeleteFileQuery>,
) -> Result<Json<()>, Error> {
    // Extract user_id from request extensions
    let user_id = req.extensions().get::<ObjectId>().cloned();
    let query = query.into_inner();

    // Handle the case where the user_id is not found
    let user_id = match user_id {
        Some(id) => id,
        None => {
            return Err(actix_web::error::ErrorUnauthorized("User ID not found"));
        }
    };

    let res = match db.get_share_link_doc(query.share_id.clone()).await {
        Ok(boolean) => boolean,
        Err(e) => {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "Failed to delete file: {}",
                e.to_string()
            )));
        }
    };

    if res.user_id != user_id {
        return Err(actix_web::error::ErrorBadRequest(format!(
            "You're not authorized to delete this file"
        )));
    }

    db.delete_file_by_share_id(query.share_id.clone()).await?;

    Ok(Json(()))
}
