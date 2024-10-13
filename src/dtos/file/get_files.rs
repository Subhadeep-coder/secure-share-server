use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

use crate::models::file_model::File;

#[derive(Debug, Serialize, Deserialize)]
pub struct FilteredFile {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub shared_at: DateTime,
    pub recipients_email: String,
    pub share_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QueryParams {
    pub skip: Option<i32>,
    pub limit: Option<usize>,
}

impl FilteredFile {
    pub fn filter_file(file: &File, recipients_email: String, share_id: Option<String>) -> Self {
        FilteredFile {
            id: file._id.to_string(),
            name: file.file_name.to_owned(),
            size: file.file_size.to_owned(),
            shared_at: file.created_at.to_owned(),
            recipients_email,
            share_id,
        }
    }
}
