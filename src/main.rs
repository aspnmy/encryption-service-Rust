use std::net::SocketAddr;
use std::sync::Arc;

use axum::{serve};
use tracing::info;
use dotenvy::dotenv;

use crate::service::EncryptionService;
use crate::api::create_router;
use crate::config::AppConfig;

mod config;
mod crypto;
mod service;
mod api;
mod scheduler;
mod cache;
mod test_instance;
mod test_config;

#[tokio::main]
async fn main() {
    // 加载环境变量
    dotenv().ok();
    
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 测试配置加载
    test_config::test_config_loading();
    
    // 加载配置
    let config = AppConfig::from_env().expect("无法加载配置");
    config.validate().expect("配置验证失败");
    
    info!("服务配置: {:?}", config);
    
    // 创建服务实例
    let config_arc = Arc::new(config.clone());
    let encryption_service = EncryptionService::new(config_arc.clone());
    let encryption_service = Arc::new(encryption_service);
    
    // 启动调度器健康检查
    encryption_service.get_scheduler().start_health_check().await;
    
    // 启动Test实例管理器定期检查
    encryption_service.get_test_instance_manager().start_periodic_check().await;
    
    // 启动缓存管理器定期清理任务
    encryption_service.get_cache_manager().start_cleanup_task().await;
    
    // 构建路由
    let app = create_router(
        encryption_service
    );
    
    // 配置服务器地址
    let addr = SocketAddr::from((
        config.server.host.parse::<std::net::IpAddr>().expect("无效的服务器地址"),
        config.server.port
    ));
    
    info!("加密服务正在启动，监听地址: {}, 服务ID: {}, 服务角色: {}", 
          addr, 
          config.service.id, 
          config.service.role);
    
    // 启动服务器
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("无法绑定地址");
    
    info!("加密服务正在运行，监听地址: {}", listener.local_addr().unwrap());
    
    serve(listener, app)
        .await
        .expect("服务器启动失败");
}
