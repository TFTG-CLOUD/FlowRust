use crate::dto::ApiResponse;
use crate::dto::{
    CreateBatchProcessingJobRequest, CreateProcessingJobRequest, ProcessingJobResponse,
    BatchProcessingJobResponse,
};
use crate::models::{
    BatchProcessingJob, ProcessingJob, ProcessingServerConfig, WebhookNotification,
};
use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use mongodb::bson::{doc, DateTime};
use mongodb::Collection;
use reqwest::Client;
use serde_json::json;
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub struct ProcessingService {
    processing_jobs_collection: Collection<ProcessingJob>,
    batch_processing_jobs_collection: Collection<BatchProcessingJob>,
    processing_servers_collection: Collection<ProcessingServerConfig>,
    webhook_notifications_collection: Collection<WebhookNotification>,
    http_client: Client,
}

impl ProcessingService {
    pub fn new(
        processing_jobs_collection: Collection<ProcessingJob>,
        batch_processing_jobs_collection: Collection<BatchProcessingJob>,
        processing_servers_collection: Collection<ProcessingServerConfig>,
        webhook_notifications_collection: Collection<WebhookNotification>,
    ) -> Self {
        Self {
            processing_jobs_collection,
            batch_processing_jobs_collection,
            processing_servers_collection,
            webhook_notifications_collection,
            http_client: Client::new(),
        }
    }

