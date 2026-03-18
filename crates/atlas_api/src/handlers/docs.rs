//! GET /docs         — Swagger UI
//! GET /openapi.json — OpenAPI 3.1 spec

use axum::response::{Html, IntoResponse, Response};
use axum::http::{header, StatusCode};

/// Serve the full OpenAPI 3.1 spec from the embedded JSON file.
pub async fn openapi_spec() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        include_str!("openapi.json"),
    ).into_response()
}

/// Serve Swagger UI (loads Swagger CDN, points to /openapi.json).
pub async fn swagger_ui() -> Html<String> {
    Html(format!(r#"<!DOCTYPE html>
<html>
<head>
  <title>Atlas API — Documentation</title>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
<div id="swagger-ui"></div>
<script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
<script>
  SwaggerUIBundle({{
    url: "/openapi.json",
    dom_id: '#swagger-ui',
    presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
    layout: "StandaloneLayout",
    deepLinking: true,
    tryItOutEnabled: true,
    requestInterceptor: function(req) {{
      req.headers['X-Api-Key'] = localStorage.getItem('atlas_api_key') || '';
      return req;
    }},
  }})
</script>
<style>
  body {{ margin: 0; }}
  .topbar {{ background: #1a1a2e; }}
</style>
</body>
</html>"#))
}
