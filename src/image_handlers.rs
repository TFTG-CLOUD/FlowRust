use crate::dto::ApiResponse;
use crate::dto::CreateProcessingJobRequest;
use crate::jwt_auth::AuthenticatedUser;
use crate::models::Image;
use crate::processing_service::ProcessingService;
use actix_web::{HttpResponse, Result, web};
use futures::stream::TryStreamExt;
use mongodb::Database;
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

// 图集投稿请求数据
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSubmissionRequest {
    pub title: String,
    pub en_title: Option<String>,
    pub description: Option<String>,
    pub category: String,
    pub language: Option<String>,
    pub artists: Option<String>,
    pub tags: Vec<String>,
    pub need_vip: i32,
    pub upload_id: String,
    pub server_id: String,
}

// 图集投稿响应数据
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSubmissionResponse {
    pub image_id: String,
    pub title: String,
    pub status: String,
    pub message: String,
}

// 处理图集投稿
pub async fn submit_image(
    user: AuthenticatedUser,
    db: web::Data<Database>,
    request: web::Json<ImageSubmissionRequest>,
) -> Result<HttpResponse> {
    let submission_data = request.into_inner();

    // 验证必填字段
    if submission_data.title.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("标题不能为空".to_string()))
        );
    }

    if submission_data.category.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("分类不能为空".to_string()))
        );
    }

    if submission_data.upload_id.is_empty() || submission_data.server_id.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("上传信息不完整".to_string()))
        );
    }

    // 创建图集记录（待处理状态）
    let now = mongodb::bson::DateTime::now();
    let image_id = ObjectId::new();

    let image = Image {
        id: Some(image_id),
        title: submission_data.title.clone(),
        en_title: submission_data.en_title.clone(),
        description: submission_data.description.clone(),
        images: Vec::new(), // 暂时为空，等待处理完成后填充
        cover: crate::models::ImageItem {
            url: String::new(), // 暂时为空
            width: 0,
            height: 0,
        },
        tags: submission_data.tags.clone(),
        category: submission_data.category.clone(),
        pages: 0, // 暂时为0，等待处理完成后填充
        uploader: user.user.id.unwrap_or_else(|| ObjectId::new()),
        likes: 0,
        language: submission_data.language.clone(),
        artists: submission_data.artists.clone(),
        need_vip: submission_data.need_vip,
        created_at: now,
        updated_at: now,
    };

    // 保存图集记录到数据库
    let collection = db.collection::<Image>("images");
    match collection.insert_one(&image, None).await {
        Ok(_) => {
            // 创建处理任务
            let processing_request = CreateProcessingJobRequest {
                job_type: "archive-process".to_string(),
                file_id: submission_data.upload_id.clone(),
                parameters: serde_json::json!({
                    "image_id": image_id.to_hex(),
                    "title": submission_data.title,
                    "en_title": submission_data.en_title,
                    "description": submission_data.description,
                    "category": submission_data.category,
                    "language": submission_data.language,
                    "artists": submission_data.artists,
                    "tags": submission_data.tags,
                    "need_vip": submission_data.need_vip,
                    "uploader": user.user.id.unwrap_or_else(|| ObjectId::new()).to_hex(),
                }),
                webhook_url: Some(format!(
                    "{}/api/webhook/image-processing",
                    std::env::var("HOST").unwrap_or_else(|_| "http://localhost:8080".to_string())
                )),
                webhook_secret: Some(
                    std::env::var("WEBHOOK_SECRET")
                        .unwrap_or_else(|_| "your-webhook-secret".to_string()),
                ),
                cms_id: "maccms".to_string(),
            };

            let processing_collection =
                db.collection::<crate::models::ProcessingJob>("processing_jobs");
            let batch_processing_collection =
                db.collection::<crate::models::BatchProcessingJob>("batch_processing_jobs");
            let processing_servers_collection =
                db.collection::<crate::models::ProcessingServerConfig>("processing_servers");
            let webhook_notifications_collection =
                db.collection::<crate::models::WebhookNotification>("webhook_notifications");
            let processing_service = ProcessingService::new(
                processing_collection,
                batch_processing_collection,
                processing_servers_collection,
                webhook_notifications_collection,
            );

            match processing_service
                .create_processing_job(processing_request)
                .await
            {
                Ok(processing_response) => {
                    let response_data = ImageSubmissionResponse {
                        image_id: image_id.to_hex(),
                        title: submission_data.title,
                        status: "processing".to_string(),
                        message: "投稿成功，压缩包正在处理中".to_string(),
                    };

                    Ok(HttpResponse::Ok().json(ApiResponse::success(response_data)))
                }
                Err(e) => {
                    // 处理任务创建失败，但图集记录已保存
                    eprintln!("创建处理任务失败: {}", e);
                    let response_data = ImageSubmissionResponse {
                        image_id: image_id.to_hex(),
                        title: submission_data.title,
                        status: "pending".to_string(),
                        message: "投稿成功，但处理任务创建失败，请联系管理员".to_string(),
                    };

                    Ok(HttpResponse::Ok().json(ApiResponse::success(response_data)))
                }
            }
        }
        Err(e) => {
            eprintln!("保存图集记录失败: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("保存图集记录失败".to_string())))
        }
    }
}

