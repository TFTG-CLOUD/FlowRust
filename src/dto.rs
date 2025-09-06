use serde::{Deserialize, Serialize, Deserializer};
use std::fmt;
// use crate::models::{Vod, Art}; // Assuming you might want to reuse these

// Custom enum to support both i64 and String for vod_id
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VodId {
    Number(i64),
    String(String),
}

impl VodId {
    pub fn to_string(&self) -> String {
        match self {
            VodId::Number(n) => n.to_string(),
            VodId::String(s) => s.clone(),
        }
    }
    
    pub fn to_i64(&self) -> Option<i64> {
        match self {
            VodId::Number(n) => Some(*n),
            VodId::String(s) => s.parse().ok(),
        }
    }
}

// Struct to capture all possible query parameters from the API request
#[derive(Debug, Deserialize)]
pub struct ApiParams {
    pub ac: Option<String>,
    pub at: Option<String>,
    pub ids: Option<String>,
    pub t: Option<i32>,
    pub pg: Option<u64>,
    pub pagesize: Option<u64>,
    pub h: Option<u64>,
    pub wd: Option<String>,
}

// Struct for the JSON response, mirroring the PHP API's output
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonResponse<T> {
    pub code: i32,
    pub msg: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub page: u64,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub pagecount: u64,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub limit: u64,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub total: u64,
    pub list: Vec<T>,
    #[serde(rename = "class", default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<Category>,
}

// Struct for video list responses (without categories field)
#[derive(Debug, Serialize, Deserialize)]
pub struct VideoListResponse {
    pub code: i32,
    pub msg: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub page: u64,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub pagecount: u64,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub limit: u64,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub total: u64,
    pub list: Vec<VodApiListEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub type_id: i32,
    pub type_name: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub type_pid: i32,
}

fn is_zero(num: &i32) -> bool {
    *num == 0
}

fn deserialize_string_or_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: serde::Deserialize<'de> + std::str::FromStr,
    <T as std::str::FromStr>::Err: fmt::Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber<T> {
        String(String),
        Number(T),
    }

    match StringOrNumber::<T>::deserialize(deserializer)? {
        StringOrNumber::String(s) => s.parse().map_err(serde::de::Error::custom),
        StringOrNumber::Number(n) => Ok(n),
    }
}

// Custom deserializer to handle empty strings as None
fn deserialize_empty_string_to_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::Deserialize<'de> + std::str::FromStr,
    <T as std::str::FromStr>::Err: fmt::Display,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => s.parse().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

// A simplified Vod structure for the API list response
// The full detail response might use the main Vod model
#[derive(Debug, Serialize, Deserialize)]
pub struct VodApiListEntry {
    pub vod_id: VodId,
    pub vod_name: String,
    pub type_id: i32,
    pub type_name: Option<String>,
    pub vod_time: String,
    pub vod_remarks: String,
    pub vod_play_from: String,
    pub vod_status: Option<i32>,
    pub vod_letter: Option<String>,
    pub vod_color: Option<String>,
    pub vod_tag: Option<String>,
    pub vod_class: Option<String>,
    pub vod_pic: Option<String>,
    pub vod_pic_thumb: Option<String>,
    pub vod_pic_slide: Option<String>,
    pub vod_pic_screenshot: Option<String>,
    pub vod_actor: Option<String>,
    pub vod_director: Option<String>,
    pub vod_writer: Option<String>,
    pub vod_behind: Option<String>,
    pub vod_blurb: Option<String>,
    pub vod_pubdate: Option<String>,
    pub vod_total: Option<i32>,
    pub vod_serial: Option<String>,
    pub vod_tv: Option<String>,
    pub vod_weekday: Option<String>,
    pub vod_area: Option<String>,
    pub vod_lang: Option<String>,
    pub vod_year: Option<String>,
    pub vod_version: Option<String>,
    pub vod_state: Option<String>,
    pub vod_author: Option<String>,
    pub vod_jumpurl: Option<String>,
    pub vod_tpl: Option<String>,
    pub vod_tpl_play: Option<String>,
    pub vod_tpl_down: Option<String>,
    pub vod_isend: Option<i32>,
    pub vod_lock: Option<i32>,
    pub vod_level: Option<i32>,
    pub vod_copyright: Option<i32>,
    pub vod_points: Option<i32>,
    pub vod_points_play: Option<i32>,
    pub vod_points_down: Option<i32>,
    pub vod_hits: Option<i32>,
    pub vod_hits_day: Option<i32>,
    pub vod_hits_week: Option<i32>,
    pub vod_hits_month: Option<i32>,
    pub vod_duration: Option<String>,
    pub vod_up: Option<i32>,
    pub vod_down: Option<i32>,
    pub vod_score: Option<String>,
    pub vod_score_all: Option<i32>,
    pub vod_score_num: Option<i32>,
    pub vod_time_add: Option<i64>,
    pub vod_time_hits: Option<i64>,
    pub vod_time_make: Option<i64>,
    pub vod_trysee: Option<i32>,
    pub vod_douban_id: Option<i64>,
    pub vod_douban_score: Option<String>,
    pub vod_reurl: Option<String>,
    pub vod_rel_vod: Option<String>,
    pub vod_rel_art: Option<String>,
    pub vod_pwd: Option<String>,
    pub vod_pwd_url: Option<String>,
    pub vod_pwd_play: Option<String>,
    pub vod_pwd_play_url: Option<String>,
    pub vod_pwd_down: Option<String>,
    pub vod_pwd_down_url: Option<String>,
    pub vod_content: Option<String>,
    pub vod_play_server: Option<String>,
    pub vod_play_note: Option<String>,
    pub vod_play_url: Option<String>,
    pub vod_down_from: Option<String>,
    pub vod_down_server: Option<String>,
    pub vod_down_note: Option<String>,
    pub vod_down_url: Option<String>,
}

