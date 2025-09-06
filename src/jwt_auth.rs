use actix_web::{
    dev::Payload, error::ErrorInternalServerError, Error, HttpMessage, HttpRequest,
    FromRequest, Result,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use crate::models::User;
use futures::future::{ready, Ready};
use std::env;
use lazy_static::lazy_static;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // 用户ID
    pub username: String,   // 用户名
    pub group_id: i32,      // 用户组ID
    pub user_status: i32,   // 用户状态
    pub exp: usize,         // 过期时间
    pub iat: usize,         // 签发时间
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    pub fn new() -> Self {
        let secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());
        
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
        }
    }

    // 生成JWT token
    pub fn generate_token(&self, user: &User) -> Result<String, Error> {
        let now = Utc::now();
        let exp = now + Duration::hours(24); // 24小时过期
        
        let claims = Claims {
            sub: user.id.as_ref().unwrap().to_string(),
            username: user.user_name.clone(),
            group_id: user.group_id,
            user_status: user.user_status,
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| ErrorInternalServerError(format!("Failed to generate token: {}", e)))
    }

    // 验证JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims, Error> {
        let validation = Validation::new(Algorithm::HS256);
        
        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| ErrorInternalServerError(format!("Invalid token: {}", e)))
    }

    // 从token中获取用户信息
    pub fn get_user_from_token(&self, token: &str) -> Result<User, Error> {
        let claims = self.validate_token(token)?;
        
        Ok(User {
            id: Some(mongodb::bson::oid::ObjectId::parse_str(&claims.sub).unwrap()),
            user_name: claims.username,
            user_pwd: "".to_string(), // 不需要密码
            group_id: claims.group_id,
            user_status: claims.user_status,
            user_nick_name: None,
            user_email: None,
            user_phone: None,
            user_portrait: None,
            user_points: 0,
            user_end_time: mongodb::bson::DateTime::now(),
            vip_level: None,
            vip_end_time: None,
            created_at: None,
        })
    }
}

// 全局JWT服务实例
lazy_static::lazy_static! {
    static ref JWT_SERVICE: JwtService = JwtService::new();
}

// 管理员用户提取器
pub struct AdminUser {
    pub user: User,
    pub claims: Claims,
}

impl FromRequest for AdminUser {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // 从Authorization头获取token
        let auth_header = req.headers().get("Authorization");
        
        match auth_header {
            Some(header) => {
                if let Ok(auth_str) = header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];
                        
                        match JWT_SERVICE.validate_token(token) {
                            Ok(claims) => {
                                // 检查用户状态
                                if claims.user_status != 1 {
                                    return ready(Err(ErrorInternalServerError("用户已被禁用")));
                                }
                                
                                // 检查用户组权限 - 假设 group_id = 1 是管理员组
                                if claims.group_id != 1 {
                                    return ready(Err(ErrorInternalServerError("需要管理员权限")));
                                }
                                
                                // 构建用户对象
                                let user = User {
                                    id: Some(mongodb::bson::oid::ObjectId::parse_str(&claims.sub).unwrap()),
                                    user_name: claims.username.clone(),
                                    user_pwd: "".to_string(),
                                    group_id: claims.group_id,
                                    user_status: claims.user_status,
                                    user_nick_name: None,
                                    user_email: None,
                                    user_phone: None,
                                    user_portrait: None,
                                    user_points: 0,
                                    user_end_time: mongodb::bson::DateTime::now(),
                                    vip_level: None,
                                    vip_end_time: None,
                                    created_at: None,
                                };
                                
                                ready(Ok(AdminUser { user, claims }))
                            }
                            Err(e) => ready(Err(e)),
                        }
                    } else {
                        ready(Err(ErrorInternalServerError("无效的Authorization头格式")))
                    }
                } else {
                    ready(Err(ErrorInternalServerError("Authorization头格式错误")))
                }
            }
            None => ready(Err(ErrorInternalServerError("缺少Authorization头"))),
        }
    }
}

