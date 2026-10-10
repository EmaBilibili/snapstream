use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDownloadPayload {
    pub url: String,
    pub title: String,
    pub mode: String,
    pub quality: String,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub ip: String,
    pub port: u16,
    pub url: String,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    pub is_direct_image: bool,
    pub is_playlist: bool,
    pub playlist_count: Option<usize>,
    pub playlist_items: Option<Vec<PlaylistItem>>,
    pub formats_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub id: String,
    pub percent: f32,
    pub speed: String,
    pub eta: String,
    pub status: String,
    pub filename: Option<String>,
    pub output_path: Option<String>,
}

fn get_ytdlp_cmd() -> std::path::PathBuf {
    // 1. Si existe en PATH del sistema
    if let Ok(p) = which::which("yt-dlp") {
        return p;
    }

    // 2. Comprobar ~/.local/bin/yt-dlp
    if let Some(home) = std::env::var_os("HOME") {
        let user_bin = std::path::PathBuf::from(home).join(".local/bin/yt-dlp");
        if user_bin.exists() {
            return user_bin;
        }
    }

    // 3. Comprobar relativo al ejecutable de la aplicación
    if let Ok(mut exe) = std::env::current_exe() {
        exe.pop();
        let sibling = exe.join("yt-dlp");
        if sibling.exists() {
            return sibling;
        }
        let in_bin = exe.join("binaries/yt-dlp");
        if in_bin.exists() {
            return in_bin;
        }
    }

    // 4. En entorno de desarrollo
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

fn get_browser_cookie_arg() -> Option<String> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let candidates = [
            ("brave", "brave"),
            ("google-chrome", "chrome"),
            ("google-chrome-stable", "chrome"),
            ("chromium", "chromium"),
            ("firefox", "firefox"),
            ("zen-browser", "firefox"),
            ("vivaldi", "vivaldi"),
            ("opera", "opera"),
        ];

        for (bin, browser_name) in candidates {
            if which::which(bin).is_ok() {
                return Some(browser_name.to_string());
            }
        }
    }
    None
}

fn normalize_social_url(url: &str) -> String {
    let clean = url.trim().to_string();
    // YouTube Music comparte IDs idénticos con YouTube estándar,
    // convertir music.youtube.com a www.youtube.com resuelve bloqueos y desvía listas automáticas de radio
    if clean.contains("music.youtube.com") {
        return clean.replace("music.youtube.com", "www.youtube.com");
    }
    clean
}

async fn extract_og_metadata(url: &str) -> Option<MediaMetadata> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
        .build()
        .ok()?;

    let res = client.get(url).send().await.ok()?;
    let text = res.text().await.ok()?;
    let document = Html::parse_document(&text);

    let title_sel = Selector::parse("meta[property='og:title']").ok()?;
    let img_sel = Selector::parse("meta[property='og:image']").ok()?;
    let video_sel = Selector::parse("meta[property='og:video']").ok()?;

    let title = document
        .select(&title_sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .unwrap_or("Recurso Multimedia")
        .to_string();

    let thumbnail = document
        .select(&img_sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string());

    let has_video = document.select(&video_sel).next().is_some();

    Some(MediaMetadata {
        title,
        thumbnail,
        duration: None,
        uploader: None,
        is_direct_image: !has_video,
        is_playlist: false,
        playlist_count: None,
        playlist_items: None,
        formats_summary: if has_video {
            vec!["Video Original".into(), "Solo Audio (MP3)".into()]
        } else {
            vec!["Imagen Alta Resolución".into()]
        },
    })
}

