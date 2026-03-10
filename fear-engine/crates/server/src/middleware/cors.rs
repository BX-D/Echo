//! CORS configuration.

use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Method};
use fear_engine_common::config::AppConfig;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Builds a [`CorsLayer`] from the application configuration.
///
/// # Example
///
/// ```
/// use fear_engine_common::config::AppConfig;
/// use fear_engine_server::middleware::cors::build_cors;
///
/// let config = AppConfig::default();
/// let cors = build_cors(&config);
/// ```
pub fn build_cors(config: &AppConfig) -> CorsLayer {
    let origins = allowed_origins(config);

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE])
}

fn allowed_origins(config: &AppConfig) -> Vec<HeaderValue> {
    let fallback = "http://localhost:5173";
    let frontend_url = if config.frontend_url.is_empty() {
        fallback
    } else {
        &config.frontend_url
    };

    let mut urls = vec![frontend_url.to_string()];

    if let Ok(parsed) = url::Url::parse(frontend_url) {
        if let Some(host) = parsed.host_str() {
            let alternate_host = match host {
                "localhost" => Some("127.0.0.1"),
                "127.0.0.1" => Some("localhost"),
                _ => None,
            };

            if let Some(alternate_host) = alternate_host {
                let mut alt = parsed.clone();
                let _ = alt.set_host(Some(alternate_host));
                urls.push(alt.to_string().trim_end_matches('/').to_string());
            }
        }
    }

    urls.sort();
    urls.dedup();

    urls.into_iter()
        .filter_map(|url| url.parse::<HeaderValue>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_cors_with_default_config() {
        let config = AppConfig::default();
        let _cors = build_cors(&config);
    }

    #[test]
    fn test_build_cors_with_custom_url() {
        let mut config = AppConfig::default();
        config.frontend_url = "http://example.com:8080".into();
        let _cors = build_cors(&config);
    }

    #[test]
    fn test_allowed_origins_include_localhost_and_loopback_variants() {
        let mut config = AppConfig::default();
        config.frontend_url = "http://localhost:5173".into();
        let origins = allowed_origins(&config);
        let values: Vec<String> = origins
            .into_iter()
            .map(|value| value.to_str().unwrap().to_string())
            .collect();

        assert!(values.contains(&"http://localhost:5173".to_string()));
        assert!(values.contains(&"http://127.0.0.1:5173".to_string()));
    }
}
