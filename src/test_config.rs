use crate::config::AppConfig;

/// 测试配置加载
pub fn test_config_loading() {
    println!("=== 测试配置加载 ===");
    
    // 测试：默认配置
    println!("\n--- 测试：默认配置 --- ");
    match AppConfig::from_env() {
        Ok(config) => {
            println!("配置加载成功");
            println!("调度策略: {:?}", config.crud_api.strategy);
            println!("实例数量: {}", config.crud_api.instances.len());
            for (index, instance) in config.crud_api.instances.iter().enumerate() {
                println!("实例 {}: ID={}, URL={}, Type={}", 
                         index + 1, 
                         instance.id, 
                         instance.url, 
                         instance.instance_type);
            }
        },
        Err(e) => {
            println!("配置加载失败: {:?}", e);
        },
    }
    
    println!("\n=== 配置加载测试完成 ===");
}