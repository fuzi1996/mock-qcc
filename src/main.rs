use actix_web::{
  error::{ErrorBadRequest, ErrorForbidden, ErrorInternalServerError},
  get, web, App, HttpResponse, HttpServer, Result,
};
use log::{info, LevelFilter};
use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[get("/{path:.*}")]
async fn handle_api_request(
  path: web::Path<String>,
  web::Query(mut params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
  // 解码URL编码的路径
  let decoded_path = percent_decode_str(&path)
    .decode_utf8()
    .map_err(|_| ErrorBadRequest("Invalid URL encoding"))?;

  // 构建基础路径
  let mut file_path = PathBuf::from("./data");

  // 分割路径并处理每一部分
  for segment in decoded_path.split('/').filter(|s| !s.is_empty()) {
    // 防止路径遍历攻击
    if segment == ".." {
      return Err(ErrorForbidden("Path traversal not allowed"));
    }
    file_path.push(segment);
  }

  // 提取并验证分页参数
  let (page_index, page_size) = match (params.remove("pageIndex"), params.remove("pageSize")) {
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
        size
          .parse()
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
        size
          .parse()
          .map_err(|_| ErrorBadRequest("pageSize must be a number"))?,
      )
    }
  };

  // 处理主查询参数: 过滤key参数，按Unicode排序后的值用下划线连接作为子目录
  let mut query_values: Vec<String> = params
    .into_iter()
    .filter(|(k, _)| k != "key")
    .filter(|(k, _)| k != "percent")
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

  // 验证路径是否在data目录内
  if !file_path.starts_with("./data") {
    return Err(ErrorForbidden("Access denied"));
  }

  info!("Request: {:?}", file_path);

  // 读取文件并返回
  match fs::read_to_string(&file_path) {
    Ok(content) => Ok(
      HttpResponse::Ok()
        .content_type("application/json")
        .body(content),
    ),
    Err(_) => {
      let error_response = serde_json::json!({
          "code": 404,
          "message": "数据文件未找到",
          "request_id": generate_request_id()
      });
      let error_body = serde_json::to_string(&error_response)
        .map_err(|_| ErrorInternalServerError("Failed to serialize error response"))?;
      Ok(
        HttpResponse::NotFound()
          .content_type("application/json")
          .body(error_body),
      )
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
  HttpServer::new(|| App::new().service(handle_api_request))
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

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
