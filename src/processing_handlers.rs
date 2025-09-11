use crate::dto::ApiResponse;
use crate::dto::{
    CreateBatchProcessingJobRequest, CreateProcessingJobRequest, WebhookVerificationRequest,
};
use crate::jwt_auth::AdminUser;
use crate::models::{ProcessingJob, BatchProcessingJob, ProcessingServerConfig, WebhookNotification};
use crate::processing_service::ProcessingService;
use actix_web::{web, HttpResponse, Result};
use mongodb::{bson::doc, Database, options::FindOptions};
use futures::TryStreamExt;
use serde_json::Value;

// 创建处理任务 - 需要用户登录
pub async fn create_processing_job(
    _user: AdminUser,
    db: web::Data<Database>,
    request: web::Json<CreateProcessingJobRequest>,
) -> Result<HttpResponse> {
    let processing_jobs_collection = db.collection::<ProcessingJob>("processing_jobs");
    let batch_processing_jobs_collection = db.collection::<BatchProcessingJob>("batch_processing_jobs");
    let processing_servers_collection = db.collection::<ProcessingServerConfig>("processing_servers");
    let webhook_notifications_collection = db.collection::<WebhookNotification>("webhook_notifications");
    
    let service = ProcessingService::new(
        processing_jobs_collection,
        batch_processing_jobs_collection,
        processing_servers_collection,
        webhook_notifications_collection,
    );

    match service.create_processing_job(request.into_inner()).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 获取处理任务状态 - 需要用户登录
pub async fn get_processing_job(
    _user: AdminUser,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let job_id = path.into_inner();
    let processing_jobs_collection = db.collection::<ProcessingJob>("processing_jobs");
    let batch_processing_jobs_collection = db.collection::<BatchProcessingJob>("batch_processing_jobs");
    let processing_servers_collection = db.collection::<ProcessingServerConfig>("processing_servers");
    let webhook_notifications_collection = db.collection::<WebhookNotification>("webhook_notifications");
    
    let service = ProcessingService::new(
        processing_jobs_collection,
        batch_processing_jobs_collection,
        processing_servers_collection,
        webhook_notifications_collection,
    );

    match service.get_processing_job(&job_id).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 创建批量处理任务 - 需要用户登录
pub async fn create_batch_processing_job(
    _user: AdminUser,
    db: web::Data<Database>,
    request: web::Json<CreateBatchProcessingJobRequest>,
) -> Result<HttpResponse> {
    let processing_jobs_collection = db.collection::<ProcessingJob>("processing_jobs");
    let batch_processing_jobs_collection = db.collection::<BatchProcessingJob>("batch_processing_jobs");
    let processing_servers_collection = db.collection::<ProcessingServerConfig>("processing_servers");
    let webhook_notifications_collection = db.collection::<WebhookNotification>("webhook_notifications");
    
    let service = ProcessingService::new(
        processing_jobs_collection,
        batch_processing_jobs_collection,
        processing_servers_collection,
        webhook_notifications_collection,
    );

    match service.create_batch_processing_job(request.into_inner()).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 获取批量处理任务状态 - 需要用户登录
pub async fn get_batch_processing_job(
    _user: AdminUser,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let batch_id = path.into_inner();
    let processing_jobs_collection = db.collection::<ProcessingJob>("processing_jobs");
    let batch_processing_jobs_collection = db.collection::<BatchProcessingJob>("batch_processing_jobs");
    let processing_servers_collection = db.collection::<ProcessingServerConfig>("processing_servers");
    let webhook_notifications_collection = db.collection::<WebhookNotification>("webhook_notifications");
    
    let service = ProcessingService::new(
        processing_jobs_collection,
        batch_processing_jobs_collection,
        processing_servers_collection,
        webhook_notifications_collection,
    );

    match service.get_batch_processing_job(&batch_id).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 处理Webhook通知 - 公开端点，需要签名验证
pub async fn handle_webhook(
    db: web::Data<Database>,
    payload: web::Json<Value>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let processing_jobs_collection = db.collection::<ProcessingJob>("processing_jobs");
    let batch_processing_jobs_collection = db.collection::<BatchProcessingJob>("batch_processing_jobs");
    let processing_servers_collection = db.collection::<ProcessingServerConfig>("processing_servers");
    let webhook_notifications_collection = db.collection::<WebhookNotification>("webhook_notifications");
    
    // Create clones for service and keep originals for webhook logic
    let service = ProcessingService::new(
        processing_jobs_collection.clone(),
        batch_processing_jobs_collection.clone(),
        processing_servers_collection.clone(),
        webhook_notifications_collection.clone(),
    );

    // 解析webhook通知
    let notification: WebhookNotification = match serde_json::from_value(payload.into_inner()) {
        Ok(notification) => notification,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                format!("Invalid webhook payload: {}", e),
            )));
        }
    };

    // 验证签名 - 从处理任务中获取webhook_secret
    let webhook_secret = if let Some(ref job_id) = notification.job_id {
        // 从单个任务中获取
        let filter = doc! { "job_id": job_id };
        let job = match processing_jobs_collection
            .find_one(filter, None)
            .await
        {
            Ok(job) => job,
            Err(e) => {
                return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(format!("Failed to find processing job: {}", e))));
            }
        };
        job.and_then(|j| j.webhook_secret)
    } else if let Some(ref batch_id) = notification.batch_id {
        // 从批量任务中获取
        let filter = doc! { "batch_id": batch_id };
        let batch_job = match batch_processing_jobs_collection
            .find_one(filter, None)
            .await
        {
            Ok(batch_job) => batch_job,
            Err(e) => {
                return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(format!("Failed to find batch processing job: {}", e))));
            }
        };
        batch_job.and_then(|j| j.webhook_secret)
    } else {
        None
    };

    if let Some(ref webhook_secret) = webhook_secret {
        let payload_str = serde_json::to_string(&notification).unwrap_or_default();
        let signature = req.headers().get("X-Webhook-Signature")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        
        match service.verify_webhook_signature(webhook_secret, signature, &payload_str) {
            Ok(true) => {
                // 签名验证成功，处理通知
                match service.handle_webhook_notification(notification).await {
                    Ok(response) => Ok(HttpResponse::Ok().json(response)),
                    Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
                }
            }
            Ok(false) => {
                Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(
                    "Invalid webhook signature".to_string(),
                )))
            }
            Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
        }
    } else {
        // 如果没有webhook secret，直接处理通知（不推荐用于生产环境）
        match service.handle_webhook_notification(notification).await {
            Ok(response) => Ok(HttpResponse::Ok().json(response)),
            Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
        }
    }
}