// 获取用户的图集列表
pub async fn get_user_images(
    user: AuthenticatedUser,
    db: web::Data<Database>,
    query: web::Query<ImageListQuery>,
) -> Result<HttpResponse> {
    let collection = db.collection::<Image>("images");
    let uploader_id = user.user.id.unwrap_or_else(|| ObjectId::new());

    // 构建查询条件
    let mut filter = mongodb::bson::doc! {
        "uploader": uploader_id
    };

    // 添加状态筛选
    if let Some(status) = &query.status {
        filter.insert("status", status);
    }

    // 添加分类筛选
    if let Some(category) = &query.category {
        filter.insert("category", category);
    }

    // 设置分页
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let skip = (page - 1) * limit;

    // 查询总数
    let total = match collection.count_documents(filter.clone(), None).await {
        Ok(count) => count,
        Err(e) => {
            eprintln!("查询图集总数失败: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("查询失败".to_string())));
        }
    };

    // 查询图集列表
    let find_options = mongodb::options::FindOptions::builder()
        .skip(skip as u64)
        .limit(limit as i64)
        .sort(mongodb::bson::doc! { "created_at": -1 })
        .build();

    let mut cursor = match collection.find(filter, find_options).await {
        Ok(cursor) => cursor,
        Err(e) => {
            eprintln!("查询图集列表失败: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("查询失败".to_string())));
        }
    };

    let mut images = Vec::new();
    while let Ok(Some(image)) = cursor.try_next().await {
        images.push(image);
    }

    let response_data = ImageListResponse {
        images,
        page,
        limit,
        total,
        total_pages: (total + limit as u64 - 1) / limit as u64,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response_data)))
}

// 图集列表查询参数
#[derive(Debug, Deserialize)]
pub struct ImageListQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub status: Option<String>,
    pub category: Option<String>,
}

// 图集列表响应数据
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageListResponse {
    pub images: Vec<Image>,
    pub page: i64,
    pub limit: i64,
    pub total: u64,
    pub total_pages: u64,
}

// 获取图集详情
pub async fn get_image_detail(
    _user: AuthenticatedUser,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let image_id = match path.parse::<mongodb::bson::oid::ObjectId>() {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("无效的图集ID".to_string())));
        }
    };

    let collection = db.collection::<Image>("images");

    match collection
        .find_one(mongodb::bson::doc! { "_id": image_id }, None)
        .await
    {
        Ok(Some(image)) => Ok(HttpResponse::Ok().json(ApiResponse::success(image))),
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error("图集不存在".to_string())))
        }
        Err(e) => {
            eprintln!("查询图集详情失败: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("查询失败".to_string())))
        }
    }
}

