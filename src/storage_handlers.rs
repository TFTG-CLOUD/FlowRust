use crate::dto::ApiResponse;
use crate::jwt_auth::AdminUser;
use crate::jwt_auth::AuthenticatedUser;
use crate::models::{
    ChunkUploadInfo, PresignedUploadResponse, StorageServer, UploadStatusResponse,
};
use crate::storage_service::{
    CreateStorageServerRequest, GeneratePresignedUrlRequest, StorageService,
    UpdateStorageServerRequest,
};
use actix_web::{HttpResponse, Result, web};
use mongodb::Database;
use rand::seq::SliceRandom;
use tera::Context;

// 获取单个
pub async fn get_storage_server(
    _admin: AdminUser,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service.get_server(&server_id).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 获取分布式储存服务器详情 - 需要用户登录权限
pub async fn get_user_storage_server(
    _user: AuthenticatedUser,
    db: web::Data<Database>,
) -> Result<HttpResponse> {
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service.get_servers().await {
        Ok(response) => {
            let servers = response.data.unwrap_or_default();
            if let Some(random_server) = servers.choose(&mut rand::thread_rng()) {
                let server_id = random_server.id.unwrap_or_default().to_string();
                Ok(HttpResponse::Ok().json(ApiResponse::success(server_id)))
            } else {
                Ok(HttpResponse::Ok().json(ApiResponse::<String>::error(
                    "No servers available".to_string(),
                )))
            }
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 获取分布式储存服务器列表 - 需要管理员权限
pub async fn get_storage_servers(
    _admin: AdminUser,
    db: web::Data<Database>,
) -> Result<HttpResponse> {
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service.get_servers().await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 创建分布式储存服务器 - 需要管理员权限
pub async fn create_storage_server(
    _admin: AdminUser,
    db: web::Data<Database>,
    request: web::Json<CreateStorageServerRequest>,
) -> Result<HttpResponse> {
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service.create_server(request.into_inner()).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 更新分布式储存服务器 - 需要管理员权限
pub async fn update_storage_server(
    _admin: AdminUser,
    db: web::Data<Database>,
    path: web::Path<String>,
    request: web::Json<UpdateStorageServerRequest>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service
        .update_server(&server_id, request.into_inner())
        .await
    {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 删除分布式储存服务器 - 需要管理员权限
pub async fn delete_storage_server(
    _admin: AdminUser,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service.delete_server(&server_id).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 测试服务器连接 - 需要管理员权限
pub async fn test_server_connection(
    _admin: AdminUser,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service.test_server_connection(&server_id).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 生成单文件上传预签名URL - 需要用户登录
pub async fn generate_single_upload_url(
    _user: AuthenticatedUser,
    db: web::Data<Database>,
    path: web::Path<String>,
    request: web::Json<GeneratePresignedUrlRequest>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service
        .generate_single_upload_url(&server_id, request.into_inner())
        .await
    {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 生成分片上传预签名URL - 需要用户登录
pub async fn generate_chunk_upload_url(
    _user: AuthenticatedUser,
    db: web::Data<Database>,
    path: web::Path<String>,
    request: web::Json<GeneratePresignedUrlRequest>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service
        .generate_chunk_upload_url(&server_id, request.into_inner())
        .await
    {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 完成分片上传 - 需要用户登录
pub async fn complete_chunk_upload(
    _user: AuthenticatedUser,
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (server_id, upload_id) = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service.complete_chunk_upload(&server_id, &upload_id).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 生成压缩包上传预签名URL - 需要用户登录
pub async fn generate_archive_upload_url(
    _user: AuthenticatedUser,
    db: web::Data<Database>,
    path: web::Path<String>,
    request: web::Json<GeneratePresignedUrlRequest>,
) -> Result<HttpResponse> {
    let server_id = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    match service
        .generate_archive_upload_url(&server_id, request.into_inner())
        .await
    {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}

// 分布式储存管理页面 - 需要管理员权限
pub async fn admin_storage_page(db: web::Data<Database>) -> Result<HttpResponse> {
    let collection = db.collection::<StorageServer>("storage_servers");
    let service = StorageService::new(collection);

    let servers = match service.get_servers().await {
        Ok(response) => response.data.unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let mut context = Context::new();
    context.insert("servers", &servers);
    context.insert("page_title", "分布式储存管理");
    context.insert("SITENAME", "maccms-rust");

    match crate::template::TERA.render("admin/storage.html", &context) {
        Ok(html) => Ok(HttpResponse::Ok().content_type("text/html").body(html)),
        Err(e) => {
            eprintln!("Failed to render template: {}", e);
            Ok(HttpResponse::InternalServerError().body("Template error"))
        }
    }
}

// 获取上传状态 - 需要管理员权限
pub async fn get_upload_status(
    _admin: AdminUser,
    db: web::Data<Database>,
    path: web::Path<(String, String)>, // (server_id, upload_id)
) -> Result<HttpResponse> {
    let (server_id, upload_id) = path.into_inner();
    let collection = db.collection::<StorageServer>("storage_servers");

    let service = StorageService::new(collection);

    match service.get_upload_status(&server_id, &upload_id).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))),
    }
}