#[tauri::command]
async fn get_media_info(url: String) -> Result<MediaMetadata, String> {
    let clean_url = normalize_social_url(&url);
    let ytdlp_path = get_ytdlp_cmd();

    // Preparar argumentos para detectar info y playlists sin descargar el flujo completo
    let cmd_base_args = [
        "--dump-single-json",
        "--flat-playlist",
        "--no-warnings",
    ];

    let browser_cookie = get_browser_cookie_arg();
    let is_private_playlist = clean_url.contains("list=LM") || clean_url.contains("list=LL") || clean_url.contains("list=WL");

    let mut output = None;

    // Si es una playlist privada como Liked Music (list=LM), usar cookies directamente
    if is_private_playlist {
        if let Some(ref browser) = browser_cookie {
            let mut args = cmd_base_args.to_vec();
            args.push("--cookies-from-browser");
            args.push(browser);
            args.push(&clean_url);
            output = Command::new(&ytdlp_path).args(args).output().ok();
        }
    }

    if output.is_none() || !output.as_ref().map(|o| o.status.success()).unwrap_or(false) {
        let mut args = cmd_base_args.to_vec();
        args.push(&clean_url);
        output = Command::new(&ytdlp_path).args(args).output().ok();
    }

    // Si falló y tenemos navegador disponible, reintentar con cookies de navegador
    if (output.is_none() || !output.as_ref().map(|o| o.status.success()).unwrap_or(false)) && !is_private_playlist {
        if let Some(ref browser) = browser_cookie {
            let mut args = cmd_base_args.to_vec();
            args.push("--cookies-from-browser");
            args.push(browser);
            args.push(&clean_url);
            output = Command::new(&ytdlp_path).args(args).output().ok();
        }
    }

    if let Some(out) = output {
        if out.status.success() {
            let json_str = String::from_utf8_lossy(&out.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let is_playlist = v.get("_type").and_then(|t| t.as_str()) == Some("playlist");
                let title = v["title"].as_str().unwrap_or("Sin título").to_string();
                let mut thumbnail = v["thumbnail"].as_str().map(|s| s.to_string());
                let duration = v["duration"].as_f64();
                let uploader = v["uploader"].as_str().map(|s| s.to_string());

                let mut items = Vec::new();
                let mut count = None;

                if is_playlist {
                    if let Some(entries) = v["entries"].as_array() {
                        count = Some(entries.len());
                        // Si la playlist no tiene thumbnail propio, usar el del primer tema
                        if thumbnail.is_none() {
                            if let Some(first) = entries.first() {
                                if let Some(thumbs) = first["thumbnails"].as_array() {
                                    if let Some(t) = thumbs.last().and_then(|th| th["url"].as_str()) {
                                        thumbnail = Some(t.to_string());
                                    }
                                }
                            }
                        }

                        for e in entries.iter().take(2000) {
                            let entry_id = e["id"].as_str().unwrap_or("");
                            let entry_url = if !entry_id.is_empty() {
                                format!("https://www.youtube.com/watch?v={}", entry_id)
                            } else {
                                e["url"]
                                    .as_str()
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| clean_url.clone())
                            };
                            let entry_title = e["title"].as_str().unwrap_or("Pista").to_string();
                            items.push(PlaylistItem {
                                url: entry_url,
                                title: entry_title,
                            });
                        }
                    }
                }

                return Ok(MediaMetadata {
                    title,
                    thumbnail,
                    duration,
                    uploader,
                    is_direct_image: false,
                    is_playlist,
                    playlist_count: count,
                    playlist_items: if is_playlist { Some(items) } else { None },
                    formats_summary: vec![
                        "Mejor calidad".to_string(),
                        "1080p".to_string(),
                        "720p".to_string(),
                        "Solo Audio (MP3)".to_string(),
                    ],
                });
            }
        }
    }

    if let Some(og) = extract_og_metadata(&clean_url).await {
        return Ok(og);
    }

    Err("No se pudo obtener información del enlace. Verifica si es público.".to_string())
}