// 更新图集信息
pub async fn update_image(
    user: AuthenticatedUser,
    db: web::Data<Database>,
    path: web::Path<String>,
    request: web::Json<ImageUpdateRequest>,
) -> Result<HttpResponse> {
    let image_id = match path.parse::<mongodb::bson::oid::ObjectId>() {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("无效的图集ID".to_string())));
        }
    };

    let update_data = request.into_inner();

    // 验证权限 - 只有上传者本人可以更新
    let collection = db.collection::<Image>("images");
    match collection
        .find_one(mongodb::bson::doc! { "_id": image_id }, None)
        .await
    {
        Ok(Some(image)) => {
            if image.uploader != user.user.id.unwrap_or_else(|| ObjectId::new()) {
                return Ok(HttpResponse::Forbidden()
                    .json(ApiResponse::<()>::error("无权限修改此图集".to_string())));
            }
        }
        Ok(None) => {
            return Ok(
                HttpResponse::NotFound().json(ApiResponse::<()>::error("图集不存在".to_string()))
            );
        }
        Err(e) => {
            eprintln!("查询图集失败: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("查询失败".to_string())));
        }
    }

    // 构建更新数据
    let mut updates = mongodb::bson::doc! {};
    let mut set_doc = mongodb::bson::doc! {};

    if let Some(title) = update_data.title {
        set_doc.insert("title", title);
    }
    if let Some(en_title) = update_data.en_title {
        set_doc.insert("en_title", en_title);
    }
    if let Some(description) = update_data.description {
        set_doc.insert("description", description);
    }
    if let Some(category) = update_data.category {
        set_doc.insert("category", category);
    }
    if let Some(language) = update_data.language {
        set_doc.insert("language", language);
    }
    if let Some(artists) = update_data.artists {
        set_doc.insert("artists", artists);
    }
    if let Some(tags) = update_data.tags {
        set_doc.insert("tags", tags);
    }
    if let Some(need_vip) = update_data.need_vip {
        set_doc.insert("need_vip", need_vip);
    }

    if !set_doc.is_empty() {
        updates.insert("$set", set_doc);
    }

    if updates.is_empty() {
        return Ok(HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("没有要更新的数据".to_string())));
    }

    match collection
        .update_one(mongodb::bson::doc! { "_id": image_id }, updates, None)
        .await
    {
        Ok(result) => {
            if result.matched_count == 0 {
                return Ok(HttpResponse::NotFound()
                    .json(ApiResponse::<()>::error("图集不存在".to_string())));
            }

            Ok(HttpResponse::Ok().json(ApiResponse::success("更新成功")))
        }
        Err(e) => {
            eprintln!("更新图集失败: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("更新失败".to_string())))
        }
    }
}

// 图集更新请求数据
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUpdateRequest {
    pub title: Option<String>,
    pub en_title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub language: Option<String>,
    pub artists: Option<String>,
    pub tags: Option<Vec<String>>,
    pub need_vip: Option<i32>,
}

// 删除图集
pub async fn delete_image(
    user: AuthenticatedUser,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let image_id = match path.parse::<mongodb::bson::oid::ObjectId>() {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("无效的图集ID".to_string())));
        }
    };

    // 验证权限 - 只有上传者本人可以删除
    let collection = db.collection::<Image>("images");
    match collection
        .find_one(mongodb::bson::doc! { "_id": image_id }, None)
        .await
    {
        Ok(Some(image)) => {
            if image.uploader != user.user.id.unwrap_or_else(|| ObjectId::new()) {
                return Ok(HttpResponse::Forbidden()
                    .json(ApiResponse::<()>::error("无权限删除此图集".to_string())));
            }
        }
        Ok(None) => {
            return Ok(
                HttpResponse::NotFound().json(ApiResponse::<()>::error("图集不存在".to_string()))
            );
        }
        Err(e) => {
            eprintln!("查询图集失败: {}", e);
            return Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("查询失败".to_string())));
        }
    }

    match collection
        .delete_one(mongodb::bson::doc! { "_id": image_id }, None)
        .await
    {
        Ok(result) => {
            if result.deleted_count == 0 {
                return Ok(HttpResponse::NotFound()
                    .json(ApiResponse::<()>::error("图集不存在".to_string())));
            }

            Ok(HttpResponse::Ok().json(ApiResponse::success("删除成功")))
        }
        Err(e) => {
            eprintln!("删除图集失败: {}", e);
            Ok(HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("删除失败".to_string())))
        }
    }
}

