use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
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
    fn get_current_cache_file(&self) -> String {
        let timestamp = self.get_current_timestamp();
        let file_name = format!("{}_{}.jsonl", self.temp_file_prefix, timestamp / self.update_interval);
        format!("{}/{}", self.cache_dir, file_name)
    }

    /// 写入缓存数据
    pub fn write_cache(&self, data_type: CacheDataType) -> Result<()> {
        let cache_entry = CacheEntry {
            timestamp: self.get_current_timestamp(),
            data_type,
        };

        // 序列化缓存条目
        let json_str = serde_json::to_string(&cache_entry)?;

        // 打开或创建缓存文件
        let file_path = self.get_current_cache_file();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        // 写入缓存条目
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{}", json_str)?;
        writer.flush()?;

        info!("缓存数据已写入文件: {}", file_path);
        Ok(())
    }

    /// 读取所有缓存数据
    pub fn read_all_cache(&self) -> Result<Vec<CacheEntry>> {
        let mut all_entries = Vec::new();

        // 遍历所有缓存文件
        let entries = fs::read_dir(&self.cache_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // 只处理JSONL文件
            if path.is_file() && path.extension() == Some("jsonl".as_ref()) {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // 读取文件中的所有条目
                for line in reader.lines() {
                    let line = line?;
                    if !line.is_empty() {
                        match serde_json::from_str::<CacheEntry>(&line) {
                            Ok(entry) => all_entries.push(entry),
                            Err(e) => {
                                warn!("无法解析缓存条目: {:?}, 行内容: {}", e, line);
                            },
                        }
                    }
                }
            }
        }

        Ok(all_entries)
    }

    /// 清理过期的缓存文件
    pub fn clean_expired_cache(&self) -> Result<()> {
        let current_timestamp = self.get_current_timestamp();
        let entries = fs::read_dir(&self.cache_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // 只处理JSONL文件
            if path.is_file() && path.extension() == Some("jsonl".as_ref()) {
                // 获取文件的修改时间
                let metadata = fs::metadata(&path)?;
                let modified_time = metadata.modified()?
                    .duration_since(UNIX_EPOCH)?
                    .as_secs();

                // 检查文件是否过期
                if current_timestamp - modified_time > self.retention_time {
                    if let Err(e) = fs::remove_file(&path) {
                        warn!("无法删除过期缓存文件: {:?}", e);
                    } else {
                        info!("已删除过期缓存文件: {:?}", path);
                    }
                }
            }
        }

        Ok(())
    }

    /// 启动定期清理任务
    pub async fn start_cleanup_task(&self) {
        let cache_manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(cache_manager.retention_time));
            loop {
                interval.tick().await;
                if let Err(e) = cache_manager.clean_expired_cache() {
                    error!("清理过期缓存失败: {:?}", e);
                }
            }
        });
    }
}