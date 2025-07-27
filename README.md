# mock-qcc

一个基于Actix-web的Rust后端服务，提供企查查接口模拟功能

## 使用

### 通过[mkcert]( https://github.com/FiloSottile/mkcert.git)配置证书

### 修复dns解析

将 `api.qichacha.com` 解析到指定IP,例如: `127.0.0.1`

### 启动服务

```bash
mock-qcc 443 "/data" "/data/cert.pem" "/data/key.pem"
```

- 443 https端口
- /data 数据目录
- /data/cert.pem 证书文件
- /data/key.pem 密钥文件