#[tauri::command]
async fn search_music(query: String) -> Result<Vec<SearchResult>, String> {
    let clean_query = query.trim();
    if clean_query.is_empty() {
        return Ok(Vec::new());
    }

    let ytdlp_path = get_ytdlp_cmd();
    let mut results = Vec::new();

    // 1. Intentar primero búsqueda en YouTube Music (prioriza versiones oficiales de audio/estudio sin intros de video)
    let yt_music_query = clean_query.replace(' ', "+");
    let yt_music_url = format!("https://music.youtube.com/search?q={}", yt_music_query);

    let output_ytm = Command::new(&ytdlp_path)
        .args([
            &yt_music_url,
            "--dump-json",
            "--flat-playlist",
            "--no-warnings",
        ])
        .output();

    if let Ok(output) = output_ytm {
        if output.status.success() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            for line in stdout_str.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    if v["ie_key"].as_str() != Some("Youtube") {
                        continue;
                    }
                    let id = v["id"].as_str().unwrap_or("").to_string();
                    if id.is_empty() {
                        continue;
                    }
                    let title = v["title"].as_str().unwrap_or("Sin título").to_string();
                    let uploader = v["uploader"]
                        .as_str()
                        .or_else(|| v["channel"].as_str())
                        .map(|s| s.to_string());
                    let duration = v["duration"].as_f64();
                    let thumbnail = v["thumbnail"]
                        .as_str()
                        .map(|s| s.to_string())
                        .or_else(|| {
                            v["thumbnails"].as_array().and_then(|arr| {
                                arr.last().and_then(|t| t["url"].as_str().map(|s| s.to_string()))
                            })
                        })
                        .or_else(|| Some(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", &id)));
                    let url = format!("https://www.youtube.com/watch?v={}", &id);

                    results.push(SearchResult {
                        id,
                        title,
                        uploader,
                        duration,
                        thumbnail,
                        url,
                    });

                    if results.len() >= 8 {
                        break;
                    }
                }
            }
        }
    }

    // 2. Si YouTube Music no devolvió resultados, fallback a búsqueda general ytsearch
    if results.is_empty() {
        let search_arg = format!("ytsearch8:{}", clean_query);
        let output = Command::new(&ytdlp_path)
            .args([
                &search_arg,
                "--dump-json",
                "--flat-playlist",
                "--no-warnings",
            ])
            .output()
            .map_err(|e| format!("Error al ejecutar búsqueda: {}", e))?;

        if !output.status.success() {
            let err_str = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Fallo en la búsqueda: {}", err_str));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        for line in stdout_str.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let id = v["id"].as_str().unwrap_or("").to_string();
                if id.is_empty() {
                    continue;
                }
                let title = v["title"].as_str().unwrap_or("Sin título").to_string();
                let uploader = v["uploader"]
                    .as_str()
                    .or_else(|| v["channel"].as_str())
                    .map(|s| s.to_string());
                let duration = v["duration"].as_f64();
                let thumbnail = v["thumbnail"]
                    .as_str()
                    .map(|s| s.to_string())
                    .or_else(|| {
                        v["thumbnails"].as_array().and_then(|arr| {
                            arr.last().and_then(|t| t["url"].as_str().map(|s| s.to_string()))
                        })
                    })
                    .or_else(|| Some(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", &id)));
                let url = format!("https://www.youtube.com/watch?v={}", &id);

                results.push(SearchResult {
                    id,
                    title,
                    uploader,
                    duration,
                    thumbnail,
                    url,
                });
            }
        }
    }

    Ok(results)
}

