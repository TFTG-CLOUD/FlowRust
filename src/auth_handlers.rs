use crate::dto::{AuthResponse, LoginRequest, RegisterRequest, UserResponse};
use crate::models::User;
use actix_web::{web, HttpResponse, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};
use mongodb::{bson::doc, Database};
use uuid::Uuid;

// 旧的login函数已被统一的unified_login替代
// pub async fn login(login_req: web::Json<LoginRequest>, db: web::Data<Database>) -> impl Responder {
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
//                         return HttpResponse::Forbidden().json(AuthResponse {
//                             code: 0,
//                             msg: "用户账户已被禁用".to_string(),
//                             token: None,
//                             user: None,
//                         });
//                     }
// 
//                     // 生成JWT令牌（简化版本，实际项目中应该使用真实的JWT库）
//                     let token = generate_token(&user.id.unwrap().to_string());
// 
//                     HttpResponse::Ok().json(AuthResponse {
//                         code: 1,
//                         msg: "登录成功".to_string(),
//                         token: Some(token),
//                         user: Some(user),
//                     })
//                 }
//                 Ok(false) => HttpResponse::Unauthorized().json(AuthResponse {
//                     code: 0,
//                     msg: "用户名或密码错误".to_string(),
//                     token: None,
//                     user: None,
//                 }),
//                 Err(e) => {
//                     eprintln!("密码验证失败: {}", e);
//                     HttpResponse::InternalServerError().json(AuthResponse {
//                         code: 0,
//                         msg: "服务器错误".to_string(),
//                         token: None,
//                         user: None,
//                     })
//                 }
//             }
//         }
//         Ok(None) => HttpResponse::Unauthorized().json(AuthResponse {
//             code: 0,
//             msg: "用户名或密码错误".to_string(),
//             token: None,
//             user: None,
//         }),
//         Err(e) => {
//             eprintln!("数据库查询失败: {}", e);
//             HttpResponse::InternalServerError().json(AuthResponse {
//                 code: 0,
//                 msg: "服务器错误".to_string(),
//                 token: None,
//                 user: None,
//             })
//         }
//     }
// }

pub async fn register(
    register_req: web::Json<RegisterRequest>,
    db: web::Data<Database>,
) -> impl Responder {
    let user_collection = db.collection::<User>("users");

    // 检查用户名是否已存在
    match user_collection
        .find_one(doc! { "user_name": &register_req.username }, None)
        .await
    {
        Ok(Some(_)) => {
            return HttpResponse::Conflict().json(AuthResponse {
                code: 0,
                msg: "用户名已存在".to_string(),
                token: None,
                user: None,
            });
        }
        Err(e) => {
            eprintln!("检查用户名失败: {}", e);
            return HttpResponse::InternalServerError().json(AuthResponse {
                code: 0,
                msg: "服务器错误".to_string(),
                token: None,
                user: None,
            });
        }
        Ok(None) => {}
    }

    // 检查邮箱是否已存在
    if !register_req.email.is_empty() {
        match user_collection
            .find_one(doc! { "user_email": &register_req.email }, None)
            .await
        {
            Ok(Some(_)) => {
                return HttpResponse::Conflict().json(AuthResponse {
                    code: 0,
                    msg: "邮箱已被注册".to_string(),
                    token: None,
                    user: None,
                });
            }
            Err(e) => {
                eprintln!("检查邮箱失败: {}", e);
                return HttpResponse::InternalServerError().json(AuthResponse {
                    code: 0,
                    msg: "服务器错误".to_string(),
                    token: None,
                    user: None,
                });
            }
            Ok(None) => {}
        }
    }

    // 密码加密
    let hashed_password = match hash(&register_req.password, DEFAULT_COST) {
        Ok(hashed) => hashed,
        Err(e) => {
            eprintln!("密码加密失败: {}", e);
            return HttpResponse::InternalServerError().json(AuthResponse {
                code: 0,
                msg: "服务器错误".to_string(),
                token: None,
                user: None,
            });
        }
    };

    // 创建新用户
    let new_user = User {
        id: None,
        user_name: register_req.username.clone(),
        user_pwd: hashed_password,
        group_id: 2,    // 默认普通用户组
        user_status: 1, // 默认启用
        user_nick_name: Some(register_req.username.clone()),
        user_email: if register_req.email.is_empty() {
            None
        } else {
            Some(register_req.email.clone())
        },
        user_phone: None,
        user_portrait: None,
        user_points: 0,
        user_end_time: mongodb::bson::DateTime::from_millis(253402300799999), // 永不过期
        vip_level: None,
        vip_end_time: None,
        created_at: Some(mongodb::bson::DateTime::now()),
    };

    // 插入用户到数据库
    match user_collection.insert_one(new_user, None).await {
        Ok(result) => {
            if let Some(id) = result.inserted_id.as_object_id() {
                // 获取刚创建的用户信息（不包含密码）
                match user_collection.find_one(doc! { "_id": id }, None).await {
                    Ok(Some(user)) => {
                        let token = crate::jwt_auth::get_jwt_service().generate_token(&user).unwrap();
                        HttpResponse::Created().json(AuthResponse {
                            code: 1,
                            msg: "注册成功".to_string(),
                            token: Some(token),
                            user: Some(user),
                        })
                    }
                    Ok(None) => HttpResponse::InternalServerError().json(AuthResponse {
                        code: 0,
                        msg: "注册成功但获取用户信息失败".to_string(),
                        token: None,
                        user: None,
                    }),
                    Err(e) => {
                        eprintln!("获取用户信息失败: {}", e);
                        HttpResponse::InternalServerError().json(AuthResponse {
                            code: 0,
                            msg: "服务器错误".to_string(),
                            token: None,
                            user: None,
                        })
                    }
                }
            } else {
                HttpResponse::InternalServerError().json(AuthResponse {
                    code: 0,
                    msg: "注册失败".to_string(),
                    token: None,
                    user: None,
                })
            }
        }
        Err(e) => {
            eprintln!("用户注册失败: {}", e);
            HttpResponse::InternalServerError().json(AuthResponse {
                code: 0,
                msg: "服务器错误".to_string(),
                token: None,
                user: None,
            })
        }
    }
}

