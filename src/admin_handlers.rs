
use crate::jwt_auth::AdminUser;
use actix_web::{web, HttpResponse, Responder, Result, FromRequest, HttpMessage, HttpRequest};
use bcrypt;
use chrono;
use futures::stream::TryStreamExt;
use mongodb::{
    bson::doc,
    options::{FindOneOptions, FindOptions},
    Database,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid;

use crate::dto::{
    CardInfo, CardListResponse, CardPageParams, DeleteCardRequest, GenerateCardRequest,
    GenerateCardResponse, SearchCardRequest, UserInfo, UserAdminInfo, UserListResponse,
    CreateUserRequest, UpdateUserRequest, DeleteUserRequest, SearchUserRequest, UserPageParams,
};
use crate::index_manager::IndexManager;
use crate::models::{Binding, Card, Collection, Config, Type, User, Vod};
use crate::scheduled_task::ScheduledTaskManager;
use crate::template::TERA;


// --- DTOs for Admin API ---
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeRequest {
    pub type_name: String,
    pub type_pid: i32,
    pub type_en: Option<String>,
    pub type_sort: Option<i32>,
    pub type_status: Option<i32>,
    pub type_mid: Option<i32>,
    pub type_key: Option<String>,
    pub type_des: Option<String>,
    pub type_title: Option<String>,
    pub subarea: Option<String>,
    pub subyear: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BindingRequest {
    pub source_flag: String, // e.g., "my_api_source"
    pub external_id: String, // e.g., "123"
    pub local_type_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub config_key: String,
    pub config_value: String,
    pub config_desc: Option<String>,
    pub config_type: String,
    pub config_group: Option<String>,
    pub config_sort: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionRequest {
    pub collect_name: String,
    pub collect_url: String,
    pub collect_type: i32,
    pub collect_mid: i32,
    pub collect_appid: String,
    pub collect_appkey: String,
    pub collect_param: String,
    pub collect_filter: String,
    #[serde(default)]
    pub collect_filter_from: String,
    pub collect_opt: i32,
    pub collect_sync_pic_opt: i32,
    pub collect_remove_ad: i32,
    pub collect_convert_webp: i32,
    pub collect_download_retry: i32,
    pub collect_status: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VodRequest {
    pub vod_name: String,
    pub type_id: i32,
    pub vod_status: i32,
    pub vod_class: Option<String>,
    pub vod_pic: Option<String>,
    pub vod_actor: Option<String>,
    pub vod_director: Option<String>,
    pub vod_remarks: Option<String>,
    pub vod_area: Option<String>,
    pub vod_lang: Option<String>,
    pub vod_year: Option<String>,
    pub vod_content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<String>,
}

// --- Category Management API ---

// GET /api/admin/types
pub async fn get_types(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {
    let collection = db.collection::<Type>("types");
    let find_options = FindOptions::builder().sort(doc! {"type_sort": 1}).build();

    match collection.find(None, find_options).await {
        Ok(cursor) => {
            let types: Vec<Type> = cursor.try_collect().await.unwrap_or_else(|_| vec![]);
            HttpResponse::Ok().json(types)
        }
        Err(e) => {
            eprintln!("Failed to fetch types: {}", e);
            HttpResponse::InternalServerError().body("Failed to fetch types")
        }
    }
}

// --- Collection Management API ---

// GET /api/admin/collections
pub async fn get_collections(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {
    let collection = db.collection::<Collection>("collections");
    let find_options = FindOptions::builder().sort(doc! {"created_at": -1}).build();

    match collection.find(None, find_options).await {
        Ok(cursor) => {
            let collections: Vec<Collection> =
                cursor.try_collect().await.unwrap_or_else(|_| vec![]);
            HttpResponse::Ok().json(collections)
        }
        Err(e) => {
            eprintln!("Failed to fetch collections: {}", e);
            HttpResponse::InternalServerError().body("Failed to fetch collections")
        }
    }
}

// POST /api/admin/collections
pub async fn create_collection(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    collection_req: web::Json<CollectionRequest>,
) -> impl Responder {
    let collection = db.collection::<Collection>("collections");

    let new_collection = Collection {
        id: None,
        collect_name: collection_req.collect_name.clone(),
        collect_url: collection_req.collect_url.clone(),
        collect_type: collection_req.collect_type,
        collect_mid: collection_req.collect_mid,
        collect_appid: collection_req.collect_appid.clone(),
        collect_appkey: collection_req.collect_appkey.clone(),
        collect_param: collection_req.collect_param.clone(),
        collect_filter: collection_req.collect_filter.clone(),
        collect_filter_from: collection_req.collect_filter_from.clone(),
        collect_opt: collection_req.collect_opt,
        collect_sync_pic_opt: collection_req.collect_sync_pic_opt,
        collect_remove_ad: collection_req.collect_remove_ad,
        collect_convert_webp: collection_req.collect_convert_webp,
        collect_download_retry: collection_req.collect_download_retry,
        collect_status: collection_req.collect_status,
        created_at: mongodb::bson::DateTime::now(),
        updated_at: mongodb::bson::DateTime::now(),
    };

    match collection.insert_one(new_collection, None).await {
        Ok(_) => {
            HttpResponse::Created().json(json!({"success": true, "message": "Collection created"}))
        }
        Err(e) => {
            eprintln!("Failed to create collection: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to create collection"}))
        }
    }
}

// POST /api/admin/collections/{id}/collect
pub async fn start_collection_collect(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
    collect_req: Option<web::Json<CollectRequest>>,
) -> impl Responder {

    let collection_id = match mongodb::bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"success": false, "message": "Invalid collection ID"}))
        }
    };

    // 获取采集源配置
    let collection = match db
        .collection::<Collection>("collections")
        .find_one(doc! {"_id": collection_id}, None)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(json!({"success": false, "message": "Collection not found"}))
        }
        Err(e) => {
            eprintln!("Failed to fetch collection: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to fetch collection"}));
        }
    };

    // 检查是否有绑定的分类
    let bindings_collection = db.collection::<Binding>("bindings");
    let bindings_count = match bindings_collection
        .count_documents(doc! {"source_flag": &collection.collect_name}, None)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Failed to count bindings: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to check bindings"}));
        }
    };

    // 如果没有绑定分类，返回错误
    if bindings_count == 0 {
        return HttpResponse::Ok().json(json!({
            "success": false,
            "message": "请先绑定分类",
            "needs_binding": true
        }));
    }

    // 解析hours参数
    let hours_text = collect_req
        .as_ref()
        .and_then(|req| req.hours)
        .map(|h| format!("采集任务已启动 ({}小时内)", h))
        .unwrap_or_else(|| "采集任务已启动 (全部数据)".to_string());

    let hours_param = collect_req
        .as_ref()
        .and_then(|req| req.hours)
        .map(|h| h.to_string());

    // 生成任务ID
    let task_id = uuid::Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();

    // 启动后台采集任务
    tokio::spawn(async move {
        if let Err(e) = crate::collect_handlers::start_batch_collect(
            &db,
            collection,
            hours_param,
            task_id_clone,
        )
        .await
        {
            eprintln!("Batch collect failed: {}", e);
        }
    });

    HttpResponse::Ok().json(json!({
        "success": true,
        "message": hours_text,
        "task_id": task_id,
        "total_pages": 1 // 将在实际采集中更新
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectRequest {
    pub hours: Option<i32>,
}

// PUT /api/admin/collections/{id}
pub async fn update_collection(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
    collection_req: web::Json<CollectionRequest>,
) -> impl Responder {
    let collection = db.collection::<Collection>("collections");
    let collection_id = match mongodb::bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid collection ID"),
    };

    let update_doc = doc! {
        "$set": {
            "collect_name": &collection_req.collect_name,
            "collect_url": &collection_req.collect_url,
            "collect_type": collection_req.collect_type,
            "collect_mid": collection_req.collect_mid,
            "collect_appid": &collection_req.collect_appid,
            "collect_appkey": &collection_req.collect_appkey,
            "collect_param": &collection_req.collect_param,
            "collect_filter": &collection_req.collect_filter,
            "collect_filter_from": &collection_req.collect_filter_from,
            "collect_opt": collection_req.collect_opt,
            "collect_sync_pic_opt": collection_req.collect_sync_pic_opt,
            "collect_remove_ad": collection_req.collect_remove_ad,
            "collect_convert_webp": collection_req.collect_convert_webp,
            "collect_download_retry": collection_req.collect_download_retry,
            "collect_status": collection_req.collect_status,
            "updated_at": mongodb::bson::DateTime::now(),
        }
    };

    match collection
        .update_one(doc! {"_id": collection_id}, update_doc, None)
        .await
    {
        Ok(result) => {
            if result.matched_count > 0 {
                HttpResponse::Ok()
                    .json(json!({"success": true, "message": "Collection updated successfully"}))
            } else {
                HttpResponse::NotFound()
                    .json(json!({"success": false, "message": "Collection not found"}))
            }
        }
        Err(e) => {
            eprintln!("Failed to update collection: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to update collection"}))
        }
    }
}