#[tauri::command]
async fn open_path_in_file_manager(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut p = std::path::PathBuf::from(&path);
        if p.is_file() {
            Command::new("explorer")
                .args(["/select,", &path])
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            Command::new("explorer")
                .arg(&path)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let p = std::path::PathBuf::from(&path);
        let target_dir = if p.is_file() {
            p.parent().unwrap_or(&p).to_string_lossy().to_string()
        } else {
            path
        };
        Command::new("xdg-open")
            .arg(&target_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let _ = path;
        Ok(())
    }
}

#[tauri::command]
async fn start_download(
    app: AppHandle,
    id: String,
    url: String,
    output_dir: String,
    mode: String,
    quality: String,
    thumbnail_url: Option<String>,
) -> Result<String, String> {
    let download_id = id.clone();
    let clean_url = normalize_social_url(&url);
    let resolved_output_dir = output_dir.clone();

    // Caso: Descarga de Imagen
    if mode == "image" {
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let target_img_url = thumbnail_url.unwrap_or_else(|| clean_url.clone());

            let _ = app.emit(
                "download-progress",
                DownloadProgress {
                    id: download_id.clone(),
                    percent: 25.0,
                    speed: "Descargando...".into(),
                    eta: "".into(),
                    status: "downloading".into(),
                    filename: None,
                    output_path: None,
                },
            );

            match client.get(&target_img_url).send().await {
                Ok(resp) => {
                    if let Ok(bytes) = resp.bytes().await {
                        let filename = format!(
                            "{}/snapstream_img_{}.jpg",
                            resolved_output_dir.trim_end_matches('/'),
                            &download_id
                        );
                        if let Ok(mut file) = File::create(&filename) {
                            let _ = file.write_all(&bytes);
                            let _ = app.emit(
                                "download-progress",
                                DownloadProgress {
                                    id: download_id,
                                    percent: 100.0,
                                    speed: "".into(),
                                    eta: "".into(),
                                    status: "finished".into(),
                                    filename: Some(filename.clone()),
                                    output_path: Some(filename),
                                },
                            );
                            return;
                        }
                    }
                }
                Err(e) => {
                    let _ = app.emit(
                        "download-progress",
                        DownloadProgress {
                            id: download_id,
                            percent: 0.0,
                            speed: "".into(),
                            eta: "".into(),
                            status: "error".into(),
                            filename: Some(e.to_string()),
                            output_path: None,
                        },
                    );
                    return;
                }
            }
        });
        return Ok(id);
    }

    // Caso: Video o Audio
    let ytdlp_path = get_ytdlp_cmd();
    std::thread::spawn(move || {
        let mut cmd = Command::new(&ytdlp_path);
        let output_template = if mode == "audio" {
            format!("{}/%(artist,uploader)s - %(title)s.%(ext)s", resolved_output_dir.trim_end_matches('/'))
        } else {
            format!("{}/%(title)s.%(ext)s", resolved_output_dir.trim_end_matches('/'))
        };

        cmd.args(["--newline", "-o", &output_template]);

        let is_private_playlist = clean_url.contains("list=LM") || clean_url.contains("list=LL") || clean_url.contains("list=WL");
        if is_private_playlist {
            let browser_cookie = get_browser_cookie_arg();
            if let Some(ref browser) = browser_cookie {
                cmd.args(["--cookies-from-browser", browser]);
            }
        }

        if clean_url.contains("/playlist?list=") || clean_url.contains("list=LM") || clean_url.contains("list=LL") {
            cmd.arg("--yes-playlist");
        } else {
            cmd.arg("--no-playlist");
        }

        if mode == "audio" {
            cmd.args([
                "-x",
                "--audio-format",
                if quality.is_empty() { "mp3" } else { &quality },
                "--embed-metadata",
                "--embed-thumbnail",
                "--sponsorblock-remove",
                "music_offtopic",
                "--parse-metadata",
                "%(uploader,channel)s:%(artist)s",
                "--parse-metadata",
                "%(title)s:%(artist)s - %(title)s",
                "--replace-in-metadata",
                "title",
                r"(?i)\s*[\(\[](official\s*(video|audio|music\s*video|lyric\s*video)|video\s*oficial|audio\s*oficial|lyrics?|remastered|video|audio|4k|hd)[\)\]]",
                "",
                "--parse-metadata",
                "%(album,title)s:%(album)s",
            ]);
        } else {
            match quality.as_str() {
                "1080p" => {
                    cmd.args(["-f", "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best"]);
                }
                "720p" => {
                    cmd.args(["-f", "bestvideo[height<=720]+bestaudio/best[height<=720]/best"]);
                }
                _ => {
                    cmd.args(["-f", "bestvideo+bestaudio/best"]);
                }
            }
            cmd.args(["--merge-output-format", "mp4"]);
        }

        cmd.arg(&clean_url);
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
                        output_path: None,
                    },
                );
                return;
            }
        };

        let mut final_file = None;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if line.contains("[download] Destination:") {
                    let parts: Vec<&str> = line.split("[download] Destination:").collect();
                    if parts.len() > 1 {
                        final_file = Some(parts[1].trim().to_string());
                    }
                } else if line.contains("[Merger] Merging formats into") {
                    let parts: Vec<&str> = line.split("[Merger] Merging formats into").collect();
                    if parts.len() > 1 {
                        let f = parts[1].trim().trim_matches('"').to_string();
                        final_file = Some(f);
                    }
                } else if line.starts_with("[download]") {
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
                            output_path: None,
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
                            output_path: None,
                        },
                    );
                }
            }
        }

        let status = child.wait();
        match status {
            Ok(s) if s.success() => {
                let out_target = final_file.unwrap_or(resolved_output_dir);
                let _ = app.emit(
                    "download-progress",
                    DownloadProgress {
                        id: download_id,
                        percent: 100.0,
                        speed: "".into(),
                        eta: "".into(),
                        status: "finished".into(),
                        filename: None,
                        output_path: Some(out_target),
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
                        filename: Some("Fallo en la descarga".into()),
                        output_path: None,
                    },
                );
            }
        }
    });

    Ok(id)
}

