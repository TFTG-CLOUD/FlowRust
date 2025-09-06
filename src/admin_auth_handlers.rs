use crate::dto::ApiResponse;
use crate::jwt_auth::{get_jwt_service, AdminUser, AuthenticatedUser};
use crate::models::User;
use actix_web::{web, HttpResponse, Result};
use bcrypt::{verify, DEFAULT_COST};
use mongodb::{bson::doc, Database};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminLoginResponse {
    pub code: i32,
    pub msg: String,
    pub token: Option<String>,
    pub user: Option<User>,
    pub success: Option<bool>,
}

// 管理员登录
// 旧的admin_login函数已被统一的unified_login替代
// pub async fn admin_login(
//     login_req: web::Json<AdminLoginRequest>,
//     db: web::Data<Database>,
// ) -> Result<HttpResponse> {
//     let user_collection = db.collection::<User>("users");
// 
//     // 查找用户
//     match user_collection
//         .find_one(doc! { "user_name": &login_req.username }, None)
//         .await
//     {
//         Ok(Some(user)) => {
//             // 验证密码
//             match verify(&login_req.password, &user.user_pwd) {
//                 Ok(true) => {
//                     // 检查用户状态
//                     if user.user_status != 1 {
//                         return Ok(HttpResponse::Forbidden().json(AdminLoginResponse {
//                             code: 403,
//                             msg: "用户账户已被禁用".to_string(),
//                             token: None,
//                             user: None,
//                             success: Some(false),
//                         }));
//                     }
// 
//                     // 检查是否为管理员
//                     if user.group_id != 1 {
//                         return Ok(HttpResponse::Forbidden().json(AdminLoginResponse {
//                             code: 403,
//                             msg: "需要管理员权限".to_string(),
//                             token: None,
//                             user: None,
//                             success: Some(false),
//                         }));
//                     }
// 
//                     // 生成JWT令牌
//                     match get_jwt_service().generate_token(&user) {
//                         Ok(token) => {
//                             Ok(HttpResponse::Ok().json(AdminLoginResponse {
//                                 code: 200,
//                                 msg: "登录成功".to_string(),
//                                 token: Some(token),
//                                 user: Some(user),
//                                 success: Some(true),
//                             }))
//                         }
//                         Err(e) => {
//                             eprintln!("生成令牌失败: {}", e);
//                             Ok(HttpResponse::InternalServerError().json(AdminLoginResponse {
//                                 code: 500,
//                                 msg: "生成令牌失败".to_string(),
//                                 token: None,
//                                 user: None,
//                                 success: Some(false),
//                             }))
//                         }
//                     }
//                 }
//                 Ok(false) => Ok(HttpResponse::Unauthorized().json(AdminLoginResponse {
//                     code: 401,
//                     msg: "用户名或密码错误".to_string(),
//                     token: None,
//                     user: None,
//                     success: Some(false),
//                 })),
//                 Err(e) => {
//                     eprintln!("密码验证失败: {}", e);
//                     Ok(HttpResponse::InternalServerError().json(AdminLoginResponse {
//                         code: 500,
//                         msg: "服务器错误".to_string(),
//                         token: None,
//                         user: None,
//                         success: Some(false),
//                     }))
//                 }
//             }
//         }
//         Ok(None) => Ok(HttpResponse::Unauthorized().json(AdminLoginResponse {
//             code: 401,
//             msg: "用户名或密码错误".to_string(),
//             token: None,
//             user: None,
//             success: Some(false),
//         })),
//         Err(e) => {
//             eprintln!("查询用户失败: {}", e);
//             Ok(HttpResponse::InternalServerError().json(AdminLoginResponse {
//                 code: 500,
//                 msg: "服务器错误".to_string(),
//                 token: None,
//                 user: None,
//                 success: Some(false),
//             }))
//         }
//     }

// 获取当前管理员信息
pub async fn get_current_admin_info(admin: AdminUser) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse {
        code: 200,
        msg: "获取成功".to_string(),
        data: Some(admin.user),
        success: Some(true),
    }))
}

// 管理员登出 (JWT是无状态的，客户端只需删除token即可)
pub async fn admin_logout() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse {
        code: 200,
        msg: "登出成功".to_string(),
        data: None as Option<()>,
        success: Some(true),
    }))
}

// 刷新令牌
pub async fn refresh_token(user: AuthenticatedUser) -> Result<HttpResponse> {
    match get_jwt_service().generate_token(&user.user) {
        Ok(token) => {
            Ok(HttpResponse::Ok().json(ApiResponse {
                code: 200,
                msg: "令牌刷新成功".to_string(),
                data: Some(serde_json::json!({
                    "token": token
                })),
                success: Some(true),
            }))
        }
        Err(e) => {
            eprintln!("刷新令牌失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse {
                code: 500,
                msg: "刷新令牌失败".to_string(),
                data: None as Option<()>,
                success: Some(false),
            }))
        }
    }
}