use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use crate::config::AppConfig;
use crate::crypto::EncryptionUtils;
use crate::scheduler::CrudApiScheduler;

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

        Self {
            config,
            crypto_utils,
            http_client,
            scheduler,
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

        // 使用调度器选择写实例
        let instance = self.scheduler.select_instance(true)?;
        
        // 调用CRUD API保存数据
        let crud_url = format!("{}/{}", instance.url, request.resource_type);
        let response = self.http_client
            .post(&crud_url)
            .json(&crud_data)
            .send()
            .await?
            .error_for_status()?;

        let crud_response: GenericResponse<serde_json::Value> = response.json().await?;
        let resource_id = crud_response.data
            .and_then(|data| data.get("id").and_then(|id| id.as_str().map(|s| s.to_string())));

        Ok(EncryptResponse {
            encrypted_data,
            resource_id,
        })
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
                // 使用调度器选择读实例
                let instance = self.scheduler.select_instance(false)?;
                
                // 从CRUD API获取加密数据
                let crud_url = format!("{}/{}/{}?select=encrypted_data", 
                                      instance.url, 
                                      request.resource_type, 
                                      resource_id);
                let response = self.http_client
                    .get(&crud_url)
                    .send()
                    .await?
                    .error_for_status()?;

                let crud_response: GenericResponse<serde_json::Value> = response.json().await?;
                let encrypted = crud_response.data
                    .and_then(|data| data.get("encrypted_data").and_then(|ed| ed.as_str().map(|s| s.to_string())))
                    .ok_or_else(|| anyhow::anyhow!("无法获取加密数据"))?;
                encrypted
            },
            None => request.encrypted_data.clone(),
        };

        // 执行解密
        let data = self.crypto_utils.decrypt(&encrypted_data, &request.password).await?;

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
