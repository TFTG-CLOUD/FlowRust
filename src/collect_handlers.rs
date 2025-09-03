use crate::dto::{Category, JsonResponse, VideoListResponse, VodApiListEntry};
use crate::models::{Binding, Collection, PlaySource, PlayUrl, Vod};
use actix_web::{web, HttpResponse, Responder};
use chrono::Timelike;
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use mongodb::Database;
use reqwest;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// 解析播放地址函数
fn parse_play_urls(vod_play_from: &str, vod_play_url: &Option<String>) -> Vec<PlaySource> {
    let mut play_sources = Vec::new();

    if let Some(play_url) = vod_play_url {
        // 按,符号分割播放源
        let sources: Vec<&str> = vod_play_from.split(',').collect();

        // 如果play_url包含#号，说明是多集内容
        if play_url.contains('#') {
            // 多集内容：按#分割各集
            let episodes: Vec<&str> = play_url
                .split('#')
                .filter(|episode| !episode.trim().is_empty()) // 过滤空的episode
                .collect();

            for (i, source_name) in sources.iter().enumerate() {
                let mut urls = Vec::new();

                // 处理每一集
                for episode in episodes.iter() {
                    if let Some((name, url)) = episode.split_once('$') {
                        urls.push(PlayUrl {
                            name: name.to_string(),
                            url: url.to_string(),
                        });
                    } else {
                        // 如果没有$分割符，可能是特殊情况
                        urls.push(PlayUrl {
                            name: episode.to_string(),
                            url: String::new(),
                        });
                    }
                }

                if !urls.is_empty() {
                    play_sources.push(PlaySource {
                        source_name: source_name.trim().to_string(),
                        urls,
                    });
                }
            }
        } else {
            // 单集内容：直接按$分割
            for source_name in sources.iter() {
                let mut urls = Vec::new();

                if let Some((name, url)) = play_url.split_once('$') {
                    urls.push(PlayUrl {
                        name: name.to_string(),
                        url: url.to_string(),
                    });
                } else {
                    // 如果没有$分割符，可能是纯URL
                    urls.push(PlayUrl {
                        name: String::new(),
                        url: play_url.to_string(),
                    });
                }

                if !urls.is_empty() {
                    play_sources.push(PlaySource {
                        source_name: source_name.trim().to_string(),
                        urls,
                    });
                }
            }
        }
    }

    play_sources
}

#[derive(Deserialize)]
pub struct CollectCategoriesQuery {
    url: String,
}

#[derive(Deserialize)]
pub struct CollectVideosQuery {
    url: String,
    page: Option<u32>,
    limit: Option<u32>,
    #[serde(rename = "type")]
    type_id: Option<String>,
    wd: Option<String>,
}

