use axum::{extract::Multipart, extract::Query, http::StatusCode};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    files::{FileService, is_disk_quota_error},
    remote::jwt::BasePayload,
    routes::GetState,
    site_websocket::has_permission,
    utils::response::{ApiResponse, ApiResponseResult},
};

#[derive(ToSchema, Deserialize)]
pub struct UploadQuery {
    token: String,
    #[serde(default)]
    directory: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadJwtPayload {
    #[serde(flatten)]
    pub base: BasePayload,
    pub site_id: String,
    pub unique_id: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(serde::Serialize, ToSchema)]
struct UploadResponse {
    path: String,
}

#[utoipa::path(
    post,
    path = "/file",
    operation_id = "post_upload_file",
    params(
        ("token" = String, Query, description = "Panel-signed JWT"),
        ("directory" = String, Query, description = "Target directory under site root"),
    ),
    request_body(content = String, description = "Multipart file field named `file`"),
    responses(
        (status = OK, body = inline(UploadResponse)),
        (status = UNAUTHORIZED, body = inline(crate::utils::response::ApiError)),
        (status = BAD_REQUEST, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn file(
    state: GetState,
    Query(query): Query<UploadQuery>,
    mut multipart: Multipart,
) -> ApiResponseResult {
    let payload: UploadJwtPayload = match state.jwt.verify(&query.token) {
        Ok(payload) => payload,
        Err(_) => {
            return ApiResponse::error("invalid token")
                .with_status(StatusCode::UNAUTHORIZED)
                .ok();
        }
    };

    if let Err(err) = payload.base.validate(&state.jwt).await {
        return ApiResponse::error(&format!("invalid token: {err}"))
            .with_status(StatusCode::UNAUTHORIZED)
            .ok();
    }

    if !state.jwt.limited_jwt_id(&payload.unique_id).await {
        return ApiResponse::error("token has already been used")
            .with_status(StatusCode::UNAUTHORIZED)
            .ok();
    }

    if !has_permission(&payload.permissions, "file.write") {
        return ApiResponse::error("missing file.write permission")
            .with_status(StatusCode::FORBIDDEN)
            .ok();
    }

    let mut file_name = None;
    let mut file_bytes = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            file_name = field.file_name().map(str::to_string);
            file_bytes = field.bytes().await.ok();
            break;
        }
    }

    let Some(name) = file_name else {
        return ApiResponse::error("missing multipart file field")
            .with_status(StatusCode::BAD_REQUEST)
            .ok();
    };
    let Some(bytes) = file_bytes else {
        return ApiResponse::error("failed to read uploaded file")
            .with_status(StatusCode::BAD_REQUEST)
            .ok();
    };

    let directory = query.directory.trim().trim_matches('/');
    let path = if directory.is_empty() {
        name.clone()
    } else {
        format!("{directory}/{name}")
    };

    match FileService::upload(&state, &payload.site_id, &path, &bytes).await {
        Ok(()) => ApiResponse::new_serialized(UploadResponse { path }).ok(),
        Err(err) if is_disk_quota_error(&err) => ApiResponse::error(&format!("{err:#}"))
            .with_status(StatusCode::PAYLOAD_TOO_LARGE)
            .ok(),
        Err(err) => ApiResponse::error(&format!("upload failed: {err:#}"))
            .with_status(StatusCode::BAD_GATEWAY)
            .ok(),
    }
}
