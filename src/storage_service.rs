use crate::models::{StorageServer, PresignedUploadResponse, ChunkUploadInfo};
use crate::dto::ApiResponse;
use mongodb::bson::{doc, DateTime};
use mongodb::Collection;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use reqwest::Client;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStorageServerRequest {
    pub name: String,
    pub host: String,
    pub api_key: String,
    pub api_secret: String,
    pub cms_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateStorageServerRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub cms_id: Option<String>,
    pub status: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratePresignedUrlRequest {
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub upload_type: String, // "single", "chunk", "archive"
}

pub struct StorageService {
    collection: Collection<StorageServer>,
    http_client: Client,
}

impl StorageService {
    pub fn new(collection: Collection<StorageServer>) -> Self {
        Self {
            collection,
            http_client: Client::new(),
        }
    }

    // 创建分布式储存服务器
    pub async fn create_server(&self, request: CreateStorageServerRequest) -> Result<ApiResponse<StorageServer>, String> {
        let now = DateTime::now();
        let server = StorageServer {
            id: None,
            name: request.name,
            host: request.host,
            api_key: request.api_key,
            api_secret: request.api_secret,
            cms_id: request.cms_id,
            status: 1,
            created_at: now,
            updated_at: now,
        };

        let result = self.collection.insert_one(&server, None).await
            .map_err(|e| format!("Failed to create storage server: {}", e))?;

        let mut created_server = server;
        created_server.id = result.inserted_id.as_object_id();

        Ok(ApiResponse::success(created_server))
    }

    // 获取所有分布式储存服务器
    pub async fn get_servers(&self) -> Result<ApiResponse<Vec<StorageServer>>, String> {
        let cursor = self.collection.find(None, None).await
            .map_err(|e| format!("Failed to query storage servers: {}", e))?;

        let servers: Vec<StorageServer> = cursor.try_collect().await
            .map_err(|e| format!("Failed to collect storage servers: {}", e))?;

        Ok(ApiResponse::success(servers))
    }

    // 获取单个分布式储存服务器
    pub async fn get_server(&self, id: &str) -> Result<ApiResponse<StorageServer>, String> {
        let object_id = mongodb::bson::oid::ObjectId::parse_str(id)
            .map_err(|e| format!("Invalid ID format: {}", e))?;

        let filter = doc! { "_id": object_id };
        let server = self.collection.find_one(filter, None).await
            .map_err(|e| format!("Failed to find storage server: {}", e))?;

        match server {
            Some(server) => Ok(ApiResponse::success(server)),
            None => Err("Storage server not found".to_string()),
        }
    }

    // 更新分布式储存服务器
    pub async fn update_server(&self, id: &str, request: UpdateStorageServerRequest) -> Result<ApiResponse<StorageServer>, String> {
        let object_id = mongodb::bson::oid::ObjectId::parse_str(id)
            .map_err(|e| format!("Invalid ID format: {}", e))?;

        let mut update_doc = doc! {};
        let now = DateTime::now();

        if let Some(name) = request.name {
            update_doc.insert("name", name);
        }
        if let Some(host) = request.host {
            update_doc.insert("host", host);
        }
        if let Some(api_key) = request.api_key {
            update_doc.insert("api_key", api_key);
        }
        if let Some(api_secret) = request.api_secret {
            update_doc.insert("api_secret", api_secret);
        }
        if let Some(cms_id) = request.cms_id {
            update_doc.insert("cms_id", cms_id);
        }
        if let Some(status) = request.status {
            update_doc.insert("status", status);
        }

        update_doc.insert("updated_at", now);

        let filter = doc! { "_id": object_id };
        let update = doc! { "$set": update_doc };

        let result = self.collection.update_one(filter.clone(), update, None).await
            .map_err(|e| format!("Failed to update storage server: {}", e))?;

        if result.matched_count == 0 {
            return Err("Storage server not found".to_string());
        }

        let updated_server = self.collection.find_one(filter, None).await
            .map_err(|e| format!("Failed to fetch updated server: {}", e))?;

        match updated_server {
            Some(server) => Ok(ApiResponse::success(server)),
            None => Err("Failed to fetch updated server".to_string()),
        }
    }

    // 删除分布式储存服务器
    pub async fn delete_server(&self, id: &str) -> Result<ApiResponse<()>, String> {
        let object_id = mongodb::bson::oid::ObjectId::parse_str(id)
            .map_err(|e| format!("Invalid ID format: {}", e))?;

        let filter = doc! { "_id": object_id };
        let result = self.collection.delete_one(filter, None).await
            .map_err(|e| format!("Failed to delete storage server: {}", e))?;

        if result.deleted_count == 0 {
            return Err("Storage server not found".to_string());
        }

        Ok(ApiResponse::success(()))
    }

    // 生成单文件上传预签名URL
    pub async fn generate_single_upload_url(&self, server_id: &str, request: GeneratePresignedUrlRequest) -> Result<ApiResponse<PresignedUploadResponse>, String> {
        let server = self.get_server(server_id).await?;
        let server_data = server.data.ok_or("Server not found")?;

        if server_data.status != 1 {
            return Err("Storage server is disabled".to_string());
        }

        let file_id = Uuid::new_v4().to_string();
        let expiration = Utc::now() + Duration::hours(1);
        
        // 生成签名
        let signature = self.generate_signature(
            &server_data.api_secret,
            &file_id,
            &request.filename,
            request.file_size,
            expiration.timestamp(),
        )?;

        // 构建上传URL
        let upload_url = format!(
            "{}/api/upload/single?file_id={}&filename={}&content_type={}&file_size={}&expiration={}&signature={}&api_key={}",
            server_data.host,
            file_id,
            urlencoding::encode(&request.filename),
            urlencoding::encode(&request.content_type),
            request.file_size,
            expiration.timestamp(),
            signature,
            server_data.api_key
        );

        let response = PresignedUploadResponse {
            upload_url,
            file_id,
            expiration: expiration.timestamp(),
            max_file_size: request.file_size,
        };

        Ok(ApiResponse::success(response))
    }

    // 生成分片上传预签名URL
    pub async fn generate_chunk_upload_url(&self, server_id: &str, request: GeneratePresignedUrlRequest) -> Result<ApiResponse<ChunkUploadInfo>, String> {
        let server = self.get_server(server_id).await?;
        let server_data = server.data.ok_or("Server not found")?;

        if server_data.status != 1 {
            return Err("Storage server is disabled".to_string());
        }

        let upload_id = Uuid::new_v4().to_string();
        let chunk_size = 5 * 1024 * 1024; // 5MB chunks
        let total_chunks = (request.file_size + chunk_size - 1) / chunk_size;
        let expiration = Utc::now() + Duration::hours(1);

        let mut chunk_urls = Vec::new();
        for chunk_index in 0..total_chunks {
            let signature = self.generate_signature(
                &server_data.api_secret,
                &upload_id,
                &request.filename,
                chunk_size,
                expiration.timestamp(),
            )?;

            let chunk_url = format!(
                "{}/api/upload/chunk?upload_id={}&chunk_index={}&chunk_size={}&filename={}&expiration={}&signature={}&api_key={}",
                server_data.host,
                upload_id,
                chunk_index,
                chunk_size,
                urlencoding::encode(&request.filename),
                expiration.timestamp(),
                signature,
                server_data.api_key
            );
            chunk_urls.push(chunk_url);
        }

        // 生成完成上传的URL
        let complete_signature = self.generate_signature(
            &server_data.api_secret,
            &upload_id,
            &request.filename,
            request.file_size,
            expiration.timestamp(),
        )?;

        let complete_url = format!(
            "{}/api/upload/complete?upload_id={}&filename={}&file_size={}&expiration={}&signature={}&api_key={}",
            server_data.host,
            upload_id,
            urlencoding::encode(&request.filename),
            request.file_size,
            expiration.timestamp(),
            complete_signature,
            server_data.api_key
        );

        let response = ChunkUploadInfo {
            upload_id,
            chunk_size,
            total_chunks: total_chunks as i32,
            chunk_urls,
            complete_url,
            expiration: expiration.timestamp(),
        };

        Ok(ApiResponse::success(response))
    }

    // 生成压缩包上传预签名URL
    pub async fn generate_archive_upload_url(&self, server_id: &str, request: GeneratePresignedUrlRequest) -> Result<ApiResponse<PresignedUploadResponse>, String> {
        let server = self.get_server(server_id).await?;
        let server_data = server.data.ok_or("Server not found")?;

        if server_data.status != 1 {
            return Err("Storage server is disabled".to_string());
        }

        let file_id = Uuid::new_v4().to_string();
        let expiration = Utc::now() + Duration::hours(2); // 压缩包上传时间更长

        // 生成签名
        let signature = self.generate_signature(
            &server_data.api_secret,
            &file_id,
            &request.filename,
            request.file_size,
            expiration.timestamp(),
        )?;

        // 构建上传URL
        let upload_url = format!(
            "{}/api/upload/archive?file_id={}&filename={}&content_type={}&file_size={}&expiration={}&signature={}&api_key={}",
            server_data.host,
            file_id,
            urlencoding::encode(&request.filename),
            urlencoding::encode(&request.content_type),
            request.file_size,
            expiration.timestamp(),
            signature,
            server_data.api_key
        );

        let response = PresignedUploadResponse {
            upload_url,
            file_id,
            expiration: expiration.timestamp(),
            max_file_size: request.file_size,
        };

        Ok(ApiResponse::success(response))
    }

    // 生成签名
    fn generate_signature(&self, api_secret: &str, file_id: &str, filename: &str, file_size: i64, expiration: i64) -> Result<String, String> {
        let message = format!("{}:{}:{}:{}", file_id, filename, file_size, expiration);
        
        let mut mac = HmacSha256::new_from_slice(api_secret.as_bytes())
            .map_err(|e| format!("Failed to create HMAC: {}", e))?;
        
        mac.update(message.as_bytes());
        
        let result = mac.finalize();
        let signature = general_purpose::STANDARD.encode(result.into_bytes());
        
        Ok(signature)
    }

    // 测试服务器连接
    pub async fn test_server_connection(&self, server_id: &str) -> Result<ApiResponse<bool>, String> {
        let server = self.get_server(server_id).await?;
        let server_data = server.data.ok_or("Server not found")?;

        let test_url = format!("{}/api/health", server_data.host);
        
        let response = self.http_client.get(&test_url)
            .header("X-API-Key", &server_data.api_key)
            .send().await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(ApiResponse::success(true))
                } else {
                    Ok(ApiResponse::success(false))
                }
            }
            Err(_) => Ok(ApiResponse::success(false)),
        }
    }
}