#[derive(Deserialize)]
pub struct CollectStartRequest {
    collection_id: String,
    source_flag: String,
    api_url: String,
    #[serde(rename = "type")]
    collect_type: String,
    video_ids: Option<Vec<String>>,
    hours: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CollectProgress {
    pub status: String,
    pub current_page: u32,
    pub total_pages: u32,
    pub success: u32,
    pub failed: u32,
    pub log: String,
}

impl Default for CollectProgress {
    fn default() -> Self {
        Self {
            status: "unknown".to_string(),
            current_page: 0,
            total_pages: 0,
            success: 0,
            failed: 0,
            log: "未知状态".to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct CollectProgressResponse {
    success: bool,
    progress: CollectProgress,
}

// 类型别名简化复杂类型
type TaskProgressMap = std::collections::HashMap<
    String,
    (CollectProgress, String, Option<tokio::task::JoinHandle<()>>),
>;
type TaskProgressStore = tokio::sync::RwLock<TaskProgressMap>;

// 全局任务进度存储
static TASK_PROGRESS: std::sync::OnceLock<TaskProgressStore> = std::sync::OnceLock::new();

// 初始化任务进度存储
fn get_task_progress_store() -> &'static TaskProgressStore {
    TASK_PROGRESS.get_or_init(|| tokio::sync::RwLock::new(std::collections::HashMap::new()))
}

// 获取任务进度
pub async fn get_task_progress(task_id: &str) -> Option<CollectProgress> {
    let store = get_task_progress_store();
    let progress_map = store.read().await;
    progress_map
        .get(task_id)
        .map(|(progress, _, _)| progress.clone())
}

// 更新任务进度
async fn update_task_progress(task_id: &str, progress: CollectProgress, collection_name: String) {
    let store = get_task_progress_store();
    let mut progress_map = store.write().await;
    if let Some((current_progress, current_name, handle)) = progress_map.get_mut(task_id) {
        *current_progress = progress;
        *current_name = collection_name;
        // 保持原有的handle不变，不需要克隆
    } else {
        progress_map.insert(task_id.to_string(), (progress, collection_name, None));
    }
}

// 停止任务
pub async fn stop_task(task_id: &str) -> bool {
    let store = get_task_progress_store();
    let mut progress_map = store.write().await;

    if let Some((mut progress, collection_name, handle)) = progress_map.remove(task_id) {
        // 取消任务
        if let Some(task_handle) = handle {
            task_handle.abort();
        }

        // 标记任务为已停止
        progress.status = "stopped".to_string();
        progress.log = "任务已手动停止".to_string();

        // 将任务重新插入，但状态为已停止且清除句柄
        progress_map.insert(task_id.to_string(), (progress, collection_name, None));

        true
    } else {
        false
    }
}

// 获取所有运行中的任务
pub async fn get_all_running_tasks() -> Vec<serde_json::Value> {
    let store = get_task_progress_store();
    let progress_map = store.read().await;

    let mut tasks = Vec::new();
    let now = chrono::Utc::now();

    for (task_id, (progress, collection_name, _)) in progress_map.iter() {
        // 只返回运行中的任务
        let should_include = progress.status == "running";

        if should_include {
            tasks.push(serde_json::json!({
                "task_id": task_id,
                "collection_name": collection_name,
                "status": progress.status,
                "current_page": progress.current_page,
                "total_pages": progress.total_pages,
                "success": progress.success,
                "failed": progress.failed,
                "log": progress.log,
                "start_time": format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second())
            }));
        }
    }

    tasks
}

// 获取采集源分类列表
pub async fn get_collect_categories(query: web::Query<CollectCategoriesQuery>) -> impl Responder {
    let mut api_url = query.url.clone();
    if api_url.contains('?') {
        // 如果URL已包含?，检查是否以?结尾或已有参数
        if api_url.ends_with('?') {
            api_url.push_str("ac=list");
        } else {
            api_url.push_str("&ac=list");
        }
    } else {
        api_url.push_str("?ac=list");
    }

    match reqwest::get(&api_url).await {
        Ok(response) => match response.text().await {
            Ok(response_text) => {
                // eprintln!("API Response: {}", response_text);
                match serde_json::from_str::<JsonResponse<Category>>(&response_text) {
                    Ok(api_response) => {
                        if api_response.code == 1 {
                            HttpResponse::Ok().json(serde_json::json!({
                                "success": true,
                                "categories": api_response.categories
                            }))
                        } else {
                            HttpResponse::Ok().json(serde_json::json!({
                                "success": false,
                                "message": "API返回错误"
                            }))
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse API response: {}", e);
                        HttpResponse::Ok().json(serde_json::json!({
                            "success": false,
                            "message": "解析API响应失败"
                        }))
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get response text: {}", e);
                HttpResponse::Ok().json(serde_json::json!({
                    "success": false,
                    "message": "获取响应失败"
                }))
            }
        },
        Err(e) => {
            eprintln!("Failed to fetch categories: {}", e);
            HttpResponse::Ok().json(serde_json::json!({
                "success": false,
                "message": "获取分类列表失败"
            }))
        }
    }
}

// 获取采集源视频列表
pub async fn get_collect_videos(query: web::Query<CollectVideosQuery>) -> impl Responder {
    let mut api_url = format!("{}?ac=detail", query.url);

    // 添加查询参数
    let mut params = Vec::new();

    if let Some(page) = query.page {
        params.push(format!("pg={}", page));
    }

    if let Some(type_id) = &query.type_id {
        params.push(format!("t={}", type_id));
    }

    if let Some(wd) = &query.wd {
        params.push(format!("wd={}", urlencoding::encode(wd)));
    }

    if !params.is_empty() {
        api_url.push('&');
        api_url.push_str(&params.join("&"));
    }

    match reqwest::get(&api_url).await {
        Ok(response) => match response.text().await {
            Ok(response_text) => match serde_json::from_str::<VideoListResponse>(&response_text) {
                Ok(api_response) => {
                    if api_response.code == 1 {
                        let limit = query.limit.unwrap_or(20) as usize;
                        let total_pages = (api_response.total as f64 / limit as f64).ceil() as u32;

                        HttpResponse::Ok().json(serde_json::json!({
                            "success": true,
                            "videos": api_response.list,
                            "total": api_response.total,
                            "total_pages": total_pages
                        }))
                    } else {
                        HttpResponse::Ok().json(serde_json::json!({
                            "success": false,
                            "message": "API返回错误"
                        }))
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse API response: {}", e);
                    HttpResponse::Ok().json(serde_json::json!({
                        "success": false,
                        "message": "解析API响应失败"
                    }))
                }
            },
            Err(e) => {
                eprintln!("Failed to get response text: {}", e);
                HttpResponse::Ok().json(serde_json::json!({
                    "success": false,
                    "message": "获取响应失败"
                }))
            }
        },
        Err(e) => {
            eprintln!("Failed to fetch videos: {}", e);
            HttpResponse::Ok().json(serde_json::json!({
                "success": false,
                "message": "获取视频列表失败"
            }))
        }
    }
}

// 开始采集任务
pub async fn start_collect_task(
    db: web::Data<Database>,
    request: web::Json<CollectStartRequest>,
) -> impl Responder {
    // 生成任务ID
    let task_id = ObjectId::new().to_hex();

    // 获取采集源配置
    let collections_collection = db.collection::<Collection>("collections");
    let collection = match collections_collection
        .find_one(
            doc! {"_id": ObjectId::parse_str(&request.collection_id).unwrap()},
            None,
        )
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "message": "采集源不存在"
            }));
        }
        Err(e) => {
            eprintln!("Failed to get collection: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "获取采集源失败"
            }));
        }
    };

    // 初始化任务进度
    let initial_progress = CollectProgress {
        status: "running".to_string(),
        current_page: 0,
        total_pages: 1,
        success: 0,
        failed: 0,
        log: "正在启动采集任务...".to_string(),
    };
    update_task_progress(
        &task_id,
        initial_progress.clone(),
        collection.collect_name.clone(),
    )
    .await;

    // 启动后台采集任务
    let db_clone = db.clone();
    let task_id_clone = task_id.clone();
    let collection_name_clone = collection.collect_name.clone();
    let handle = tokio::spawn(async move {
        let hours = request.hours.map(|h| h.to_string());
        let task_id_for_closure = task_id_clone.clone();
        match start_batch_collect(&db_clone, collection.clone(), hours, task_id_clone).await {
            Ok(_) => {
                // 任务正常完成
                let mut progress = get_task_progress(&task_id_for_closure)
                    .await
                    .unwrap_or_default();
                progress.status = "completed".to_string();
                progress.log = format!(
                    "采集完成，成功: {}，失败: {}",
                    progress.success, progress.failed
                );
                update_task_progress(&task_id_for_closure, progress, collection_name_clone).await;
            }
            Err(e) => {
                // 任务失败
                let mut progress = get_task_progress(&task_id_for_closure)
                    .await
                    .unwrap_or_default();
                progress.status = "failed".to_string();
                progress.log = format!("采集失败: {}", e);
                update_task_progress(&task_id_for_closure, progress, collection_name_clone).await;
            }
        }
    });

    // 存储任务句柄
    let store = get_task_progress_store();
    let mut progress_map = store.write().await;
    if let Some((progress, collection_name, _)) = progress_map.get_mut(&task_id) {
        *progress_map.get_mut(&task_id).unwrap() =
            (progress.clone(), collection_name.clone(), Some(handle));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "task_id": task_id,
        "message": "采集任务已启动"
    }))
}

