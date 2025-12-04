use axum::{extract::State, Json, http::StatusCode};
use std::sync::Arc;
use serde_json;
use crate::service::{EncryptionService, EncryptRequest, EncryptResponse, DecryptRequest, DecryptResponse, GenericResponse};

/// 健康检查处理函数
#[axum::debug_handler]
pub async fn health_check(
    State(service): State<Arc<EncryptionService>>,
) -> (StatusCode, Json<GenericResponse<serde_json::Value>>) {
    // 调用服务健康检查
    match service.health_check().await {
        Ok(_) => {
            let response = GenericResponse {
                success: true,
                message: "服务正常运行".to_string(),
                data: Some(serde_json::json!({ 
                    "service_id": service.get_service_id(), 
                    "service_role": service.get_service_role(),
                    "status": "ok" 
                })),
            };
            (StatusCode::OK, Json(response))
        },
        Err(e) => {
            let response = GenericResponse {
                success: false,
                message: format!("服务健康检查失败: {}", e),
                data: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        },
    }
}

/// 加密处理函数
#[axum::debug_handler]
pub async fn encrypt(
    State(service): State<Arc<EncryptionService>>,
    Json(request): Json<EncryptRequest>,
) -> (StatusCode, Json<GenericResponse<EncryptResponse>>) {
    match service.encrypt(request).await {
        Ok(response) => {
            let response = GenericResponse {
                success: true,
                message: "加密成功".to_string(),
                data: Some(response),
            };
            (StatusCode::OK, Json(response))
        },
        Err(e) => {
            let response = GenericResponse {
                success: false,
                message: format!("加密失败: {}", e),
                data: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        },
    }
}

/// 解密处理函数
#[axum::debug_handler]
pub async fn decrypt(
    State(service): State<Arc<EncryptionService>>,
    Json(request): Json<DecryptRequest>,
) -> (StatusCode, Json<GenericResponse<DecryptResponse>>) {
    match service.decrypt(request).await {
        Ok(response) => {
            let response = GenericResponse {
                success: true,
                message: "解密成功".to_string(),
                data: Some(response),
            };
            (StatusCode::OK, Json(response))
        },
        Err(e) => {
            let response = GenericResponse {
                success: false,
                message: format!("解密失败: {}", e),
                data: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        },
    }
}

/// 批量加密处理函数
#[axum::debug_handler]
pub async fn batch_encrypt(
    State(service): State<Arc<EncryptionService>>,
    Json(requests): Json<Vec<EncryptRequest>>,
) -> (StatusCode, Json<GenericResponse<Vec<EncryptResponse>>>) {
    match service.batch_encrypt(requests).await {
        Ok(responses) => {
            let response = GenericResponse {
                success: true,
                message: "批量加密成功".to_string(),
                data: Some(responses),
            };
            (StatusCode::OK, Json(response))
        },
        Err(e) => {
            let response = GenericResponse {
                success: false,
                message: format!("批量加密失败: {}", e),
                data: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        },
    }
}

/// 批量解密处理函数
#[axum::debug_handler]
pub async fn batch_decrypt(
    State(service): State<Arc<EncryptionService>>,
    Json(requests): Json<Vec<DecryptRequest>>,
) -> (StatusCode, Json<GenericResponse<Vec<DecryptResponse>>>) {
    match service.batch_decrypt(requests).await {
        Ok(responses) => {
            let response = GenericResponse {
                success: true,
                message: "批量解密成功".to_string(),
                data: Some(responses),
            };
            (StatusCode::OK, Json(response))
        },
        Err(e) => {
            let response = GenericResponse {
                success: false,
                message: format!("批量解密失败: {}", e),
                data: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        },
    }
}
