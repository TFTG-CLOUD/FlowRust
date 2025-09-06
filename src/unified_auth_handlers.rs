use actix_web::{web, HttpResponse, Result};
use bcrypt::verify;
use mongodb::{bson::doc, Database};
use serde::{Deserialize, Serialize};
use crate::jwt_auth::{get_jwt_service, AuthenticatedUser, AdminUser};
use crate::models::User;

#[derive(Debug, Serialize, Deserialize)]
pub struct UnifiedLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnifiedLoginResponse {
    pub success: bool,
    pub msg: String,
    pub token: Option<String>,
    pub user: Option<User>,
    pub is_admin: bool,
}

/// 统一登录API - 处理普通用户和管理员登录
pub async fn unified_login(
    login_req: web::Json<UnifiedLoginRequest>,
    db: web::Data<Database>,
) -> Result<HttpResponse> {
    let user_collection = db.collection::<User>("users");

    // 查找用户
    match user_collection
        .find_one(doc! { "user_name": &login_req.username }, None)
        .await
    {
        Ok(Some(user)) => {
            // 验证密码
            match verify(&login_req.password, &user.user_pwd) {
                Ok(true) => {
                    // 检查用户状态
                    if user.user_status != 1 {
                        return Ok(HttpResponse::Forbidden().json(UnifiedLoginResponse {
                            success: false,
                            msg: "用户账户已被禁用".to_string(),
                            token: None,
                            user: None,
                            is_admin: false,
                        }));
                    }

                    // 生成JWT令牌
                    match get_jwt_service().generate_token(&user) {
                        Ok(token) => {
                            let is_admin = user.group_id == 1;
                            
                            Ok(HttpResponse::Ok().json(UnifiedLoginResponse {
                                success: true,
                                msg: "登录成功".to_string(),
                                token: Some(token),
                                user: Some(user),
                                is_admin,
                            }))
                        }
                        Err(e) => {
                            eprintln!("生成令牌失败: {}", e);
                            Ok(HttpResponse::InternalServerError().json(UnifiedLoginResponse {
                                success: false,
                                msg: "服务器错误".to_string(),
                                token: None,
                                user: None,
                                is_admin: false,
                            }))
                        }
                    }
                }
                Ok(false) => Ok(HttpResponse::Unauthorized().json(UnifiedLoginResponse {
                    success: false,
                    msg: "用户名或密码错误".to_string(),
                    token: None,
                    user: None,
                    is_admin: false,
                })),
                Err(e) => {
                    eprintln!("密码验证失败: {}", e);
                    Ok(HttpResponse::InternalServerError().json(UnifiedLoginResponse {
                        success: false,
                        msg: "服务器错误".to_string(),
                        token: None,
                        user: None,
                        is_admin: false,
                    }))
                }
            }
        }
        Ok(None) => Ok(HttpResponse::Unauthorized().json(UnifiedLoginResponse {
            success: false,
            msg: "用户名或密码错误".to_string(),
            token: None,
            user: None,
            is_admin: false,
        })),
        Err(e) => {
            eprintln!("数据库查询失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(UnifiedLoginResponse {
                success: false,
                msg: "服务器错误".to_string(),
                token: None,
                user: None,
                is_admin: false,
            }))
        }
    }
}