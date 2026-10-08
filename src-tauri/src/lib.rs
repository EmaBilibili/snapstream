use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    pub formats_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub id: String,
    pub percent: f32,
    pub speed: String,
    pub eta: String,
    pub status: String,
    pub filename: Option<String>,
}

fn get_ytdlp_cmd() -> std::path::PathBuf {
    let local = std::path::Path::new("binaries/yt-dlp");
    if local.exists() {
        return local.to_path_buf();
    }
    let local_src = std::path::Path::new("src-tauri/binaries/yt-dlp");
    if local_src.exists() {
        return local_src.to_path_buf();
    }
    std::path::PathBuf::from("yt-dlp")
}

#[tauri::command]
fn get_media_info(url: String) -> Result<MediaMetadata, String> {
    let ytdlp_path = get_ytdlp_cmd();

    let output = Command::new(&ytdlp_path)
        .args([
            "--dump-json",
            "--no-playlist",
            "--no-warnings",
            &url,
        ])
        .output()
        .map_err(|e| format!("Error al ejecutar yt-dlp: {}", e))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr);
        return Err(format!("No se pudo obtener información del enlace: {}", err_str));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Error analizando metadatos: {}", e))?;

    let title = v["title"].as_str().unwrap_or("Sin título").to_string();
    let thumbnail = v["thumbnail"].as_str().map(|s| s.to_string());
    let duration = v["duration"].as_f64();
    let uploader = v["uploader"].as_str().map(|s| s.to_string());

    Ok(MediaMetadata {
        title,
        thumbnail,
        duration,
        uploader,
        formats_summary: vec![
            "Mejor calidad".to_string(),
            "1080p".to_string(),
            "720p".to_string(),
            "Solo Audio (MP3)".to_string(),
        ],
    })
}

#[tauri::command]
fn start_download(
    app: AppHandle,
    id: String,
    url: String,
    output_dir: String,
    mode: String,
    quality: String,
) -> Result<String, String> {
    let ytdlp_path = get_ytdlp_cmd();
    let download_id = id.clone();

    std::thread::spawn(move || {
        let mut cmd = Command::new(&ytdlp_path);
        let output_template = format!("{}/%(title)s.%(ext)s", output_dir.trim_end_matches('/'));

        cmd.args(["--newline", "-o", &output_template]);

        if mode == "audio" {
            cmd.args([
                "-x",
                "--audio-format",
                if quality.is_empty() { "mp3" } else { &quality },
            ]);
        } else {
            match quality.as_str() {
                "1080p" => {
                    cmd.args(["-f", "bestvideo[height<=1080]+bestaudio/best[height<=1080]"]);
                }
                "720p" => {
                    cmd.args(["-f", "bestvideo[height<=720]+bestaudio/best[height<=720]"]);
                }
                _ => {
                    cmd.args(["-f", "bestvideo+bestaudio/best"]);
                }
            }
            cmd.args(["--merge-output-format", "mp4"]);
        }

        cmd.arg(&url);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit(
                    "download-progress",
                    DownloadProgress {
                        id: download_id.clone(),
                        percent: 0.0,
                        speed: "".into(),
                        eta: "".into(),
                        status: "error".into(),
                        filename: Some(e.to_string()),
                    },
                );
                return;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if line.starts_with("[download]") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let mut percent = 0.0;
                    let mut speed = String::new();
                    let mut eta = String::new();

                    for (i, p) in parts.iter().enumerate() {
                        if p.ends_with('%') {
                            if let Ok(val) = p.trim_end_matches('%').parse::<f32>() {
                                percent = val;
                            }
                        }
                        if *p == "at" && i + 1 < parts.len() {
                            speed = parts[i + 1].to_string();
                        }
                        if *p == "ETA" && i + 1 < parts.len() {
                            eta = parts[i + 1].to_string();
                        }
                    }

                    let _ = app.emit(
                        "download-progress",
                        DownloadProgress {
                            id: download_id.clone(),
                            percent,
                            speed,
                            eta,
                            status: "downloading".into(),
                            filename: None,
                        },
                    );
                } else if line.starts_with("[ExtractAudio]") || line.starts_with("[Merger]") {
                    let _ = app.emit(
                        "download-progress",
                        DownloadProgress {
                            id: download_id.clone(),
                            percent: 99.0,
                            speed: "".into(),
                            eta: "".into(),
                            status: "processing".into(),
                            filename: None,
                        },
                    );
                }
            }
        }

        let status = child.wait();
        match status {
            Ok(s) if s.success() => {
                let _ = app.emit(
                    "download-progress",
                    DownloadProgress {
                        id: download_id,
                        percent: 100.0,
                        speed: "".into(),
                        eta: "".into(),
                        status: "finished".into(),
                        filename: None,
                    },
                );
            }
            _ => {
                let _ = app.emit(
                    "download-progress",
                    DownloadProgress {
                        id: download_id,
                        percent: 0.0,
                        speed: "".into(),
                        eta: "".into(),
                        status: "error".into(),
                        filename: Some("Fallo en la descarga o procesamiento".into()),
                    },
                );
            }
        }
    });

    Ok(id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![get_media_info, start_download])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
