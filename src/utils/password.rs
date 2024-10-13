use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

const MAX_PASSWORD_LENGTH: usize = 64;

pub fn hash(password: impl Into<String>) -> Result<String, String> {
    let password: String = password.into();

    if password.is_empty() {
        return Err("Empty Password".to_string());
    }

    if password.len() > MAX_PASSWORD_LENGTH {
        return Err("Password is too long".to_string());
    }

    let salt: SaltString = SaltString::generate(&mut OsRng);
    let hashed_password: String = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e: argon2::password_hash::Error| format!("Password hashing failed: {}", e))?
        .to_string();

    Ok(hashed_password)
}

pub fn compare(password: &str, hashed_password: &str) -> Result<bool, String> {
    if password.is_empty() {
        return Err("Empty Password".to_string());
    }

    if password.len() > MAX_PASSWORD_LENGTH {
        return Err("Password is too long".to_string());
    }

    let password_hash: PasswordHash<'_> = PasswordHash::new(hashed_password)
        .map_err(|e| format!("Invalid password hash format: {}", e))?;

    // Verify the password using Argon2
    match Argon2::default().verify_password(password.as_bytes(), &password_hash) {
        Ok(_) => Ok(true), // Password matches
        Err(_) => {
            Ok(false) // Password does not match
        }
    }
}
