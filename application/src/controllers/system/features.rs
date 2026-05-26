use crate::{
    api_spec::build_openapi,
    feature_status::{
        FeatureCatalogResponse, apply_openapi_extensions, catalog_response,
        collect_openapi_endpoint_statuses,
    },
    routes::GetState,
    utils::response::{ApiResponse, ApiResponseResult},
};

#[utoipa::path(
    get,
    path = "/",
    operation_id = "get_system_features",
    security(("bearer_auth" = [])),
    responses((status = OK, body = inline(FeatureCatalogResponse))),
)]
pub async fn get(state: GetState) -> ApiResponseResult {
    let app_name = state.config.load().app_name.clone();
    let mut openapi = build_openapi(&app_name);
    apply_openapi_extensions(&mut openapi);

    let mut response = catalog_response();
    response.endpoints = collect_openapi_endpoint_statuses(&openapi);

    ApiResponse::new_serialized(response).ok()
}