fn get_local_ip() -> String {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

#[tauri::command]
fn get_server_info() -> ServerInfo {
    let ip = get_local_ip();
    let port = 48792;
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let is_running = true;
    #[cfg(any(target_os = "android", target_os = "ios"))]
    let is_running = false;

    ServerInfo {
        url: format!("http://{}:{}", ip, port),
        ip,
        port,
        is_running,
    }
}

fn resolve_target_url(raw: &str, endpoint: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    let endpoint_clean = endpoint.trim_start_matches('/');

    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        let (proto, rest) = if let Some(stripped) = trimmed.strip_prefix("https://") {
            ("https://", stripped)
        } else {
            ("http://", trimmed.strip_prefix("http://").unwrap())
        };

        if rest.contains(':') || rest.contains('/') || proto == "https://" {
            format!("{}{}/{}", proto, rest, endpoint_clean)
        } else {
            format!("{}{}:48792/{}", proto, rest, endpoint_clean)
        }
    } else {
        if trimmed.contains(':') || trimmed.contains('/') {
            format!("http://{}/{}", trimmed, endpoint_clean)
        } else if trimmed.contains("trycloudflare.com")
            || trimmed.ends_with(".net")
            || trimmed.ends_with(".com")
            || trimmed.ends_with(".org")
            || trimmed.ends_with(".app")
        {
            format!("https://{}/{}", trimmed, endpoint_clean)
        } else {
            format!("http://{}:48792/{}", trimmed, endpoint_clean)
        }
    }
}

#[tauri::command]
async fn ping_remote_server(pc_ip: String) -> Result<serde_json::Value, String> {
    let target = resolve_target_url(&pc_ip, "api/ping");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&target)
        .send()
        .await
        .map_err(|e| format!("No se pudo conectar a la PC ({}): {}", target, e))?;
    let data = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Respuesta no válida: {}", e))?;
    Ok(data)
}

#[tauri::command]
async fn send_remote_download(
    pc_ip: String,
    item: RemoteDownloadPayload,
) -> Result<String, String> {
    let target = resolve_target_url(&pc_ip, "api/download");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&target)
        .json(&item)
        .send()
        .await
        .map_err(|e| format!("Error enviando orden a la PC: {}", e))?;

    if resp.status().is_success() {
        Ok(format!("Descarga enviada a {}", target))
    } else {
        let err_text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Error desconocido".into());
        Err(format!("PC respondió con error: {}", err_text))
    }
}

