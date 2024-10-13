use std::{
    fs::{self, File},
    io::Write,
};

use actix_web::web::Data;
use base64::{engine::general_purpose::STANDARD, Engine};
use mongodb::bson::Bson;
use rand::rngs::OsRng;
use rsa::{
    pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey},
    RsaPrivateKey, RsaPublicKey,
};

use crate::services::db::Database;

pub async fn generate_key(db: Data<Database>, user_id: Bson) -> Result<String, String> {
    let mut rng = OsRng;

    let private_key = RsaPrivateKey::new(&mut rng, 2048)
        .map_err(|e| format!("Error occured while creating private key: {}", e))?;

    let public_key = RsaPublicKey::from(&private_key);

    let private_key_pem = private_key
        .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
        .map_err(|e| format!("Error while private key pem: {}", e))?;

    let public_key_pem = public_key
        .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
        .map_err(|e| format!("Error while public key pem: {}", e))?;

    let public_key_b64 = STANDARD.encode(public_key_pem.as_bytes());

    db.update_public_key(user_id.clone(), public_key_b64.clone())
        .await
        .map_err(|e| format!("Error while updating user: {}", e))?;

    let private_keys_dir = "assets/private_keys";
    fs::create_dir_all(&private_keys_dir)
        .map_err(|e| format!("Error while saving private key: {}", e))?;

    let user_id = if let Bson::ObjectId(id) = user_id {
        id.to_hex() // This will give you just the hex string
    } else {
        return Err("Invalid user_id format".to_string());
    };

    let pem_file_path = format!("{}/{}.pem", private_keys_dir, user_id.to_string());

    let mut file = File::create(&pem_file_path)
        .map_err(|e| format!("Error while saving private key: {}", e))?;

    file.write_all(private_key_pem.as_bytes())
        .map_err(|e| format!("Error while saving private key: {}", e))?;

    Ok("true".to_string())
}
