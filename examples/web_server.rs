use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::{Local, NaiveDateTime, TimeZone};
use hik_net_sdk::device::{Channel, HikDevice};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::fs as tokio_fs;

// 嵌入 HTML 文件到程序中
const INDEX_HTML: &str = include_str!("web_index.html");

// 统一的错误响应类型
#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    message: String,
}

// 自定义错误类型，包装 anyhow::Error 并实现 IntoResponse
#[derive(Debug)]
struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let body = Json(ErrorResponse {
            success: false,
            message: self.0.to_string(),
        });
        (status, body).into_response()
    }
}

// 实现 From trait，方便从 anyhow::Error 转换
impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError(error)
    }
}

// 为常见错误类型实现 From trait
impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError(anyhow::Error::from(error))
    }
}

impl From<axum::http::header::InvalidHeaderValue> for AppError {
    fn from(error: axum::http::header::InvalidHeaderValue) -> Self {
        AppError(anyhow::Error::from(error))
    }
}

#[derive(Clone)]
struct AppState {
    devices: Arc<Mutex<HashMap<String, HikDevice>>>,
    images_dir: PathBuf,
}

#[derive(Deserialize)]
struct LoginRequest {
    host: String,
    port: u16,
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    success: bool,
    message: String,
    session_id: Option<String>,
}

#[derive(Serialize)]
struct ChannelInfo {
    channel_num: u16,
    channel_type: String,
    enabled: bool,
    ipv4_address: Option<String>,
}