pub async fn get_current_user(
    req: actix_web::HttpRequest,
    db: web::Data<Database>,
) -> impl Responder {
    // 从Authorization头获取token
    let auth_header = req.headers().get("Authorization");
    let token = match auth_header {
        Some(header) => {
            if let Ok(header_str) = header.to_str() {
                if header_str.starts_with("Bearer ") {
                    header_str[7..].to_string()
                } else {
                    return HttpResponse::Unauthorized().json(UserResponse {
                        code: 0,
                        msg: "无效的认证格式".to_string(),
                        user: None,
                    });
                }
            } else {
                return HttpResponse::Unauthorized().json(UserResponse {
                    code: 0,
                    msg: "无效的认证格式".to_string(),
                    user: None,
                });
            }
        }
        None => {
            return HttpResponse::Unauthorized().json(UserResponse {
                code: 0,
                msg: "缺少认证信息".to_string(),
                user: None,
            });
        }
    };

    // 使用JWT服务验证token
    let jwt_service = crate::jwt_auth::get_jwt_service();
    let claims = match jwt_service.validate_token(&token) {
        Ok(claims) => claims,
        Err(e) => {
            return HttpResponse::Unauthorized().json(UserResponse {
                code: 0,
                msg: format!("认证失败: {}", e),
                user: None,
            });
        }
    };

    // 从数据库获取用户信息
    let user_collection = db.collection::<User>("users");
    match user_collection
        .find_one(
            doc! { "_id": mongodb::bson::oid::ObjectId::parse_str(&claims.sub).unwrap() },
            None,
        )
        .await
    {
        Ok(Some(user)) => HttpResponse::Ok().json(UserResponse {
            code: 1,
            msg: "获取用户信息成功".to_string(),
            user: Some(user),
        }),
        Ok(None) => HttpResponse::NotFound().json(UserResponse {
            code: 0,
            msg: "用户不存在".to_string(),
            user: None,
        }),
        Err(e) => {
            eprintln!("获取用户信息失败: {}", e);
            HttpResponse::InternalServerError().json(UserResponse {
                code: 0,
                msg: "服务器错误".to_string(),
                user: None,
            })
        }
    }
}

pub async fn logout() -> impl Responder {
    // 简化的注销处理，实际项目中可能需要将token加入黑名单
    HttpResponse::Ok().json(serde_json::json!({
        "code": 1,
        "msg": "注销成功"
    }))
}

