#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub access_token_maxage: i64,
    pub refresh_token_maxage: i64,
    pub port: u16,
}

impl Config {
    pub fn init() -> Config {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET_KEY must be set");
        let access_token_maxage = std::env::var("ACCESS_TOKEN_MAXAGE").expect("JWT_MAXAGE must be set");
        let refresh_token_maxage = std::env::var("RFRESH_TOKEN_MAXAGE").expect("JWT_MAXAGE must be set");

        Config {
            database_url,
            jwt_secret,
            access_token_maxage: access_token_maxage.parse::<i64>().unwrap(),
            refresh_token_maxage: refresh_token_maxage.parse::<i64>().unwrap(),
            port: 8080,
        }
    }
}
