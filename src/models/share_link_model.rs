use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareLink {
    pub _id: ObjectId,
    pub reciepents_user_id: ObjectId,
    pub file_id: ObjectId,
    pub password: String,
    pub expires_at: DateTime,
    pub created_at: DateTime,
}
