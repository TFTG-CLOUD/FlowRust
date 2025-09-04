mod admin_handlers;
mod api_handlers;
mod auth;
mod auth_handlers;
mod auth_middleware;
mod collect_handlers;
mod db;
mod dto;
mod index_manager;
mod init_data;
mod models;
mod scheduled_task;
mod site_data;
mod template;
mod web_handlers;

use admin_handlers::{
    admin_cards_page, admin_users_page, batch_delete_source, batch_delete_vods, batch_set_vip, create_collection, create_config, create_indexes,
    create_or_update_binding, create_type, create_user, create_vod, delete_binding, delete_collection,
    delete_cards, delete_config, delete_type, delete_users, delete_vod, generate_cards, get_batch_delete_progress_handler, get_bindings,
    get_cards_list, get_collect_progress, get_collection_binding_status, get_collections, get_config_by_key,
    get_configs, get_index_status, get_indexes_data, get_running_batch_delete_tasks_handler,
    get_running_tasks, get_scheduled_task_logs, get_scheduled_task_status, get_statistics,
    search_cards, search_users, get_types, get_user_by_id, get_users_list, get_vods_admin, list_indexes, start_collection_collect, start_scheduled_task,
    stop_batch_delete_task_handler, stop_collect_task, stop_scheduled_task, update_collection,
    update_config, update_scheduled_task_config, update_type, update_user, update_vod,
};
use web_handlers::{get_buy_card_config, get_user_vip_info, use_card, vip_check_handler};
use auth_handlers::{get_current_user, login, logout, register};
use auth_middleware::AuthMiddleware;
use collect_handlers::{get_collect_categories, get_collect_videos, start_collect_task};
use site_data::SiteDataManager;

use actix_files::Files;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::dev::{forward_ready, Service, Transform};
use actix_web::http::header::{HeaderValue, CACHE_CONTROL};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    get, middleware, web, App, Error, HttpResponse, HttpServer, Responder, Result,
};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use futures::stream::TryStreamExt;
use mongodb::Database;
use std::env;
use std::future::{ready, Ready};
use std::rc::Rc;

// Static file cache middleware
pub struct StaticCacheMiddleware;

impl<S, B> Transform<S, ServiceRequest> for StaticCacheMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = StaticCacheMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(StaticCacheMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct StaticCacheMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for StaticCacheMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            let is_static = req.path().starts_with("/static/");
            let mut res = service.call(req).await?;

            if is_static {
                // Set cache headers for static files (24 hours)
                res.headers_mut().insert(
                    CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=86400"),
                );
            }

            Ok(res)
        })
    }
}