// GET /api/admin/collect/progress/{task_id}
pub async fn get_collect_progress(admin: crate::jwt_auth::AdminUser, path: web::Path<String>) -> impl Responder {

    let task_id = path.into_inner();

    // 从内存中获取任务进度（简化版本）
    let progress = crate::collect_handlers::get_task_progress(&task_id)
        .await
        .unwrap_or(crate::collect_handlers::CollectProgress {
            status: "not_found".to_string(),
            current_page: 0,
            total_pages: 0,
            success: 0,
            failed: 0,
            log: "任务不存在".to_string(),
        });

    HttpResponse::Ok().json(json!({
        "success": true,
        "progress": progress
    }))
}

// GET /api/admin/collect/running-tasks
pub async fn get_running_tasks(admin: crate::jwt_auth::AdminUser) -> impl Responder {

    // 获取所有运行中的任务（从collect_handlers中的全局存储获取）
    let tasks = crate::collect_handlers::get_all_running_tasks().await;

    HttpResponse::Ok().json(json!({
        "success": true,
        "tasks": tasks
    }))
}

// POST /api/admin/collect/stop/{task_id}
pub async fn stop_collect_task(admin: crate::jwt_auth::AdminUser, path: web::Path<String>) -> impl Responder {

    let task_id = path.into_inner();

    // 调用collect_handlers中的停止任务函数
    let stopped = crate::collect_handlers::stop_task(&task_id).await;

    if stopped {
        HttpResponse::Ok().json(json!({
            "success": true,
            "message": "任务已成功停止"
        }))
    } else {
        HttpResponse::NotFound().json(json!({
            "success": false,
            "message": "任务不存在或已经停止"
        }))
    }
}

// DELETE /api/admin/collections/{id}
pub async fn delete_collection(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
) -> impl Responder {
    let collection = db.collection::<Collection>("collections");
    let collection_id = match mongodb::bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid collection ID"),
    };

    match collection
        .delete_one(doc! {"_id": collection_id}, None)
        .await
    {
        Ok(result) => {
            if result.deleted_count > 0 {
                HttpResponse::Ok()
                    .json(json!({"success": true, "message": "Collection deleted successfully"}))
            } else {
                HttpResponse::NotFound()
                    .json(json!({"success": false, "message": "Collection not found"}))
            }
        }
        Err(e) => {
            eprintln!("Failed to delete collection: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to delete collection"}))
        }
    }
}

// --- Video Management API ---

#[derive(Debug, Deserialize)]
pub struct VodsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub type_id: Option<i32>,
    pub status: Option<i32>,
    pub search: Option<String>,
}

// GET /api/admin/vods
pub async fn get_vods_admin(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    query: web::Query<VodsQuery>,
) -> impl Responder {

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let skip = (page - 1) * limit;

    // 构建查询条件
    let mut filter_doc = doc! {};

    // 分类筛选
    if let Some(type_id) = query.type_id {
        filter_doc.insert("type_id", type_id);
    }

    // 状态筛选
    if let Some(status) = query.status {
        filter_doc.insert("vod_status", status);
    }

    // 搜索功能
    if let Some(search_term) = &query.search {
        if !search_term.trim().is_empty() {
            filter_doc.insert("vod_name", doc! {"$regex": search_term, "$options": "i"});
        }
    }

    let collection = db.collection::<Vod>("vods");
    let find_options = FindOptions::builder()
        .sort(doc! {"vod_pubdate": -1})
        .skip(skip as u64)
        .limit(limit as i64)
        .build();

    // 获取总数
    let total = match collection.count_documents(filter_doc.clone(), None).await {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Failed to count vods: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "code": 0,
                "msg": "Failed to count videos",
                "page": page,
                "limit": limit,
                "total": 0,
                "videos": []
            }));
        }
    };

    // 获取分页数据
    match collection.find(filter_doc, find_options).await {
        Ok(cursor) => {
            let vods: Vec<Vod> = cursor.try_collect().await.unwrap_or_else(|_| vec![]);

            HttpResponse::Ok().json(json!({
                "code": 1,
                "msg": "success",
                "page": page,
                "limit": limit,
                "total": total,
                "videos": vods
            }))
        }
        Err(e) => {
            eprintln!("Failed to fetch vods: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "code": 0,
                "msg": "Failed to fetch videos",
                "page": page,
                "limit": limit,
                "total": 0,
                "videos": []
            }))
        }
    }
}

// POST /api/admin/vods
pub async fn create_vod(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    vod_req: web::Json<VodRequest>,
) -> impl Responder {
    let collection = db.collection::<Vod>("vods");

    let new_vod = Vod {
        id: None,
        vod_name: vod_req.vod_name.clone(),
        type_id: vod_req.type_id,
        vod_status: vod_req.vod_status,
        vod_class: vod_req.vod_class.clone(),
        vod_pic: vod_req.vod_pic.clone(),
        vod_actor: vod_req.vod_actor.clone(),
        vod_director: vod_req.vod_director.clone(),
        vod_remarks: vod_req.vod_remarks.clone(),
        vod_pubdate: mongodb::bson::DateTime::now(),
        vod_area: vod_req.vod_area.clone(),
        vod_lang: vod_req.vod_lang.clone(),
        vod_year: vod_req.vod_year.clone(),
        vod_content: vod_req.vod_content.clone(),
        vod_hits: Some(0),
        vod_hits_day: Some(0),
        vod_hits_week: Some(0),
        vod_hits_month: Some(0),
        vod_score: Some("0.0".to_string()),
        need_vip: 0,
        vod_play_urls: vec![], // Empty initially
    };

    match collection.insert_one(new_vod, None).await {
        Ok(_) => HttpResponse::Created().json(json!({
            "success": true,
            "message": "Video created successfully"
        })),
        Err(e) => {
            eprintln!("Failed to create video: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "Failed to create video"
            }))
        }
    }
}

// PUT /api/admin/vods/{id}
pub async fn update_vod(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
    vod_req: web::Json<VodRequest>,
) -> impl Responder {
    let collection = db.collection::<Vod>("vods");
    let vod_id = match mongodb::bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid video ID"),
    };

    let update_doc = doc! {
        "$set": {
            "vod_name": &vod_req.vod_name,
            "type_id": vod_req.type_id,
            "vod_status": vod_req.vod_status,
            "vod_class": &vod_req.vod_class,
            "vod_pic": &vod_req.vod_pic,
            "vod_actor": &vod_req.vod_actor,
            "vod_director": &vod_req.vod_director,
            "vod_remarks": &vod_req.vod_remarks,
            "vod_area": &vod_req.vod_area,
            "vod_lang": &vod_req.vod_lang,
            "vod_year": &vod_req.vod_year,
            "vod_content": &vod_req.vod_content,
        }
    };

    match collection
        .update_one(doc! {"_id": vod_id}, update_doc, None)
        .await
    {
        Ok(result) => {
            if result.matched_count > 0 {
                HttpResponse::Ok().json(json!({
                    "success": true,
                    "message": "Video updated successfully"
                }))
            } else {
                HttpResponse::NotFound().json(json!({
                    "success": false,
                    "message": "Video not found"
                }))
            }
        }
        Err(e) => {
            eprintln!("Failed to update video: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "Failed to update video"
            }))
        }
    }
}

// DELETE /api/admin/vods/{id}
pub async fn delete_vod(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
) -> impl Responder {
    let collection = db.collection::<Vod>("vods");
    let vod_id = match mongodb::bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().body("Invalid video ID"),
    };

    match collection.delete_one(doc! {"_id": vod_id}, None).await {
        Ok(result) => {
            if result.deleted_count > 0 {
                HttpResponse::Ok()
                    .json(json!({"success": true, "message": "Video deleted successfully"}))
            } else {
                HttpResponse::NotFound()
                    .json(json!({"success": false, "message": "Video not found"}))
            }
        }
        Err(e) => {
            eprintln!("Failed to delete video: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to delete video"}))
        }
    }
}

// DELETE /api/admin/vods/batch
pub async fn batch_delete_vods(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    batch_req: web::Json<BatchDeleteRequest>,
) -> impl Responder {

    let collection = db.collection::<Vod>("vods");
    let mut object_ids = Vec::new();
    let mut invalid_ids = Vec::new();

    // Parse all IDs and separate valid from invalid
    for id_str in &batch_req.ids {
        match mongodb::bson::oid::ObjectId::parse_str(id_str) {
            Ok(id) => object_ids.push(id),
            Err(_) => invalid_ids.push(id_str.clone()),
        }
    }

    if object_ids.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "No valid video IDs provided",
            "invalid_ids": invalid_ids
        }));
    }

    // Delete all valid videos
    match collection
        .delete_many(doc! {"_id": {"$in": object_ids}}, None)
        .await
    {
        Ok(result) => {
            let response = json!({
                "success": true,
                "message": "Videos deleted successfully",
                "deleted_count": result.deleted_count,
                "invalid_ids": invalid_ids.len(),
                "invalid_id_list": invalid_ids
            });
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            eprintln!("Failed to batch delete videos: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "Failed to delete videos",
                "error": e.to_string()
            }))
        }
    }
}

