use axum::http::HeaderMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Toon,
}

/// Determine output format from Accept header or `format` query param.
/// The `format` query param takes precedence over the Accept header.
pub fn negotiate(headers: &HeaderMap, format_param: Option<&str>) -> OutputFormat {
    if let Some(f) = format_param {
        if f.eq_ignore_ascii_case("toon") { return OutputFormat::Toon; }
        // Any explicit format param that isn't "toon" means JSON — stops Accept from overriding
        return OutputFormat::Json;
    }
    if let Some(accept) = headers.get("accept").and_then(|v| v.to_str().ok()) {
        if accept.contains("text/toon") { return OutputFormat::Toon; }
    }
    OutputFormat::Json
}

/// Build the appropriate response body.
pub fn respond(format: OutputFormat, json_val: impl serde::Serialize, toon_str: String)
    -> axum::response::Response
{
    use axum::response::IntoResponse;
    match format {
        OutputFormat::Json => axum::Json(json_val).into_response(),
        OutputFormat::Toon => (
            [(axum::http::header::CONTENT_TYPE, atlas_toon::CONTENT_TYPE)],
            toon_str,
        ).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    fn headers_with_accept(accept: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("accept", accept.parse().unwrap());
        h
    }

    #[test]
    fn query_param_toon_lowercase() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(&h, Some("toon")), OutputFormat::Toon);
    }

    #[test]
    fn query_param_toon_uppercase() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(&h, Some("TOON")), OutputFormat::Toon);
    }

    #[test]
    fn query_param_toon_mixed_case() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(&h, Some("Toon")), OutputFormat::Toon);
    }

    #[test]
    fn accept_header_text_toon() {
        let h = headers_with_accept("text/toon");
        assert_eq!(negotiate(&h, None), OutputFormat::Toon);
    }

    #[test]
    fn default_is_json() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(&h, None), OutputFormat::Json);
    }

    #[test]
    fn query_param_wins_over_accept() {
        let h = headers_with_accept("text/toon");
        assert_eq!(negotiate(&h, Some("json")), OutputFormat::Json);
    }
}
