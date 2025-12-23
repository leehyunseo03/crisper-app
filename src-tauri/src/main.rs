// src-tauri/src/main.rs (수정 제안)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use std::env;

#[tauri::command]
async fn download_model(app_handle: tauri::AppHandle, url: String, filename: String) -> Result<String, String> {
    eprintln!("🚀 다운로드 요청 수신: {} -> {}", url, filename);
    
    // 모델이 저장될 폴더 경로 (src-tauri/models)
    let model_dir = app_handle.path().resource_dir().unwrap().join("models");
    
    // 폴더가 없으면 생성
    if !model_dir.exists() {
        std::fs::create_dir_all(&model_dir).map_err(|e| e.to_string())?;
    }

    // 여기에 실제 다운로드 로직이 들어갑니다. (현재는 성공 메시지만 반환)
    // 실제 구현은 reqwest 등의 라이브러리를 사용하게 됩니다.
    
    Ok(format!("{} 모델 다운로드 준비 완료 (경로: {:?})", filename, model_dir))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![download_model])
        .setup(|app| {
            let resource_path = app.path().resource_dir().unwrap().join("binaries");
            
            // PATH 설정 유지
            let mut path_env = env::var_os("PATH").unwrap_or_default();
            let mut paths = env::split_paths(&path_env).collect::<Vec<_>>();
            paths.push(resource_path.clone());
            let new_path_env = env::join_paths(paths).unwrap();

            let model_path = "C:/eoraha/crisper_app/crisper-app/src-tauri/models/ggml-model-Q4_K_M.gguf";

            let sidecar_command = app.shell().sidecar("llama-server").unwrap()
                .current_dir(resource_path)
                .args([
                    "--model", model_path,
                    "--port", "8080",
                    "--host", "127.0.0.1",
                    // 스트리밍 성능을 위해 아래 인자들을 추가하면 좋습니다 (선택사항)
                    "--ctx-size", "2048",
                    "--parallel", "1"
                ]);

            let (mut rx, _child) = sidecar_command.spawn().expect("사이드카 실행 실패");

            // 사이드카의 콘솔 로그만 터미널에 출력 (디버깅용)
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv() .await {
                    if let tauri_plugin_shell::process::CommandEvent::Stderr(line) = event {
                        if let Ok(text) = String::from_utf8(line) {
                            eprintln!("LLAMA LOG: {}", text);
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("앱 실행 오류");
}