// --- Website Configuration Management API ---

// GET /api/admin/configs
pub async fn get_configs(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {
    let collection = db.collection::<Config>("configs");
    let find_options = FindOptions::builder().sort(doc! {"config_sort": 1}).build();

    match collection.find(None, find_options).await {
        Ok(cursor) => {
            let configs: Vec<Config> = cursor.try_collect().await.unwrap_or_else(|_| vec![]);
            HttpResponse::Ok().json(configs)
        }
        Err(e) => {
            eprintln!("Failed to fetch configs: {}", e);
            HttpResponse::InternalServerError().body("Failed to fetch configs")
        }
    }
}

// GET /api/admin/configs/{key}
pub async fn get_config_by_key(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
) -> impl Responder {
    let collection = db.collection::<Config>("configs");
    let config_key = path.into_inner();

    match collection
        .find_one(doc! {"config_key": &config_key}, None)
        .await
    {
        Ok(Some(config)) => HttpResponse::Ok().json(config),
        Ok(None) => HttpResponse::NotFound().body("Config not found"),
        Err(e) => {
            eprintln!("Failed to fetch config: {}", e);
            HttpResponse::InternalServerError().body("Failed to fetch config")
        }
    }
}

// POST /api/admin/configs
pub async fn create_config(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    config_req: web::Json<ConfigRequest>,
) -> impl Responder {
    let collection = db.collection::<Config>("configs");

    let new_config = Config {
        id: None,
        config_key: config_req.config_key.clone(),
        config_value: config_req.config_value.clone(),
        config_desc: config_req.config_desc.clone(),
        config_type: config_req.config_type.clone(),
        config_group: config_req.config_group.clone(),
        config_sort: config_req.config_sort,
        updated_at: mongodb::bson::DateTime::now(),
    };

    match collection.insert_one(new_config, None).await {
        Ok(_) => {
            HttpResponse::Created().json(json!({"success": true, "message": "Config created"}))
        }
        Err(e) => {
            if e.to_string().contains("E11000 duplicate key error") {
                HttpResponse::Conflict()
                    .json(json!({"success": false, "message": "Config key already exists"}))
            } else {
                eprintln!("Failed to create config: {}", e);
                HttpResponse::InternalServerError()
                    .json(json!({"success": false, "message": "Failed to create config"}))
            }
        }
    }
}

// PUT /api/admin/configs/{key}
pub async fn update_config(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
    config_req: web::Json<ConfigRequest>,
) -> impl Responder {
    let collection = db.collection::<Config>("configs");
    let config_key = path.into_inner();

    let update_doc = doc! {
        "$set": {
            "config_value": &config_req.config_value,
            "config_desc": &config_req.config_desc,
            "config_type": &config_req.config_type,
            "config_group": &config_req.config_group,
            "config_sort": config_req.config_sort,
            "updated_at": mongodb::bson::DateTime::now(),
        }
    };

    match collection
        .update_one(doc! {"config_key": &config_key}, update_doc, None)
        .await
    {
        Ok(result) => {
            if result.matched_count > 0 {
                HttpResponse::Ok()
                    .json(json!({"success": true, "message": "Config updated successfully"}))
            } else {
                HttpResponse::NotFound()
                    .json(json!({"success": false, "message": "Config not found"}))
            }
        }
        Err(e) => {
            eprintln!("Failed to update config: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to update config"}))
        }
    }
}

// DELETE /api/admin/configs/{key}
pub async fn delete_config(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
) -> impl Responder {
    let collection = db.collection::<Config>("configs");
    let config_key = path.into_inner();

    match collection
        .delete_one(doc! {"config_key": &config_key}, None)
        .await
    {
        Ok(result) => {
            if result.deleted_count > 0 {
                HttpResponse::Ok()
                    .json(json!({"success": true, "message": "Config deleted successfully"}))
            } else {
                HttpResponse::NotFound()
                    .json(json!({"success": false, "message": "Config not found"}))
            }
        }
        Err(e) => {
            eprintln!("Failed to delete config: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to delete config"}))
        }
    }
}

// POST /api/admin/types
pub async fn create_type(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    type_req: web::Json<TypeRequest>,
) -> impl Responder {
    let collection = db.collection::<Type>("types");

    // In a real system, you'd generate type_id and handle type_mid, etc.
    // For simplicity, let's assume type_id is auto-incremented or managed externally for now.
    // Or, query max type_id and increment.
    let new_type_id = match collection
        .find_one(
            None,
            FindOneOptions::builder().sort(doc! {"type_id": -1}).build(),
        )
        .await
    {
        Ok(Some(last_type)) => last_type.type_id + 1,
        _ => 1, // Start from 1 if no types exist
    };

    let new_type = Type {
        id: None, // MongoDB will generate ObjectId
        type_id: new_type_id,
        type_name: type_req.type_name.clone(),
        type_pid: type_req.type_pid,
        type_en: type_req.type_en.clone(),
        type_sort: type_req.type_sort.unwrap_or(0),
        type_status: type_req.type_status.unwrap_or(1),
        type_mid: type_req.type_mid,
        type_key: type_req.type_key.clone(),
        type_des: type_req.type_des.clone(),
        type_title: type_req.type_title.clone(),
        type_tpl: None,
        type_tpl_list: None,
        type_tpl_detail: None,
        type_tpl_play: None,
        type_tpl_down: None,
        subarea: type_req.subarea.clone(),
        subyear: type_req.subyear.clone(),
    };

    match collection.insert_one(new_type, None).await {
        Ok(_) => HttpResponse::Created().json(json!({"success": true, "message": "Type created"})),
        Err(e) => {
            eprintln!("Failed to create type: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to create type"}))
        }
    }
}

// PUT /api/admin/types/{id}
pub async fn update_type(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
    type_req: web::Json<TypeRequest>,
) -> impl Responder {
    let collection = db.collection::<Type>("types");
    let type_id: i32 = match path.into_inner().parse() {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"success": false, "message": "Invalid type ID"}))
        }
    };

    let mut update_fields = doc! {
        "type_name": &type_req.type_name,
        "type_pid": type_req.type_pid,
    };

    if let Some(ref type_en) = type_req.type_en {
        update_fields.insert("type_en", type_en);
    }
    if let Some(type_sort) = type_req.type_sort {
        update_fields.insert("type_sort", type_sort);
    }
    if let Some(type_status) = type_req.type_status {
        update_fields.insert("type_status", type_status);
    }
    if let Some(type_mid) = type_req.type_mid {
        update_fields.insert("type_mid", type_mid);
    }
    if let Some(ref type_key) = type_req.type_key {
        update_fields.insert("type_key", type_key);
    }
    if let Some(ref type_des) = type_req.type_des {
        update_fields.insert("type_des", type_des);
    }
    if let Some(ref type_title) = type_req.type_title {
        update_fields.insert("type_title", type_title);
    }
    if let Some(ref subarea) = type_req.subarea {
        update_fields.insert("subarea", subarea);
    }
    if let Some(ref subyear) = type_req.subyear {
        update_fields.insert("subyear", subyear);
    }

    let update_doc = doc! {
        "$set": update_fields
    };

    match collection
        .update_one(doc! {"type_id": type_id}, update_doc, None)
        .await
    {
        Ok(result) => {
            if result.matched_count > 0 {
                HttpResponse::Ok()
                    .json(json!({"success": true, "message": "Type updated successfully"}))
            } else {
                HttpResponse::NotFound()
                    .json(json!({"success": false, "message": "Type not found"}))
            }
        }
        Err(e) => {
            eprintln!("Failed to update type: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to update type"}))
        }
    }
}

// DELETE /api/admin/types/{id}
pub async fn delete_type(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
) -> impl Responder {
    let collection = db.collection::<Type>("types");
    let type_id: i32 = match path.into_inner().parse() {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"success": false, "message": "Invalid type ID"}))
        }
    };

    match collection.delete_one(doc! {"type_id": type_id}, None).await {
        Ok(result) => {
            if result.deleted_count > 0 {
                HttpResponse::Ok()
                    .json(json!({"success": true, "message": "Type deleted successfully"}))
            } else {
                HttpResponse::NotFound()
                    .json(json!({"success": false, "message": "Type not found"}))
            }
        }
        Err(e) => {
            eprintln!("Failed to delete type: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to delete type"}))
        }
    }
}

