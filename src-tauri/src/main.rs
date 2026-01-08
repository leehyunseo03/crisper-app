// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod database;
mod utils;
mod llm;
mod commands;

use tauri::{Manager, RunEvent, AppHandle, Emitter};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent}; // CommandEvent 추가
use std::env;
use std::sync::{Arc, Mutex};
use std::time::Duration; // 딜레이용
use tokio::time::sleep;  // 비동기 딜레이
use rig::providers::openai::Client as OpenAiClient;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

// AppState 구조체
struct AppState {
    db: Surreal<Db>,
    embed_client: OpenAiClient, // Port 8080
    gen_client: OpenAiClient,   // Port 8081
    server_handles: Arc<Mutex<Vec<CommandChild>>>,
}

// ♻️ 서버 실행/재시작을 담당하는 핵심 함수
async fn start_servers(app: &AppHandle, use_gpu: bool) {
    let state = app.state::<AppState>();
    
    // 1. 기존 프로세스 죽이기 (Clean up)
    {
        let mut handles = state.server_handles.lock().unwrap();
        if !handles.is_empty() {
            println!("🛑 기존 서버 종료 중...");
            for child in handles.drain(..) {
                let _ = child.kill();
            }
        }
    }
    // 포트 반환 대기 (안전장치)
    sleep(Duration::from_secs(2)).await;

    // 2. 경로 및 환경변수 설정
    let resource_path = app.path().resource_dir().unwrap().join("binaries");
    let path_env = env::var_os("PATH").unwrap_or_default();
    let mut paths = env::split_paths(&path_env).collect::<Vec<_>>();
    paths.push(resource_path.clone());
    let new_path_env = env::join_paths(paths).unwrap();

    // 🚨 모델 경로 (본인 경로로 확인 필수!)
    let embed_model_path = "C:/eoraha/crisper_app/crisper-app/src-tauri/models/ggml-model-Q4_K_M.gguf";
    let chat_model_path  = "C:/eoraha/crisper_app/crisper-app/src-tauri/models/qwen2.5-7b-instruct-q2_k.gguf";

    // 3. GPU 옵션 결정
    // GPU 모드면 99레이어(전부), CPU 모드면 0레이어
    let embed_gpu = if use_gpu { "99" } else { "0" };
    let chat_gpu  = if use_gpu { "10" } else { "0" }; // 채팅은 VRAM 부족 방지로 10만

    println!("🚀 서버 시작 (GPU 모드: {})", use_gpu);

    // 4. 임베딩 서버 (8080) 실행
    let (mut rx1, child1) = app.shell().sidecar("llama-server").unwrap()
        .current_dir(&resource_path)
        .env("PATH", &new_path_env)
        .args([
            "--model", embed_model_path,
            "--port", "8080", "--host", "127.0.0.1",
            "--embedding", "--pooling", "mean",
            "--ctx-size", "2048", "--batch-size", "2048", "--ubatch-size", "2048",
            "--parallel", "1",
            "--n-gpu-layers", embed_gpu // 👈 동적 할당
        ])
        .spawn().expect("8080 서버 실패");

    state.server_handles.lock().unwrap().push(child1);

    // 5. 채팅 서버 (8081) 실행
    let (mut rx2, child2) = app.shell().sidecar("llama-server").unwrap()
        .current_dir(&resource_path)
        .env("PATH", &new_path_env)
        .args([
            "--model", chat_model_path,
            "--alias", "gpt-3.5-turbo",
            "--port", "8081", 
            "--host", "127.0.0.1",
            //"--api", "openai",
            "--ctx-size", "4096", "--batch-size", "2048", "--ubatch-size", "2048",
            "--parallel", "2",
            "--n-gpu-layers", chat_gpu // 👈 동적 할당
        ])
        .spawn().expect("8081 서버 실패");

    state.server_handles.lock().unwrap().push(child2);

    // (선택) 로그 모니터링은 여기서 간단히 처리하거나 생략 가능
    // ...
    println!("🚀 적용 완료! (GPU 모드: {})", use_gpu);
}

// 🎛️ 프론트엔드에서 호출할 토글 커맨드
#[tauri::command]
async fn toggle_gpu(app: AppHandle, enable: bool) -> Result<String, String> {
    println!("🎛️ GPU 토글 요청: {}", enable);
    start_servers(&app, enable).await;
    Ok(if enable { "GPU Mode ON" } else { "CPU Mode ON" }.to_string())
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "reqwest=trace"); // reqwest의 디버그 로그만 봅니다
    }
    env_logger::init();

    let db = database::init_db().await.expect("DB Init Failed");
    let embed_client = OpenAiClient::builder().base_url("http://127.0.0.1:8080/v1").api_key("sk-no-key").build().unwrap();
    let gen_client = OpenAiClient::builder().base_url("http://127.0.0.1:8081/v1").api_key("sk-no-key").build().unwrap();
    
    // 핸들 저장소 생성
    let server_handles = Arc::new(Mutex::new(Vec::new()));

    let app_state = AppState { 
        db, embed_client, gen_client, 
        server_handles: server_handles.clone() 
    };

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            crate::commands::ingest::process_pdfs,
            crate::commands::query::fetch_graph_data,
            toggle_gpu, // 👈 커맨드 등록!
        ])
        .setup(move |app| {
            // 앱 켜질 때는 기본적으로 CPU 모드(false)로 시작 (혹은 true로 설정 가능)
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                start_servers(&handle, false).await; 
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Error building app");

    app.run(move |_app_handle, event| {
        if let RunEvent::Exit = event {
            // 종료 시 정리
            let mut guards = server_handles.lock().unwrap();
            for child in guards.drain(..) { let _ = child.kill(); }
        }
    });
}
