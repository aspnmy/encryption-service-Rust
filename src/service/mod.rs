use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tracing::{warn, error};
use crate::config::AppConfig;
use crate::crypto::EncryptionUtils;
use crate::scheduler::CrudApiScheduler;
use crate::cache::{CacheManager, CacheDataType, EncryptCacheData, DecryptCacheData};
use crate::test_instance::TestInstanceManager;

/// 加密请求结构体
#[derive(Debug, Deserialize, Serialize)]
pub struct EncryptRequest {
    pub data: String,
    pub password: String,
    pub resource_type: String,
}

/// 解密请求结构体
#[derive(Debug, Deserialize, Serialize)]
pub struct DecryptRequest {
    pub encrypted_data: String,
    pub password: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
}

/// 加密响应结构体
#[derive(Debug, Deserialize, Serialize)]
pub struct EncryptResponse {
    pub encrypted_data: String,
    pub resource_id: Option<String>,
}

/// 解密响应结构体
#[derive(Debug, Deserialize, Serialize)]
pub struct DecryptResponse {
    pub data: String,
    pub resource_id: Option<String>,
}

/// 通用响应结构体
#[derive(Debug, Deserialize, Serialize)]
pub struct GenericResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

/// 加密服务结构体
#[derive(Debug, Clone)]
pub struct EncryptionService {
    config: Arc<AppConfig>,
    crypto_utils: EncryptionUtils,
    http_client: Client,
    scheduler: CrudApiScheduler,
    cache_manager: CacheManager,
    test_instance_manager: TestInstanceManager,
}

impl EncryptionService {
    /// 获取服务ID
    pub fn get_service_id(&self) -> String {
        self.config.service.id.clone()
    }
    
    /// 获取服务角色
    pub fn get_service_role(&self) -> String {
        self.config.service.role.clone()
    }
    
    /// 获取调度器
    pub fn get_scheduler(&self) -> &CrudApiScheduler {
        &self.scheduler
    }

    /// 获取Test实例管理器
    pub fn get_test_instance_manager(&self) -> &TestInstanceManager {
        &self.test_instance_manager
    }

    /// 获取缓存管理器
    pub fn get_cache_manager(&self) -> &CacheManager {
        &self.cache_manager
    }
}

