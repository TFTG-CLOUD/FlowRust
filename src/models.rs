use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

// Note: In a real application, you would want to use a library like `chrono` for more robust date/time handling.
// Here we use mongodb::bson::DateTime for simplicity.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vod {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub vod_name: String,
    pub type_id: i32,
    pub vod_status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_pic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_director: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_remarks: Option<String>,
    pub vod_pubdate: DateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_area: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_hits: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_hits_day: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_hits_week: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_hits_month: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_score: Option<String>,
    #[serde(default)]
    pub need_vip: i32, // 0=no, 1=vip level 1, 2=vip level2, 3=vip level3
    // In MongoDB, this is better represented as a nested structure
    pub vod_play_urls: Vec<PlaySource>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaySource {
    pub source_name: String,
    pub urls: Vec<PlayUrl>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayUrl {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Art {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub art_name: String,
    pub type_id: i32,
    pub art_status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art_pic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art_author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art_remarks: Option<String>,
    pub art_pubdate: DateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art_content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_name: String,
    // IMPORTANT: Passwords should ALWAYS be hashed. This is just the data model.
    pub user_pwd: String,
    pub group_id: i32,
    pub user_status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_nick_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_portrait: Option<String>,
    pub user_points: i32,
    pub user_end_time: DateTime,
    pub vip_level: Option<i32>,
    pub vip_end_time: Option<DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Type {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub type_id: i32,
    pub type_name: String,
    pub type_pid: i32, // Parent ID, 0 for top-level categories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_en: Option<String>, // English name
    pub type_sort: i32, // Sort order
    pub type_status: i32, // Status: 1=enabled, 0=disabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_mid: Option<i32>, // Model ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_key: Option<String>, // SEO keywords
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_des: Option<String>, // Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_title: Option<String>, // SEO title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_tpl: Option<String>, // Template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_tpl_list: Option<String>, // List template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_tpl_detail: Option<String>, // Detail template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_tpl_play: Option<String>, // Play template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_tpl_down: Option<String>, // Download template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subarea: Option<String>, // Available areas for filtering (comma-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subyear: Option<String>, // Available years for filtering (comma-separated)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Binding {
    #[serde(rename = "_id")] // Use the binding key as the MongoDB document ID
    pub id: String, // e.g., "source_flag_external_id"
    pub source_flag: String,     // 采集源标识，如API的唯一标识符
    pub external_id: String,     // 外部分类ID
    pub local_type_id: i32,      // 本地分类ID
    pub local_type_name: String, // 本地分类名称
    pub created_at: DateTime,    // 创建时间
    pub updated_at: DateTime,    // 更新时间
}

// Website configuration model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub config_key: String,   // Configuration key (unique)
    pub config_value: String, // Configuration value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_desc: Option<String>, // Description
    pub config_type: String,  // Type: text, textarea, select, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_group: Option<String>, // Group: site, seo, upload, etc.
    pub config_sort: i32,     // Sort order
    pub updated_at: DateTime,
}

// Default value functions for Collection
fn default_convert_webp() -> i32 {
    0 // Default to not convert WebP
}

fn default_download_retry() -> i32 {
    3 // Default to 3 retry attempts
}

// Collection source model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub collect_name: String,   // Collection source name
    pub collect_url: String,    // API URL
    pub collect_type: i32,      // Type: 1=video, 2=article
    pub collect_mid: i32,       // Model ID
    pub collect_appid: String,  // App ID
    pub collect_appkey: String, // App Key
    pub collect_param: String,  // Additional parameters
    pub collect_filter: String, // Filter rules
    #[serde(default)]
    pub collect_filter_from: String, // Filter play sources
    pub collect_opt: i32,       // Collection option: 0=all, 1=today, 2=yesterday, 3=week
    pub collect_sync_pic_opt: i32, // Sync picture option
    pub collect_remove_ad: i32, // Remove ads: 0=no, 1=yes
    #[serde(default = "default_convert_webp")]
    pub collect_convert_webp: i32, // Convert to WebP: 0=no, 1=yes
    #[serde(default = "default_download_retry")]
    pub collect_download_retry: i32, // Download retry times
    pub collect_status: i32,    // Status: 1=enabled, 0=disabled
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

// Collection task model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollectTask {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub task_name: String,    // Task name
    pub collect_id: ObjectId, // Collection source ID
    pub task_status: i32,     // Status: 0=pending, 1=running, 2=completed, 3=failed
    pub task_progress: i32,   // Progress percentage
    pub task_total: i32,      // Total items
    pub task_success: i32,    // Success count
    pub task_failed: i32,     // Failed count
    pub task_log: String,     // Task log
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

// Media type enum
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MediaType {
    Vod,   // Video
    Image, // Image gallery
    Audio, // Audio collection
}

// User watching history model (supports all media types)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserHistory {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,            // User ID
    pub media_type: MediaType,        // Media type (Vod, Image, Audio)
    pub media_id: ObjectId,           // Media ID (vod_id, image_id, or audio_id)
    pub media_name: String,           // Media name
    pub media_url: String,            // Media URL (play_url for vod, view_url for image/audio)
    pub poster: String,               // Poster URL
    pub episode_name: Option<String>, // Episode name (for vod only)
    pub current_time: Option<i64>,    // Current playback time in seconds (for vod only)
    pub watch_time: Option<String>,   // Formatted watch time (for vod only)
    pub timestamp: i64,               // Unix timestamp when record was created
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

// User favorites/collections model (supports all media types)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserFavorite {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,        // User ID
    pub media_type: MediaType,    // Media type (Vod, Image, Audio)
    pub media_id: ObjectId,       // Media ID (vod_id, image_id, or audio_id)
    pub media_name: String,       // Media name
    pub poster: String,           // Poster URL
    pub category: Option<String>, // Category (for image/audio)
    pub remarks: Option<String>,  // Remarks or description
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

// Card model for membership cards
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Card {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub code: String,       // Card code
    pub used: bool,         // Whether the card has been used
    pub vip_level: i32,     // VIP level this card provides
    pub duration_days: i32, // Duration in days this card provides
    pub created_at: DateTime,
    pub used_by: Option<ObjectId>, // User who used this card
    pub used_at: Option<DateTime>, // When the card was used
}

// Image model for image galleries
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Image {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub title: String,               // Image title
    pub en_title: Option<String>,    // English title
    pub description: Option<String>, // Image description
    pub images: Vec<ImageItem>,      // Array of images
    pub cover: ImageItem,            // Cover image
    pub tags: Vec<String>,           // Tags
    pub category: String,            // Category
    pub pages: i32,                  // Number of pages
    pub uploader: ObjectId,          // User who uploaded
    pub likes: i32,                  // Number of likes
    pub language: Option<String>,    // Language
    pub artists: Option<String>,     // Artists
    pub need_vip: i32,               // Whether VIP is required: 0=no, 1=yes
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

// Image item structure for images array
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageItem {
    pub url: String, // Image URL
    pub width: i32,  // Image width
    pub height: i32, // Image height
}

// Audio model for audio collections
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Audio {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub title: String,               // Audio title
    pub en_title: Option<String>,    // English title
    pub description: Option<String>, // Audio description
    pub tags: Vec<String>,           // Tags
    pub category: String,            // Category
    pub cover: ImageItem,            // Cover image
    pub audios: Vec<String>,         // Array of audio URLs
    pub need_vip: i32,               // Whether VIP is required: 0=no, 1=yes
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
