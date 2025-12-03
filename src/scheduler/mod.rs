use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, error};
use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

use crate::config::{AppConfig, SchedulerStrategy, CrudApiInstance};

/// 实例健康状态
#[derive(Debug, Clone, PartialEq)]
pub enum InstanceHealthStatus {
    /// 健康
    Healthy,
    /// 不健康
    Unhealthy,
    /// 未知
    Unknown,
}

/// 健康检查响应
#[derive(Debug, Deserialize)]
struct HealthCheckResponse {
    status: String,
}

/// 调度器结构体
#[derive(Debug, Clone)]
pub struct CrudApiScheduler {
    /// 配置
    config: Arc<AppConfig>,
    /// HTTP客户端
    http_client: Client,
    /// 实例健康状态
    instance_health: Arc<RwLock<Vec<(CrudApiInstance, InstanceHealthStatus)>>>,
    /// 负载均衡计数器
    load_balance_counter: Arc<RwLock<usize>>,
}

impl CrudApiScheduler {
    /// 创建新的调度器实例
    pub fn new(config: Arc<AppConfig>) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_millis(config.crud_api.timeout))
            .build()
            .expect("无法创建HTTP客户端");

        // 初始化实例健康状态
        let instance_health = config.crud_api.instances.iter()
            .map(|instance| (instance.clone(), InstanceHealthStatus::Unknown))
            .collect();

        let scheduler = Self {
            config,
            http_client,
            instance_health: Arc::new(RwLock::new(instance_health)),
            load_balance_counter: Arc::new(RwLock::new(0)),
        };

        scheduler
    }

    /// 启动健康检查
    pub async fn start_health_check(&self) {
        let scheduler = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(scheduler.config.crud_api.health_check_interval));
            loop {
                interval.tick().await;
                if let Err(e) = scheduler.perform_health_check().await {
                    error!("健康检查失败: {:?}", e);
                }
            }
        });
    }

    /// 执行健康检查
    async fn perform_health_check(&self) -> Result<()> {
        // 1. 首先获取所有实例的副本，避免在await期间持有锁
        let instances: Vec<CrudApiInstance> = {
            let health_status = self.instance_health.read().unwrap();
            health_status.iter().map(|(instance, _)| instance.clone()).collect()
        };
        
        // 2. 检查每个实例的健康状态，不持有锁
        let mut new_health_status = Vec::with_capacity(instances.len());
        for instance in instances {
            let health_url = format!("{}/health", instance.url);
            
            let status = match self.http_client.get(&health_url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<HealthCheckResponse>().await {
                            Ok(health_response) => {
                                if health_response.status == "ok" {
                                    InstanceHealthStatus::Healthy
                                } else {
                                    InstanceHealthStatus::Unhealthy
                                }
                            },
                            Err(_) => InstanceHealthStatus::Unhealthy,
                        }
                    } else {
                        InstanceHealthStatus::Unhealthy
                    }
                },
                Err(_) => InstanceHealthStatus::Unhealthy,
            };
            
            new_health_status.push((instance, status));
        }
        
        // 3. 更新健康状态，只在更新时持有锁
        let mut health_status = self.instance_health.write().unwrap();
        for i in 0..health_status.len() {
            let (ref instance, ref new_status) = new_health_status[i];
            let current_status = &mut health_status[i].1;
            
            if *current_status != *new_status {
                info!("CRUD API实例 {:?} 健康状态变化: {:?} -> {:?}", instance.id, current_status, new_status);
                *current_status = new_status.clone();
            }
        }
        
        Ok(())
    }

    /// 获取健康的实例列表
    fn get_healthy_instances(&self, instance_type: &str) -> Vec<CrudApiInstance> {
        let health_status = self.instance_health.read().unwrap();
        
        health_status.iter()
            .filter(|(instance, status)| {
                *status == InstanceHealthStatus::Healthy && 
                (instance.instance_type == instance_type || instance.instance_type == "mixed")
            })
            .map(|(instance, _)| instance.clone())
            .collect()
    }

    /// 根据请求类型选择实例
    pub fn select_instance(&self, is_write_operation: bool) -> Result<CrudApiInstance> {
        let strategy = &self.config.crud_api.strategy;
        
        match strategy {
            SchedulerStrategy::Single => {
                // 单实例模式直接返回第一个健康实例
                let healthy_instances = self.get_healthy_instances("mixed");
                healthy_instances.first().cloned()
                    .ok_or_else(|| anyhow::anyhow!("没有健康的CRUD API实例可用"))
            },
            SchedulerStrategy::ReadWriteSplit => {
                // 读写分离模式
                if is_write_operation {
                    // 写操作选择写实例或混合实例
                    let healthy_write_instances = self.get_healthy_instances("write");
                    healthy_write_instances.first().cloned()
                        .ok_or_else(|| anyhow::anyhow!("没有健康的写实例可用"))
                } else {
                    // 读操作选择读实例或混合实例
                    let healthy_read_instances = self.get_healthy_instances("read");
                    healthy_read_instances.first().cloned()
                        .ok_or_else(|| anyhow::anyhow!("没有健康的读实例可用"))
                }
            },
            SchedulerStrategy::LoadBalance => {
                // 负载均衡模式
                let instance_type = if is_write_operation { "write" } else { "read" };
                let healthy_instances = self.get_healthy_instances(instance_type);
                
                if healthy_instances.is_empty() {
                    return Err(anyhow::anyhow!("没有健康的{}实例可用", instance_type));
                }
                
                // 简单轮询负载均衡
                let mut counter = self.load_balance_counter.write().unwrap();
                let index = *counter % healthy_instances.len();
                *counter = *counter + 1;
                
                Ok(healthy_instances[index].clone())
            },
        }
    }

    /// 获取所有实例状态
    pub fn get_all_instance_status(&self) -> Vec<(String, String, InstanceHealthStatus)> {
        let health_status = self.instance_health.read().unwrap();
        
        health_status.iter()
            .map(|(instance, status)| {
                (instance.id.clone(), instance.url.clone(), status.clone())
            })
            .collect()
    }
}
