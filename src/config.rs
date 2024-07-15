use dotenv::dotenv;
use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
pub struct Config {
    pub app_url: String,
    pub service_port: String,
    pub database_url: String,
    pub cors_domains: Vec<String>,
    pub is_development: bool,

    pub aws_region: String,
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,

    pub aws_s3_bucket_name: String,

    pub aws_cloudfront_kvs_arn: String,
}

// TODO: potentially replace this with arctix settings later
impl Config {
    pub fn new() -> Self {
        dotenv().ok();

        Config {
            app_url: Self::get_env("APP_URL", "127.0.0.1:8080"),
            service_port: Self::get_env("SERVICE_PORT", "5775"),
            database_url: Self::get_env("DATABASE_URL", "/data/db.sqlite"),
            cors_domains: Self::get_env_list("CORS_DOMAINS", ""),
            is_development: Self::get_env_bool("IS_DEVELOPMENT", false),

            aws_region: Self::get_env("AWS_REGION", "us-east-1"),
            aws_access_key_id: Self::get_env("AWS_ACCESS_KEY_ID", ""),
            aws_secret_access_key: Self::get_env("AWS_SECRET_ACCESS_KEY", ""),

            aws_s3_bucket_name: Self::get_env("AWS_S3_BUCKET_NAME", ""),
            aws_cloudfront_kvs_arn: Self::get_env("AWS_CLOUDFRONT_KVS_ARN", ""),
        }
    }

    fn get_env(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_string())
    }

    fn get_env_list(key: &str, default: &str) -> Vec<String> {
        env::var(key)
            .unwrap_or_else(|_| default.to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn get_env_usize(key: &str, default: usize) -> usize {
        env::var(key)
            .unwrap_or_else(|_| default.to_string())
            .parse()
            .expect(&format!("Failed to parse {}", key))
    }

    fn get_env_bool(key: &str, default: bool) -> bool {
        env::var(key)
            .unwrap_or_else(|_| default.to_string())
            .parse()
            .expect(&format!("Failed to parse {}", key))
    }
}