#[tauri::command]
async fn get_remote_media_info(pc_ip: String, url: String) -> Result<MediaMetadata, String> {
    let target = resolve_target_url(&pc_ip, "api/info");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({ "url": url });

    let resp = client
        .post(&target)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Error conectando a la PC: {}", e))?;

    if resp.status().is_success() {
        let meta = resp
            .json::<MediaMetadata>()
            .await
            .map_err(|e| format!("Error procesando info: {}", e))?;
        Ok(meta)
    } else {
        let err_text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Error desconocido".into());
        Err(format!("PC respondió con error: {}", err_text))
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn start_embedded_server(app: AppHandle, port: u16) {
    std::thread::spawn(move || {
        let addr = format!("0.0.0.0:{}", port);
        let server = match tiny_http::Server::http(&addr) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[SnapStream Remote] No se pudo iniciar servidor en {}: {}", addr, e);
                return;
            }
        };

        for mut request in server.incoming_requests() {
            let method = request.method().clone();
            let url = request.url().to_string();

            // Preflight CORS
            if method == tiny_http::Method::Options {
                let response = tiny_http::Response::empty(200)
                    .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                    .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap())
                    .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type, Authorization"[..]).unwrap());
                let _ = request.respond(response);
                continue;
            }

            if url == "/api/ping" || url == "/api/status" {
                let local_ip = get_local_ip();
                let body = serde_json::json!({
                    "status": "ok",
                    "app": "SnapStream",
                    "version": "0.1.0",
                    "ip": local_ip,
                    "port": port
                }).to_string();

                let response = tiny_http::Response::from_string(body)
                    .with_status_code(200)
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                    .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                let _ = request.respond(response);
                continue;
            }

            if url == "/api/download" && method == tiny_http::Method::Post {
                let mut content = String::new();
                let _ = request.as_reader().read_to_string(&mut content);

                if let Ok(payload) = serde_json::from_str::<RemoteDownloadPayload>(&content) {
                    let download_id = format!("remote_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                    let app_clone = app.clone();

                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                    let navidrome_dir = format!("{}/navidrome/music", home);
                    let target_dir = if std::path::Path::new(&navidrome_dir).exists() {
                        navidrome_dir
                    } else {
                        format!("{}/Downloads", home)
                    };

                    let dl_url = payload.url.clone();
                    let dl_mode = payload.mode.clone();
                    let dl_quality = payload.quality.clone();
                    let dl_thumb = payload.thumbnail.clone();
                    let dl_id = download_id.clone();

                    tauri::async_runtime::spawn(async move {
                        let _ = start_download(
                            app_clone,
                            dl_id,
                            dl_url,
                            target_dir,
                            dl_mode,
                            dl_quality,
                            dl_thumb,
                        ).await;
                    });

                    let body = serde_json::json!({
                        "status": "queued",
                        "id": download_id,
                        "title": payload.title
                    }).to_string();

                    let response = tiny_http::Response::from_string(body)
                        .with_status_code(200)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                    let _ = request.respond(response);
                    continue;
                } else {
                    let response = tiny_http::Response::from_string(r#"{"error": "JSON no válido"}"#)
                        .with_status_code(400)
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                        .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                    let _ = request.respond(response);
                    continue;
                }
            }

            if (url == "/api/info" || url.starts_with("/api/info?")) && (method == tiny_http::Method::Post || method == tiny_http::Method::Get) {
                let target_url = if method == tiny_http::Method::Post {
                    let mut content = String::new();
                    let _ = request.as_reader().read_to_string(&mut content);
                    serde_json::from_str::<serde_json::Value>(&content)
                        .ok()
                        .and_then(|v| v["url"].as_str().map(|s| s.to_string()))
                } else {
                    url.split("url=").nth(1).map(|u| u.to_string())
                };

                if let Some(u) = target_url {
                    let res = tauri::async_runtime::block_on(get_media_info(u));
                    match res {
                        Ok(meta) => {
                            let body = serde_json::to_string(&meta).unwrap_or_default();
                            let response = tiny_http::Response::from_string(body)
                                .with_status_code(200)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                                .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                            let _ = request.respond(response);
                            continue;
                        }
                        Err(e) => {
                            let body = serde_json::json!({ "error": e }).to_string();
                            let response = tiny_http::Response::from_string(body)
                                .with_status_code(500)
                                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
                                .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                            let _ = request.respond(response);
                            continue;
                        }
                    }
                }
            }

            let response = tiny_http::Response::from_string(r#"{"error": "Ruta no encontrada"}"#)
                .with_status_code(404)
                .with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
            let _ = request.respond(response);
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init());

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .setup(|app| {
                let handle = app.handle().clone();
                start_embedded_server(handle, 48792);
                Ok(())
            });
    }

    builder
        .invoke_handler(tauri::generate_handler![
            get_media_info,
            start_download,
            open_path_in_file_manager,
            search_music,
            get_server_info,
            ping_remote_server,
            send_remote_download,
            get_remote_media_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