#[derive(Serialize)]
struct ChannelsResponse {
    success: bool,
    channels: Vec<ChannelInfo>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct CaptureImageRequest {
    channel: u16,
}

#[derive(Serialize)]
struct CaptureImageResponse {
    success: bool,
    image_url: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct DownloadRequest {
    channel: u16,
    start_time: String,
    end_time: String,
}

#[derive(Serialize)]
struct DownloadResponse {
    success: bool,
    message: String,
    download_id: Option<String>,
}

#[tokio::main]
async fn main() {
    // 创建图片存储目录
    let images_dir = PathBuf::from("images");
    if !images_dir.exists() {
        fs::create_dir_all(&images_dir).expect("Failed to create images directory");
    }

    let app_state = AppState {
        devices: Arc::new(Mutex::new(HashMap::new())),
        images_dir,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/login", post(login))
        .route("/api/channels", get(get_channels))
        .route("/api/capture", post(capture_image))
        .route("/api/download", post(download_recording))
        .route("/images/:filename", get(get_image))
        .route("/recordings/:filename", get(get_recording))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let mut device = HikDevice::new();
    device.login(&req.host, &req.username, &req.password, req.port)?;

    let session_id = format!("{}_{}", req.host, req.port);
    let mut devices = state.devices.lock().unwrap();
    devices.insert(session_id.clone(), device);

    Ok(Json(LoginResponse {
        success: true,
        message: "Login successful".to_string(),
        session_id: Some(session_id),
    }))
}

async fn get_channels(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ChannelsResponse>, AppError> {
    let session_id = params
        .get("session_id")
        .ok_or_else(|| anyhow::anyhow!("session_id is required"))?;

    let devices = state.devices.lock().unwrap();
    let device = devices
        .get(session_id)
        .ok_or_else(|| anyhow::anyhow!("Device not found. Please login first."))?;

    let channels = device.get_channels()?;

    let channel_infos: Vec<ChannelInfo> = channels
        .iter()
        .map(|ch| match ch {
            Channel::Logic(info) => ChannelInfo {
                channel_num: info.get_chan_num(),
                channel_type: "Logic".to_string(),
                enabled: info.is_enabled(),
                ipv4_address: None,
            },
            Channel::IP(info) => ChannelInfo {
                channel_num: info.get_chan_num(),
                channel_type: "IP".to_string(),
                enabled: info.is_enabled(),
                ipv4_address: info.get_ipv4_address().cloned(),
            },
        })
        .collect();

    Ok(Json(ChannelsResponse {
        success: true,
        channels: channel_infos,
        message: None,
    }))
}

async fn capture_image(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    Json(req): Json<CaptureImageRequest>,
) -> Result<Json<CaptureImageResponse>, AppError> {
    let session_id = params
        .get("session_id")
        .ok_or_else(|| anyhow::anyhow!("session_id is required"))?;

    let devices = state.devices.lock().unwrap();
    let device = devices
        .get(session_id)
        .ok_or_else(|| anyhow::anyhow!("Device not found. Please login first."))?;

    let filename = format!(
        "channel_{}_{}.jpg",
        req.channel,
        chrono::Utc::now().timestamp()
    );
    let filepath = state.images_dir.join(&filename);

    device.capture_jpeg_picture(req.channel, filepath.to_str().unwrap())?;

    // 检查文件是否存在
    if !filepath.exists() {
        return Err(AppError::from(anyhow::anyhow!(
            "Image file not found after capture"
        )));
    }

    Ok(Json(CaptureImageResponse {
        success: true,
        image_url: Some(format!("/images/{}", filename)),
        message: None,
    }))
}

async fn download_recording(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    Json(req): Json<DownloadRequest>,
) -> Result<Json<DownloadResponse>, AppError> {
    let session_id = params
        .get("session_id")
        .ok_or_else(|| anyhow::anyhow!("session_id is required"))?;

    let devices = state.devices.lock().unwrap();
    let device = devices
        .get(session_id)
        .ok_or_else(|| anyhow::anyhow!("Device not found. Please login first."))?;

    // 解析时间字符串
    let start_time =
        NaiveDateTime::parse_from_str(&req.start_time, "%Y-%m-%d %H:%M:%S").map_err(|_| {
            AppError::from(anyhow::anyhow!(
                "Invalid start_time format. Use: YYYY-MM-DD HH:MM:SS"
            ))
        })?;
    let start_time = match Local.from_local_datetime(&start_time) {
        chrono::LocalResult::Single(t) => t,
        _ => {
            return Err(AppError::from(anyhow::anyhow!(
                "Invalid start_time: ambiguous or non-existent time"
            )))
        }
    };

    let end_time =
        NaiveDateTime::parse_from_str(&req.end_time, "%Y-%m-%d %H:%M:%S").map_err(|_| {
            AppError::from(anyhow::anyhow!(
                "Invalid end_time format. Use: YYYY-MM-DD HH:MM:SS"
            ))
        })?;
    let end_time = match Local.from_local_datetime(&end_time) {
        chrono::LocalResult::Single(t) => t,
        _ => {
            return Err(AppError::from(anyhow::anyhow!(
                "Invalid end_time: ambiguous or non-existent time"
            )))
        }
    };

    let filename = format!(
        "recording_ch{}_{}_{}.dav",
        req.channel,
        start_time.format("%Y%m%d_%H%M%S"),
        end_time.format("%Y%m%d_%H%M%S")
    );
    let filepath = state.images_dir.join("recordings").join(&filename);

    // 确保目录存在
    if let Some(parent) = filepath.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut download = device.get_file_by_time(
        filepath.to_str().unwrap(),
        req.channel,
        start_time,
        end_time,
    )?;

    download.start()?;

    // 异步等待下载完成（简化版本，实际应该用后台任务）
    tokio::spawn(async move {
        loop {
            match download.get_progress() {
                Ok(progress) => {
                    if progress >= 100 {
                        break;
                    }
                }
                Err(_) => break,
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    Ok(Json(DownloadResponse {
        success: true,
        message: "Download started".to_string(),
        download_id: Some(filename),
    }))
}

async fn get_image(Path(filename): Path<String>) -> Result<Response, AppError> {
    let filepath = PathBuf::from("images").join(&filename);

    let data = tokio_fs::read(&filepath)
        .await
        .map_err(|_| AppError::from(anyhow::anyhow!("Image not found")))?;

    let content_type = if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_str(content_type)?,
    );

    Ok((StatusCode::OK, headers, Bytes::from(data)).into_response())
}

async fn get_recording(Path(filename): Path<String>) -> Result<Response, AppError> {
    let filepath = PathBuf::from("images").join("recordings").join(&filename);

    let data = tokio_fs::read(&filepath)
        .await
        .map_err(|_| AppError::from(anyhow::anyhow!("Recording not found")))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_str("application/octet-stream")?,
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))?,
    );

    Ok((StatusCode::OK, headers, Bytes::from(data)).into_response())
}