    // 创建处理任务
    pub async fn create_processing_job(
        &self,
        request: CreateProcessingJobRequest,
    ) -> Result<ApiResponse<ProcessingJobResponse>, String> {
        let now = DateTime::now();
        let job_id = Uuid::new_v4().to_string();

        // 获取可用的处理服务器
        let server = self.get_active_processing_server().await?;
        let server_config = server.ok_or("No active processing server available")?;

        // 调用处理服务器创建任务
        let processing_response = self
            .call_processing_server_create_job(&server_config, &request, &job_id)
            .await?;

        // 创建本地任务记录
        let job = ProcessingJob {
            id: None,
            job_id: processing_response.job_id.clone(),
            file_id: request.file_id.clone(),
            file_name: processing_response.file_name.clone(),
            job_type: request.job_type.clone(),
            status: "pending".to_string(),
            progress: 0,
            parameters: request.parameters.clone(),
            result: None,
            error: None,
            webhook_url: request.webhook_url.clone(),
            webhook_secret: request.webhook_secret.clone(),
            cms_id: request.cms_id.clone(),
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        let result = self
            .processing_jobs_collection
            .insert_one(&job, None)
            .await
            .map_err(|e| format!("Failed to create processing job: {}", e))?;

        let mut created_job = job;
        created_job.id = result.inserted_id.as_object_id();

        let response = ProcessingJobResponse {
            job_id: created_job.job_id.clone(),
            file_id: created_job.file_id.clone(),
            file_name: created_job.file_name.clone(),
            job_type: created_job.job_type.clone(),
            status: created_job.status.clone(),
            progress: created_job.progress,
            created_at: created_job.created_at,
            updated_at: created_job.updated_at,
        };

        Ok(ApiResponse::success(response))
    }

    // 创建批量处理任务
    pub async fn create_batch_processing_job(
        &self,
        request: CreateBatchProcessingJobRequest,
    ) -> Result<ApiResponse<BatchProcessingJobResponse>, String> {
        let now = DateTime::now();
        let batch_id = Uuid::new_v4().to_string();

        // 获取可用的处理服务器
        let server = self.get_active_processing_server().await?;
        let server_config = server.ok_or("No active processing server available")?;

        // 调用处理服务器创建批量任务
        let batch_response = self
            .call_processing_server_create_batch_job(&server_config, &request, &batch_id)
            .await?;

        // 创建本地批量任务记录
        let batch_job = BatchProcessingJob {
            id: None,
            batch_id: batch_response.batch_id.clone(),
            file_ids: request.file_ids.clone(),
            processing_type: request.processing_type.clone(),
            parameters: request.parameters.clone(),
            status: "pending".to_string(),
            progress: 0,
            completed_files: 0,
            failed_files: 0,
            total_files: request.file_ids.len() as i32,
            webhook_url: request.webhook_url.clone(),
            webhook_secret: request.webhook_secret.clone(),
            cms_id: request.cms_id.clone(),
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        let result = self
            .batch_processing_jobs_collection
            .insert_one(&batch_job, None)
            .await
            .map_err(|e| format!("Failed to create batch processing job: {}", e))?;

        let mut created_batch_job = batch_job;
        created_batch_job.id = result.inserted_id.as_object_id();

        let response = BatchProcessingJobResponse {
            batch_id: created_batch_job.batch_id.clone(),
            file_ids: created_batch_job.file_ids.clone(),
            processing_type: created_batch_job.processing_type.clone(),
            status: created_batch_job.status.clone(),
            progress: created_batch_job.progress,
            completed_files: created_batch_job.completed_files,
            total_files: created_batch_job.total_files,
            created_at: created_batch_job.created_at,
            updated_at: created_batch_job.updated_at,
        };

        Ok(ApiResponse::success(response))
    }

    // 获取处理任务状态
    pub async fn get_processing_job(
        &self,
        job_id: &str,
    ) -> Result<ApiResponse<ProcessingJobResponse>, String> {
        let filter = doc! { "job_id": job_id };
        let job = self
            .processing_jobs_collection
            .find_one(filter, None)
            .await
            .map_err(|e| format!("Failed to find processing job: {}", e))?;

        match job {
            Some(job) => {
                let response = ProcessingJobResponse {
                    job_id: job.job_id,
                    file_id: job.file_id,
                    file_name: job.file_name,
                    job_type: job.job_type,
                    status: job.status,
                    progress: job.progress,
                    created_at: job.created_at,
                    updated_at: job.updated_at,
                };
                Ok(ApiResponse::success(response))
            }
            None => Err("Processing job not found".to_string()),
        }
    }

    // 获取批量处理任务状态
    pub async fn get_batch_processing_job(
        &self,
        batch_id: &str,
    ) -> Result<ApiResponse<BatchProcessingJobResponse>, String> {
        let filter = doc! { "batch_id": batch_id };
        let batch_job = self
            .batch_processing_jobs_collection
            .find_one(filter, None)
            .await
            .map_err(|e| format!("Failed to find batch processing job: {}", e))?;

        match batch_job {
            Some(batch_job) => {
                let response = BatchProcessingJobResponse {
                    batch_id: batch_job.batch_id,
                    file_ids: batch_job.file_ids,
                    processing_type: batch_job.processing_type,
                    status: batch_job.status,
                    progress: batch_job.progress,
                    completed_files: batch_job.completed_files,
                    total_files: batch_job.total_files,
                    created_at: batch_job.created_at,
                    updated_at: batch_job.updated_at,
                };
                Ok(ApiResponse::success(response))
            }
            None => Err("Batch processing job not found".to_string()),
        }
    }

    // 处理Webhook通知
    pub async fn handle_webhook_notification(
        &self,
        notification: WebhookNotification,
    ) -> Result<ApiResponse<()>, String> {
        let now = DateTime::now();

        // 保存webhook通知
        let notification_result = self
            .webhook_notifications_collection
            .insert_one(&notification, None)
            .await
            .map_err(|e| format!("Failed to save webhook notification: {}", e))?;

        // 更新相应的处理任务状态
        if let Some(job_id) = &notification.job_id {
            self.update_processing_job_status(job_id, &notification).await?;
        }

        if let Some(batch_id) = &notification.batch_id {
            self.update_batch_processing_job_status(batch_id, &notification)
                .await?;
        }

        Ok(ApiResponse::success(()))
    }

    // 验证webhook签名
    pub fn verify_webhook_signature(
        &self,
        webhook_secret: &str,
        signature: &str,
        payload: &str,
    ) -> Result<bool, String> {
        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
            .map_err(|e| format!("Failed to create HMAC: {}", e))?;

        mac.update(payload.as_bytes());

        let result = mac.finalize();
        let expected_signature = general_purpose::STANDARD.encode(result.into_bytes());

        Ok(expected_signature == signature)
    }

    // 获取活跃的处理服务器
    async fn get_active_processing_server(
        &self,
    ) -> Result<Option<ProcessingServerConfig>, String> {
        let filter = doc! { "status": 1 };
        let server = self
            .processing_servers_collection
            .find_one(filter, None)
            .await
            .map_err(|e| format!("Failed to find active processing server: {}", e))?;

        Ok(server)
    }

    // 调用处理服务器创建任务
    async fn call_processing_server_create_job(
        &self,
        server: &ProcessingServerConfig,
        request: &CreateProcessingJobRequest,
        job_id: &str,
    ) -> Result<ProcessingJobResponse, String> {
        let url = format!("{}/api/processing/job", server.host);

        let request_body = json!({
            "fileId": request.file_id,
            "type": request.job_type,
            "parameters": request.parameters,
            "webhookUrl": request.webhook_url,
            "webhookSecret": request.webhook_secret,
            "cmsId": request.cms_id
        });

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-API-Key", &server.api_key)
            .header("X-API-Secret", &server.api_secret)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to call processing server: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Processing server returned error status {}: {}", status, error_text));
        }

        let job_response: ProcessingJobResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse processing server response: {}", e))?;

        Ok(job_response)
    }

    // 调用处理服务器创建批量任务
    async fn call_processing_server_create_batch_job(
        &self,
        server: &ProcessingServerConfig,
        request: &CreateBatchProcessingJobRequest,
        batch_id: &str,
    ) -> Result<BatchProcessingJobResponse, String> {
        let url = format!("{}/api/processing/batch", server.host);

        let request_body = json!({
            "fileIds": request.file_ids,
            "processingType": request.processing_type,
            "parameters": request.parameters,
            "webhookUrl": request.webhook_url,
            "webhookSecret": request.webhook_secret,
            "cmsId": request.cms_id
        });

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-API-Key", &server.api_key)
            .header("X-API-Secret", &server.api_secret)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to call processing server: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Processing server returned error status {}: {}", status, error_text));
        }

        let batch_response: BatchProcessingJobResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse processing server response: {}", e))?;

        Ok(batch_response)
    }

