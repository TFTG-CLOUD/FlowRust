# 分布式储存管理系统使用说明

## 功能概述

本系统为MacCMS-rust添加了分布式储存管理功能，支持：

1. **分布式储存服务器管理**
   - 添加、编辑、删除分布式储存服务器
   - 测试服务器连接状态
   - 服务器启用/禁用管理

2. **预签名上传URL生成**
   - 单文件上传预签名URL
   - 分片上传预签名URL  
   - 压缩包上传预签名URL

## 系统架构

### 数据模型

```rust
// 分布式储存服务器
pub struct StorageServer {
    pub name: String,           // 服务器名称
    pub host: String,           // 服务器地址
    pub api_key: String,        // API密钥
    pub api_secret: String,     // API密钥
    pub cms_id: String,         // CMS标识
    pub status: i32,           // 状态：1启用，0禁用
    pub created_at: DateTime,   // 创建时间
    pub updated_at: DateTime,   // 更新时间
}

// 预签名上传响应
pub struct PresignedUploadResponse {
    pub upload_url: String,     // 预签名上传URL
    pub file_id: String,        // 文件ID
    pub expiration: i64,        // 过期时间戳
    pub max_file_size: i64,     // 最大文件大小
}

// 分片上传信息
pub struct ChunkUploadInfo {
    pub upload_id: String,      // 上传ID
    pub chunk_size: i64,        // 分片大小
    pub total_chunks: i32,       // 总分片数
    pub chunk_urls: Vec<String>, // 分片URL列表
}
```

### API接口

#### 1. 服务器管理

**获取服务器列表**
```
GET /api/admin/storage/servers
```

**添加服务器**
```
POST /api/admin/storage/servers
Content-Type: application/json

{
    "name": "服务器名称",
    "host": "https://storage.example.com",
    "api_key": "your_api_key",
    "api_secret": "your_api_secret", 
    "cms_id": "your_cms_id"
}
```

**更新服务器**
```
PUT /api/admin/storage/servers/{id}
Content-Type: application/json

{
    "name": "新名称",
    "host": "https://new-host.example.com",
    "status": 1
}
```

**删除服务器**
```
DELETE /api/admin/storage/servers/{id}
```

**测试连接**
```
POST /api/admin/storage/servers/{id}/test
```

#### 2. 预签名URL生成

**单文件上传**
```
POST /api/admin/storage/upload/single/{server_id}
Content-Type: application/json

{
    "filename": "example.jpg",
    "content_type": "image/jpeg",
    "file_size": 1048576,
    "upload_type": "single"
}
```

**分片上传**
```
POST /api/admin/storage/upload/chunk/{server_id}
Content-Type: application/json

{
    "filename": "large_file.mp4",
    "content_type": "video/mp4", 
    "file_size": 104857600,
    "upload_type": "chunk"
}
```

**压缩包上传**
```
POST /api/admin/storage/upload/archive/{server_id}
Content-Type: application/json

{
    "filename": "archive.zip",
    "content_type": "application/zip",
    "file_size": 52428800,
    "upload_type": "archive"
}
```

## 使用流程

### 1. 添加分布式储存服务器

1. 登录MacCMS-rust管理后台
2. 点击左侧菜单中的"分布式储存"
3. 点击"添加服务器"按钮
4. 填写服务器信息：
   - **服务器名称**: 给服务器起个易于识别的名称
   - **服务器地址**: 分布式储存处理程序的API地址
   - **API Key**: 与分布式储存处理程序约定的密钥
   - **API Secret**: 与分布式储存处理程序约定的密钥
   - **CMS ID**: 用于标识当前CMS系统
5. 点击"创建"完成添加

### 2. 测试服务器连接

1. 在服务器列表中找到目标服务器
2. 点击操作列的"测试连接"按钮（闪电图标）
3. 系统会显示连接是否成功

### 3. 生成上传预签名URL

#### 方法一：通过管理界面

1. 在服务器列表中找到目标服务器
2. 点击操作列的"编辑"按钮
3. 在弹出的编辑窗口中可以选择生成不同类型的上传URL

#### 方法二：通过API调用

可以使用上述API接口直接调用生成预签名URL。

### 4. 前端集成

生成的预签名URL可以直接传递给前端用于文件上传：

```javascript
// 单文件上传示例
const uploadUrl = response.data.upload_url;
const file = document.getElementById('fileInput').files[0];

fetch(uploadUrl, {
    method: 'PUT',
    headers: {
        'Content-Type': file.type
    },
    body: file
})
.then(response => response.json())
.then(data => {
    console.log('上传成功:', data);
})
.catch(error => {
    console.error('上传失败:', error);
});
```

## 安全特性

1. **HMAC-SHA256签名**: 使用API密钥对上传请求进行签名验证
2. **过期时间**: 预签名URL具有时效性，默认1小时过期
3. **文件大小限制**: 在生成URL时指定最大文件大小
4. **密钥管理**: API密钥和密钥安全存储，不在前端暴露

## 注意事项

1. 确保分布式储存处理程序的API地址可正常访问
2. API密钥和密钥需要与分布式储存处理程序保持一致
3. 预签名URL有过期时间，请及时使用
4. 建议定期测试服务器连接状态
5. 分片上传适合大文件，单文件上传适合小文件

## 错误处理

系统会返回标准化的错误响应：

```json
{
    "code": 500,
    "msg": "错误信息",
    "data": null,
    "success": false
}
```

常见错误码：
- 200: 成功
- 500: 服务器内部错误
- 404: 服务器未找到
- 400: 请求参数错误