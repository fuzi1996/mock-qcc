use actix_web::{
    error::{ErrorBadRequest, ErrorForbidden, ErrorInternalServerError},
    get, web, App, HttpResponse, HttpServer, Result,
};
use log::{error, info, LevelFilter};
use percent_encoding::percent_decode_str;
use serde_json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, env::current_dir};

mod parse_628;
mod parse_handler;
use crate::parse_628::Parse628 as Parse628Struct;
use crate::parse_handler::ParseHandler as ParseHandlerTrait;

fn init_log() {
    if let Ok(log_level) = std::env::var("LOG_LEVEL") {
        let log_level = log_level
            .parse::<LevelFilter>()
            .unwrap_or(LevelFilter::Info);
        env_logger::Builder::from_default_env()
            .filter_level(log_level)
            .target(env_logger::Target::Stdout)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(LevelFilter::Info)
            .target(env_logger::Target::Stdout)
            .init();
    }
}

fn set_work_dir(work_dir: &str) {
    if !Path::new(work_dir).exists() {
        error!("Work directory {} does not exist", work_dir);
        std::process::exit(1);
    }
    std::env::set_current_dir(work_dir).unwrap();
}

#[get("/{path:.*}")]
async fn handle_api_request(
    path: web::Path<String>,
    web::Query(mut params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    // 解码URL编码的路径
    let decoded_path = percent_decode_str(&path)
        .decode_utf8()
        .map_err(|_| ErrorBadRequest("Invalid URL encoding"))?;

    let work_dir = current_dir().unwrap();
    // 构建基础路径
    let mut file_path = PathBuf::from(format!("{}/data", work_dir.display()));

    // 分割路径并处理每一部分
    for segment in decoded_path.split('/').filter(|s| !s.is_empty()) {
        // 防止路径遍历攻击
        if segment == ".." {
            error!("Path {} not allowed", decoded_path);
            return Err(ErrorForbidden("Path traversal not allowed"));
        }
        file_path.push(segment);
    }

    let handlers: Vec<Box<dyn ParseHandlerTrait>> = vec![Box::new(Parse628Struct)];

    let mut is_matched = false;
    for handler in handlers.iter() {
        if handler.is_match(&path) {
            let (query_values, filename) = handler.parse(params.clone());
            if !query_values.is_empty() {
                let combined_params = query_values.join("_");
                file_path.push(combined_params);
            }
            file_path.push(filename);
            is_matched = true;
            break;
        }
    }

    if !is_matched {
        error!("Path {} not matched", decoded_path);
        return Err(ErrorBadRequest("Path not matched"));
    }

    info!("Request: {:?}", file_path);

    // 读取文件并返回
    match fs::read_to_string(&file_path) {
        Ok(content) => Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(content)),
        Err(_) => {
            let error_response = serde_json::json!({"code": 404, "message": "数据文件未找到", "request_id": generate_request_id()});
            let error_body = serde_json::to_string(&error_response)
                .map_err(|_| ErrorInternalServerError("Failed to serialize error response"))?;
            Ok(HttpResponse::NotFound()
                .content_type("application/json")
                .body(error_body))
        }
    }
}

// 生成随机请求ID
fn generate_request_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    (0..16)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_log();

    let host = env::var("HOST").unwrap_or("127.0.0.1".to_string());
    let port = env::args().nth(1).unwrap_or("7878".to_string());
    let work_dir = env::args().nth(2).unwrap_or(".".to_string());

    set_work_dir(&work_dir);

    info!(
        "Server is running on http://{}:{} in {}",
        host, port, work_dir
    );

    HttpServer::new(|| App::new().service(handle_api_request))
        .bind(format!("{}:{}", host, port))?
        .run()
        .await
}
