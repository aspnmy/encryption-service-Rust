use axum::Router;
use std::sync::Arc;
use crate::service::EncryptionService;

// 导入处理函数
mod handlers;

/// 创建API路由
pub fn create_router(
    service: Arc<EncryptionService>,
) -> Router {
    // 创建基础路由
    let router = Router::new()
        // 健康检查路由
        .route("/health", axum::routing::get(handlers::health_check))
        // 加密路由
        .route("/encrypt", axum::routing::post(handlers::encrypt))
        // 解密路由
        .route("/decrypt", axum::routing::post(handlers::decrypt))
        // 批量加密路由
        .route("/batch/encrypt", axum::routing::post(handlers::batch_encrypt))
        // 批量解密路由
        .route("/batch/decrypt", axum::routing::post(handlers::batch_decrypt))
        // 应用状态
        .with_state(service);

    router
}
