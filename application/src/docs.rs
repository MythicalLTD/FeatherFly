use axum::response::{Html, IntoResponse};

const SWAGGER_UI_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>FeatherFly API Docs</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui-bundle.js"></script>
    <script>
        window.onload = () => {
            SwaggerUIBundle({
                url: "/openapi.json",
                dom_id: "#swagger-ui",
                deepLinking: true,
                persistAuthorization: true,
                presets: [SwaggerUIBundle.presets.apis],
                layout: "BaseLayout",
            });
        };
    </script>
</body>
</html>
"##;

pub async fn swagger_ui() -> impl IntoResponse {
    Html(SWAGGER_UI_HTML)
}