// TODO: Define structs for XML serialization using quick-xml attributes
// This will be more involved and will be handled in the handler implementation.

#[derive(Debug, Deserialize)]
pub struct ListPageParams {
    #[serde(default, deserialize_with = "deserialize_empty_string_to_none")]
    pub sub_type: Option<i32>,
    pub area: Option<String>,
    pub year: Option<String>,
    pub pg: Option<u64>,
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VideoFilterParams {
    pub sub_type: Option<i32>,
    pub area: Option<String>,
    pub year: Option<String>,
    pub pg: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryHierarchy {
    pub category: crate::models::Type,
    pub sub_categories: Vec<crate::models::Type>,
}

// User authentication DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub code: i32,
    pub msg: String,
    pub token: Option<String>,
    pub user: Option<crate::models::User>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub code: i32,
    pub msg: String,
    pub user: Option<crate::models::User>,
}

// Card management DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct CardListResponse {
    pub code: i32,
    pub msg: String,
    pub cards: Vec<CardInfo>,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CardInfo {
    #[serde(rename = "_id")]
    pub id: String,
    pub code: String,
    pub used: bool,
    pub vip_level: i32,
    pub duration_days: i32,
    pub created_at: String,
    pub used_by: Option<UserInfo>,
    pub used_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    #[serde(rename = "_id")]
    pub id: String,
    pub user_name: String,
    pub user_nick_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateCardRequest {
    pub count: i32,
    pub vip_level: i32,
    pub duration_days: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateCardResponse {
    pub code: i32,
    pub msg: String,
    pub generated_count: i32,
    pub cards: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteCardRequest {
    pub card_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchCardRequest {
    pub code: String,
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CardPageParams {
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

// VIP validation DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct VipCheckRequest {
    pub video_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VipCheckResponse {
    pub success: bool,
    pub has_access: bool,
    pub message: String,
    pub play_url: Option<String>,
    pub episode_name: Option<String>,
}

// User management DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct UserListResponse {
    pub code: i32,
    pub msg: String,
    pub users: Vec<UserAdminInfo>,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAdminInfo {
    #[serde(rename = "_id")]
    pub id: String,
    pub user_name: String,
    pub user_nick_name: Option<String>,
    pub user_email: String,
    pub vip_level: i32,
    pub vip_end_time: Option<String>,
    pub created_at: String,
    pub last_login: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub user_name: String,
    pub user_email: String,
    pub user_nick_name: Option<String>,
    pub password: String,
    pub vip_level: Option<i32>,
    pub vip_duration_days: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub user_id: String,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub user_nick_name: Option<String>,
    pub password: Option<String>,
    pub vip_level: Option<i32>,
    pub vip_duration_days: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteUserRequest {
    pub user_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchUserRequest {
    pub query: String,
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPageParams {
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

// Generic API Response
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
    pub success: Option<bool>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            msg: "Success".to_string(),
            data: Some(data),
            success: Some(true),
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            code: 500,
            msg,
            data: None,
            success: Some(false),
        }
    }
}
