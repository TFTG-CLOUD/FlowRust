use crate::models::{Binding, Collection, Config, PlaySource, PlayUrl, Type, Vod};
use mongodb::bson::DateTime;
use mongodb::{bson::doc, Database};

// 检查配置是否已存在
async fn config_exists(db: &Database, config_key: &str) -> Result<bool, mongodb::error::Error> {
    let collection = db.collection::<Config>("configs");
    let count = collection
        .count_documents(doc! { "config_key": config_key }, None)
        .await?;
    Ok(count > 0)
}

// 初始化网站基本配置
pub async fn init_website_config(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let collection = db.collection::<Config>("configs");

    let configs = vec![
        Config {
            id: None,
            config_key: "site_name".to_string(),
            config_value: "苹果CMS Rust版".to_string(),
            config_desc: Some("网站名称".to_string()),
            config_type: "text".to_string(),
            config_group: Some("基本信息".to_string()),
            config_sort: 1,
            updated_at: DateTime::now(),
        },
        Config {
            id: None,
            config_key: "site_url".to_string(),
            config_value: "http://localhost:8080".to_string(),
            config_desc: Some("网站域名".to_string()),
            config_type: "text".to_string(),
            config_group: Some("基本信息".to_string()),
            config_sort: 2,
            updated_at: DateTime::now(),
        },
        Config {
            id: None,
            config_key: "site_keywords".to_string(),
            config_value: "在线视频,电影,电视剧,动漫".to_string(),
            config_desc: Some("网站关键词".to_string()),
            config_type: "text".to_string(),
            config_group: Some("SEO设置".to_string()),
            config_sort: 3,
            updated_at: DateTime::now(),
        },
        Config {
            id: None,
            config_key: "site_description".to_string(),
            config_value: "提供最新最全的在线视频观看服务".to_string(),
            config_desc: Some("网站描述".to_string()),
            config_type: "textarea".to_string(),
            config_group: Some("SEO设置".to_string()),
            config_sort: 4,
            updated_at: DateTime::now(),
        },
        Config {
            id: None,
            config_key: "site_logo".to_string(),
            config_value: "/static/images/logo.png".to_string(),
            config_desc: Some("网站LOGO".to_string()),
            config_type: "image".to_string(),
            config_group: Some("外观设置".to_string()),
            config_sort: 5,
            updated_at: DateTime::now(),
        },
        Config {
            id: None,
            config_key: "buy_card".to_string(),
            config_value: "https://google.com".to_string(),
            config_desc: Some("购买卡卷的发卡链接".to_string()),
            config_type: "text".to_string(),
            config_group: Some("用户设置".to_string()),
            config_sort: 6,
            updated_at: DateTime::now(),
        },
    ];

    let mut created_count = 0;

    for config in configs {
        // 先检查配置是否已存在
        match config_exists(db, &config.config_key).await {
            Ok(true) => {
                println!("⚪ 配置已存在，跳过: {}", config.config_key);
            }
            Ok(false) => {
                // 配置不存在，创建它
                let filter = doc! { "config_key": &config.config_key };
                let update = doc! {
                    "$setOnInsert": {
                        "config_key": &config.config_key,
                        "config_value": &config.config_value,
                        "config_desc": &config.config_desc,
                        "config_type": &config.config_type,
                        "config_group": &config.config_group,
                        "config_sort": config.config_sort,
                        "updated_at": config.updated_at,
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!("✅ 创建配置: {}", config.config_key);
                created_count += 1;
            }
            Err(e) => {
                eprintln!("❌ 检查配置存在性失败: {}: {}", config.config_key, e);
                // 继续尝试创建
                let filter = doc! { "config_key": &config.config_key };
                let update = doc! {
                    "$setOnInsert": {
                        "config_key": &config.config_key,
                        "config_value": &config.config_value,
                        "config_desc": &config.config_desc,
                        "config_type": &config.config_type,
                        "config_group": &config.config_group,
                        "config_sort": config.config_sort,
                        "updated_at": config.updated_at,
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!("✅ 创建配置（检查失败）: {}", config.config_key);
                created_count += 1;
            }
        }
    }

    if created_count > 0 {
        println!("网站配置初始化完成，新增 {} 个配置", created_count);
    } else {
        println!("网站配置已存在，无需初始化");
    }
    Ok(())
}

// 检查分类是否已存在
async fn type_exists(db: &Database, type_id: i32) -> Result<bool, mongodb::error::Error> {
    let collection = db.collection::<Type>("types");
    let count = collection
        .count_documents(doc! { "type_id": type_id }, None)
        .await?;
    Ok(count > 0)
}

// 初始化测试分类数据
pub async fn init_test_categories(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let collection = db.collection::<Type>("types");

    let categories = vec![
        // 一级分类
        Type {
            id: None,
            type_id: 1,
            type_name: "电影".to_string(),
            type_pid: 0,
            type_en: Some("movie".to_string()),
            type_sort: 1,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("电影,影片".to_string()),
            type_des: Some("最新电影资源".to_string()),
            type_title: Some("电影频道".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: Some(
                "中国大陆,中国香港,中国台湾,美国,日本,韩国,泰国,印度,英国,法国".to_string(),
            ),
            subyear: Some(
                "2025,2024,2023,2022,2021,2020,2019,2018,2017,2016,2015,2014,2013,2012,2011,2010"
                    .to_string(),
            ),
        },
        Type {
            id: None,
            type_id: 2,
            type_name: "电视剧".to_string(),
            type_pid: 0,
            type_en: Some("tv".to_string()),
            type_sort: 2,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("电视剧,连续剧".to_string()),
            type_des: Some("热门电视剧资源".to_string()),
            type_title: Some("电视剧频道".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: Some(
                "中国大陆,中国香港,中国台湾,美国,日本,韩国,泰国,印度,英国,法国".to_string(),
            ),
            subyear: Some(
                "2025,2024,2023,2022,2021,2020,2019,2018,2017,2016,2015,2014,2013,2012,2011,2010"
                    .to_string(),
            ),
        },
        Type {
            id: None,
            type_id: 3,
            type_name: "动漫".to_string(),
            type_pid: 0,
            type_en: Some("anime".to_string()),
            type_sort: 3,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("动漫,动画".to_string()),
            type_des: Some("精彩动漫资源".to_string()),
            type_title: Some("动漫频道".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: Some(
                "中国大陆,中国香港,中国台湾,美国,日本,韩国,泰国,印度,英国,法国".to_string(),
            ),
            subyear: Some(
                "2025,2024,2023,2022,2021,2020,2019,2018,2017,2016,2015,2014,2013,2012,2011,2010"
                    .to_string(),
            ),
        },
        Type {
            id: None,
            type_id: 4,
            type_name: "综艺".to_string(),
            type_pid: 0,
            type_en: Some("Variety shows".to_string()),
            type_sort: 35,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("综艺,综艺节目".to_string()),
            type_des: Some("综艺节目".to_string()),
            type_title: Some("综艺频道".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: Some(
                "中国大陆,中国香港,中国台湾,美国,日本,韩国,泰国,印度,英国,法国".to_string(),
            ),
            subyear: Some(
                "2025,2024,2023,2022,2021,2020,2019,2018,2017,2016,2015,2014,2013,2012,2011,2010"
                    .to_string(),
            ),
        },
        // 二级分类 - 电影子分类
        Type {
            id: None,
            type_id: 11,
            type_name: "动作片".to_string(),
            type_pid: 1,
            type_en: Some("action".to_string()),
            type_sort: 11,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("动作,武打".to_string()),
            type_des: Some("动作电影".to_string()),
            type_title: Some("动作片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 12,
            type_name: "喜剧片".to_string(),
            type_pid: 1,
            type_en: Some("comedy".to_string()),
            type_sort: 12,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("喜剧,搞笑".to_string()),
            type_des: Some("喜剧电影".to_string()),
            type_title: Some("喜剧片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 13,
            type_name: "科幻片".to_string(),
            type_pid: 1,
            type_en: Some("scifi".to_string()),
            type_sort: 13,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("科幻,未来".to_string()),
            type_des: Some("科幻电影".to_string()),
            type_title: Some("科幻片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 14,
            type_name: "爱情片".to_string(),
            type_pid: 1,
            type_en: Some("romance".to_string()),
            type_sort: 14,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("爱情,言情".to_string()),
            type_des: Some("爱情电影".to_string()),
            type_title: Some("爱情片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 15,
            type_name: "恐怖片".to_string(),
            type_pid: 1,
            type_en: Some("horror".to_string()),
            type_sort: 15,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("恐怖,惊悚".to_string()),
            type_des: Some("恐怖电影".to_string()),
            type_title: Some("恐怖片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 16,
            type_name: "剧情片".to_string(),
            type_pid: 1,
            type_en: Some("drama".to_string()),
            type_sort: 4,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("剧情,故事".to_string()),
            type_des: Some("剧情电影".to_string()),
            type_title: Some("剧情片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 17,
            type_name: "战争片".to_string(),
            type_pid: 1,
            type_en: Some("war".to_string()),
            type_sort: 16,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("战争,军事".to_string()),
            type_des: Some("战争电影".to_string()),
            type_title: Some("战争片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 18,
            type_name: "惊悚片".to_string(),
            type_pid: 1,
            type_en: Some("thriller".to_string()),
            type_sort: 17,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("惊悚,悬疑".to_string()),
            type_des: Some("惊悚电影".to_string()),
            type_title: Some("惊悚片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 19,
            type_name: "悬疑片".to_string(),
            type_pid: 1,
            type_en: Some("suspense".to_string()),
            type_sort: 18,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("悬疑,推理".to_string()),
            type_des: Some("悬疑电影".to_string()),
            type_title: Some("悬疑片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 20,
            type_name: "纪录片".to_string(),
            type_pid: 1,
            type_en: Some("documentary".to_string()),
            type_sort: 19,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("纪录片,纪实".to_string()),
            type_des: Some("纪录片电影".to_string()),
            type_title: Some("纪录片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 21,
            type_name: "犯罪片".to_string(),
            type_pid: 1,
            type_en: Some("crime".to_string()),
            type_sort: 20,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("犯罪,警匪".to_string()),
            type_des: Some("犯罪电影".to_string()),
            type_title: Some("犯罪片".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 22,
            type_name: "动画电影".to_string(),
            type_pid: 1,
            type_en: Some("animated movie".to_string()),
            type_sort: 21,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("动画,卡通".to_string()),
            type_des: Some("动画电影".to_string()),
            type_title: Some("动画电影".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        // 二级分类 - 电视剧子分类
        Type {
            id: None,
            type_id: 23,
            type_name: "韩国电视剧".to_string(),
            type_pid: 2,
            type_en: Some("Korean TV series".to_string()),
            type_sort: 23,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("韩剧,韩国剧".to_string()),
            type_des: Some("韩国电视剧".to_string()),
            type_title: Some("韩国电视剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 24,
            type_name: "欧美电视剧".to_string(),
            type_pid: 2,
            type_en: Some("European and American TV series".to_string()),
            type_sort: 24,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("美剧,欧美剧".to_string()),
            type_des: Some("欧美电视剧".to_string()),
            type_title: Some("欧美电视剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 25,
            type_name: "海外电视剧".to_string(),
            type_pid: 2,
            type_en: Some("Overseas TV series".to_string()),
            type_sort: 25,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("海外剧,国际剧".to_string()),
            type_des: Some("海外电视剧".to_string()),
            type_title: Some("海外电视剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 26,
            type_name: "泰国电视剧".to_string(),
            type_pid: 2,
            type_en: Some("Thai TV series".to_string()),
            type_sort: 26,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("泰剧,泰国剧".to_string()),
            type_des: Some("泰国电视剧".to_string()),
            type_title: Some("泰国电视剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 27,
            type_name: "中国台湾剧".to_string(),
            type_pid: 2,
            type_en: Some("Chinese Taiwanese TV series".to_string()),
            type_sort: 27,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("台剧,台湾剧".to_string()),
            type_des: Some("中国台湾剧".to_string()),
            type_title: Some("中国台湾剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 28,
            type_name: "日本电视剧".to_string(),
            type_pid: 2,
            type_en: Some("Japanese TV series".to_string()),
            type_sort: 28,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("日剧,日本剧".to_string()),
            type_des: Some("日本电视剧".to_string()),
            type_title: Some("日本电视剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 29,
            type_name: "国产电视剧".to_string(),
            type_pid: 2,
            type_en: Some("Chinese TV series".to_string()),
            type_sort: 29,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("国产剧,大陆剧".to_string()),
            type_des: Some("国产电视剧".to_string()),
            type_title: Some("国产电视剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 30,
            type_name: "中国香港剧".to_string(),
            type_pid: 2,
            type_en: Some("Hong Kong TV series".to_string()),
            type_sort: 30,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("港剧,香港剧".to_string()),
            type_des: Some("中国香港剧".to_string()),
            type_title: Some("中国香港剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 31,
            type_name: "短剧".to_string(),
            type_pid: 2,
            type_en: Some("Short TV series".to_string()),
            type_sort: 31,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("短剧,短视频剧".to_string()),
            type_des: Some("短剧".to_string()),
            type_title: Some("短剧".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        // 二级分类 - 动漫子分类
        Type {
            id: None,
            type_id: 32,
            type_name: "国产动漫".to_string(),
            type_pid: 3,
            type_en: Some("Chinese Donghua".to_string()),
            type_sort: 32,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("国漫,国产动画".to_string()),
            type_des: Some("国产动漫".to_string()),
            type_title: Some("国产动漫".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 33,
            type_name: "欧美动漫".to_string(),
            type_pid: 3,
            type_en: Some("European and American anime".to_string()),
            type_sort: 33,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("欧美动漫,西方动画".to_string()),
            type_des: Some("欧美动漫".to_string()),
            type_title: Some("欧美动漫".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 34,
            type_name: "日韩动漫".to_string(),
            type_pid: 3,
            type_en: Some("Japanese and Korean anime".to_string()),
            type_sort: 34,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("日漫,韩漫,日韩动画".to_string()),
            type_des: Some("日韩动漫".to_string()),
            type_title: Some("日韩动漫".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        // 二级分类 - 综艺子分类
        Type {
            id: None,
            type_id: 36,
            type_name: "大陆综艺".to_string(),
            type_pid: 4,
            type_en: Some("Chinese variety shows".to_string()),
            type_sort: 36,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("大陆综艺,国产综艺".to_string()),
            type_des: Some("大陆综艺".to_string()),
            type_title: Some("大陆综艺".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 37,
            type_name: "港台综艺".to_string(),
            type_pid: 4,
            type_en: Some("Hong Kong and Taiwan variety shows".to_string()),
            type_sort: 37,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("港台综艺,香港台湾综艺".to_string()),
            type_des: Some("港台综艺".to_string()),
            type_title: Some("港台综艺".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 38,
            type_name: "日韩综艺".to_string(),
            type_pid: 4,
            type_en: Some("Japanese and Korean variety shows".to_string()),
            type_sort: 38,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("日韩综艺,日韩综艺节目".to_string()),
            type_des: Some("日韩综艺".to_string()),
            type_title: Some("日韩综艺".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
        Type {
            id: None,
            type_id: 39,
            type_name: "欧美综艺".to_string(),
            type_pid: 4,
            type_en: Some("European and American variety shows".to_string()),
            type_sort: 39,
            type_status: 1,
            type_mid: Some(1),
            type_key: Some("欧美综艺,西方综艺".to_string()),
            type_des: Some("欧美综艺".to_string()),
            type_title: Some("欧美综艺".to_string()),
            type_tpl: None,
            type_tpl_list: None,
            type_tpl_detail: None,
            type_tpl_play: None,
            type_tpl_down: None,
            subarea: None,
            subyear: None,
        },
    ];

    let mut created_count = 0;

    for category in categories {
        // 先检查分类是否已存在
        match type_exists(db, category.type_id).await {
            Ok(true) => {
                println!(
                    "⚪ 分类已存在，跳过: {} (ID: {})",
                    category.type_name, category.type_id
                );
            }
            Ok(false) => {
                // 分类不存在，创建它
                let filter = doc! { "type_id": category.type_id };
                let update = doc! {
                    "$setOnInsert": {
                        "type_id": category.type_id,
                        "type_name": &category.type_name,
                        "type_pid": category.type_pid,
                        "type_en": &category.type_en,
                        "type_sort": category.type_sort,
                        "type_status": category.type_status,
                        "type_mid": category.type_mid,
                        "type_key": &category.type_key,
                        "type_des": &category.type_des,
                        "type_title": &category.type_title,
                        "subarea": &category.subarea,
                        "subyear": &category.subyear,
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!(
                    "✅ 创建分类: {} (ID: {})",
                    category.type_name, category.type_id
                );
                created_count += 1;
            }
            Err(e) => {
                eprintln!(
                    "❌ 检查分类存在性失败: {} (ID: {}): {}",
                    category.type_name, category.type_id, e
                );
                // 继续尝试创建
                let filter = doc! { "type_id": category.type_id };
                let update = doc! {
                    "$setOnInsert": {
                        "type_id": category.type_id,
                        "type_name": &category.type_name,
                        "type_pid": category.type_pid,
                        "type_en": &category.type_en,
                        "type_sort": category.type_sort,
                        "type_status": category.type_status,
                        "type_mid": category.type_mid,
                        "type_key": &category.type_key,
                        "type_des": &category.type_des,
                        "type_title": &category.type_title,
                        "subarea": &category.subarea,
                        "subyear": &category.subyear,
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!(
                    "✅ 创建分类（检查失败）: {} (ID: {})",
                    category.type_name, category.type_id
                );
                created_count += 1;
            }
        }
    }

    if created_count > 0 {
        println!("测试分类数据初始化完成，新增 {} 个分类", created_count);
    } else {
        println!("测试分类已存在，无需初始化");
    }
    Ok(())
}

// 检查视频是否已存在
async fn vod_exists(db: &Database, vod_name: &str) -> Result<bool, mongodb::error::Error> {
    let collection = db.collection::<Vod>("vods");
    let count = collection
        .count_documents(doc! { "vod_name": vod_name }, None)
        .await?;
    Ok(count > 0)
}

// 初始化测试视频数据
pub async fn init_test_videos(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let collection = db.collection::<Vod>("vods");

    let videos = vec![
        Vod {
            id: None,
            vod_name: "复仇者联盟4：终局之战".to_string(),
            type_id: 11, // 动作片
            vod_status: 1,
            vod_class: Some("动作,科幻,冒险".to_string()),
            vod_pic: Some("https://img.example.com/avengers4.jpg".to_string()),
            vod_actor: Some("小罗伯特·唐尼,克里斯·埃文斯,马克·鲁法洛".to_string()),
            vod_director: Some("安东尼·罗素,乔·罗素".to_string()),
            vod_remarks: Some("超清".to_string()),
            vod_pubdate: DateTime::now(),
            vod_area: Some("美国".to_string()),
            vod_lang: Some("英语".to_string()),
            vod_year: Some("2019".to_string()),
            vod_content: Some(
                "漫威电影宇宙的史诗级终章，超级英雄们为了拯救宇宙而展开最后的战斗。".to_string(),
            ),
            vod_hits: Some(0),
            vod_hits_day: Some(0),
            vod_hits_week: Some(0),
            vod_hits_month: Some(0),
            vod_score: Some("9.2".to_string()),
            need_vip: 0,
            vod_play_urls: vec![PlaySource {
                source_name: "高清播放".to_string(),
                urls: vec![PlayUrl {
                    name: "第01集".to_string(),
                    url: "https://example.com/video/avengers4.m3u8".to_string(),
                }],
            }],
        },
        Vod {
            id: None,
            vod_name: "流浪地球".to_string(),
            type_id: 13, // 科幻片
            vod_status: 1,
            vod_class: Some("科幻,灾难,冒险".to_string()),
            vod_pic: Some("https://img.example.com/wandering_earth.jpg".to_string()),
            vod_actor: Some("吴京,易烊千玺,屈楚萧".to_string()),
            vod_director: Some("郭帆".to_string()),
            vod_remarks: Some("超清".to_string()),
            vod_pubdate: DateTime::now(),
            vod_area: Some("中国".to_string()),
            vod_lang: Some("中文".to_string()),
            vod_year: Some("2019".to_string()),
            vod_content: Some(
                "太阳即将毁灭，人类在地球表面建造出巨大的推进器，寻找新的家园。".to_string(),
            ),
            vod_hits: Some(0),
            vod_hits_day: Some(0),
            vod_hits_week: Some(0),
            vod_hits_month: Some(0),
            vod_score: Some("8.8".to_string()),
            need_vip: 0,
            vod_play_urls: vec![PlaySource {
                source_name: "高清播放".to_string(),
                urls: vec![PlayUrl {
                    name: "第01集".to_string(),
                    url: "https://example.com/video/wandering_earth.m3u8".to_string(),
                }],
            }],
        },
        Vod {
            id: None,
            vod_name: "你好,李焕英".to_string(),
            type_id: 12, // 喜剧片
            vod_status: 1,
            vod_class: Some("喜剧,奇幻,家庭".to_string()),
            vod_pic: Some("https://img.example.com/hello_mom.jpg".to_string()),
            vod_actor: Some("贾玲,张小斐,沈腾".to_string()),
            vod_director: Some("贾玲".to_string()),
            vod_remarks: Some("超清".to_string()),
            vod_pubdate: DateTime::now(),
            vod_area: Some("中国".to_string()),
            vod_lang: Some("中文".to_string()),
            vod_year: Some("2021".to_string()),
            vod_content: Some("女儿穿越回到过去，想要让母亲过上更好的生活。".to_string()),
            vod_hits: Some(0),
            vod_hits_day: Some(0),
            vod_hits_week: Some(0),
            vod_hits_month: Some(0),
            vod_score: Some("8.5".to_string()),
            need_vip: 0,
            vod_play_urls: vec![PlaySource {
                source_name: "高清播放".to_string(),
                urls: vec![PlayUrl {
                    name: "第01集".to_string(),
                    url: "https://example.com/video/hello_mom.m3u8".to_string(),
                }],
            }],
        },
    ];

    let mut created_count = 0;

    for video in videos {
        // 先检查视频是否已存在
        match vod_exists(db, &video.vod_name).await {
            Ok(true) => {
                println!("⚪ 视频已存在，跳过: {}", video.vod_name);
            }
            Ok(false) => {
                // 视频不存在，创建它
                let filter = doc! { "vod_name": &video.vod_name };
                let update = doc! {
                    "$setOnInsert": {
                        "vod_name": &video.vod_name,
                        "type_id": video.type_id,
                        "vod_status": video.vod_status,
                        "vod_class": &video.vod_class,
                        "vod_pic": &video.vod_pic,
                        "vod_actor": &video.vod_actor,
                        "vod_director": &video.vod_director,
                        "vod_remarks": &video.vod_remarks,
                        "vod_pubdate": video.vod_pubdate,
                        "vod_area": &video.vod_area,
                        "vod_lang": &video.vod_lang,
                        "vod_year": &video.vod_year,
                        "vod_content": &video.vod_content,
                        "vod_play_urls": mongodb::bson::to_bson(&video.vod_play_urls).unwrap(),
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!("✅ 创建视频: {}", video.vod_name);
                created_count += 1;
            }
            Err(e) => {
                eprintln!("❌ 检查视频存在性失败: {}: {}", video.vod_name, e);
                // 继续尝试创建
                let filter = doc! { "vod_name": &video.vod_name };
                let update = doc! {
                    "$setOnInsert": {
                        "vod_name": &video.vod_name,
                        "type_id": video.type_id,
                        "vod_status": video.vod_status,
                        "vod_class": &video.vod_class,
                        "vod_pic": &video.vod_pic,
                        "vod_actor": &video.vod_actor,
                        "vod_director": &video.vod_director,
                        "vod_remarks": &video.vod_remarks,
                        "vod_pubdate": video.vod_pubdate,
                        "vod_area": &video.vod_area,
                        "vod_lang": &video.vod_lang,
                        "vod_year": &video.vod_year,
                        "vod_content": &video.vod_content,
                        "vod_play_urls": mongodb::bson::to_bson(&video.vod_play_urls).unwrap(),
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!("✅ 创建视频（检查失败）: {}", video.vod_name);
                created_count += 1;
            }
        }
    }

    if created_count > 0 {
        println!("测试视频数据初始化完成，新增 {} 个视频", created_count);
    } else {
        println!("测试视频已存在，无需初始化");
    }
    Ok(())
}

// 检查采集源是否已存在
async fn collection_exists(
    db: &Database,
    collect_name: &str,
) -> Result<bool, mongodb::error::Error> {
    let collection = db.collection::<Collection>("collections");
    let count = collection
        .count_documents(doc! { "collect_name": collect_name }, None)
        .await?;
    Ok(count > 0)
}

// 初始化采集源数据
pub async fn init_collection_sources(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let collection = db.collection::<Collection>("collections");

    let collections = vec![
        Collection {
            id: None,
            collect_name: "测试采集源1".to_string(),
            collect_url: "https://api.example1.com/api.php/provide/vod/".to_string(),
            collect_type: 1,
            collect_mid: 1,
            collect_appid: "test_app_1".to_string(),
            collect_appkey: "test_key_1".to_string(),
            collect_param: "ac=videolist".to_string(),
            collect_filter: "".to_string(),
            collect_filter_from: "".to_string(),
            collect_opt: 0,
            collect_sync_pic_opt: 1,
            collect_remove_ad: 1,
            collect_convert_webp: 1,   // 启用webp转换
            collect_download_retry: 3, // 重试3次
            collect_status: 1,
            created_at: DateTime::now(),
            updated_at: DateTime::now(),
        },
        Collection {
            id: None,
            collect_name: "测试采集源2".to_string(),
            collect_url: "https://api.example2.com/api.php/provide/vod/".to_string(),
            collect_type: 1,
            collect_mid: 1,
            collect_appid: "test_app_2".to_string(),
            collect_appkey: "test_key_2".to_string(),
            collect_param: "ac=videolist".to_string(),
            collect_filter: "".to_string(),
            collect_filter_from: "".to_string(),
            collect_opt: 0,
            collect_sync_pic_opt: 1,
            collect_remove_ad: 1,
            collect_convert_webp: 1,   // 启用webp转换
            collect_download_retry: 3, // 重试3次
            collect_status: 1,
            created_at: DateTime::now(),
            updated_at: DateTime::now(),
        },
    ];

    let mut created_count = 0;

    for collect in collections {
        // 先检查采集源是否已存在
        match collection_exists(db, &collect.collect_name).await {
            Ok(true) => {
                println!("⚪ 采集源已存在，跳过: {}", collect.collect_name);
            }
            Ok(false) => {
                // 采集源不存在，创建它
                let filter = doc! { "collect_name": &collect.collect_name };
                let update = doc! {
                    "$setOnInsert": {
                        "collect_name": &collect.collect_name,
                        "collect_url": &collect.collect_url,
                        "collect_type": collect.collect_type,
                        "collect_mid": collect.collect_mid,
                        "collect_appid": &collect.collect_appid,
                        "collect_appkey": &collect.collect_appkey,
                        "collect_param": &collect.collect_param,
                        "collect_filter": &collect.collect_filter,
                        "collect_filter_from": &collect.collect_filter_from,
                        "collect_opt": collect.collect_opt,
                        "collect_sync_pic_opt": collect.collect_sync_pic_opt,
                        "collect_remove_ad": collect.collect_remove_ad,
                        "collect_convert_webp": collect.collect_convert_webp,
                        "collect_download_retry": collect.collect_download_retry,
                        "collect_status": collect.collect_status,
                        "created_at": collect.created_at,
                        "updated_at": collect.updated_at,
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!("✅ 创建采集源: {}", collect.collect_name);
                created_count += 1;
            }
            Err(e) => {
                eprintln!("❌ 检查采集源存在性失败: {}: {}", collect.collect_name, e);
                // 继续尝试创建
                let filter = doc! { "collect_name": &collect.collect_name };
                let update = doc! {
                    "$setOnInsert": {
                        "collect_name": &collect.collect_name,
                        "collect_url": &collect.collect_url,
                        "collect_type": collect.collect_type,
                        "collect_mid": collect.collect_mid,
                        "collect_appid": &collect.collect_appid,
                        "collect_appkey": &collect.collect_appkey,
                        "collect_param": &collect.collect_param,
                        "collect_filter": &collect.collect_filter,
                        "collect_filter_from": &collect.collect_filter_from,
                        "collect_opt": collect.collect_opt,
                        "collect_sync_pic_opt": collect.collect_sync_pic_opt,
                        "collect_remove_ad": collect.collect_remove_ad,
                        "collect_convert_webp": collect.collect_convert_webp,
                        "collect_download_retry": collect.collect_download_retry,
                        "collect_status": collect.collect_status,
                        "created_at": collect.created_at,
                        "updated_at": collect.updated_at,
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!("✅ 创建采集源（检查失败）: {}", collect.collect_name);
                created_count += 1;
            }
        }
    }

    if created_count > 0 {
        println!("采集源数据初始化完成，新增 {} 个采集源", created_count);
    } else {
        println!("采集源已存在，无需初始化");
    }
    Ok(())
}

// 检查绑定是否已存在
async fn binding_exists(db: &Database, binding_id: &str) -> Result<bool, mongodb::error::Error> {
    let collection = db.collection::<Binding>("bindings");
    let count = collection
        .count_documents(doc! { "_id": binding_id }, None)
        .await?;
    Ok(count > 0)
}

// 初始化绑定数据
pub async fn init_bindings(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let collection = db.collection::<Binding>("bindings");
    let now = DateTime::now();

    // 模拟PHP采集管理中的绑定关系：采集源标识_外部分类ID => 本地分类ID
    let bindings = vec![
        Binding {
            id: "7a4856e7b6a1e1a2580a9b69cdc7233c_5".to_string(), // 模拟PHP中的绑定格式
            source_flag: "7a4856e7b6a1e1a2580a9b69cdc7233c".to_string(), // 采集源标识
            external_id: "5".to_string(),                         // 外部分类ID
            local_type_id: 6,                                     // 本地分类ID
            local_type_name: "动作片".to_string(),
            created_at: now,
            updated_at: now,
        },
        Binding {
            id: "test_api_source_1".to_string(),
            source_flag: "test_api_source".to_string(),
            external_id: "1".to_string(),
            local_type_id: 11,
            local_type_name: "动作片".to_string(),
            created_at: now,
            updated_at: now,
        },
        Binding {
            id: "test_api_source_2".to_string(),
            source_flag: "test_api_source".to_string(),
            external_id: "2".to_string(),
            local_type_id: 12,
            local_type_name: "喜剧片".to_string(),
            created_at: now,
            updated_at: now,
        },
    ];

    let mut created_count = 0;

    for binding in bindings {
        // 先检查绑定是否已存在
        match binding_exists(db, &binding.id).await {
            Ok(true) => {
                println!(
                    "⚪ 绑定已存在，跳过: {} -> {}",
                    binding.source_flag, binding.local_type_name
                );
            }
            Ok(false) => {
                // 绑定不存在，创建它
                let filter = doc! { "_id": &binding.id };
                let update = doc! {
                    "$setOnInsert": {
                        "_id": &binding.id,
                        "source_flag": &binding.source_flag,
                        "external_id": &binding.external_id,
                        "local_type_id": binding.local_type_id,
                        "local_type_name": &binding.local_type_name,
                        "created_at": binding.created_at,
                        "updated_at": binding.updated_at,
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!(
                    "✅ 创建绑定: {} -> {}",
                    binding.source_flag, binding.local_type_name
                );
                created_count += 1;
            }
            Err(e) => {
                eprintln!(
                    "❌ 检查绑定存在性失败: {} -> {}: {}",
                    binding.source_flag, binding.local_type_name, e
                );
                // 继续尝试创建
                let filter = doc! { "_id": &binding.id };
                let update = doc! {
                    "$setOnInsert": {
                        "_id": &binding.id,
                        "source_flag": &binding.source_flag,
                        "external_id": &binding.external_id,
                        "local_type_id": binding.local_type_id,
                        "local_type_name": &binding.local_type_name,
                        "created_at": binding.created_at,
                        "updated_at": binding.updated_at,
                    }
                };

                let options = mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build();
                collection.update_one(filter, update, options).await?;
                println!(
                    "✅ 创建绑定（检查失败）: {} -> {}",
                    binding.source_flag, binding.local_type_name
                );
                created_count += 1;
            }
        }
    }

    if created_count > 0 {
        println!("绑定数据初始化完成，新增 {} 个绑定", created_count);
    } else {
        println!("绑定数据已存在，无需初始化");
    }
    Ok(())
}

// 检查数据库是否为空（没有任何数据）
async fn is_database_empty(db: &Database) -> Result<bool, Box<dyn std::error::Error>> {
    // 检查主要集合是否都为空
    let configs_count = db
        .collection::<mongodb::bson::Document>("configs")
        .count_documents(None, None)
        .await?;
    let types_count = db
        .collection::<mongodb::bson::Document>("types")
        .count_documents(None, None)
        .await?;
    let vods_count = db
        .collection::<mongodb::bson::Document>("vods")
        .count_documents(None, None)
        .await?;

    // 如果所有主要集合都为空，则认为数据库是空的
    Ok(configs_count == 0 && types_count == 0 && vods_count == 0)
}

// 执行所有初始化
pub async fn init_all_data(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    // 检查数据库是否为空
    match is_database_empty(db).await {
        Ok(true) => {
            println!("数据库为空，开始初始化数据...");

            init_website_config(db).await?;
            init_test_categories(db).await?;
            init_test_videos(db).await?;
            init_collection_sources(db).await?;
            init_bindings(db).await?;

            println!("所有数据初始化完成！");
        }
        Ok(false) => {
            println!("数据库已包含数据，跳过初始化");
        }
        Err(e) => {
            eprintln!("检查数据库状态失败: {}，跳过初始化", e);
        }
    }

    Ok(())
}