// --- Binding Management API ---
// DELETE /api/admin/bindings/{id}
pub async fn delete_binding(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    path: web::Path<String>,
) -> impl Responder {
    let collection = db.collection::<Binding>("bindings");
    let binding_id = path.into_inner();

    match collection.delete_one(doc! {"_id": binding_id}, None).await {
        Ok(result) => {
            if result.deleted_count > 0 {
                HttpResponse::Ok()
                    .json(json!({"success": true, "message": "Binding deleted successfully"}))
            } else {
                HttpResponse::NotFound()
                    .json(json!({"success": false, "message": "Binding not found"}))
            }
        }
        Err(e) => {
            eprintln!("Failed to delete binding: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to delete binding"}))
        }
    }
}
// GET /api/admin/bindings
pub async fn get_bindings(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {
    let collection = db.collection::<Binding>("bindings");

    match collection.find(None, None).await {
        Ok(cursor) => {
            let bindings: Vec<Binding> = cursor.try_collect().await.unwrap_or_else(|_| vec![]);
            HttpResponse::Ok().json(bindings)
        }
        Err(e) => {
            eprintln!("Failed to fetch bindings: {}", e);
            HttpResponse::InternalServerError().body("Failed to fetch bindings")
        }
    }
}

// GET /api/admin/collections/{id}/binding-status
pub async fn get_collection_binding_status(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
    db: web::Data<Database>,
) -> impl Responder {

    let collection_id = match mongodb::bson::oid::ObjectId::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(json!({"success": false, "message": "Invalid collection ID"}))
        }
    };

    // 获取采集源配置
    let collection = match db
        .collection::<Collection>("collections")
        .find_one(doc! {"_id": collection_id}, None)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(json!({"success": false, "message": "Collection not found"}))
        }
        Err(e) => {
            eprintln!("Failed to fetch collection: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to fetch collection"}));
        }
    };

    // 检查是否有绑定的分类
    let bindings_collection = db.collection::<Binding>("bindings");
    let bindings_count = match bindings_collection
        .count_documents(doc! {"source_flag": &collection.collect_name}, None)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Failed to count bindings: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"success": false, "message": "Failed to check bindings"}));
        }
    };

    let has_bindings = bindings_count > 0;

    HttpResponse::Ok().json(json!({
        "success": true,
        "has_bindings": has_bindings,
        "bindings_count": bindings_count,
        "source_flag": collection.collect_name,
        "message": if has_bindings {
            format!("已绑定 {} 个分类", bindings_count)
        } else {
            "请先绑定分类".to_string()
        }
    }))
}

// POST /api/admin/bindings
pub async fn create_or_update_binding(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    binding_req: web::Json<BindingRequest>,
) -> impl Responder {
    let collection = db.collection::<Binding>("bindings");

    let binding_id = format!("{}_{}", binding_req.source_flag, binding_req.external_id);

    // Fetch local type name for the binding
    let type_collection = db.collection::<Type>("types");
    let local_type_name = match type_collection
        .find_one(doc! {"type_id": binding_req.local_type_id}, None)
        .await
    {
        Ok(Some(t)) => t.type_name,
        _ => "Unknown Type".to_string(), // Default if type not found
    };

    let now = mongodb::bson::DateTime::now();
    let new_binding = Binding {
        id: binding_id.clone(),
        source_flag: binding_req.source_flag.clone(),
        external_id: binding_req.external_id.clone(),
        local_type_id: binding_req.local_type_id,
        local_type_name: local_type_name.clone(),
        created_at: now,
        updated_at: now,
    };

    match collection.insert_one(new_binding, None).await {
        Ok(_) => HttpResponse::Created()
            .json(json!({"success": true, "message": "Binding created/updated"})),
        Err(e) => {
            // If it's a duplicate key error, try to update instead (upsert behavior)
            if e.to_string().contains("E11000 duplicate key error") {
                let update_doc = doc! {"$set": {
                    "source_flag": &binding_req.source_flag,
                    "external_id": &binding_req.external_id,
                    "local_type_id": binding_req.local_type_id,
                    "local_type_name": local_type_name.clone(),
                    "updated_at": mongodb::bson::DateTime::now()
                }};
                match collection
                    .update_one(doc! {"_id": binding_id}, update_doc, None)
                    .await
                {
                    Ok(_) => HttpResponse::Ok()
                        .json(json!({"success": true, "message": "Binding updated"})),
                    Err(e) => {
                        eprintln!("Failed to update binding: {}", e);
                        HttpResponse::InternalServerError()
                            .json(json!({"success": false, "message": "Failed to update binding"}))
                    }
                }
            } else {
                eprintln!("Failed to create binding: {}", e);
                HttpResponse::InternalServerError()
                    .json(json!({"success": false, "message": "Failed to create binding"}))
            }
        }
    }
}

// --- Index Management API ---

// POST /api/admin/indexes/create
pub async fn create_indexes(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {

    let index_manager = IndexManager::new(db.get_ref().clone());

    match index_manager.create_all_indexes().await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "索引创建完成"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "message": format!("索引创建失败: {}", e)
        })),
    }
}

// GET /api/admin/indexes/status
pub async fn get_index_status(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {

    let index_manager = IndexManager::new(db.get_ref().clone());

    match index_manager.verify_indexes().await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "所有索引状态正常"
        })),
        Err(e) => HttpResponse::Ok().json(json!({
            "success": false,
            "message": format!("索引验证失败: {}", e)
        })),
    }
}

// GET /api/admin/indexes/list
pub async fn list_indexes(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {

    let index_manager = IndexManager::new(db.get_ref().clone());

    // 使用IndexManager的show_index_status方法获取索引信息
    match index_manager.show_index_status().await {
        Ok(_) => {
            // 返回简单的成功响应，详细状态在控制台输出
            HttpResponse::Ok().json(json!({
                "success": true,
                "message": "索引状态已输出到控制台"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "message": format!("获取索引状态失败: {}", e)
        })),
    }
}

// GET /api/admin/indexes/data
pub async fn get_indexes_data(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {

    let index_manager = IndexManager::new(db.get_ref().clone());
    match index_manager.get_all_indexes().await {
        Ok(indexes) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": indexes
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "message": format!("获取索引数据失败: {}", e)
        })),
    }
}

// GET /api/admin/statistics
pub async fn get_statistics(admin: crate::jwt_auth::AdminUser, db: web::Data<Database>) -> impl Responder {

    let mut stats = json!({
        "success": true,
        "data": {
            "vods": 0,
            "types": 0,
            "collections": 0,
            "bindings": 0,
            "configs": 0,
            "users": 0
        }
    });

    // 获取视频数量
    if let Ok(count) = db
        .collection::<mongodb::bson::Document>("vods")
        .count_documents(None, None)
        .await
    {
        stats["data"]["vods"] = count.into();
    }

    // 获取分类数量
    if let Ok(count) = db
        .collection::<mongodb::bson::Document>("types")
        .count_documents(None, None)
        .await
    {
        stats["data"]["types"] = count.into();
    }

    // 获取采集源数量
    if let Ok(count) = db
        .collection::<mongodb::bson::Document>("collections")
        .count_documents(None, None)
        .await
    {
        stats["data"]["collections"] = count.into();
    }

    // 获取绑定数量
    if let Ok(count) = db
        .collection::<mongodb::bson::Document>("bindings")
        .count_documents(None, None)
        .await
    {
        stats["data"]["bindings"] = count.into();
    }

    // 获取配置数量
    if let Ok(count) = db
        .collection::<mongodb::bson::Document>("configs")
        .count_documents(None, None)
        .await
    {
        stats["data"]["configs"] = count.into();
    }

    // 获取用户数量
    if let Ok(count) = db
        .collection::<mongodb::bson::Document>("users")
        .count_documents(None, None)
        .await
    {
        stats["data"]["users"] = count.into();
    }

    HttpResponse::Ok().json(stats)
}

// === 定时任务管理 API ===

// GET /api/admin/scheduled-task/status
pub async fn get_scheduled_task_status(admin: crate::jwt_auth::AdminUser, 
    task_manager: web::Data<std::sync::Arc<ScheduledTaskManager>>,
) -> impl Responder {

    match task_manager.get_task_status().await {
        Ok(status) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": status
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "message": format!("获取定时任务状态失败: {}", e)
        })),
    }
}

// POST /api/admin/scheduled-task/start
pub async fn start_scheduled_task(admin: crate::jwt_auth::AdminUser, 
    task_manager: web::Data<std::sync::Arc<ScheduledTaskManager>>,
) -> impl Responder {
    match task_manager.start_scheduled_task().await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "定时采集任务已启动"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "message": format!("启动定时任务失败: {}", e)
        })),
    }
}

// POST /api/admin/scheduled-task/stop
pub async fn stop_scheduled_task(admin: crate::jwt_auth::AdminUser, 
    task_manager: web::Data<std::sync::Arc<ScheduledTaskManager>>,
) -> impl Responder {
    match task_manager.stop_scheduled_task().await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "定时采集任务已停止"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "message": format!("停止定时任务失败: {}", e)
        })),
    }
}