// 验证webhook签名
pub async fn verify_webhook_signature(
    db: web::Data<Database>,
    request: web::Json<WebhookVerificationRequest>,
) -> Result<HttpResponse> {
    let processing_jobs_collection = db.collection::<ProcessingJob>("processing_jobs");
    let batch_processing_jobs_collection = db.collection::<BatchProcessingJob>("batch_processing_jobs");
    let processing_servers_collection = db.collection::<ProcessingServerConfig>("processing_servers");
    let webhook_notifications_collection = db.collection::<WebhookNotification>("webhook_notifications");
    
    let service = ProcessingService::new(
        processing_jobs_collection,
        batch_processing_jobs_collection,
        processing_servers_collection,
        webhook_notifications_collection,
    );

    match service.verify_webhook_signature(
        &request.webhook_secret,
        &request.signature,
        &request.payload,
    ) {
        Ok(valid) => {
            if valid {
                Ok(HttpResponse::Ok().json(ApiResponse::success(true)))
            } else {
                Ok(HttpResponse::Unauthorized().json(ApiResponse::success(false)))
            }
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 获取处理任务列表 - 需要管理员权限
pub async fn get_processing_jobs(
    _admin: AdminUser,
    db: web::Data<Database>,
    query: web::Query<Value>,
) -> Result<HttpResponse> {
    let processing_jobs_collection = db.collection::<ProcessingJob>("processing_jobs");
    
    let page = query.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
    let limit = query.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
    let status = query.get("status").and_then(|v| v.as_str());
    
    let skip = (page - 1) * limit;
    
    let mut filter = doc! {};
    if let Some(status_str) = status {
        filter.insert("status", status_str);
    }
    
    let mut options = mongodb::options::FindOptions::default();
    options.skip = Some(skip as u64);
    options.limit = Some(limit as i64);
    
    let cursor = match processing_jobs_collection
        .find(filter.clone(), options)
        .await
    {
        Ok(cursor) => cursor,
        Err(e) => {
            eprintln!("Failed to query processing jobs: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to query processing jobs".to_string())));
        }
    };
    
    let jobs: Vec<ProcessingJob> = match cursor.try_collect().await {
        Ok(jobs) => jobs,
        Err(e) => {
            eprintln!("Failed to collect processing jobs: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to collect processing jobs".to_string())));
        }
    };
    
    let total = match processing_jobs_collection
        .count_documents(filter.clone(), None)
        .await
    {
        Ok(total) => total,
        Err(e) => {
            eprintln!("Failed to count processing jobs: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to count processing jobs".to_string())));
        }
    };
    
    let response = serde_json::json!({
        "jobs": jobs,
        "page": page,
        "limit": limit,
        "total": total,
        "pages": (total + limit - 1) / limit
    });
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// 获取批量处理任务列表 - 需要管理员权限
pub async fn get_batch_processing_jobs(
    _admin: AdminUser,
    db: web::Data<Database>,
    query: web::Query<Value>,
) -> Result<HttpResponse> {
    let batch_processing_jobs_collection = db.collection::<BatchProcessingJob>("batch_processing_jobs");
    
    let page = query.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
    let limit = query.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
    let status = query.get("status").and_then(|v| v.as_str());
    
    let skip = (page - 1) * limit;
    
    let mut filter = doc! {};
    if let Some(status_str) = status {
        filter.insert("status", status_str);
    }
    
    let mut options = mongodb::options::FindOptions::default();
    options.skip = Some(skip as u64);
    options.limit = Some(limit as i64);
    
    let cursor = match batch_processing_jobs_collection
        .find(filter.clone(), options)
        .await
    {
        Ok(cursor) => cursor,
        Err(e) => {
            eprintln!("Failed to query batch processing jobs: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to query batch processing jobs".to_string())));
        }
    };
    
    let jobs: Vec<BatchProcessingJob> = match cursor.try_collect().await {
        Ok(jobs) => jobs,
        Err(e) => {
            eprintln!("Failed to collect batch processing jobs: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to collect batch processing jobs".to_string())));
        }
    };
    
    let total = match batch_processing_jobs_collection
        .count_documents(filter.clone(), None)
        .await
    {
        Ok(total) => total,
        Err(e) => {
            eprintln!("Failed to count batch processing jobs: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to count batch processing jobs".to_string())));
        }
    };
    
    let response = serde_json::json!({
        "jobs": jobs,
        "page": page,
        "limit": limit,
        "total": total,
        "pages": (total + limit - 1) / limit
    });
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// 获取webhook通知列表 - 需要管理员权限
pub async fn get_webhook_notifications(
    _admin: AdminUser,
    db: web::Data<Database>,
    query: web::Query<Value>,
) -> Result<HttpResponse> {
    let webhook_notifications_collection = db.collection::<WebhookNotification>("webhook_notifications");
    
    let page = query.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
    let limit = query.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
    
    let skip = (page - 1) * limit;
    
    let filter = doc! {};
    
    let mut options = mongodb::options::FindOptions::default();
    options.skip = Some(skip as u64);
    options.limit = Some(limit as i64);
    options.sort = Some(doc! { "timestamp": -1 });
    
    let cursor = match webhook_notifications_collection
        .find(filter.clone(), options)
        .await
    {
        Ok(cursor) => cursor,
        Err(e) => {
            eprintln!("Failed to query webhook notifications: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to query webhook notifications".to_string())));
        }
    };
    
    let notifications: Vec<WebhookNotification> = match cursor.try_collect().await {
        Ok(notifications) => notifications,
        Err(e) => {
            eprintln!("Failed to collect webhook notifications: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to collect webhook notifications".to_string())));
        }
    };
    
    let total = match webhook_notifications_collection
        .count_documents(filter.clone(), None)
        .await
    {
        Ok(total) => total,
        Err(e) => {
            eprintln!("Failed to count webhook notifications: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to count webhook notifications".to_string())));
        }
    };
    
    let response = serde_json::json!({
        "notifications": notifications,
        "page": page,
        "limit": limit,
        "total": total,
        "pages": (total + limit - 1) / limit
    });
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}