    // 更新处理任务状态
    async fn update_processing_job_status(
        &self,
        job_id: &str,
        notification: &WebhookNotification,
    ) -> Result<(), String> {
        let now = DateTime::now();
        let filter = doc! { "job_id": job_id };

        let mut update_doc = doc! {
            "$set": {
                "status": &notification.status,
                "progress": notification.progress,
                "updated_at": now
            }
        };

        if let Some(ref result) = notification.result {
            update_doc.get_mut("$set").unwrap().as_document_mut().unwrap()
                .insert("result", mongodb::bson::to_bson(result).unwrap_or_default());
        }

        if let Some(ref error) = notification.error {
            update_doc.get_mut("$set").unwrap().as_document_mut().unwrap()
                .insert("error", error.clone());
        }

        if notification.status == "completed" || notification.status == "failed" {
            update_doc.get_mut("$set").unwrap().as_document_mut().unwrap()
                .insert("completed_at", now);
        }

        self.processing_jobs_collection
            .update_one(filter, update_doc, None)
            .await
            .map_err(|e| format!("Failed to update processing job: {}", e))?;

        Ok(())
    }

    // 更新批量处理任务状态
    async fn update_batch_processing_job_status(
        &self,
        batch_id: &str,
        notification: &WebhookNotification,
    ) -> Result<(), String> {
        let now = DateTime::now();
        let filter = doc! { "batch_id": batch_id };

        let mut update_doc = doc! {
            "$set": {
                "status": &notification.status,
                "progress": notification.progress,
                "updated_at": now
            }
        };

        if let Some(completed_files) = notification.completed_files {
            update_doc.get_mut("$set").unwrap().as_document_mut().unwrap()
                .insert("completed_files", completed_files);
        }

        if let Some(failed_files) = notification.failed_files {
            update_doc.get_mut("$set").unwrap().as_document_mut().unwrap()
                .insert("failed_files", failed_files);
        }

        if notification.status == "completed" || notification.status == "failed" {
            update_doc.get_mut("$set").unwrap().as_document_mut().unwrap()
                .insert("completed_at", now);
        }

        self.batch_processing_jobs_collection
            .update_one(filter, update_doc, None)
            .await
            .map_err(|e| format!("Failed to update batch processing job: {}", e))?;

        Ok(())
    }
}