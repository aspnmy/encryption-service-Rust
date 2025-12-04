use std::env;
use encryption_service::config::AppConfig;

fn main() {
    println!("=== 测试读写分离配置 ===");
    
    // 设置环境变量
    env::set_var("CRUD_API_BACKEND_TYPE", "read_write_split");
    env::set_var("CRUD_API_WRITE_INSTANCE_URL", "http://10.168.3.165:7981");
    env::set_var("CRUD_API_READ_INSTANCE_URL", "http://10.168.3.168:7982");
    env::set_var("JWT_SECRET", "12345678901234567890");
    
    // 加载配置
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
    
    println!("\n=== 测试完成 ===");
}