// PUT /api/admin/scheduled-task/config
#[derive(Debug, Deserialize)]
pub struct ScheduledTaskConfigRequest {
    pub enabled: bool,
    pub interval_hours: Option<i32>,
}

pub async fn update_scheduled_task_config(admin: crate::jwt_auth::AdminUser, 
    task_manager: web::Data<std::sync::Arc<ScheduledTaskManager>>,
    config: web::Json<ScheduledTaskConfigRequest>,
) -> impl Responder {
    match task_manager
        .update_config(config.enabled, config.interval_hours)
        .await
    {
        Ok(true) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "定时任务配置已更新"
        })),
        Ok(false) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "配置更新失败"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "message": format!("更新配置失败: {}", e)
        })),
    }
}

// GET /api/admin/scheduled-task/logs
pub async fn get_scheduled_task_logs(admin: crate::jwt_auth::AdminUser, 
    task_manager: web::Data<std::sync::Arc<ScheduledTaskManager>>,
    query: web::Query<ScheduledTaskLogsQuery>,
) -> impl Responder {
    match task_manager.get_task_logs(query.limit).await {
        Ok(logs) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": logs
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "message": format!("获取任务日志失败: {}", e)
        })),
    }
}

#[derive(Debug, Deserialize)]
pub struct ScheduledTaskLogsQuery {
    pub limit: Option<i32>,
}

// --- Batch Delete Source API ---

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchDeleteSourceRequest {
    pub source_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchDeleteProgress {
    pub status: String,
    pub processed_count: u64,
    pub deleted_count: u64,
    pub total_count: u64,
    pub log: String,
}

impl Default for BatchDeleteProgress {
    fn default() -> Self {
        Self {
            status: "unknown".to_string(),
            processed_count: 0,
            deleted_count: 0,
            total_count: 0,
            log: "未知状态".to_string(),
        }
    }
}

// 类型别名简化复杂类型
type BatchDeleteProgressMap = std::collections::HashMap<
    String,
    (
        BatchDeleteProgress,
        String,
        Option<tokio::task::JoinHandle<()>>,
    ),
>;
type BatchDeleteProgressStore = tokio::sync::RwLock<BatchDeleteProgressMap>;

// 全局批量删除任务进度存储
static BATCH_DELETE_PROGRESS: std::sync::OnceLock<BatchDeleteProgressStore> =
    std::sync::OnceLock::new();

// 初始化批量删除任务进度存储
fn get_batch_delete_progress_store() -> &'static BatchDeleteProgressStore {
    BATCH_DELETE_PROGRESS.get_or_init(|| tokio::sync::RwLock::new(std::collections::HashMap::new()))
}

// 获取批量删除任务进度
pub async fn get_batch_delete_progress(task_id: &str) -> Option<BatchDeleteProgress> {
    let store = get_batch_delete_progress_store();
    let progress_map = store.read().await;
    progress_map
        .get(task_id)
        .map(|(progress, _, _)| progress.clone())
}

// 更新批量删除任务进度
async fn update_batch_delete_progress(
    task_id: &str,
    progress: BatchDeleteProgress,
    task_name: String,
) {
    let store = get_batch_delete_progress_store();
    let mut progress_map = store.write().await;
    if let Some((current_progress, current_name, handle)) = progress_map.get_mut(task_id) {
        *current_progress = progress;
        *current_name = task_name;
        // 保持原有的handle不变，不需要克隆
    } else {
        progress_map.insert(task_id.to_string(), (progress, task_name, None));
    }
}

// 停止批量删除任务
pub async fn stop_batch_delete_task(task_id: &str) -> bool {
    let store = get_batch_delete_progress_store();
    let mut progress_map = store.write().await;

    if let Some((mut progress, task_name, handle)) = progress_map.remove(task_id) {
        // 取消任务
        if let Some(task_handle) = handle {
            task_handle.abort();
        }

        // 标记任务为已停止
        progress.status = "stopped".to_string();
        progress.log = "任务已手动停止".to_string();

        // 将任务重新插入，但状态为已停止且清除句柄
        progress_map.insert(task_id.to_string(), (progress, task_name, None));

        true
    } else {
        false
    }
}

// 获取所有运行中的批量删除任务
pub async fn get_all_batch_delete_tasks() -> Vec<serde_json::Value> {
    let store = get_batch_delete_progress_store();
    let progress_map = store.read().await;

    let mut tasks = Vec::new();
    for (task_id, (progress, task_name, _)) in progress_map.iter() {
        tasks.push(json!({
            "task_id": task_id,
            "task_name": task_name,
            "status": progress.status,
            "processed_count": progress.processed_count,
            "deleted_count": progress.deleted_count,
            "total_count": progress.total_count,
            "log": progress.log
        }));
    }

    tasks
}

// 启动批量删除任务
pub async fn start_batch_delete_source(db: web::Data<Database>, source_name: String) -> String {
    let task_id = uuid::Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();

    let collection = db.collection::<Vod>("vods");
    const BATCH_SIZE: i64 = 2000;

    // 获取总视频数量
    let total_count = match collection.count_documents(None, None).await {
        Ok(count) => count as i64,
        Err(e) => {
            eprintln!("Failed to count vods: {}", e);

            // 初始化失败状态
            let failed_progress = BatchDeleteProgress {
                status: "failed".to_string(),
                processed_count: 0,
                deleted_count: 0,
                total_count: 0,
                log: "无法获取视频总数".to_string(),
            };

            update_batch_delete_progress(
                &task_id,
                failed_progress,
                format!("批量删除播放源: {}", source_name),
            )
            .await;
            return task_id;
        }
    };

    let total_count_u64 = total_count as u64;

    // 初始化进度
    let initial_progress = BatchDeleteProgress {
        status: "running".to_string(),
        processed_count: 0,
        deleted_count: 0,
        total_count: total_count_u64,
        log: "开始批量删除播放源任务".to_string(),
    };

    update_batch_delete_progress(
        &task_id,
        initial_progress,
        format!("批量删除播放源: {}", source_name),
    )
    .await;

    // 启动后台任务
    let db_clone = db.clone();
    let source_name_clone = source_name.clone();
    let task_handle = tokio::spawn(async move {
        if let Err(e) =
            execute_batch_delete_inner(db_clone, &task_id_clone, &source_name_clone, BATCH_SIZE)
                .await
        {
            eprintln!("Batch delete failed: {}", e);

            let failed_progress = BatchDeleteProgress {
                status: "failed".to_string(),
                processed_count: 0,
                deleted_count: 0,
                total_count: total_count_u64,
                log: format!("批量删除失败: {}", e),
            };
            update_batch_delete_progress(
                &task_id_clone,
                failed_progress,
                format!("批量删除播放源: {}", source_name_clone),
            )
            .await;
        }
    });

    // 将任务句柄存储到进度Map中
    let store = get_batch_delete_progress_store();
    let mut progress_map = store.write().await;
    if let Some((_, _, handle_ref)) = progress_map.get_mut(&task_id) {
        *handle_ref = Some(task_handle);
    } else {
        progress_map.insert(
            task_id.clone(),
            (
                BatchDeleteProgress {
                    status: "running".to_string(),
                    processed_count: 0,
                    deleted_count: 0,
                    total_count: total_count_u64,
                    log: "开始批量删除播放源任务".to_string(),
                },
                format!("批量删除播放源: {}", source_name),
                Some(task_handle),
            ),
        );
    }

    task_id
}