// 已认证用户提取器（包括普通用户）
pub struct AuthenticatedUser {
    pub user: User,
    pub claims: Claims,
}

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // 从Authorization头获取token
        let auth_header = req.headers().get("Authorization");
        
        match auth_header {
            Some(header) => {
                if let Ok(auth_str) = header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];
                        
                        match JWT_SERVICE.validate_token(token) {
                            Ok(claims) => {
                                // 检查用户状态
                                if claims.user_status != 1 {
                                    return ready(Err(ErrorInternalServerError("用户已被禁用")));
                                }
                                
                                // 构建用户对象
                                let user = User {
                                    id: Some(mongodb::bson::oid::ObjectId::parse_str(&claims.sub).unwrap()),
                                    user_name: claims.username.clone(),
                                    user_pwd: "".to_string(),
                                    group_id: claims.group_id,
                                    user_status: claims.user_status,
                                    user_nick_name: None,
                                    user_email: None,
                                    user_phone: None,
                                    user_portrait: None,
                                    user_points: 0,
                                    user_end_time: mongodb::bson::DateTime::now(),
                                    vip_level: None,
                                    vip_end_time: None,
                                    created_at: None,
                                };
                                
                                ready(Ok(AuthenticatedUser { user, claims }))
                            }
                            Err(e) => ready(Err(e)),
                        }
                    } else {
                        ready(Err(ErrorInternalServerError("无效的Authorization头格式")))
                    }
                } else {
                    ready(Err(ErrorInternalServerError("Authorization头格式错误")))
                }
            }
            None => ready(Err(ErrorInternalServerError("缺少Authorization头"))),
        }
    }
}

// 获取JWT服务实例
pub fn get_jwt_service() -> &'static JwtService {
    &JWT_SERVICE
}

// 辅助函数：从请求中获取当前用户
pub fn get_current_user(req: &HttpRequest) -> Option<User> {
    // 从Authorization头获取token
    let auth_header = req.headers().get("Authorization")?;
    
    let auth_str = auth_header.to_str().ok()?;
    if !auth_str.starts_with("Bearer ") {
        return None;
    }
    
    let token = &auth_str[7..];
    match JWT_SERVICE.get_user_from_token(token) {
        Ok(user) => Some(user),
        Err(_) => None,
    }
}

// 辅助函数：检查是否为管理员
pub fn is_admin_user(req: &HttpRequest) -> bool {
    if let Some(user) = get_current_user(req) {
        user.user_status == 1 && user.group_id == 1
    } else {
        false
    }
}

// 辅助函数：检查用户是否已登录（包括普通用户）
pub fn is_authenticated_user(req: &HttpRequest) -> bool {
    if let Some(user) = get_current_user(req) {
        user.user_status == 1
    } else {
        false
    }
}

// 可选认证用户包装器 - 用于可选登录的API端点
pub struct OptionalAuthenticatedUser(pub Option<AuthenticatedUser>);

impl FromRequest for OptionalAuthenticatedUser {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // 从Authorization头获取token
        let auth_header = req.headers().get("Authorization");
        
        match auth_header {
            Some(header) => {
                if let Ok(auth_str) = header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];
                        
                        match JWT_SERVICE.validate_token(token) {
                            Ok(claims) => {
                                // 检查用户状态
                                if claims.user_status != 1 {
                                    return ready(Ok(OptionalAuthenticatedUser(None)));
                                }
                                
                                // 构建用户对象
                                let user = User {
                                    id: Some(mongodb::bson::oid::ObjectId::parse_str(&claims.sub).unwrap()),
                                    user_name: claims.username.clone(),
                                    user_pwd: "".to_string(),
                                    group_id: claims.group_id,
                                    user_status: claims.user_status,
                                    user_nick_name: None,
                                    user_email: None,
                                    user_phone: None,
                                    user_portrait: None,
                                    user_points: 0,
                                    user_end_time: mongodb::bson::DateTime::now(),
                                    vip_level: None,
                                    vip_end_time: None,
                                    created_at: None,
                                };
                                
                                ready(Ok(OptionalAuthenticatedUser(Some(AuthenticatedUser { user, claims }))))
                            }
                            Err(_) => ready(Ok(OptionalAuthenticatedUser(None))), // token无效时返回None而不是错误
                        }
                    } else {
                        ready(Ok(OptionalAuthenticatedUser(None)))
                    }
                } else {
                    ready(Ok(OptionalAuthenticatedUser(None)))
                }
            }
            None => ready(Ok(OptionalAuthenticatedUser(None))), // 没有Authorization头时返回None
        }
    }
}