// 获取采集进度
pub async fn get_collect_progress(path: web::Path<String>) -> impl Responder {
    let task_id = path.into_inner();

    if let Some(progress) = get_task_progress(&task_id).await {
        HttpResponse::Ok().json(CollectProgressResponse {
            success: true,
            progress,
        })
    } else {
        HttpResponse::Ok().json(CollectProgressResponse {
            success: false,
            progress: CollectProgress {
                status: "not_found".to_string(),
                current_page: 0,
                total_pages: 0,
                success: 0,
                failed: 0,
                log: "任务不存在".to_string(),
            },
        })
    }
}

// 带超时的HTTP请求
async fn fetch_with_timeout(
    url: &str,
    timeout_secs: u64,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use tokio::time::{timeout, Duration};

    match timeout(Duration::from_secs(timeout_secs), reqwest::get(url)).await {
        Ok(Ok(response)) => match response.text().await {
            Ok(text) => Ok(text),
            Err(e) => Err(format!("读取响应失败: {}", e).into()),
        },
        Ok(Err(e)) => Err(format!("请求失败: {}", e).into()),
        Err(_) => Err("请求超时".into()),
    }
}

// 带重试的获取总页数函数
async fn get_total_pages_with_retry(
    api_url: &str,
    max_retries: usize,
    timeout_secs: u64,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;

    for attempt in 1..=max_retries {
        let first_page_url = format!("{}&pg=1", api_url);

        println!("🔄 获取总页数 (尝试 {}/{})", attempt, max_retries);

        match fetch_with_timeout(&first_page_url, timeout_secs).await {
            Ok(response_text) => match serde_json::from_str::<VideoListResponse>(&response_text) {
                Ok(api_response) => {
                    if api_response.code == 1 {
                        let total_pages =
                            (api_response.total as f64 / api_response.limit as f64).ceil() as u32;
                        println!("✅ 获取总页数成功: {} 页", total_pages);
                        return Ok(total_pages);
                    } else {
                        let error = format!("API返回错误: {:?}", api_response);
                        println!("❌ {}", error);
                        last_error = Some(error.into());
                    }
                }
                Err(e) => {
                    let error = format!("解析API响应失败: {}", e);
                    println!("❌ {}", error);
                    last_error = Some(error.into());
                }
            },
            Err(e) => {
                let error = format!("获取总页数失败: {}", e);
                println!("❌ {}", error);
                last_error = Some(error.into());
            }
        }

        // 如果不是最后一次尝试，等待一段时间再重试
        if attempt < max_retries {
            let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32 - 1));
            println!("⏳ 等待 {} 秒后重试...", delay.as_secs());
            tokio::time::sleep(delay).await;
        }
    }

    Err(last_error.unwrap_or_else(|| "未知错误".into()))
}

