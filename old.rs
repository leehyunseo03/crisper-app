#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;
use std::path::Path;
use std::fs;
use std::env;

// --- [수정 1] 명시적인 임포트 (에러 해결의 핵심) ---
use rig::providers::openai::{self, Client};
use rig::vector_store::in_memory_store::InMemoryVectorStore;
use rig::embeddings::EmbeddingsBuilder;
use rig::completion::Prompt; // .prompt() 메서드 사용을 위해 필수
use rig::Embed; // #[derive(Embed)] 사용을 위해 필수
use rig::vector_store::VectorStoreIndex;

use serde::{Serialize, Deserialize};
use pdf_extract::extract_text;
use anyhow::Context;
use dotenvy::dotenv;

// ---------------------------------------------------------
// 1. 데이터 구조체 정의
// ---------------------------------------------------------
// [수정 2] Default 추가: InMemoryVectorStore::default() 사용을 위해 필요
#[derive(Embed, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Default)]
struct Document {
    id: String,
    name: String,
    #[embed]
    content: String,
}

// ---------------------------------------------------------
// 2. AppState 정의
// ---------------------------------------------------------
struct AppState {
    vector_store: Mutex<InMemoryVectorStore<Document>>,
    openai_client: Client,
}

// ---------------------------------------------------------
// 3. 헬퍼 함수
// ---------------------------------------------------------
fn load_pdf_content<P: AsRef<Path>>(file_path: P) -> anyhow::Result<String> {
    extract_text(file_path.as_ref())
        .with_context(|| format!("Failed to extract text from PDF: {:?}", file_path.as_ref()))
}

// [추가됨] 텍스트 청킹 함수 (Chunking)
// text: 전체 텍스트
// chunk_size: 자를 글자 수 (예: 2000)
// overlap: 겹칠 글자 수 (예: 200 - 문맥 끊김 방지)
fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        let end = std::cmp::min(start + chunk_size, chars.len());
        let chunk: String = chars[start..end].iter().collect();
        
        // 너무 짧은 청크(예: 공백만 남은 경우)는 무시
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }

        // 끝에 도달했으면 종료
        if end == chars.len() {
            break;
        }

        // 다음 시작점 계산 (overlap 만큼 뒤로 당겨서 시작) 
        start += chunk_size - overlap;
    }

    chunks
}
// ---------------------------------------------------------
// Command: PDF 처리 및 임베딩
// ---------------------------------------------------------
#[tauri::command]
async fn process_pdfs(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    println!("📂 PDF 처리 시작 경로: {}", path);

    let directory_path = Path::new(&path);
    let entries = fs::read_dir(directory_path).map_err(|e| e.to_string())?;

    let embedding_model = state.openai_client.embedding_model("text-embedding-3-small");

    let mut docs: Vec<Document> = Vec::new();

    let chunk_size = 2000;
    let overlap = 200;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();

        if file_path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

            if let Ok(content) = load_pdf_content(&file_path) {
                if !content.trim().is_empty() {
                    // [변경] 전체 내용을 한번에 넣는게 아니라, 청킹해서 여러 개로 넣습니다.
                    let chunks = chunk_text(&content, chunk_size, overlap);
                    
                    for (i, chunk) in chunks.into_iter().enumerate() {
                        docs.push(Document {
                            // ID를 유니크하게 만들기 위해 파일명 + 번호를 붙입니다.
                            id: format!("{}_part_{}", file_name, i), 
                            name: file_name.clone(),
                            content: chunk,
                        });
                    }
                    println!("✅ 로드 및 청킹 완료: {} ({}개의 조각)", file_name, docs.len());
                }
            }
        }
    }

    if docs.is_empty() {
        return Err("처리할 PDF가 없거나 내용을 읽을 수 없습니다.".into());
    }
    let total_chunks = docs.len();

    println!("🚀 {}개의 청크에 대해 임베딩 생성 시작...", total_chunks);

    // 임베딩 생성
    let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
        .documents(docs)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    // 벡터 스토어에 추가
    let mut store = state.vector_store.lock().await;
    
    // [수정 3] .await 제거
    // InMemoryVectorStore의 add_documents는 동기 함수이거나 즉시 완료되므로 await가 필요 없습니다.
    store.add_documents(embeddings); 

    Ok(format!("{}개의 청크가 성공적으로 학습되었습니다.", total_chunks))
}
// ---------------------------------------------------------
// Command : 문서 검색 (Context Retrieval)
// ---------------------------------------------------------
// 질문을 받아서 벡터 DB에서 유사한 텍스트 조각을 찾아 문자열로 반환합니다.
#[tauri::command]
async fn search_docs(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let store = state.vector_store.lock().await;
    let embedding_model = state.openai_client.embedding_model("text-embedding-3-small");
    
    // 1. 인덱스 생성 (store 복제)
    let index = store.clone().index(embedding_model);

    // 2. 상위 3개 유사 문서 검색
    let results = index.top_n::<Document>(&query, 3)
        .await
        .map_err(|e| e.to_string())?;

    // 3. 텍스트만 추출하여 하나의 문자열로 합침
    // 형식:
    // [참고문서: 파일명]
    // 내용...
    let mut context_string = String::new();
    for (score, _id, doc) in results {
        // 유사도가 너무 낮은건 제외할 수도 있음 (예: score < 0.7)
        println!("{}",&format!("\n[참고문서: {} (유사도: {:.2})]\n{}\n", doc.name, score, doc.content));
        if score > 0.0 { 
            context_string.push_str(&format!("\n[참고문서: {} (유사도: {:.2})]\n{}\n", doc.name, score, doc.content));
        }
    }

    if context_string.is_empty() {
        return Ok("관련된 문서를 찾지 못했습니다.".to_string());
    }

    Ok(context_string)
}

