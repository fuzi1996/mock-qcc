use actix_web::middleware::Logger;
use actix_web::{
    error::{ErrorBadRequest, ErrorForbidden, ErrorInternalServerError},
    get, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result,
};
use log::{error, info, LevelFilter};
use percent_encoding::percent_decode_str;
use rand::Rng;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};


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
            .filter(None, log_level)
            .filter(Some("actix_http"), LevelFilter::Debug)
            .target(env_logger::Target::Stdout)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter(None, LevelFilter::Info)
            .filter(Some("actix_http"), LevelFilter::Debug)
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

fn load_tls_config(cert_path: &str, key_path: &str) -> ServerConfig {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();
    // load TLS key/cert files
    let cert_chain = CertificateDer::pem_file_iter(cert_path)
        .unwrap()
        .flatten()
        .collect();

    let key_der =
        PrivateKeyDer::from_pem_file(key_path).expect("Could not locate PKCS 8 private keys.");

    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)
        .unwrap()
}

#[get("/{path:.*}")]
async fn handle_api_request(
    req: HttpRequest,
    path: web::Path<String>,
    web::Query(mut params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    // 解码URL编码的路径
    let decoded_path = percent_decode_str(&path)
        .decode_utf8()
        .map_err(|_| ErrorBadRequest("Invalid URL encoding"))?;

    // 记录请求头信息
    info!("Request headers: {:?}", req.headers());
    // 获取当前工作目录
    let work_dir = std::env::current_dir().unwrap();

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

    // 处理路径
    let mut parse_handler: Vec<Box<dyn ParseHandlerTrait>> = Vec::new();
    parse_handler.push(Box::new(Parse628Struct));

    let mut file_name = String::from("");;
    let mut path_from_url = String::from("");
    let mut is_match = false;
    for handler in parse_handler.iter() {
        if handler.is_match(&file_name) {
            let (params, new_file_name) = handler.parse(params);
            file_name = new_file_name;
            path_from_url = params.join("/").to_string();
            is_match = true;
            break;
        }
    }
    if !is_match {
        return Err(ErrorBadRequest("Invalid path"));
    }
    file_path.push(path_from_url);
    file_path.set_file_name(file_name);

    info!("Request: {:?}", file_path);

    // 验证路径是否在data目录内
    if !file_path.starts_with(format!("{}/data", work_dir.display())) {
        return Err(ErrorForbidden("Access denied"));
    }

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
    let cert_path = env::args().nth(3).unwrap_or("cert.pem".to_string());
    let key_path = env::args().nth(4).unwrap_or("key.pem".to_string());

    set_work_dir(&work_dir);

    // 加载TLS配置
    let tls_config = load_tls_config(&cert_path, &key_path);

    info!(
        "Server is running on https://{}:{} and http://{}:{}",
        host, port, host, port
    );

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
    })
        .bind_rustls_0_23(format!("{}:{}", host, port), tls_config)?
        .run()
        .await;

    Ok(())
}