// 批量采集主函数
pub async fn start_batch_collect(
    db: &Database,
    collection: Collection,
    hours: Option<String>,
    task_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 初始化任务进度
    let initial_progress = CollectProgress {
        status: "running".to_string(),
        current_page: 0,
        total_pages: 1,
        success: 0,
        failed: 0,
        log: "正在获取总页数...".to_string(),
    };
    update_task_progress(
        &task_id,
        initial_progress.clone(),
        collection.collect_name.clone(),
    )
    .await;

    // 构建API URL
    let mut api_url = collection.collect_url.clone();
    if api_url.contains('?') {
        // 如果URL已包含?，检查是否以?结尾或已有参数
        if api_url.ends_with('?') {
            api_url.push_str("ac=detail");
        } else {
            api_url.push_str("&ac=detail");
        }
    } else {
        api_url.push_str("?ac=detail");
    }

    // 添加hours参数
    if let Some(h) = hours {
        api_url.push_str(&format!("&h={}", h));
    }

    // 获取总页数（带重试机制）
    let total_pages = match get_total_pages_with_retry(&api_url, 3, 30).await {
        Ok(pages) => pages,
        Err(e) => {
            eprintln!("❌ 获取总页数失败，已重试3次: {}", e);
            return Err(format!("获取总页数失败: {}", e).into());
        }
    };

    // 更新进度信息
    let mut progress = initial_progress;
    progress.total_pages = total_pages;
    progress.log = format!("开始采集，总页数: {}", total_pages);
    update_task_progress(&task_id, progress.clone(), collection.collect_name.clone()).await;

    // 逐页采集
    for page in 1..=total_pages {
        // 检查任务是否被停止
        if let Some(current_progress) = get_task_progress(&task_id).await {
            if current_progress.status == "stopped" {
                return Ok(()); // 任务已被停止，直接返回
            }
        }

        progress.current_page = page;
        progress.log = format!("正在采集第 {}/{} 页", page, total_pages);
        update_task_progress(&task_id, progress.clone(), collection.collect_name.clone()).await;

        let page_url = format!("{}&pg={}", api_url, page);
        if let Err(e) = collect_page(db, &collection, &page_url, &mut progress, &task_id).await {
            progress.failed += 1;
            progress.log = format!("第 {} 页采集失败: {}", page, e);
            update_task_progress(&task_id, progress.clone(), collection.collect_name.clone()).await;
            continue;
        }

        // 添加延时避免请求过快
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // 完成采集
    progress.status = "completed".to_string();
    progress.log = format!(
        "采集完成，成功: {}，失败: {}",
        progress.success, progress.failed
    );
    update_task_progress(&task_id, progress, collection.collect_name).await;

    Ok(())
}

// 采集单页数据（带超时）
async fn collect_page(
    db: &Database,
    collection: &Collection,
    page_url: &str,
    progress: &mut CollectProgress,
    task_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response_text = fetch_with_timeout(page_url, 30).await?;
    let api_response: VideoListResponse = serde_json::from_str(&response_text)?;

    if api_response.code != 1 {
        return Err(format!("API返回错误: {:?}", api_response).into());
    }

    let mut page_success = 0;
    let mut page_failed = 0;

    for vod_data in api_response.list {
        // 检查任务是否被停止
        if let Some(current_progress) = get_task_progress(task_id).await {
            if current_progress.status == "stopped" {
                return Ok(()); // 任务已被停止，直接返回
            }
        }

        match collect_single_video(db, collection, &vod_data).await {
            Ok(_) => page_success += 1,
            Err(e) => {
                eprintln!("采集视频失败 {}: {}", vod_data.vod_name, e);
                page_failed += 1;
            }
        }
    }

    progress.success += page_success;
    progress.failed += page_failed;
    progress.log = format!(
        "本页采集完成，成功: {}，失败: {}",
        page_success, page_failed
    );
    update_task_progress(task_id, progress.clone(), collection.collect_name.clone()).await;

    Ok(())
}

// 采集单个视频
pub async fn collect_single_video(
    db: &Database,
    collection: &Collection,
    vod_data: &VodApiListEntry,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // 查找分类绑定
    let bindings_collection = db.collection::<Binding>("bindings");
    let binding = bindings_collection
        .find_one(
            doc! {
                "source_flag": &collection.collect_name,
                "external_id": vod_data.type_id.to_string()
            },
            None,
        )
        .await?;

    let local_type_id = match binding {
        Some(b) => b.local_type_id,
        None => {
            eprintln!(
                "未找到分类绑定: source_flag={}, external_id={}",
                collection.collect_name, vod_data.type_id
            );
            return Err("未找到分类绑定".into());
        }
    };

    // 检查视频是否已存在（基于vod_name和vod_year）
    let vods_collection = db.collection::<Vod>("vods");
    let existing_vod = if let Some(ref year) = vod_data.vod_year {
        vods_collection
            .find_one(
                doc! {
                    "vod_name": &vod_data.vod_name,
                    "vod_year": year
                },
                None,
            )
            .await?
    } else {
        vods_collection
            .find_one(
                doc! {
                    "vod_name": &vod_data.vod_name
                },
                None,
            )
            .await?
    };

    let current_time = DateTime::from_millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    );

    if let Some(mut existing) = existing_vod {
        // 更新现有视频 - 处理播放源替换
        let new_play_sources = parse_play_urls(&vod_data.vod_play_from, &vod_data.vod_play_url);

        // 根据source_name匹配更新播放源
        let mut updated = false;
        for new_source in new_play_sources {
            if let Some(pos) = existing
                .vod_play_urls
                .iter()
                .position(|s| s.source_name == new_source.source_name)
            {
                // 替换现有播放源
                existing.vod_play_urls[pos] = new_source;
                updated = true;
            } else {
                // 添加新播放源
                existing.vod_play_urls.push(new_source);
                updated = true;
            }
        }

        if updated {
            existing.vod_pubdate = current_time;
            existing.vod_remarks = Some(vod_data.vod_remarks.clone());
            vods_collection
                .replace_one(doc! { "_id": existing.id }, &existing, None)
                .await?;
        }

        Ok(true)
    } else {
        // 创建新视频
        let new_vod = Vod {
            id: None,
            vod_name: vod_data.vod_name.clone(),
            type_id: local_type_id,
            vod_status: vod_data.vod_status.unwrap_or(1),
            vod_class: vod_data.vod_class.clone(),
            vod_pic: vod_data.vod_pic.clone(),
            vod_actor: vod_data.vod_actor.clone(),
            vod_director: vod_data.vod_director.clone(),
            vod_remarks: Some(vod_data.vod_remarks.clone()),
            vod_pubdate: current_time.clone(),
            vod_area: vod_data.vod_area.clone(),
            vod_lang: vod_data.vod_lang.clone(),
            vod_year: vod_data.vod_year.clone(),
            vod_content: vod_data.vod_content.clone(),
            vod_hits: Some(0),
            vod_hits_day: Some(0),
            vod_hits_week: Some(0),
            vod_hits_month: Some(0),
            vod_score: Some("0.0".to_string()),
            need_vip: 0,
            vod_play_urls: parse_play_urls(&vod_data.vod_play_from, &vod_data.vod_play_url),
        };

        // 如果启用了图片本地化，下载海报
        let final_vod_pic = if collection.collect_sync_pic_opt == 1 {
            if let Some(ref pic_url) = vod_data.vod_pic {
                match download_image_to_local_with_config(pic_url, collection).await {
                    Ok(local_path) => Some(local_path),
                    Err(e) => {
                        eprintln!("下载图片失败 {}: {}", pic_url, e);
                        vod_data.vod_pic.clone()
                    }
                }
            } else {
                vod_data.vod_pic.clone()
            }
        } else {
            vod_data.vod_pic.clone()
        };

        let mut final_vod = new_vod;
        final_vod.vod_pic = final_vod_pic;

        vods_collection.insert_one(&final_vod, None).await?;
        Ok(true)
    }
}

