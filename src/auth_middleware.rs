use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::{ready, Ready};
use std::{
    future::Future,
    pin::Pin,
    rc::Rc,
};
use crate::models::User;
use crate::auth_handlers::validate_token;
use mongodb::{bson::doc, Database};
use actix_web::web;

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        
        Box::pin(async move {
            // 尝试从Authorization头获取token并验证用户
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];
                        
                        // 验证token
                        if let Ok(user_id) = validate_token(token) {
                            // 从数据库获取用户信息
                            if let Some(db) = req.app_data::<web::Data<Database>>() {
                                let user_collection = db.collection::<User>("users");
                                
                                if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(&user_id) {
                                    if let Ok(Some(user)) = user_collection
                                        .find_one(doc! { "_id": object_id }, None)
                                        .await
                                    {
                                        // 将用户信息注入到请求中
                                        req.extensions_mut().insert(user);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            service.call(req).await
        })
    }
}