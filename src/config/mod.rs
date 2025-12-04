use std::env;
use serde::Deserialize;
use tracing::info;
use anyhow::Result;

/// 调度策略枚举
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum SchedulerStrategy {
    /// 单容器模式
    #[serde(rename = "single")]
    Single,
    /// 读写分离模式
    #[serde(rename = "read_write_split")]
    ReadWriteSplit,
    /// 负载均衡模式
    #[serde(rename = "load_balance")]
    LoadBalance,
}

/// CRUD API实例配置
#[derive(Debug, Deserialize, Clone)]
pub struct CrudApiInstance {
    /// 实例ID
    pub id: String,
    /// 实例URL
    pub url: String,
    /// 实例类型：read, write, mixed
    pub instance_type: String,
    /// 连接超时时间（毫秒）
    #[allow(dead_code)]
    pub timeout: u64,
    /// 重试次数
    #[allow(dead_code)]
    pub retries: u32,
}

/// 应用配置结构体
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    /// 服务器配置
    pub server: ServerConfig,
    /// JWT配置
    pub jwt: JwtConfig,
    /// 加密配置
    pub encryption: EncryptionConfig,
    /// 服务角色配置
    pub service: ServiceRoleConfig,
    /// CRUD API服务配置
    pub crud_api: CrudApiConfig,
}

/// 服务器配置
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// 服务器地址
    pub host: String,
    /// 服务器端口
    pub port: u16,
    /// 是否启用HTTPS
    #[allow(dead_code)]
    pub https: bool,
}

/// JWT配置
#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    /// JWT密钥
    pub secret: String,
    /// JWT过期时间（秒）
    #[allow(dead_code)]
    pub expires_in: i64,
    /// JWT刷新时间（秒）
    #[allow(dead_code)]
    pub refresh_in: i64,
}

/// 加密配置
#[derive(Debug, Deserialize, Clone)]
pub struct EncryptionConfig {
    /// 加密算法
    pub algorithm: String,
    /// 密钥长度
    pub key_length: u32,
    /// 迭代次数
    pub iterations: u32,
    /// 盐值
    pub salt: String,
}

/// 服务角色配置
#[derive(Debug, Deserialize, Clone)]
pub struct ServiceRoleConfig {
    /// 服务角色：encrypt, decrypt, mixed
    pub role: String,
    /// 服务ID
    pub id: String,
}

/// CRUD API服务配置
#[derive(Debug, Deserialize, Clone)]
pub struct CrudApiConfig {
    /// CRUD API实例列表
    pub instances: Vec<CrudApiInstance>,
    /// 调度策略
    pub strategy: SchedulerStrategy,
    /// 健康检查间隔（秒）
    pub health_check_interval: u64,
    /// 连接超时时间（毫秒）
    pub timeout: u64,
    /// 重试次数
    #[allow(dead_code)]
    pub retries: u32,
}

impl AppConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self> {
        info!("从环境变量加载配置");
        
        // 获取后端类型
        let backend_type = env::var("CRUD_API_BACKEND_TYPE").unwrap_or("read_write_split".to_string());
        
        // 读写分离配置参数
        // 必须配置写实例URL，否则容器启动失败
        let write_instance_url = env::var("CRUD_API_WRITE_INSTANCE_URL")
            .map_err(|_| anyhow::anyhow!("CRUD_API_WRITE_INSTANCE_URL环境变量必须设置"))?;
        let write_instance_timeout = env::var("CRUD_API_WRITE_INSTANCE_TIMEOUT").unwrap_or("5000".to_string()).parse()?;
        let write_instance_retries = env::var("CRUD_API_WRITE_INSTANCE_RETRIES").unwrap_or("3".to_string()).parse()?;
        
        // 读实例URL默认与写实例URL相同，支持单独配置
        let read_instance_url = env::var("CRUD_API_READ_INSTANCE_URL").unwrap_or(write_instance_url.clone());
        let read_instance_timeout = env::var("CRUD_API_READ_INSTANCE_TIMEOUT").unwrap_or("5000".to_string()).parse()?;
        let read_instance_retries = env::var("CRUD_API_READ_INSTANCE_RETRIES").unwrap_or("3".to_string()).parse()?;
        