// Handler to get a list of vods
#[get("/vods")]
async fn get_vods(db: web::Data<Database>) -> impl Responder {
    let collection = db.collection::<models::Vod>("vods");

    match collection.find(None, None).await {
        Ok(cursor) => {
            let vods: Vec<models::Vod> = match cursor.try_collect().await {
                Ok(docs) => docs,
                Err(e) => {
                    eprintln!("Failed to collect documents: {}", e);
                    return HttpResponse::InternalServerError().body("Failed to collect documents");
                }
            };
            HttpResponse::Ok().json(vods)
        }
        Err(e) => {
            eprintln!("Failed to execute find: {}", e);
            HttpResponse::InternalServerError().body("Failed to fetch videos")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the database
    let db = match db::init().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to connect to the database: {}", e);
            // Exit the application if the database connection fails
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "DB connection failed",
            ));
        }
    };

    println!("Database connection successful!");

    // 初始化数据库索引
    let index_manager = index_manager::IndexManager::new(db.clone());
    println!("🔧 正在检查和创建数据库索引...");
    match index_manager.create_all_indexes().await {
        Ok(_) => {
            println!("✅ 数据库索引初始化完成");
        }
        Err(e) => {
            eprintln!("⚠️  索引创建过程中出现警告: {}", e);
            // 不退出应用，因为基本功能仍可使用
        }
    }

    auth::ensure_admin_user_exists(&db).await;

    // 初始化站点数据管理器
    let site_data_manager = SiteDataManager::new(db.clone());
    println!("🔧 正在初始化站点数据缓存...");
    match site_data_manager.initialize().await {
        Ok(_) => {
            println!("✅ 站点数据缓存初始化完成");
        }
        Err(e) => {
            eprintln!("⚠️  站点数据缓存初始化失败: {}", e);
            // 不退出应用，因为基本功能仍可使用
        }
    }

    // 初始化测试数据
    println!("🔧 正在初始化测试数据...");
    match init_data::init_all_data(&db).await {
        Ok(_) => {
            println!("✅ 测试数据初始化完成");
        }
        Err(e) => {
            eprintln!("⚠️  测试数据初始化失败: {}", e);
            // 不退出应用，因为基本功能仍可使用
        }
    }

    // 初始化定时任务配置
    println!("🔧 正在初始化定时任务配置...");
    let scheduled_task_manager =
        std::sync::Arc::new(scheduled_task::ScheduledTaskManager::new(db.clone()));
    match scheduled_task_manager.initialize_config().await {
        Ok(_) => {
            println!("✅ 定时任务配置初始化完成");
        }
        Err(e) => {
            eprintln!("⚠️  定时任务配置初始化失败: {}", e);
            // 不退出应用，因为基本功能仍可使用
        }
    }

    let session_secret_key = Key::generate();

    println!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            // Store the database connection in the application state
            .app_data(web::Data::new(db.clone()))
            // Store the site data manager in the application state
            .app_data(web::Data::new(site_data_manager.clone()))
            // Store the scheduled task manager in the application state
            .app_data(web::Data::new(scheduled_task_manager.clone()))
            // Gzip compression middleware
            .wrap(middleware::Compress::default())
            // Static file cache middleware
            .wrap(StaticCacheMiddleware)
            // Authentication middleware
            .wrap(AuthMiddleware)
            // Session and Flash Messages Middleware
            .wrap(
                FlashMessagesFramework::builder(
                    CookieMessageStore::builder(Key::generate()).build(),
                )
                .build(),
            )
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    session_secret_key.clone(),
                )
                .build(),
            )
            // Web routes
            .service(web::resource("/").route(web::get().to(web_handlers::home_page_wrapper)))
            .service(
                web::resource("/list/{type_id}")
                    .route(web::get().to(web_handlers::list_page_handler_wrapper)),
            )
            .service(
                web::resource("/detail/{vod_id}")
                    .route(web::get().to(web_handlers::video_detail_handler_wrapper)),
            )
            .service(
                web::resource("/play/{vod_id}/{play_index}")
                    .route(web::get().to(web_handlers::video_player_handler_wrapper)),
            )
            .service(
                web::resource("/search")
                    .route(web::get().to(web_handlers::search_page_handler_wrapper)),
            )
            // Static pages
            .service(web::resource("/about").route(web::get().to(web_handlers::about_page)))
            .service(web::resource("/contact").route(web::get().to(web_handlers::contact_page)))
            .service(web::resource("/privacy").route(web::get().to(web_handlers::privacy_page)))
            .service(web::resource("/terms").route(web::get().to(web_handlers::terms_page)))
            // User pages
            .service(
                web::resource("/user/profile")
                    .route(web::get().to(web_handlers::user_profile_page)),
            )
            // Static files with cache configuration
            .service(
                Files::new("/static", "./static")
                    .show_files_listing()
                    .use_etag(true)
                    .use_last_modified(true)
                    .prefer_utf8(true),
            )
            // Admin Web routes
            .service(
                web::resource("/admin/login")
                    .route(web::get().to(web_handlers::login_page))
                    .route(web::post().to(web_handlers::login_post)),
            )
            .service(web::resource("/admin").route(web::get().to(web_handlers::admin_dashboard)))
            .service(web::resource("/admin/logout").route(web::get().to(web_handlers::logout)))
            .service(
                web::resource("/admin/types").route(web::get().to(web_handlers::admin_types_page)),
            )
            .service(
                web::resource("/admin/vods").route(web::get().to(web_handlers::admin_vods_page)),
            )
            .service(
                web::resource("/admin/collect")
                    .route(web::get().to(web_handlers::admin_collect_page)),
            )
            .service(
                web::resource("/admin/collect_vod")
                    .route(web::get().to(web_handlers::admin_collect_vod_page)),
            )
            .service(
                web::resource("/admin/bindings")
                    .route(web::get().to(web_handlers::admin_bindings_page)),
            )
            .service(
                web::resource("/admin/config")
                    .route(web::get().to(web_handlers::admin_config_page)),
            )
            .service(
                web::resource("/admin/indexes")
                    .route(web::get().to(web_handlers::admin_indexes_page)),
            )
            .service(
                web::resource("/admin/cards")
                    .route(web::get().to(admin_handlers::admin_cards_page)),
            )
            .service(
                web::resource("/admin/users")
                    .route(web::get().to(admin_handlers::admin_users_page)),
            )
            .service(
                web::resource("/admin/init-data")
                    .route(web::post().to(web_handlers::init_data_handler)),
            )
            .service(
                web::resource("/admin/refresh-cache")
                    .route(web::post().to(web_handlers::refresh_cache_handler)),
            )
            // API routes
            .service(get_vods)
            .service(
                web::resource("/api/provide/vod").route(web::get().to(api_handlers::provide_vod)),
            )
            .service(
                web::resource("/api/videos/{type_id}")
                    .route(web::get().to(api_handlers::get_videos_by_type)),
            )
            .service(
                web::resource("/api/categories/hierarchy")
                    .route(web::get().to(api_handlers::get_category_hierarchy)),
            )
            .service(
                web::resource("/api/videos/detail/{vod_id}")
                    .route(web::get().to(api_handlers::get_video_details)),
            )
            .service(
                web::resource("/api/filter-options")
                    .route(web::get().to(api_handlers::get_filter_options)),
            )
            // Authentication API routes
            .service(web::resource("/api/auth/login").route(web::post().to(login)))
            .service(web::resource("/api/auth/register").route(web::post().to(register)))
            .service(web::resource("/api/auth/logout").route(web::post().to(logout)))
            .service(web::resource("/api/auth/me").route(web::get().to(get_current_user)))
            // Admin API routes
            .service(
                web::scope("/api/admin")
                    // Category Management
                    .service(
                        web::resource("/types")
                            .route(web::get().to(get_types))
                            .route(web::post().to(create_type)),
                    )
                    .service(
                        web::resource("/types/{id}")
                            .route(web::put().to(update_type))
                            .route(web::delete().to(delete_type)),
                    )
                    // Binding Management
                    .service(
                        web::resource("/bindings")
                            .route(web::get().to(get_bindings))
                            .route(web::post().to(create_or_update_binding)),
                    )
                    .service(
                        web::resource("/bindings/{id}").route(web::delete().to(delete_binding)),
                    )
                    // Website Configuration
                    .service(
                        web::resource("/configs")
                            .route(web::get().to(get_configs))
                            .route(web::post().to(create_config)),
                    )
                    .service(
                        web::resource("/configs/{key}")
                            .route(web::get().to(get_config_by_key))
                            .route(web::put().to(update_config))
                            .route(web::delete().to(delete_config)),
                    )
                    // Collection Management
                    .service(
                        web::resource("/collections")
                            .route(web::get().to(get_collections))
                            .route(web::post().to(create_collection)),
                    )
                    .service(
                        web::resource("/collections/{id}")
                            .route(web::put().to(update_collection))
                            .route(web::delete().to(delete_collection)),
                    )
                    .service(
                        web::resource("/collections/{id}/binding-status")
                            .route(web::get().to(get_collection_binding_status)),
                    )
                    .service(
                        web::resource("/collections/{id}/collect")
                            .route(web::post().to(start_collection_collect)),
                    )
                    .service(
                        web::resource("/collect/progress/{task_id}")
                            .route(web::get().to(get_collect_progress)),
                    )
                    .service(
                        web::resource("/running-tasks").route(web::get().to(get_running_tasks)),
                    )
                    // Video Management
                    .service(
                        web::resource("/vods")
                            .route(web::get().to(get_vods_admin))
                            .route(web::post().to(create_vod))
                            .route(web::delete().to(batch_delete_vods)),
                    )
                    .service(
                        web::resource("/vods/batch-set-vip")
                            .route(web::post().to(batch_set_vip)),
                    )
                    .service(
                        web::resource("/batch-delete-source")
                            .route(web::post().to(batch_delete_source)),
                    )
                    .service(
                        web::resource("/batch-delete/progress/{task_id}")
                            .route(web::get().to(get_batch_delete_progress_handler)),
                    )
                    .service(
                        web::resource("/batch-delete/running-tasks")
                            .route(web::get().to(get_running_batch_delete_tasks_handler)),
                    )
                    .service(
                        web::resource("/batch-delete/stop/{task_id}")
                            .route(web::post().to(stop_batch_delete_task_handler)),
                    )
                    .service(
                        web::resource("/vods/{id}")
                            .route(web::put().to(update_vod))
                            .route(web::delete().to(delete_vod)),
                    )
                    // Index Management
                    .service(web::resource("/indexes/create").route(web::post().to(create_indexes)))
                    .service(
                        web::resource("/indexes/status").route(web::get().to(get_index_status)),
                    )
                    .service(web::resource("/indexes/list").route(web::get().to(list_indexes)))
                    .service(web::resource("/indexes/data").route(web::get().to(get_indexes_data)))
                    // Statistics
                    .service(web::resource("/statistics").route(web::get().to(get_statistics)))
                    // Card Management
                    .service(
                        web::resource("/cards")
                            .route(web::get().to(get_cards_list)),
                    )
                    .service(
                        web::resource("/cards/generate")
                            .route(web::post().to(generate_cards)),
                    )
                    .service(
                        web::resource("/cards/delete")
                            .route(web::post().to(delete_cards)),
                    )
                    .service(
                        web::resource("/cards/search")
                            .route(web::post().to(search_cards)),
                    )
                    // User Management
                    .service(
                        web::resource("/users")
                            .route(web::get().to(get_users_list))
                            .route(web::post().to(create_user))
                            .route(web::delete().to(delete_users)),
                    )
                    .service(
                        web::resource("/users/search")
                            .route(web::post().to(search_users)),
                    )
                    .service(
                        web::resource("/users/{id}")
                            .route(web::get().to(get_user_by_id))
                            .route(web::put().to(update_user)),
                    )
                    // Scheduled Task Management
                    .service(
                        web::resource("/scheduled-task/status")
                            .route(web::get().to(get_scheduled_task_status)),
                    )
                    .service(
                        web::resource("/scheduled-task/start")
                            .route(web::post().to(start_scheduled_task)),
                    )
                    .service(
                        web::resource("/scheduled-task/stop")
                            .route(web::post().to(stop_scheduled_task)),
                    )
                    .service(
                        web::resource("/scheduled-task/config")
                            .route(web::put().to(update_scheduled_task_config)),
                    )
                    .service(
                        web::resource("/scheduled-task/logs")
                            .route(web::get().to(get_scheduled_task_logs)),
                    ),
            )
            // Collect API routes
            .service(
                web::scope("/api/collect")
                    .service(
                        web::resource("/categories").route(web::get().to(get_collect_categories)),
                    )
                    .service(web::resource("/videos").route(web::get().to(get_collect_videos)))
                    .service(web::resource("/start").route(web::post().to(start_collect_task)))
                    .service(
                        web::resource("/stop/{task_id}").route(web::post().to(stop_collect_task)),
                    )
                    .service(
                        web::resource("/progress/{task_id}")
                            .route(web::get().to(get_collect_progress)),
                    ),
            )
            // User API routes
            .service(
                web::scope("/api/user")
                    .service(
                        web::resource("/use-card")
                            .route(web::post().to(use_card)),
                    )
                    .service(
                        web::resource("/vip-info")
                            .route(web::get().to(get_user_vip_info)),
                    )
                    .service(
                        web::resource("/vip-check")
                            .route(web::post().to(vip_check_handler)),
                    ),
            )
            // Config API routes
            .service(
                web::resource("/api/config/buy_card")
                    .route(web::get().to(get_buy_card_config)),
            )
    })
    .bind((
        env::var("SERVER_HOST").unwrap_or("0.0.0.0".to_string()),
        env::var("SERVER_PORT")
            .unwrap_or("8080".to_string())
            .parse()
            .unwrap(),
    ))?
    .run()
    .await
}