// 执行批量删除的核心逻辑
async fn execute_batch_delete_inner(
    db: web::Data<Database>,
    task_id: &str,
    source_name: &str,
    batch_size: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let collection = db.collection::<Vod>("vods");

    let mut processed_count = 0u64;
    let mut deleted_count = 0u64;
    let mut last_id: Option<mongodb::bson::oid::ObjectId> = None;

    // 获取总视频数量
    let total_count = collection.count_documents(None, None).await?;
    let total_count_u64 = total_count as u64;

    // 分批处理视频
    loop {
        // 构建查询，使用大于last_id来获取下一批
        let mut filter = doc! {};

        if let Some(last) = last_id {
            filter.insert("_id", doc! {"$gt": last});
        }

        let find_options = FindOptions::builder()
            .sort(doc! {"_id": 1})
            .limit(batch_size)
            .build();

        let cursor = collection.find(filter, find_options).await?;
        let mut vods_in_batch: Vec<Vod> = cursor.try_collect().await.unwrap_or_else(|_| vec![]);

        if vods_in_batch.is_empty() {
            // 更新最终状态
            let completed_progress = BatchDeleteProgress {
                status: "completed".to_string(),
                processed_count: total_count_u64,
                deleted_count,
                total_count: total_count_u64,
                log: format!(
                    "批量删除完成：处理了 {} 个视频，删除了 {} 个播放源",
                    processed_count, deleted_count
                ),
            };
            update_batch_delete_progress(
                task_id,
                completed_progress,
                format!("批量删除播放源: {}", source_name),
            )
            .await;
            break;
        }

        // 更新last_id为当前批次的最后一个视频ID
        if let Some(last_vod) = vods_in_batch.last() {
            last_id = last_vod.id;
        }

        // 处理这一批视频
        for vod in &mut vods_in_batch {
            if let Some(vod_id) = vod.id {
                let mut has_changed = false;

                // 检查vod_play_urls数组中是否有匹配的source_name
                let mut new_play_urls = Vec::new();

                for play_source in &vod.vod_play_urls {
                    if play_source.source_name != source_name {
                        new_play_urls.push(play_source.to_owned());
                    } else {
                        deleted_count += 1;
                        has_changed = true;
                    }
                }

                if has_changed {
                    vod.vod_play_urls = new_play_urls;
                }

                // 如果有更改，更新数据库
                if has_changed {
                    // 使用mongodb::bson::to_document来序列化vod结构
                    let mut update_doc = mongodb::bson::to_document(&vod)?;
                    // 移除_id字段，因为我们不能更新主键
                    update_doc.remove("_id");

                    let update_doc = doc! {
                        "$set": update_doc
                    };

                    // 这里我们可以选择不等待update_one，增加并发性
                    if let Err(e) = collection
                        .update_one(doc! {"_id": vod_id}, update_doc, None)
                        .await
                    {
                        eprintln!("Failed to update vod {}: {}", vod_id, e);
                        // 继续处理，不因为单个错误而停止
                    }
                }
            }

            processed_count += 1;

            // 每处理100个视频更新一次进度
            if processed_count % 100 == 0 {
                let progress = BatchDeleteProgress {
                    status: "running".to_string(),
                    processed_count,
                    deleted_count,
                    total_count: total_count_u64,
                    log: format!(
                        "正在处理中... 已处理 {}/{} 个视频",
                        processed_count, total_count_u64
                    ),
                };
                update_batch_delete_progress(
                    task_id,
                    progress,
                    format!("批量删除播放源: {}", source_name),
                )
                .await;
            }
        }

        // 如果这一批没有达到BATCH_SIZE，说明已经处理完了所有数据
        if vods_in_batch.len() < batch_size as usize {
            // 更新最终状态
            let completed_progress = BatchDeleteProgress {
                status: "completed".to_string(),
                processed_count: total_count_u64,
                deleted_count,
                total_count: total_count_u64,
                log: format!(
                    "批量删除完成：处理了 {} 个视频，删除了 {} 个播放源",
                    processed_count, deleted_count
                ),
            };
            update_batch_delete_progress(
                task_id,
                completed_progress,
                format!("批量删除播放源: {}", source_name),
            )
            .await;
            break;
        }
    }

    Ok(())
}

// POST /api/admin/batch-delete-source
pub async fn batch_delete_source(admin: crate::jwt_auth::AdminUser, 
    db: web::Data<Database>,
    request: web::Json<BatchDeleteSourceRequest>,
) -> impl Responder {

    let source_name = request.source_name.trim();
    if source_name.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "播放源名称不能为空"
        }));
    }

    // 检查是否存在正在运行的任务
    let running_tasks = get_all_batch_delete_tasks().await;
    let has_running = running_tasks.iter().any(|task| task["status"] == "running");

    if has_running {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "已有正在运行的批量删除任务，请等待完成后重试"
        }));
    }

    // 启动后台任务
    let task_id = start_batch_delete_source(db, source_name.to_string()).await;

    HttpResponse::Ok().json(json!({
        "success": true,
        "message": "批量删除任务已启动",
        "task_id": task_id,
        "source_name": source_name
    }))
}

// GET /api/admin/batch-delete/progress/{task_id}
pub async fn get_batch_delete_progress_handler(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
) -> impl Responder {

    let task_id = path.into_inner();

    let progress = get_batch_delete_progress(&task_id)
        .await
        .unwrap_or_else(|| BatchDeleteProgress {
            status: "not_found".to_string(),
            processed_count: 0,
            deleted_count: 0,
            total_count: 0,
            log: "任务不存在".to_string(),
        });

    HttpResponse::Ok().json(json!({
        "success": true,
        "progress": progress
    }))
}

// GET /api/admin/batch-delete/running-tasks
pub async fn get_running_batch_delete_tasks_handler(admin: crate::jwt_auth::AdminUser) -> impl Responder {

    let tasks = get_all_batch_delete_tasks().await;

    HttpResponse::Ok().json(json!({
        "success": true,
        "tasks": tasks
    }))
}

// POST /api/admin/batch-delete/stop/{task_id}
pub async fn stop_batch_delete_task_handler(admin: crate::jwt_auth::AdminUser, 
    path: web::Path<String>,
) -> impl Responder {

    let task_id = path.into_inner();

    let stopped = stop_batch_delete_task(&task_id).await;

    if stopped {
        HttpResponse::Ok().json(json!({
            "success": true,
            "message": "批量删除任务已成功停止"
        }))
    } else {
        HttpResponse::NotFound().json(json!({
            "success": false,
            "message": "任务不存在或已经停止"
        }))
    }
}

// ============= 卡卷管理功能 =============

// GET /admin/cards
pub async fn admin_cards_page(
    db: web::Data<Database>,
    site_data_manager: web::Data<crate::site_data::SiteDataManager>,
) -> impl Responder {

    match crate::web_handlers::with_site_data(
        db.clone(),
        site_data_manager.clone(),
        |mut context, _site_data| async move {
            context.insert("page_title", "卡卷管理");

            TERA.render("admin/cards.html", &context).map_err(|e| {
                crate::web_handlers::handle_template_rendering_error(
                    "admin/cards.html",
                    &e,
                    Some("Admin cards management page"),
                    Some("Admin access required"),
                );
                Box::new(e) as Box<dyn std::error::Error>
            })
        },
    )
    .await
    {
        Ok(response) => response,
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Template error: {}", e)
        })),
    }
}

// GET /api/admin/cards
pub async fn get_cards_list(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    query: web::Query<CardPageParams>,
) -> impl Responder {

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let skip = (page - 1) * limit;

    let cards_collection = db.collection::<Card>("cards");
    let users_collection = db.collection::<User>("users");

    // 获取卡卷总数
    let total = cards_collection
        .count_documents(None, None)
        .await
        .unwrap_or(0);

    // 获取卡卷列表
    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit as i64)
        .sort(doc! {"created_at": -1})
        .build();

    let mut cards = match cards_collection.find(None, find_options).await {
        Ok(cursor) => cursor.try_collect().await.unwrap_or_else(|_| vec![]),
        Err(_) => vec![],
    };

    // 获取用户信息并转换格式
    let mut card_infos = Vec::new();
    for card in &mut cards {
        let used_by = if card.used {
            if let Some(user_id) = card.used_by {
                match users_collection.find_one(doc! {"_id": user_id}, None).await {
                    Ok(Some(user)) => Some(UserInfo {
                        id: user_id.to_string(),
                        user_name: user.user_name,
                        user_nick_name: user.user_nick_name,
                    }),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        let card_info = CardInfo {
            id: card.id.unwrap_or_default().to_string(),
            code: card.code.clone(),
            used: card.used,
            vip_level: card.vip_level,
            duration_days: card.duration_days,
            created_at: card.created_at.to_string(),
            used_by,
            used_at: card.used_at.map(|dt| dt.to_string()),
        };
        card_infos.push(card_info);
    }

    HttpResponse::Ok().json(CardListResponse {
        code: 1,
        msg: "获取卡卷列表成功".to_string(),
        cards: card_infos,
        total: total.try_into().unwrap_or(0),
    })
}

// POST /api/admin/cards/generate
pub async fn generate_cards(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    request: web::Json<GenerateCardRequest>,
) -> impl Responder {

    // 验证参数
    if request.count <= 0 || request.count > 1000 {
        return HttpResponse::BadRequest().json(json!({
            "code": 0,
            "msg": "生成数量必须在1-1000之间"
        }));
    }

    if request.vip_level < 1 || request.duration_days <= 0 {
        return HttpResponse::BadRequest().json(json!({
            "code": 0,
            "msg": "会员等级和时长必须大于0"
        }));
    }

    let cards_collection = db.collection::<Card>("cards");
    let mut generated_cards = Vec::new();
    let mut generated_codes = Vec::new();

    // 生成随机卡卷
    for _ in 0..request.count {
        let code = generate_random_code();
        let new_card = Card {
            id: None,
            code: code.clone(),
            used: false,
            vip_level: request.vip_level,
            duration_days: request.duration_days,
            created_at: mongodb::bson::DateTime::now(),
            used_by: None,
            used_at: None,
        };

        match cards_collection.insert_one(new_card, None).await {
            Ok(_) => {
                generated_cards.push(code.clone());
                generated_codes.push(code);
            }
            Err(e) => {
                eprintln!("Failed to insert card: {}", e);
            }
        }
    }

    HttpResponse::Ok().json(GenerateCardResponse {
        code: 1,
        msg: format!("成功生成 {} 张卡卷", generated_cards.len()),
        generated_count: generated_cards.len() as i32,
        cards: generated_codes,
    })
}

// POST /api/admin/cards/delete
pub async fn delete_cards(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    request: web::Json<DeleteCardRequest>,
) -> impl Responder {

    if request.card_ids.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "code": 0,
            "msg": "请选择要删除的卡卷"
        }));
    }

    let cards_collection = db.collection::<Card>("cards");
    let mut deleted_count = 0;

    for card_id_str in &request.card_ids {
        if let Ok(card_id) = mongodb::bson::oid::ObjectId::parse_str(card_id_str) {
            match cards_collection
                .delete_one(doc! {"_id": card_id}, None)
                .await
            {
                Ok(result) => {
                    if result.deleted_count > 0 {
                        deleted_count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to delete card {}: {}", card_id_str, e);
                }
            }
        }
    }

    HttpResponse::Ok().json(json!({
        "code": 1,
        "msg": format!("成功删除 {} 张卡卷", deleted_count),
        "deleted_count": deleted_count
    }))
}