impl EncryptionService {
    /// 创建新的加密服务实例
    pub fn new(config: Arc<AppConfig>) -> Self {
        let crypto_utils = EncryptionUtils::new(
            config.encryption.algorithm.clone(),
            config.encryption.key_length,
            config.encryption.iterations,
            config.encryption.salt.clone(),
        );

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.crud_api.timeout))
            .build()
            .expect("无法创建HTTP客户端");

        // 创建并初始化调度器
        let scheduler = CrudApiScheduler::new(config.clone());

        // 创建缓存管理器
        let cache_manager = CacheManager::new();

        // 创建Test实例管理器
        let test_instance_manager = TestInstanceManager::new(config.clone(), cache_manager.clone());

        Self {
            config,
            crypto_utils,
            http_client,
            scheduler,
            cache_manager,
            test_instance_manager,
        }
    }

    /// 加密数据并保存到CRUD API
    pub async fn encrypt(&self, request: EncryptRequest) -> Result<EncryptResponse> {
        // 检查服务角色是否允许加密
        if self.config.service.role != "encrypt" && self.config.service.role != "mixed" {
            anyhow::bail!("当前服务角色不允许执行加密操作");
        }

        // 执行加密
        let encrypted_data = self.crypto_utils.encrypt(&request.data, &request.password).await?;

        // 准备保存到CRUD API的数据
        let crud_data = serde_json::json!({
            "encrypted_data": encrypted_data,
            "resource_type": request.resource_type,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        });

        // 创建缓存数据
        let encrypt_cache_data = EncryptCacheData {
            data: request.data.clone(),
            password: request.password.clone(),
            resource_type: request.resource_type.clone(),
            encrypted_data: encrypted_data.clone(),
        };

        // 尝试调用CRUD API
        match self.scheduler.select_instance(true) {
            Ok(instance) => {
                // 调用CRUD API保存数据
                let crud_url = format!("{}/{}", instance.url, request.resource_type);
                match self.http_client
                    .post(&crud_url)
                    .json(&crud_data)
                    .send()
                    .await
                    .and_then(|resp| resp.error_for_status())
                {
                    Ok(response) => {
                        // CRUD API调用成功，缓存数据
                        if let Err(e) = self.cache_manager.write_cache(CacheDataType::Encrypt(encrypt_cache_data)) {
                            warn!("缓存数据失败: {:?}", e);
                        }

                        let crud_response: GenericResponse<serde_json::Value> = response.json().await?;
                        let resource_id = crud_response.data
                            .and_then(|data| data.get("id").and_then(|id| id.as_str().map(|s| s.to_string())));

                        Ok(EncryptResponse {
                            encrypted_data,
                            resource_id,
                        })
                    },
                    Err(e) => {
                        // CRUD API调用失败，缓存数据并处理容错
                        error!("调用CRUD API失败: {:?}", e);
                        if let Err(cache_err) = self.cache_manager.write_cache(CacheDataType::Encrypt(encrypt_cache_data)) {
                            warn!("缓存数据失败: {:?}", cache_err);
                        }

                        // TODO: 实现test实例创建和数据导入逻辑
                        // 目前先返回加密后的数据，不依赖CRUD API
                        Ok(EncryptResponse {
                            encrypted_data,
                            resource_id: None,
                        })
                    },
                }
            },
            Err(e) => {
                // 没有健康的CRUD API实例，缓存数据并处理容错
                error!("没有健康的CRUD API实例: {:?}", e);
                if let Err(cache_err) = self.cache_manager.write_cache(CacheDataType::Encrypt(encrypt_cache_data)) {
                    warn!("缓存数据失败: {:?}", cache_err);
                }

                // 创建Test实例并导入缓存数据
                if let Err(ti_err) = self.test_instance_manager.create_test_instance().await {
                    error!("创建Test实例失败: {:?}", ti_err);
                } else if let Err(import_err) = self.test_instance_manager.import_cache_data().await {
                    error!("导入缓存数据失败: {:?}", import_err);
                }

                // 返回加密后的数据，不依赖CRUD API
                Ok(EncryptResponse {
                    encrypted_data,
                    resource_id: None,
                })
            },
        }
    }

    /// 从CRUD API获取数据并解密
    pub async fn decrypt(&self, request: DecryptRequest) -> Result<DecryptResponse> {
        // 检查服务角色是否允许解密
        if self.config.service.role != "decrypt" && self.config.service.role != "mixed" {
            anyhow::bail!("当前服务角色不允许执行解密操作");
        }

        // 克隆resource_id用于返回
        let resource_id = request.resource_id.clone();
        
        let encrypted_data = match &request.resource_id {
            Some(resource_id) => {
                // 尝试从CRUD API获取加密数据
                match self.scheduler.select_instance(false) {
                    Ok(instance) => {
                        // 从CRUD API获取加密数据
                        let crud_url = format!("{}/{}/{}?select=encrypted_data", 
                                            instance.url, 
                                            request.resource_type, 
                                            resource_id);
                        match self.http_client
                            .get(&crud_url)
                            .send()
                            .await
                            .and_then(|resp| resp.error_for_status())
                        {
                            Ok(response) => {
                                let crud_response: GenericResponse<serde_json::Value> = response.json().await?;
                                crud_response.data
                                    .and_then(|data| data.get("encrypted_data").and_then(|ed| ed.as_str().map(|s| s.to_string())))
                                    .ok_or_else(|| anyhow::anyhow!("无法获取加密数据"))?
                            },
                            Err(e) => {
                                // CRUD API调用失败，使用请求中的encrypted_data
                                error!("从CRUD API获取加密数据失败: {:?}", e);
                                request.encrypted_data.clone()
                            },
                        }
                    },
                    Err(e) => {
                        // 没有健康的CRUD API实例，使用请求中的encrypted_data
                        error!("没有健康的CRUD API实例: {:?}", e);
                        request.encrypted_data.clone()
                    },
                }
            },
            None => request.encrypted_data.clone(),
        };

        // 执行解密
        let data = self.crypto_utils.decrypt(&encrypted_data, &request.password).await?;

        // 创建缓存数据
        let decrypt_cache_data = DecryptCacheData {
            encrypted_data: encrypted_data.clone(),
            password: request.password.clone(),
            resource_type: request.resource_type.clone(),
            resource_id: resource_id.clone(),
            decrypted_data: data.clone(),
        };

        // 缓存数据
        if let Err(e) = self.cache_manager.write_cache(CacheDataType::Decrypt(decrypt_cache_data)) {
            warn!("缓存解密数据失败: {:?}", e);
        }

        Ok(DecryptResponse {
            data,
            resource_id,
        })
    }

    /// 批量加密数据
    pub async fn batch_encrypt(&self, requests: Vec<EncryptRequest>) -> Result<Vec<EncryptResponse>> {
        // 检查服务角色是否允许加密
        if self.config.service.role != "encrypt" && self.config.service.role != "mixed" {
            anyhow::bail!("当前服务角色不允许执行加密操作");
        }

        let mut responses = Vec::with_capacity(requests.len());
        for request in requests {
            let response = self.encrypt(request).await?;
            responses.push(response);
        }

        Ok(responses)
    }

    /// 批量解密数据
    pub async fn batch_decrypt(&self, requests: Vec<DecryptRequest>) -> Result<Vec<DecryptResponse>> {
        // 检查服务角色是否允许解密
        if self.config.service.role != "decrypt" && self.config.service.role != "mixed" {
            anyhow::bail!("当前服务角色不允许执行解密操作");
        }

        let mut responses = Vec::with_capacity(requests.len());
        for request in requests {
            let response = self.decrypt(request).await?;
            responses.push(response);
        }

        Ok(responses)
    }

    /// 服务健康检查
    pub async fn health_check(&self) -> Result<()> {
        // 检查配置是否有效
        self.config.validate()?;
        
        // 执行调度器健康检查
        let instance_status = self.scheduler.get_all_instance_status();
        
        // 检查是否有健康的实例
        let has_healthy_instance = instance_status.iter()
            .any(|(_, _, status)| *status == crate::scheduler::InstanceHealthStatus::Healthy);
        
        if !has_healthy_instance {
            anyhow::bail!("没有健康的CRUD API实例可用");
        }
        
        Ok(())
    }
}
