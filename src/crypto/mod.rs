use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use hkdf::Hkdf;
use sha2::Sha256;
use std::convert::TryInto;

/// 加密工具结构体
#[derive(Debug, Clone)]
pub struct EncryptionUtils {
    algorithm: String,
    key_length: u32,
    #[allow(dead_code)]
    iterations: u32,
    salt: Vec<u8>,
}

impl EncryptionUtils {
    /// 创建新的加密工具实例
    pub fn new(algorithm: String, key_length: u32, iterations: u32, salt: String) -> Self {
        Self {
            algorithm,
            key_length,
            iterations,
            salt: salt.into_bytes(),
        }
    }

    /// 生成加密密钥
    pub fn generate_key(&self, password: &str) -> Result<Vec<u8>> {
        // 使用HKDF从密码和盐生成密钥
        let hkdf = Hkdf::<Sha256>::new(Some(&self.salt), password.as_bytes());
        let mut key = vec![0u8; self.key_length.try_into()?];
        hkdf.expand(b"encryption", &mut key)
            .map_err(|e| anyhow::anyhow!("HKDF密钥生成失败: {:?}", e))?;
        Ok(key)
    }

    /// 加密数据
    pub async fn encrypt(&self, data: &str, password: &str) -> Result<String> {
        match self.algorithm.as_str() {
            "aes-256-gcm" => self.encrypt_aes_256_gcm(data, password),
            _ => anyhow::bail!("不支持的加密算法: {}", self.algorithm),
        }
    }

    /// 解密数据
    pub async fn decrypt(&self, encrypted_data: &str, password: &str) -> Result<String> {
        match self.algorithm.as_str() {
            "aes-256-gcm" => self.decrypt_aes_256_gcm(encrypted_data, password),
            _ => anyhow::bail!("不支持的加密算法: {}", self.algorithm),
        }
    }

    /// 使用AES-256-GCM加密数据
    fn encrypt_aes_256_gcm(&self, data: &str, password: &str) -> Result<String> {
        // 生成密钥
        let key = self.generate_key(password)?;
        let key = Key::<Aes256Gcm>::from_slice(&key);

        // 创建加密器
        let cipher = Aes256Gcm::new(key);

        // 生成随机nonce
        let mut nonce_bytes = [0u8; 12];
        getrandom::getrandom(&mut nonce_bytes)
            .map_err(|e| anyhow::anyhow!("生成随机nonce失败: {:?}", e))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密数据
        let ciphertext = cipher.encrypt(nonce, data.as_bytes())
            .map_err(|e| anyhow::anyhow!("AES-GCM加密失败: {:?}", e))?;

        // 组合nonce和密文
        let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        // Base64编码
        let encrypted = general_purpose::STANDARD.encode(combined);
        Ok(encrypted)
    }

    /// 使用AES-256-GCM解密数据
    fn decrypt_aes_256_gcm(&self, encrypted_data: &str, password: &str) -> Result<String> {
        // Base64解码
        let combined = general_purpose::STANDARD.decode(encrypted_data)?;

        // 分离nonce和密文
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // 生成密钥
        let key = self.generate_key(password)?;
        let key = Key::<Aes256Gcm>::from_slice(&key);

        // 创建解密器
        let cipher = Aes256Gcm::new(key);

        // 解密数据
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("AES-GCM解密失败: {:?}", e))?;
        let plaintext = String::from_utf8(plaintext)?;
        Ok(plaintext)
    }
}
