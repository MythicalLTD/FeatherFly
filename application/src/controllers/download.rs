use serde::Deserialize;

use crate::{
    files::FileService,
    remote::jwt::BasePayload,
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};
use axum::{extract::Query, http::StatusCode};
use utoipa::ToSchema;

#[derive(ToSchema, Deserialize)]
pub struct DownloadQuery {
    token: String,
}

#[derive(Debug, Deserialize)]
pub struct FileJwtPayload {
    #[serde(flatten)]
    pub base: BasePayload,
    pub site_id: String,
    pub file_path: String,
    pub unique_id: String,
}

#[utoipa::path(
    get,
    path = "/file",
    operation_id = "get_download_file",
    params(("token" = String, Query, description = "Panel-signed JWT")),
    responses(
        (status = OK, description = "File bytes"),
        (status = UNAUTHORIZED, body = inline(crate::utils::response::ApiError)),
        (status = NOT_FOUND, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn file(state: GetState, Query(query): Query<DownloadQuery>) -> ApiResponseResult {
    let payload: FileJwtPayload = match state.jwt.verify(&query.token) {
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

    match FileService::download(&state, &payload.site_id, &payload.file_path).await {
        Ok(bytes) => {
            let filename = payload.file_path.rsplit('/').next().unwrap_or("download");
            ApiResponse::new(axum::body::Body::from(bytes))
                .with_header("content-type", "application/octet-stream")
                .with_header(
                    "content-disposition",
                    format!("attachment; filename=\"{filename}\""),
                )
                .ok()
        }
        Err(err) => ApiResponse::error(&format!("file not found: {err:#}"))
            .with_status(StatusCode::NOT_FOUND)
            .ok(),
    }
}

#[derive(Debug, Deserialize)]
pub struct BackupJwtPayload {
    #[serde(flatten)]
    pub base: BasePayload,
    pub site_id: String,
    pub backup_id: String,
    pub unique_id: String,
}

#[utoipa::path(
    get,
    path = "/backup",
    operation_id = "get_download_backup",
    params(("token" = String, Query, description = "Panel-signed JWT")),
    responses(
        (status = OK, description = "Backup archive bytes"),
        (status = UNAUTHORIZED, body = inline(crate::utils::response::ApiError)),
        (status = NOT_FOUND, body = inline(crate::utils::response::ApiError)),
    ),
)]
pub async fn backup(state: GetState, Query(query): Query<DownloadQuery>) -> ApiResponseResult {
    let payload: BackupJwtPayload = match state.jwt.verify(&query.token) {
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

    let Some(cache) = &state.cache else {
        return ApiResponse::error("cache unavailable")
            .with_status(StatusCode::SERVICE_UNAVAILABLE)
            .ok();
    };

    let backups = match cache.list_backups(&payload.site_id).await {
        Ok(backups) => backups,
        Err(err) => {
            return ApiResponse::error(&format!("failed to load backups: {err:#}"))
                .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                .ok();
        }
    };

    let Some(record) = backups.into_iter().find(|b| b.id == payload.backup_id) else {
        return ApiResponse::error("backup not found")
            .with_status(StatusCode::NOT_FOUND)
            .ok();
    };

    let bytes = match tokio::fs::read(&record.path).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return ApiResponse::error(&format!("backup not found: {err}"))
                .with_status(StatusCode::NOT_FOUND)
                .ok();
        }
    };

    ApiResponse::new(axum::body::Body::from(bytes))
        .with_header("content-type", "application/gzip")
        .with_header(
            "content-disposition",
            format!("attachment; filename=\"{}.tar.gz\"", payload.backup_id),
        )
        .ok()
}