// ---------------------------------------------------------
// Command : 모델 다운로드
// ---------------------------------------------------------
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

// ---------------------------------------------------------
// Command: RAG 채팅
// ---------------------------------------------------------
#[tauri::command]
async fn chat_with_docs(
    question: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let store = state.vector_store.lock().await;
    
    let embedding_model = state.openai_client.embedding_model("text-embedding-3-small");

    let index = store.clone().index(embedding_model);

    let rag_agent = state.openai_client.agent("gpt-4o") 
        .preamble("You are a helpful assistant answering questions based on the provided PDF documents.")
        .dynamic_context(2, index)
        .build();

    // Prompt 트레이트가 임포트되어 있어야 이 메서드가 작동합니다.
    let response = rag_agent.prompt(&question).await.map_err(|e| e.to_string())?;

    Ok(response)
}

// ---------------------------------------------------------
// Main
// ---------------------------------------------------------
fn main() {
    dotenv().ok();
    let openai_client = Client::from_env();
    let vector_store = InMemoryVectorStore::<Document>::default();
    let app_state = AppState {
        vector_store: Mutex::new(vector_store),
        openai_client,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        // search_docs 핸들러 추가
        .invoke_handler(tauri::generate_handler![process_pdfs, search_docs]) 
        .setup(|app| {
            // --- 사용자 제공 사이드카 로직 ---
            let resource_path = app.path().resource_dir().unwrap().join("binaries");
            let mut path_env = env::var_os("PATH").unwrap_or_default();
            let mut paths = env::split_paths(&path_env).collect::<Vec<_>>();
            paths.push(resource_path.clone());
            let _ = env::join_paths(paths).unwrap(); // new_path_env (사용 안함 경고 방지 위해 _ 처리)

            // 모델 경로는 실제 배포시 resource_path 등을 활용하는게 좋습니다.
            // 현재는 하드코딩된 경로 유지
            let model_path = "C:/eoraha/crisper_app/crisper-app/src-tauri/models/ggml-model-Q4_K_M.gguf";

            let sidecar_command = app.shell().sidecar("llama-server").unwrap()
                .current_dir(resource_path)
                .args([
                    "--model", model_path,
                    "--port", "8080",
                    "--host", "127.0.0.1",
                    "--ctx-size", "4096", // RAG를 위해 컨텍스트 사이즈 넉넉하게
                    "--parallel", "1",
                    "--n-gpu-layers", "99" // GPU 사용 가능하다면 추가
                ]);

            let (mut rx, _) = sidecar_command.spawn().expect("사이드카 실행 실패");

            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    if let tauri_plugin_shell::process::CommandEvent::Stderr(line) = event {
                         if let Ok(text) = String::from_utf8(line) {
                             // 로그가 너무 많으면 주석 처리 하세요
                             println!("LLAMA: {}", text.trim());
                         }
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("앱 실행 오류");
}