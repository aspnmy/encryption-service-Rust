use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};
use anyhow::Result;

/// 缓存数据类型
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CacheDataType {
    /// 加密数据
    Encrypt(EncryptCacheData),
    /// 解密数据
    Decrypt(DecryptCacheData),
}

/// 加密缓存数据
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EncryptCacheData {
    pub data: String,
    pub password: String,
    pub resource_type: String,
    pub encrypted_data: String,
}

/// 解密缓存数据
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DecryptCacheData {
    pub encrypted_data: String,
    pub password: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub decrypted_data: String,
}

/// 缓存条目
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CacheEntry {
    /// 时间戳
    pub timestamp: u64,
    /// 数据类型
    pub data_type: CacheDataType,
}

/// 缓存管理器
#[derive(Debug, Clone)]
pub struct CacheManager {
    /// 缓存目录
    cache_dir: String,
    /// 临时文件前缀
    temp_file_prefix: String,
    /// 临时文件更新间隔（秒）
    update_interval: u64,
    /// 临时文件保留时间（秒）
    retention_time: u64,
}

impl CacheManager {
    /// 创建新的缓存管理器实例
    pub fn new() -> Self {
        // 默认配置
        let cache_dir = String::from("data/cache");
        let temp_file_prefix = String::from("crud_api_cache");
        let update_interval = 3600; // 1小时
        let retention_time = 86400; // 24小时

        // 创建缓存目录
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            error!("无法创建缓存目录: {:?}", e);
        }

        Self {
            cache_dir,
            temp_file_prefix,
            update_interval,
            retention_time,
        }
    }

    /// 获取当前时间戳（秒）
    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("无法获取当前时间")
            .as_secs()
    }

    /// 获取当前缓存文件路径
   