// POST /api/admin/cards/search
pub async fn search_cards(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    request: web::Json<SearchCardRequest>,
) -> impl Responder {

    if request.code.trim().is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "code": 0,
            "msg": "请输入卡卷代码"
        }));
    }

    let cards_collection = db.collection::<Card>("cards");
    let users_collection = db.collection::<User>("users");

    let card = match cards_collection
        .find_one(doc! {"code": request.code.trim()}, None)
        .await
    {
        Ok(Some(card)) => card,
        Ok(None) => {
            return HttpResponse::Ok().json(CardListResponse {
                code: 1,
                msg: "未找到匹配的卡卷".to_string(),
                cards: vec![],
                total: 0,
            });
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "code": 0,
                "msg": "服务器错误"
            }));
        }
    };

    // 获取用户信息
    let used_by = if card.used {
        if let Some(user_id) = card.used_by {
            match users_collection.find_one(doc! {"_id": user_id}, None).await {
                Ok(Some(user)) => Some(UserInfo {
                    id: user_id.to_string(),
                    user_name: user.user_name,
                    user_nick_name: user.user_nick_name,
                }),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    let card_info = CardInfo {
        id: card.id.unwrap_or_default().to_string(),
        code: card.code,
        used: card.used,
        vip_level: card.vip_level,
        duration_days: card.duration_days,
        created_at: card.created_at.to_string(),
        used_by,
        used_at: card.used_at.map(|dt| dt.to_string()),
    };

    HttpResponse::Ok().json(CardListResponse {
        code: 1,
        msg: "搜索成功".to_string(),
        cards: vec![card_info],
        total: 1,
    })
}

// 生成10位随机卡卷代码
fn generate_random_code() -> String {
    use rand::{thread_rng, Rng};
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = thread_rng();
    let code: String = (0..10)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    code
}

// DTO for batch VIP setting request
#[derive(Debug, Deserialize)]
pub struct BatchSetVipRequest {
    pub vod_ids: Vec<String>,
    pub need_vip: i32,
}

// Batch set VIP content
pub async fn batch_set_vip(
    _admin: AdminUser,
    db: web::Data<Database>,
    request: web::Json<BatchSetVipRequest>,
) -> Result<HttpResponse> {

    let vod_ids = &request.vod_ids;
    let need_vip = request.need_vip;

    if vod_ids.is_empty() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "请选择要设置的视频"
        })));
    }

    if need_vip < 0 || need_vip > 5 {
        return Ok(HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "VIP等级必须在0-5之间"
        })));
    }

    let vod_collection = db.collection::<Vod>("vods");

    // Convert string IDs to ObjectId
    let object_ids: Result<Vec<_>, _> = vod_ids
        .iter()
        .map(|id| mongodb::bson::oid::ObjectId::parse_str(id))
        .collect();

    let object_ids = match object_ids {
        Ok(ids) => ids,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": format!("无效的视频ID: {}", e)
            })));
        }
    };

    // Update multiple videos
    let result = vod_collection
        .update_many(
            doc! { "_id": { "$in": object_ids } },
            doc! { "$set": { "need_vip": need_vip } },
            None,
        )
        .await;

    match result {
        Ok(update_result) => Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "message": format!("成功设置 {} 个视频的VIP等级", update_result.modified_count),
            "modified_count": update_result.modified_count
        }))),
        Err(e) => {
            eprintln!("Database error when batch setting VIP: {}", e);
            Ok(HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "服务器错误"
            })))
        }
    }
}

// User management functions
pub async fn admin_users_page(
    db: web::Data<Database>,
    site_data_manager: web::Data<crate::site_data::SiteDataManager>,
) -> impl Responder {

    match crate::web_handlers::with_site_data(
        db.clone(),
        site_data_manager.clone(),
        |mut context, _site_data| async move {
            context.insert("page_title", "用户管理");

            TERA.render("admin/users.html", &context).map_err(|e| {
                crate::web_handlers::handle_template_rendering_error(
                    "admin/users.html",
                    &e,
                    Some("Admin user management page"),
                    Some("Admin access required"),
                );
                Box::new(e) as Box<dyn std::error::Error>
            })
        },
    )
    .await
    {
        Ok(response) => response,
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Template error: {}", e)
        })),
    }
}

pub async fn get_users_list(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    query: web::Query<crate::dto::UserPageParams>,
) -> impl Responder {

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let skip = (page - 1) * limit;

    let user_collection = db.collection::<User>("users");
    
    // Get total count
    let total = match user_collection.count_documents(None, None).await {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Database error when counting users: {}", e);
            return HttpResponse::InternalServerError().json(crate::dto::UserListResponse {
                code: 500,
                msg: "服务器错误".to_string(),
                users: vec![],
                total: 0,
            });
        }
    };

    // Get users with pagination
    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit as i64)
        .sort(doc! { "created_at": -1 })
        .build();

    let users: Vec<User> = match user_collection.find(None, find_options).await {
        Ok(cursor) => cursor.try_collect().await.unwrap_or_else(|_| vec![]),
        Err(e) => {
            eprintln!("Database error when fetching users: {}", e);
            return HttpResponse::InternalServerError().json(crate::dto::UserListResponse {
                code: 500,
                msg: "服务器错误".to_string(),
                users: vec![],
                total: 0,
            });
        }
    };

    // Convert to UserAdminInfo format
    let user_infos: Vec<crate::dto::UserAdminInfo> = users
        .into_iter()
        .map(|user| crate::dto::UserAdminInfo {
            id: user.id.map_or_else(|| "".to_string(), |id| id.to_string()),
            user_name: user.user_name,
            user_nick_name: user.user_nick_name,
            user_email: user.user_email.unwrap_or_else(|| "".to_string()),
            vip_level: user.vip_level.unwrap_or(0),
            vip_end_time: user.vip_end_time.map(|end| end.to_string()),
            created_at: user.created_at.map_or_else(|| "".to_string(), |dt| dt.to_string()),
            last_login: None,
        })
        .collect();

    HttpResponse::Ok().json(crate::dto::UserListResponse {
        code: 200,
        msg: "获取成功".to_string(),
        users: user_infos,
        total: total as i64,
    })
}

pub async fn create_user(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    request: web::Json<crate::dto::CreateUserRequest>,
) -> impl Responder {

    // Check if username already exists
    let user_collection = db.collection::<User>("users");
    let existing_user = user_collection
        .find_one(doc! { "user_name": &request.user_name }, None)
        .await;

    if let Ok(Some(_)) = existing_user {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "用户名已存在"
        }));
    }

    // Check if email already exists
    let existing_email = user_collection
        .find_one(doc! { "user_email": &request.user_email }, None)
        .await;

    if let Ok(Some(_)) = existing_email {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "邮箱已存在"
        }));
    }

    // Hash password
    let hashed_password = match bcrypt::hash(&request.password, 12) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("Password hashing error: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "服务器错误"
            }));
        }
    };

    // Calculate VIP end time if provided
    let vip_end_time = if let (Some(vip_level), Some(duration_days)) = (request.vip_level, request.vip_duration_days) {
        if vip_level > 0 && duration_days > 0 {
            let duration_ms = (duration_days as i64) * 24 * 60 * 60 * 1000;
            Some(mongodb::bson::DateTime::from_millis(
                chrono::Utc::now().timestamp_millis() + duration_ms
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Create new user
    let new_user = User {
        id: Some(mongodb::bson::oid::ObjectId::new()),
        user_name: request.user_name.clone(),
        user_pwd: hashed_password,
        group_id: 1, // Default group ID
        user_status: 1, // Active status
        user_nick_name: request.user_nick_name.clone(),
        user_email: Some(request.user_email.clone()),
        user_phone: None,
        user_portrait: None,
        user_points: 0, // Default points
        user_end_time: mongodb::bson::DateTime::now(), // Default end time
        vip_level: Some(request.vip_level.unwrap_or(0)),
        vip_end_time,
        created_at: Some(mongodb::bson::DateTime::now()),
    };

    match user_collection.insert_one(new_user, None).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "用户创建成功"
        })),
        Err(e) => {
            eprintln!("Database error when creating user: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "服务器错误"
            }))
        }
    }
}