        // 健康检查间隔
        let health_check_interval = env::var("CRUD_API_HEALTH_CHECK_INTERVAL").unwrap_or("30".to_string()).parse()?;
        
        // 根据后端类型动态配置实例列表
        let (instances, strategy) = match backend_type.as_str() {
            // 单容器模式：读实例和写实例指向同一个URL
            "single" => {
                let instance_url = write_instance_url.clone();
                let instances = vec![
                    // 写实例
                    CrudApiInstance {
                        id: "write-01".to_string(),
                        url: instance_url.clone(),
                        instance_type: "write".to_string(),
                        timeout: write_instance_timeout,
                        retries: write_instance_retries,
                    },
                    // 读实例，指向同一个URL
                    CrudApiInstance {
                        id: "read-01".to_string(),
                        url: instance_url,
                        instance_type: "read".to_string(),
                        timeout: read_instance_timeout,
                        retries: read_instance_retries,
                    },
                ];
                (instances, SchedulerStrategy::Single)
            },
            // 读写分离模式：读实例和写实例指向不同的URL
            "read_write_split" => {
                let instances = vec![
                    // 写实例
                    CrudApiInstance {
                        id: "write-01".to_string(),
                        url: write_instance_url,
                        instance_type: "write".to_string(),
                        timeout: write_instance_timeout,
                        retries: write_instance_retries,
                    },
                    // 读实例
                    CrudApiInstance {
                        id: "read-01".to_string(),
                        url: read_instance_url,
                        instance_type: "read".to_string(),
                        timeout: read_instance_timeout,
                        retries: read_instance_retries,
                    },
                ];
                (instances, SchedulerStrategy::ReadWriteSplit)
            },
            // 负载均衡模式：多个混合实例
            "load_balance" => {
                // 加载负载均衡实例配置
                let mut instances = Vec::new();
                let mut index = 0;
                loop {
                    // 尝试读取第index个实例的配置
                    let instance_id = env::var(format!("CRUD_API_INSTANCE_{}_ID", index)).unwrap_or_default();
                    let instance_url = env::var(format!("CRUD_API_INSTANCE_{}_URL", index)).unwrap_or_default();
                    let instance_type = env::var(format!("CRUD_API_INSTANCE_{}_TYPE", index)).unwrap_or("mixed".to_string());
                    let instance_timeout = env::var(format!("CRUD_API_INSTANCE_{}_TIMEOUT", index)).unwrap_or("5000".to_string()).parse()?;
                    let instance_retries = env::var(format!("CRUD_API_INSTANCE_{}_RETRIES", index)).unwrap_or("3".to_string()).parse()?;
                    
                    // 如果没有配置实例ID或URL，说明已经没有更多实例了
                    if instance_id.is_empty() || instance_url.is_empty() {
                        break;
                    }
                    
                    instances.push(CrudApiInstance {
                        id: instance_id,
                        url: instance_url,
                        instance_type,
                        timeout: instance_timeout,
                        retries: instance_retries,
                    });
                    
                    index += 1;
                }
                
                // 如果没有配置任何实例，使用默认配置
                if instances.is_empty() {
                    instances.push(CrudApiInstance {
                        id: "crud-01".to_string(),
                        url: write_instance_url,
                        instance_type: "mixed".to_string(),
                        timeout: write_instance_timeout,
                        retries: write_instance_retries,
                    });
                }
                
                (instances, SchedulerStrategy::LoadBalance)
            },
            // 默认使用读写分离模式
            _ => {
                let instances = vec![
                    // 写实例
                    CrudApiInstance {
                        id: "write-01".to_string(),
                        url: write_instance_url,
                        instance_type: "write".to_string(),
                        timeout: write_instance_timeout,
                        retries: write_instance_retries,
                    },
                    // 读实例
                    CrudApiInstance {
                        id: "read-01".to_string(),
                        url: read_instance_url,
                        instance_type: "read".to_string(),
                        timeout: read_instance_timeout,
                        retries: read_instance_retries,
                    },
                ];
                (instances, SchedulerStrategy::ReadWriteSplit)
            },
        };