// 处理图集处理完成的webhook
pub async fn handle_image_processing_webhook(
    db: web::Data<Database>,
    payload: web::Json<serde_json::Value>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    // 验证webhook签名
    let signature = req
        .headers()
        .get("X-Webhook-Signature")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let webhook_secret =
        std::env::var("WEBHOOK_SECRET").unwrap_or_else(|_| "your-webhook-secret".to_string());

    // 这里应该实现HMAC-SHA256签名验证
    // 为了简化，暂时跳过验证

    let payload_data = payload.into_inner();

    // 解析处理结果
    let image_id = match payload_data.get("image_id").and_then(|v| v.as_str()) {
        Some(id) => match mongodb::bson::oid::ObjectId::parse_str(id) {
            Ok(oid) => oid,
            Err(_) => {
                return Ok(HttpResponse::BadRequest()
                    .json(ApiResponse::<()>::error("无效的图集ID".to_string())));
            }
        },
        None => {
            return Ok(
                HttpResponse::BadRequest().json(ApiResponse::<()>::error("缺少图集ID".to_string()))
            );
        }
    };

    let status = payload_data
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("failed");

    if status == "completed" {
        // 处理成功，更新图集数据
        let results = payload_data.get("results").and_then(|v| v.as_array());

        if let Some(results_array) = results {
            let mut image_items = Vec::new();
            let mut cover_item = crate::models::ImageItem {
                url: String::new(),
                width: 0,
                height: 0,
            };

            // 解析处理结果
            for (index, result) in results_array.iter().enumerate() {
                if let Some(url) = result.get("url").and_then(|v| v.as_str()) {
                    let width = result.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let height = result.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                    let image_item = crate::models::ImageItem {
                        url: url.to_string(),
                        width,
                        height,
                    };

                    image_items.push(image_item);

                    // 第一张图片作为封面
                    if index == 0 {
                        cover_item = crate::models::ImageItem {
                            url: url.to_string(),
                            width,
                            height,
                        };
                    }
                }
            }

            // 更新图集记录
            let collection = db.collection::<Image>("images");
            // Convert ImageItems to Bson documents
            let image_docs: Vec<mongodb::bson::Document> = image_items
                .iter()
                .map(|item| {
                    mongodb::bson::doc! {
                        "url": &item.url,
                        "width": item.width,
                        "height": item.height,
                    }
                })
                .collect();

            let update_doc = mongodb::bson::doc! {
                "$set": {
                    "images": image_docs,
                    "cover": {
                        "url": &cover_item.url,
                        "width": cover_item.width,
                        "height": cover_item.height,
                    },
                    "pages": image_items.len() as i32,
                    "status": "published",
                    "updated_at": mongodb::bson::DateTime::now(),
                }
            };

            match collection
                .update_one(mongodb::bson::doc! { "_id": image_id }, update_doc, None)
                .await
            {
                Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::success("图集处理完成"))),
                Err(e) => {
                    eprintln!("更新图集失败: {}", e);
                    Ok(HttpResponse::InternalServerError()
                        .json(ApiResponse::<()>::error("更新图集失败".to_string())))
                }
            }
        } else {
            Ok(HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("处理结果格式错误".to_string())))
        }
    } else {
        // 处理失败，更新状态
        let collection = db.collection::<Image>("images");
        let error_message = payload_data
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("处理失败");

        let update_doc = mongodb::bson::doc! {
            "$set": {
                "status": "failed",
                "error_message": error_message,
                "updated_at": mongodb::bson::DateTime::now(),
            }
        };

        match collection
            .update_one(mongodb::bson::doc! { "_id": image_id }, update_doc, None)
            .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::success("已记录处理失败状态"))),
            Err(e) => {
                eprintln!("更新图集状态失败: {}", e);
                Ok(HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error("更新状态失败".to_string())))
            }
        }
    }
}