pub async fn update_user(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    request: web::Json<crate::dto::UpdateUserRequest>,
) -> impl Responder {

    // Parse user ID
    let user_id = match mongodb::bson::oid::ObjectId::parse_str(&request.user_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "无效的用户ID"
            }));
        }
    };

    let user_collection = db.collection::<User>("users");
    
    // Check if user exists
    let existing_user = match user_collection.find_one(doc! { "_id": user_id }, None).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "success": false,
                "message": "用户不存在"
            }));
        }
        Err(e) => {
            eprintln!("Database error when finding user: {}", e);
            return HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "服务器错误"
            }));
        }
    };

    // Build update document
    let mut update_doc = doc! {};

    if let Some(user_name) = &request.user_name {
        // Check if username already exists (excluding current user)
        if let Ok(Some(_)) = user_collection
            .find_one(doc! { "user_name": user_name, "_id": { "$ne": user_id } }, None)
            .await
        {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "用户名已存在"
            }));
        }
        update_doc.insert("user_name", user_name);
    }

    if let Some(user_email) = &request.user_email {
        // Check if email already exists (excluding current user)
        if let Ok(Some(_)) = user_collection
            .find_one(doc! { "user_email": user_email, "_id": { "$ne": user_id } }, None)
            .await
        {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "邮箱已存在"
            }));
        }
        update_doc.insert("user_email", user_email);
    }

    if let Some(user_nick_name) = &request.user_nick_name {
        update_doc.insert("user_nick_name", user_nick_name);
    }

    if let Some(password) = &request.password {
        if !password.is_empty() {
            let hashed_password = match bcrypt::hash(password, 12) {
                Ok(hash) => hash,
                Err(e) => {
                    eprintln!("Password hashing error: {}", e);
                    return HttpResponse::InternalServerError().json(json!({
                        "success": false,
                        "message": "服务器错误"
                    }));
                }
            };
            update_doc.insert("user_password", hashed_password);
        }
    }

    if let Some(vip_level) = request.vip_level {
        update_doc.insert("vip_level", vip_level);
    }

    // Handle VIP duration
    if let (Some(vip_level), Some(duration_days)) = (request.vip_level, request.vip_duration_days) {
        if vip_level > 0 && duration_days > 0 {
            let duration_ms = (duration_days as i64) * 24 * 60 * 60 * 1000;
            let current_vip_end = existing_user.vip_end_time
                .map(|end| end.timestamp_millis())
                .unwrap_or(0);
            let current_time = chrono::Utc::now().timestamp_millis();
            
            let new_vip_end = if current_vip_end > current_time && existing_user.vip_level.unwrap_or(0) == vip_level {
                // Same VIP level, extend current duration
                current_vip_end + duration_ms
            } else {
                // Different VIP level or not VIP, reset duration
                current_time + duration_ms
            };
            
            update_doc.insert("vip_end_time", mongodb::bson::DateTime::from_millis(new_vip_end));
        } else if vip_level == 0 {
            // Remove VIP status
            update_doc.insert("vip_end_time", None::<mongodb::bson::DateTime>);
        }
    }

    if update_doc.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "没有提供更新内容"
        }));
    }

    match user_collection
        .update_one(doc! { "_id": user_id }, doc! { "$set": update_doc }, None)
        .await
    {
        Ok(_) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "用户更新成功"
        })),
        Err(e) => {
            eprintln!("Database error when updating user: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "服务器错误"
            }))
        }
    }
}

pub async fn delete_users(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    request: web::Json<crate::dto::DeleteUserRequest>,
) -> impl Responder {

    // Convert string IDs to ObjectId
    let mut object_ids = Vec::new();
    for user_id_str in &request.user_ids {
        match mongodb::bson::oid::ObjectId::parse_str(user_id_str) {
            Ok(id) => object_ids.push(id),
            Err(_) => {
                return HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "message": format!("无效的用户ID: {}", user_id_str)
                }));
            }
        }
    }

    if object_ids.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": "没有提供有效的用户ID"
        }));
    }

    let user_collection = db.collection::<User>("users");
    
    match user_collection
        .delete_many(doc! { "_id": { "$in": object_ids } }, None)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": format!("成功删除 {} 个用户", result.deleted_count),
            "deleted_count": result.deleted_count
        })),
        Err(e) => {
            eprintln!("Database error when deleting users: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "服务器错误"
            }))
        }
    }
}

// Get single user by ID
pub async fn get_user_by_id(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    user_id: web::Path<String>,
) -> impl Responder {

    // Parse user ID
    let object_id = match mongodb::bson::oid::ObjectId::parse_str(&**user_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "无效的用户ID格式"
            }));
        }
    };

    let user_collection = db.collection::<User>("users");
    
    match user_collection.find_one(
        mongodb::bson::doc! {
            "_id": object_id
        },
        None,
    ).await {
        Ok(Some(user)) => {
            HttpResponse::Ok().json(json!({
                "success": true,
                "user": {
                    "_id": user.id.map_or_else(|| "".to_string(), |id| id.to_string()),
                    "user_name": user.user_name,
                    "user_nick_name": user.user_nick_name,
                    "user_email": user.user_email.unwrap_or_else(|| "".to_string()),
                    "vip_level": user.vip_level.unwrap_or(0),
                    "vip_end_time": user.vip_end_time.map(|dt| dt.to_string()),
                    "created_at": user.created_at.map_or_else(|| "".to_string(), |dt| dt.to_string()),
                    "group_id": user.group_id,
                    "user_status": user.user_status,
                    "user_points": user.user_points
                }
            }))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(json!({
                "success": false,
                "message": "用户不存在"
            }))
        }
        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": "获取用户信息失败"
            }))
        }
    }
}

pub async fn search_users(admin: crate::jwt_auth::AdminUser,
    db: web::Data<Database>,
    request: web::Json<crate::dto::SearchUserRequest>,
) -> impl Responder {

    let page = request.page.unwrap_or(1);
    let limit = request.limit.unwrap_or(20);
    let skip = (page - 1) * limit;

    let user_collection = db.collection::<User>("users");
    
    // Create search query
    let search_query = doc! {
        "$or": [
            { "user_name": { "$regex": &request.query, "$options": "i" } },
            { "user_email": { "$regex": &request.query, "$options": "i" } },
            { "user_nick_name": { "$regex": &request.query, "$options": "i" } }
        ]
    };

    // Get total count
    let total = match user_collection.count_documents(search_query.clone(), None).await {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Database error when counting users: {}", e);
            return HttpResponse::InternalServerError().json(crate::dto::UserListResponse {
                code: 500,
                msg: "服务器错误".to_string(),
                users: vec![],
                total: 0,
            });
        }
    };

    // Get users with pagination
    let find_options = FindOptions::builder()
        .skip(skip as u64)
        .limit(limit as i64)
        .sort(doc! { "created_at": -1 })
        .build();

    let users: Vec<User> = match user_collection.find(search_query, find_options).await {
        Ok(cursor) => cursor.try_collect().await.unwrap_or_else(|_| vec![]),
        Err(e) => {
            eprintln!("Database error when searching users: {}", e);
            return HttpResponse::InternalServerError().json(crate::dto::UserListResponse {
                code: 500,
                msg: "服务器错误".to_string(),
                users: vec![],
                total: 0,
            });
        }
    };

    // Convert to UserAdminInfo format
    let user_infos: Vec<crate::dto::UserAdminInfo> = users
        .into_iter()
        .map(|user| crate::dto::UserAdminInfo {
            id: user.id.map_or_else(|| "".to_string(), |id| id.to_string()),
            user_name: user.user_name,
            user_nick_name: user.user_nick_name,
            user_email: user.user_email.unwrap_or_else(|| "".to_string()),
            vip_level: user.vip_level.unwrap_or(0),
            vip_end_time: user.vip_end_time.map(|end| end.to_string()),
            created_at: user.created_at.map_or_else(|| "".to_string(), |dt| dt.to_string()),
            last_login: None,
        })
        .collect();

    HttpResponse::Ok().json(crate::dto::UserListResponse {
        code: 200,
        msg: "搜索成功".to_string(),
        users: user_infos,
        total: total as i64,
    })
}

// Admin page wrappers with JWT authentication
