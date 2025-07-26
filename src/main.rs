use actix_web::{get, web, App, HttpResponse, HttpServer, Result};
use std::fs;
use std::path::PathBuf;
#[get("/{path:.*}")]
async fn handle_api_request(path: web::Path<String>) -> Result<HttpResponse> {
    let path_str = path.into_inner();
    let mut file_path = PathBuf::from("./data");
    file_path.push(path_str);
    file_path.set_extension("json");
    match fs::read_to_string(&file_path) {
        Ok(content) => Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(content)),
        Err(_) => Ok(HttpResponse::NotFound()
            .content_type("application/json")
            .body(format!(
                "{{\"error\":\"File {} not found\"}}",
                file_path.display()
            ))),
    }
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = 80;
    println!("服务器启动中，监听端口 {}...", port);
    HttpServer::new(|| App::new().service(handle_api_request))
        .bind(format!("127.0.0.1:{}", port))?
        .run()
        .await
}
