use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    env,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_path: PathBuf,
    pub admin_username: String,
    pub admin_password: String,
    pub jwt_secret: String,
    pub encryption_key: [u8; 32],
    pub cookie_secure: bool,
    pub allowed_origin: Option<String>,
    pub trusted_proxy_ips: Vec<IpAddr>,
    pub gateway_enabled: bool,
    pub gateway_api_key: Option<String>,
    pub gateway_default_account: Option<String>,
    pub gateway_auto_switch: bool,
    pub user_registration_enabled: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let bind_addr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3001".to_string())
            .parse::<SocketAddr>()
            .map_err(|error| format!("BIND_ADDR is invalid: {error}"))?;
        let database_path = PathBuf::from(
            env::var("DATABASE_PATH").unwrap_or_else(|_| "/data/kiro.db".to_string()),
        );
        let admin_username = required("ADMIN_USERNAME")?;
        let admin_password = required("ADMIN_PASSWORD")?;
        let jwt_secret = required("JWT_SECRET")?;
        let raw_key = required("DATA_ENCRYPTION_KEY")?;

        if admin_password.len() < 12 {
            return Err("ADMIN_PASSWORD must contain at least 12 characters".to_string());
        }
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must contain at least 32 characters".to_string());
        }

        let encryption_key = parse_encryption_key(&raw_key)?;
        let allowed_origin = env::var("ALLOWED_ORIGIN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        if let Some(origin) = allowed_origin.as_deref() {
            validate_origin(origin)?;
        }
        let gateway_enabled = parse_bool("GATEWAY_ENABLED", false)?;
        let gateway_api_key = env::var("GATEWAY_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());
        if gateway_enabled
            && gateway_api_key
                .as_ref()
                .is_none_or(|key| key.trim().len() < 32)
        {
            return Err(
                "GATEWAY_API_KEY is required and must contain at least 32 characters when the gateway is enabled"
                    .to_string(),
            );
        }

        Ok(Self {
            bind_addr,
            database_path,
            admin_username,
            admin_password,
            jwt_secret,
            encryption_key,
            cookie_secure: parse_bool("COOKIE_SECURE", true)?,
            allowed_origin,
            trusted_proxy_ips: parse_trusted_proxy_ips()?,
            gateway_enabled,
            gateway_api_key,
            gateway_default_account: env::var("GATEWAY_DEFAULT_ACCOUNT")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            gateway_auto_switch: parse_bool("GATEWAY_AUTO_SWITCH", true)?,
            user_registration_enabled: parse_bool("USER_REGISTRATION_ENABLED", true)?,
        })
    }
}

fn validate_origin(origin: &str) -> Result<(), String> {
    let url = url::Url::parse(origin)
        .map_err(|_| "ALLOWED_ORIGIN must be an absolute HTTP(S) origin".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "ALLOWED_ORIGIN must contain only an HTTP(S) scheme, host, and optional port"
                .to_string(),
        );
    }
    Ok(())
}

fn parse_trusted_proxy_ips() -> Result<Vec<IpAddr>, String> {
    env::var("TRUSTED_PROXY_IPS")
        .unwrap_or_else(|_| "127.0.0.1,::1".to_string())
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<IpAddr>()
                .map_err(|_| format!("TRUSTED_PROXY_IPS contains invalid IP address: {value}"))
        })
        .collect()
}

fn required(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("required environment variable {name} is missing"))
}

fn parse_bool(name: &str, default: bool) -> Result<bool, String> {
    match env::var(name) {
        Ok(value) => {
            bool::from_str(&value).map_err(|_| format!("{name} must be either true or false"))
        }
        Err(_) => Ok(default),
    }
}

fn parse_encryption_key(value: &str) -> Result<[u8; 32], String> {
    let bytes = STANDARD
        .decode(value.trim())
        .map_err(|_| "DATA_ENCRYPTION_KEY must be base64 encoded".to_string())?;
    bytes
        .try_into()
        .map_err(|_| "DATA_ENCRYPTION_KEY must decode to exactly 32 bytes".to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_encryption_key, validate_origin};
    use base64::{engine::general_purpose::STANDARD, Engine};

    #[test]
    fn accepts_32_byte_encryption_key() {
        let key = STANDARD.encode([7_u8; 32]);
        assert_eq!(parse_encryption_key(&key).unwrap(), [7_u8; 32]);
    }

    #[test]
    fn rejects_short_encryption_key() {
        let key = STANDARD.encode([7_u8; 16]);
        assert!(parse_encryption_key(&key).is_err());
    }

    #[test]
    fn validates_exact_cors_origin() {
        assert!(validate_origin("https://kiro.nvdx.de").is_ok());
        assert!(validate_origin("*").is_err());
        assert!(validate_origin("https://kiro.nvdx.de/path").is_err());
    }
}
