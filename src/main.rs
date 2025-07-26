use actix_web::{
    error::{ErrorBadRequest, ErrorForbidden, ErrorInternalServerError},
    get, web, App, HttpResponse, HttpServer, Result,
};
use log::{error, info, LevelFilter};
use percent_encoding::percent_decode_str;
use rand::Rng;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tokio::try_join;

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

fn load_tls_config(cert_path: &str, key_path: &str) -> ServerConfig {
    // 读取证书文件
    let cert_file = File::open(cert_path).expect("Failed to open certificate file");
    let mut cert_reader = BufReader::new(cert_file);
    let cert_chain = certs(&mut cert_reader)
        .expect("Failed to parse certificate")
        .into_iter()
        .map(Certificate)
        .collect();

    // 读取私钥文件
    let key_file = File::open(key_path).expect("Failed to open private key file");
    let mut key_reader = BufReader::new(key_file);
    let mut keys = pkcs8_private_keys(&mut key_reader)
        .expect("Failed to parse private key")
        .into_iter()
        .map(PrivateKey);

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.next().expect("No private key found"))
        .expect("Failed to create TLS configuration");

    config
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

    // 检查是否需要使用Parse628解析器
    let parser = Parse628Struct;
    if parser.is_match(&decoded_path) {
        let (query_values, filename) = parser.parse(params.clone());
        if !query_values.is_empty() {
            let combined_params = query_values.join("_");
            file_path.push(combined_params);
        }
        file_path.push(filename);
    } else {
        // 提取并验证分页参数
        let (page_index, page_size) = match (params.remove("pageIndex"), params.remove("pageSize"))
        {
            (None, None) => (1, 10),
            (Some(pi), None) => {
                let index = percent_decode_str(&pi)
                    .decode_utf8()
                    .map_err(|_| ErrorBadRequest("Invalid pageIndex encoding"))?;
                (
                    index
                        .parse()
                        .map_err(|_| ErrorBadRequest("pageIndex must be a number"))?,
                    10,
                )
            }
            (None, Some(ps)) => {
                let size = percent_decode_str(&ps)
                    .decode_utf8()
                    .map_err(|_| ErrorBadRequest("Invalid pageSize encoding"))?;
                (
                    1,
                    size.parse()
                        .map_err(|_| ErrorBadRequest("pageSize must be a number"))?,
                )
            }
            (Some(pi), Some(ps)) => {
                let index = percent_decode_str(&pi)
                    .decode_utf8()
                    .map_err(|_| ErrorBadRequest("Invalid pageIndex encoding"))?;
                let size = percent_decode_str(&ps)
                    .decode_utf8()
                    .map_err(|_| ErrorBadRequest("Invalid pageSize encoding"))?;
                (
                    index
                        .parse()
                        .map_err(|_| ErrorBadRequest("pageIndex must be a number"))?,
                    size.parse()
                        .map_err(|_| ErrorBadRequest("pageSize must be a number"))?,
                )
            }
        };

        // 处理主查询参数: 过滤key参数，按Unicode排序后的值用下划线连接作为子目录
        let mut query_values: Vec<String> = params
            .into_iter()
            .filter(|(k, _)| k != "key")
            .map(|(_, v)| {
                // 解码查询参数值并转换为String
                percent_decode_str(&v)
                    .decode_utf8()
                    .map(|s| s.into_owned())
                    .map_err(|_| ErrorBadRequest("Invalid URL encoding in query parameters"))
            })
            .collect::<Result<_, _>>()?;

        // 按Unicode排序
        query_values.sort();

        // 多个参数值用下划线连接
        if !query_values.is_empty() {
            let combined_params = query_values.join("_");
            file_path.push(combined_params);
        }

        // 设置分页文件名
        let filename = format!("{}_{}.json", page_index, page_size);
        file_path.push(filename);
    }

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

    let https_server = HttpServer::new(|| App::new().service(handle_api_request))
        .bind_rustls(format!("{}:{}", host, port), tls_config)?
        .run();

    let http_server = HttpServer::new(|| App::new().service(handle_api_request))
        .bind(format!("{}:443", host))?
        .run();

    tokio::try_join!(https_server, http_server)?;
    Ok(())
}