// 下载图片到本地（带重试机制和webp转换）
async fn download_image_to_local_with_config(
    image_url: &str,
    collection: &Collection,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 创建static目录（如果不存在）
    tokio::fs::create_dir_all("static/images").await?;

    // 获取重试次数和webp转换设置
    let max_retries = if collection.collect_download_retry > 0 {
        collection.collect_download_retry as usize
    } else {
        3 // 默认重试3次
    };

    let convert_to_webp = collection.collect_convert_webp == 1;

    // 生成文件名
    let file_extension = if convert_to_webp {
        "webp"
    } else {
        image_url.split('.').last().unwrap_or("jpg")
    };
    let file_name = format!("{}.{}", uuid::Uuid::new_v4(), file_extension);
    let local_path = format!("static/images/{}", file_name);

    // 重试下载
    let mut last_error = None;
    for attempt in 1..=max_retries {
        match download_and_process_image(image_url, &local_path, convert_to_webp, attempt).await {
            Ok(_) => {
                println!("图片下载成功: {} (尝试次数: {})", image_url, attempt);
                return Ok(format!("/static/images/{}", file_name));
            }
            Err(e) => {
                let error_msg = format!("下载失败 (尝试 {}/{}): {}", attempt, max_retries, e);
                println!("{}", error_msg);
                last_error = Some(e);

                // 如果不是最后一次尝试，等待一段时间再重试
                if attempt < max_retries {
                    let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32 - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    // 所有重试都失败了
    Err(last_error.unwrap_or_else(|| "未知下载错误".into()))
}

// 下载并处理图片
async fn download_and_process_image(
    image_url: &str,
    local_path: &str,
    convert_to_webp: bool,
    attempt: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 下载图片
    let response = reqwest::get(image_url)
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP错误: {}", response.status()).into());
    }

    let image_data = response
        .bytes()
        .await
        .map_err(|e| format!("读取响应数据失败: {}", e))?;

    if convert_to_webp {
        // 转换为webp格式
        convert_to_webp_format(&image_data, local_path).await?;
    } else {
        // 直接保存原格式
        tokio::fs::write(local_path, &image_data)
            .await
            .map_err(|e| format!("保存文件失败: {}", e))?;
    }

    Ok(())
}

// 转换图片为webp格式
async fn convert_to_webp_format(
    image_data: &[u8],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use image::io::Reader as ImageReader;
    use std::io::Cursor;

    // 在tokio线程池中执行图片转换
    let output_path_owned = output_path.to_string();
    let image_data_owned = image_data.to_vec();

    tokio::task::spawn_blocking(move || {
        // 从字节数据读取图片
        let reader = ImageReader::new(Cursor::new(&image_data_owned))
            .with_guessed_format()
            .map_err(|e| format!("无法识别图片格式: {}", e))?;

        let img = reader
            .decode()
            .map_err(|e| format!("图片解码失败: {}", e))?;

        // 转换为RGB格式
        let rgb_image = img.to_rgb8();

        // 使用webp编码器编码
        let webp_data =
            webp::Encoder::from_rgb(rgb_image.as_raw(), rgb_image.width(), rgb_image.height())
                .encode(75.0); // 质量75

        // 保存webp文件 (需要解引用WebPMemory)
        std::fs::write(output_path_owned, &*webp_data)
            .map_err(|e| format!("保存webp文件失败: {}", e))?;

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await
    .map_err(|e| format!("图片转换任务失败: {}", e))?
}

// 保持原有函数用于向后兼容
async fn download_image_to_local(
    image_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 创建一个默认的collection配置用于兼容性
    let default_collection = Collection {
        id: None,
        collect_name: "default".to_string(),
        collect_url: "".to_string(),
        collect_type: 1,
        collect_mid: 1,
        collect_appid: "".to_string(),
        collect_appkey: "".to_string(),
        collect_param: "".to_string(),
        collect_filter: "".to_string(),
        collect_filter_from: "".to_string(),
        collect_opt: 0,
        collect_sync_pic_opt: 1,
        collect_remove_ad: 1,
        collect_convert_webp: 0,   // 默认不转换webp
        collect_download_retry: 3, // 默认重试3次
        collect_status: 1,
        created_at: mongodb::bson::DateTime::now(),
        updated_at: mongodb::bson::DateTime::now(),
    };

    download_image_to_local_with_config(image_url, &default_collection).await
}

// 采集单个视频详情（保留原有函数用于兼容性）
pub async fn collect_video_detail(
    db: web::Data<Database>,
    api_url: &str,
    vod_id: &str,
    source_flag: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // 构建详情API URL
    let detail_url = format!("{}?ac=detail&h=24&ids={}", api_url, vod_id);

    // 获取视频详情
    let response = reqwest::get(&detail_url).await?;
    let api_response: JsonResponse<VodApiListEntry> = response.json().await?;

    if api_response.code != 1 || api_response.list.is_empty() {
        return Err("获取视频详情失败".into());
    }

    let vod_data = &api_response.list[0];

    // 查找分类绑定
    let bindings_collection = db.collection::<Binding>("bindings");
    let binding = bindings_collection
        .find_one(
            doc! {
                "source_flag": source_flag,
                "external_id": vod_data.type_id.to_string()
            },
            None,
        )
        .await?;

    let local_type_id = match binding {
        Some(b) => b.local_type_id,
        None => {
            eprintln!(
                "未找到分类绑定: source_flag={}, external_id={}",
                source_flag, vod_data.type_id
            );
            return Err("未找到分类绑定".into());
        }
    };

    // 检查视频是否已存在
    let vods_collection = db.collection::<Vod>("vods");
    let existing_vod = vods_collection
        .find_one(
            doc! {
                "vod_name": &vod_data.vod_name
            },
            None,
        )
        .await?;

    let current_time = DateTime::from_millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    );

    if let Some(mut existing) = existing_vod {
        // 更新现有视频 - 使用VodApiListEntry中的所有可用字段
        existing.vod_name = vod_data.vod_name.clone();
        existing.type_id = local_type_id;
        existing.vod_status = 1; // 默认状态
                                 // 更新所有可用字段
        existing.vod_remarks = Some(vod_data.vod_remarks.clone());
        if let Some(ref pubdate) = vod_data.vod_pubdate {
            existing.vod_pubdate = current_time;
        }
        if let Some(ref class) = vod_data.vod_class {
            existing.vod_class = Some(class.clone());
        }
        if let Some(ref pic) = vod_data.vod_pic {
            existing.vod_pic = Some(pic.clone());
        }
        if let Some(ref actor) = vod_data.vod_actor {
            existing.vod_actor = Some(actor.clone());
        }
        if let Some(ref director) = vod_data.vod_director {
            existing.vod_director = Some(director.clone());
        }
        if let Some(ref area) = vod_data.vod_area {
            existing.vod_area = Some(area.clone());
        }
        if let Some(ref lang) = vod_data.vod_lang {
            existing.vod_lang = Some(lang.clone());
        }
        if let Some(ref year) = vod_data.vod_year {
            existing.vod_year = Some(year.clone());
        }
        if let Some(ref content) = vod_data.vod_content {
            existing.vod_content = Some(content.clone());
        }
        // 解析播放地址
        if !vod_data.vod_play_from.is_empty() {
            existing.vod_play_urls =
                parse_play_urls(&vod_data.vod_play_from, &vod_data.vod_play_url);
        }

        vods_collection
            .replace_one(doc! { "_id": existing.id }, &existing, None)
            .await?;
    } else {
        // 创建新视频 - 只使用VodApiListEntry中实际存在的字段
        let new_vod = Vod {
            id: None,
            vod_name: vod_data.vod_name.clone(),
            type_id: local_type_id,
            vod_status: vod_data.vod_status.unwrap_or(1),
            vod_class: vod_data.vod_class.clone(),
            vod_pic: vod_data.vod_pic.clone(),
            vod_actor: vod_data.vod_actor.clone(),
            vod_director: vod_data.vod_director.clone(),
            vod_remarks: Some(vod_data.vod_remarks.clone()),
            vod_pubdate: current_time.clone(),
            vod_area: vod_data.vod_area.clone(),
            vod_lang: vod_data.vod_lang.clone(),
            vod_year: vod_data.vod_year.clone(),
            vod_content: vod_data.vod_content.clone(),
            vod_hits: Some(0),
            vod_hits_day: Some(0),
            vod_hits_week: Some(0),
            vod_hits_month: Some(0),
            vod_score: Some("0.0".to_string()),
            need_vip: 0,
            vod_play_urls: parse_play_urls(&vod_data.vod_play_from, &vod_data.vod_play_url),
        };

        vods_collection.insert_one(&new_vod, None).await?;
    }

    Ok(true)
}
