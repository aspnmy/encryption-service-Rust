use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{info, warn, error};
use anyhow::Result;
use reqwest::Client;

use crate::config::AppConfig;
use crate::cache::CacheManager;

/// Test实例状态
#[derive(Debug, Clone, PartialEq)]
pub enum TestInstanceState {
    /// 未创建
    NotCreated,
    /// 已创建
    Created,
    /// 已过期
    Expired,
}

/// Test实例配置
#[derive(Debug, Clone)]
pub struct TestInstanceConfig {
    /// 实例ID
    pub id: String,
    /// 实例URL
    pub url: String,
    /// 数据库前缀
    pub db_prefix: String,
    /// 创建时间（秒）
    pub created_at: u64,
    /// 过期时间（秒）
    pub expired_at: u64,
    /// 状态
    pub state: TestInstanceState,
}

/// Test实例管理器
#[derive(Debug, Clone)]
pub struct TestInstanceManager {
    /// 配置
    config: Arc<AppConfig>,
    /// HTTP客户端
    http_client: Client,
    /// 缓存管理器
    cache_manager: CacheManager,
    /// Test实例配置
    test_instance: Arc<RwLock<Option<TestInstanceConfig>>>,
    /// 企业微信群机器人URL
    wechat_webhook_url: String,
}

impl TestInstanceManager {
    /// 创建新的Test实例管理器
    pub fn new(config: Arc<AppConfig>, cache_manager: CacheManager) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_millis(config.crud_api.timeout))
            .build()
            .expect("无法创建HTTP客户端");

        // 默认企业微信群机器人URL
        let wechat_webhook_url = std::env::var("WECHAT_WEBHOOK_URL")
            .unwrap_or_default();

        Self {
            config,
            http_client,
            cache_manager,
            test_instance: Arc::new(RwLock::new(None)),
            wechat_webhook_url,
        }
    }

    /// 获取当前时间戳（秒）
    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("无法获取当前时间")
            .as_secs()
    }

    /// 创建Test实例
    pub async fn create_test_instance(&self) -> Result<TestInstanceConfig> {
        let mut test_instance = self.test_instance.write().unwrap();

        // 如果Test实例已存在且未过期，直接返回
        if let Some(ref instance) = *test_instance {
            if instance.state == TestInstanceState::Created && self.get_current_timestamp() < instance.expired_at {
                return Ok(instance.clone());
            }
        }

        // TODO: 实现Test实例创建逻辑
        // 目前使用模拟数据
        let created_at = self.get_current_timestamp();
        let expired_at = created_at + 172800; // 48小时后过期
        
        let test_instance_config = TestInstanceConfig {
            id: String::from("test-instance-01"),
            url: format!("http://localhost:8001"),
            db_prefix: String::from("test_"),
            created_at,
            expired_at,
            state: TestInstanceState::Created,
        };

        // 保存Test实例配置
        *test_instance = Some(test_instance_config.clone());

        info!("已创建Test实例: {:?}", test_instance_config);
        Ok(test_instance_config)
    }

    /// 导入缓存数据到Test实例
    pub async fn import_cache_data(&self) -> Result<()> {
        // 检查Test实例是否存在
        let has_created_instance = {
            let test_instance_opt = self.test_instance.read().unwrap();
            test_instance_opt.as_ref()
                .map(|instance| instance.state == TestInstanceState::Created)
                .unwrap_or(false)
        };
        
        let _test_instance = if has_created_instance {
            // Test实例已存在，获取实例
            let test_instance_opt = self.test_instance.read().unwrap();
            test_instance_opt.clone().unwrap()
        } else {
            // Test实例不存在，创建Test实例
            self.create_test_instance().await?
        };

        // 读取所有缓存数据
        let cache_entries = self.cache_manager.read_all_cache()?;
        info!("准备导入 {} 条缓存数据到Test实例", cache_entries.len());

        // TODO: 实现缓存数据导入逻辑
        // 目前只记录日志
        for entry in cache_entries {
            info!("准备导入缓存数据: {:?}", entry);
            // 这里应该实现具体的数据导入逻辑
        }

        info!("缓存数据导入完成");
        Ok(())
    }

    /// 发送企业微信提醒
    pub async fn send_wechat_reminder(&self) -> Result<()> {
        if self.wechat_webhook_url.is_empty() {
            warn!("企业微信机器人URL未配置，无法发送提醒");
            return Ok(());
        }

        let message = serde_json::json!({
            "msgtype": "text",
            "text": {
                "content": "Test实例已存在超过48小时，请及时处理",
            }
        });

        let _response = self.http_client
            .post(&self.wechat_webhook_url)
            .json(&message)
            .send()
            .await?
            .error_for_status()?;

        info!("已发送企业微信提醒");
        Ok(())
    }

    /// 启动定期检查
    pub async fn start_periodic_check(&self) {
        let test_instance_manager = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // 每小时检查一次
            loop {
                interval.tick().await;
                if let Err(e) = test_instance_manager.periodic_check().await {
                    error!("定期检查失败: {:?}", e);
                }
            }
        });
    }

    /// 定期检查Test实例
    async fn periodic_check(&self) -> Result<()> {
        let current_timestamp = self.get_current_timestamp();
        
        // 检查Test实例是否存在
        let test_instance = self.test_instance.read().unwrap().clone();
        if let Some(instance) = test_instance {
            // 检查Test实例是否过期
            if current_timestamp > instance.expired_at && instance.state != TestInstanceState::Expired {
                // 更新Test实例状态
                {  // 使用块确保锁在await前释放
                    let mut test_instance_write = self.test_instance.write().unwrap();
                    if let Some(ref mut instance_write) = *test_instance_write {
                        instance_write.state = TestInstanceState::Expired;
                        info!("Test实例已过期: {:?}", instance_write);
                    }
                    // 锁会在这里自动释放
                }

                // 发送企业微信提醒
                if let Err(e) = self.send_wechat_reminder().await {
                    warn!("发送企业微信提醒失败: {:?}", e);
                }
            }
        }

        Ok(())
    }
}