        let config = Self {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or("0.0.0.0".to_string()),
                port: env::var("SERVER_PORT").unwrap_or("9999".to_string()).parse()?,
                https: env::var("HTTPS").unwrap_or("false".to_string()).parse()?,
            },
            jwt: JwtConfig {
                secret: env::var("JWT_SECRET").unwrap_or("12345678901234567890".to_string()),
                expires_in: env::var("JWT_EXPIRES_IN").unwrap_or("3600".to_string()).parse()?,
                refresh_in: env::var("JWT_REFRESH_IN").unwrap_or("86400".to_string()).parse()?,
            },
            encryption: EncryptionConfig {
                algorithm: env::var("ENCRYPTION_ALGORITHM").unwrap_or("aes-256-gcm".to_string()),
                key_length: env::var("ENCRYPTION_KEY_LENGTH").unwrap_or("32".to_string()).parse()?,
                iterations: env::var("ENCRYPTION_ITERATIONS").unwrap_or("100000".to_string()).parse()?,
                salt: env::var("ENCRYPTION_SALT").unwrap_or("default_salt".to_string()),
            },
            service: ServiceRoleConfig {
                role: env::var("SERVICE_ROLE").unwrap_or("mixed".to_string()),
                id: env::var("SERVICE_ID").unwrap_or("encryption-01".to_string()),
            },
            crud_api: CrudApiConfig {
                instances,
                strategy,
                health_check_interval,
                timeout: write_instance_timeout, // 默认使用写实例的超时时间
                retries: write_instance_retries, // 默认使用写实例的重试次数
            },
        };
        
        Ok(config)
    }
    
    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        info!("验证配置");
        
        // 验证服务角色
        let valid_roles = vec!["encrypt", "decrypt", "mixed"];
        if !valid_roles.contains(&self.service.role.as_str()) {
            anyhow::bail!("无效的服务角色: {}", self.service.role);
        }
        
        // 验证JWT密钥长度
        if self.jwt.secret.len() < 16 {
            anyhow::bail!("JWT密钥长度至少为16个字符");
        }
        
        // 验证CRUD API实例配置
        if self.crud_api.instances.is_empty() {
            anyhow::bail!("CRUD API实例列表不能为空");
        }
        
        // 验证每个CRUD API实例
        for instance in &self.crud_api.instances {
            if instance.id.is_empty() {
                anyhow::bail!("CRUD API实例ID不能为空");
            }
            if instance.url.is_empty() {
                anyhow::bail!("CRUD API实例URL不能为空");
            }
            let valid_instance_types = vec!["read", "write", "mixed"];
            if !valid_instance_types.contains(&instance.instance_type.as_str()) {
                anyhow::bail!("无效的CRUD API实例类型: {}", instance.instance_type);
            }
        }
        
        // 根据调度策略验证实例分布
        match self.crud_api.strategy {
            SchedulerStrategy::ReadWriteSplit => {
                // 读写分离模式需要至少一个读实例和一个写实例
                let has_write_instance = self.crud_api.instances.iter().any(|i| 
                    i.instance_type == "write" || i.instance_type == "mixed"
                );
                let has_read_instance = self.crud_api.instances.iter().any(|i| 
                    i.instance_type == "read" || i.instance_type == "mixed"
                );
                
                if !has_write_instance {
                    anyhow::bail!("读写分离模式需要至少一个写实例或混合实例");
                }
                if !has_read_instance {
                    anyhow::bail!("读写分离模式需要至少一个读实例或混合实例");
                }
            },
            SchedulerStrategy::LoadBalance => {
                // 负载均衡模式需要至少一个实例
                if self.crud_api.instances.len() < 1 {
                    anyhow::bail!("负载均衡模式需要至少一个CRUD API实例");
                }
            },
            SchedulerStrategy::Single => {
                // 单实例模式需要恰好一个实例
                if self.crud_api.instances.len() != 1 {
                    anyhow::bail!("单实例模式需要恰好一个CRUD API实例");
                }
            },
        }
        
        info!("配置验证通过");
        Ok(())
    }
}