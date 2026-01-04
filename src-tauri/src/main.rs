#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, RunEvent};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandChild;
use std::path::Path;
use std::fs;
use std::env;
use std::sync::{Arc, Mutex};
use std::collections::{HashSet, HashMap};

// --- Rig & OpenAI ---
use rig::providers::openai::Client;
use rig::embeddings::{EmbeddingsBuilder, Embed, TextEmbedder, EmbedError};
use rig::client::{ProviderClient, EmbeddingsClient};

// --- SurrealDB ---
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use surrealdb::sql::Thing;

// --- Utils ---
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use pdf_extract::extract_text;
use anyhow::Context;
use dotenvy::dotenv;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ---------------------------------------------------------
// 1. 데이터 구조체 정의
// ---------------------------------------------------------
#[derive(Debug, Deserialize)]
struct RawRecord {
    id: Thing,
    #[serde(flatten)]
    content: HashMap<String, JsonValue>,
}

// Layer 3: 사건 (Event) - 언제 데이터를 넣었는가?
#[derive(Debug, Serialize, Deserialize)]
struct EventNode {
    id: Option<Thing>,
    summary: String,
    created_at: DateTime<Utc>,
}

// Layer 1: 문서 (Document) - 파일 그 자체
#[derive(Debug, Serialize, Deserialize)]
struct DocumentNode {
    id: Option<Thing>,
    filename: String,
    created_at: DateTime<Utc>,
}

// Layer 1-2: 청크 (Chunk) - 실제 내용과 벡터
#[derive(Debug, Serialize, Deserialize)]
struct ChunkNode {
    id: Option<Thing>,
    content: String,
    embedding: Vec<f32>,
    page_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RigDoc {
    id: String,
    content: String,
}

// Rig의 Embed trait 구현 (단순 텍스트 반환)
impl Embed for RigDoc {
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        embedder.embed(self.content.clone());
        Ok(())
    }
}

// ---------------------------------------------------------
// Graph Visualize Structure
// ---------------------------------------------------------
#[derive(Serialize, Debug)]
struct GraphNode {
    id: String,
    group: String, // 색상 구분을 위해 (event, document, chunk)
    label: String, // 화면에 표시할 이름
    val: usize,    // 노드 크기
}

#[derive(Debug, Deserialize)]
struct EdgeRecord {
    id: Thing,
    #[serde(rename = "in")] // Rust 예약어 'in' 회피
    in_: Thing,
    out: Thing,
}

#[derive(Serialize, Debug)]
struct GraphLink {
    source: String,
    target: String,
}

#[derive(Serialize, Debug)]
struct GraphData {
    nodes: Vec<GraphNode>,
    links: Vec<GraphLink>,
}

fn json_val_to_id(val: Option<&JsonValue>) -> String {
    match val {
        Some(JsonValue::String(s)) => s.clone(),
        Some(JsonValue::Object(o)) => {
            // { tb: "table", id: "uuid" } 형태일 때
            let tb = o.get("tb").and_then(|v| v.as_str()).unwrap_or("");
            let id = o.get("id").map(|v| v.to_string().replace("\"", "")).unwrap_or_default();
            format!("{}:{}", tb, id)
        },
        _ => String::new(),
    }
}

#[tauri::command]
async fn fetch_graph_data(state: tauri::State<'_, AppState>) -> Result<GraphData, String> {
    println!("🚀 [Graph] 데이터 조회 시작 (Generic Mode)");
    let db = &state.db;
    
    let mut nodes = Vec::new();
    let mut raw_links = Vec::new();
    let mut valid_node_ids = HashSet::new();

    // 🛠️ 제네릭 쿼리 실행 함수 (T: 어떤 구조체로도 변환 가능)
    async fn exec_query<T: DeserializeOwned>(db: &Surreal<Db>, sql: &str, table_name: &str) -> Result<Vec<T>, String> {
        let mut response = db.query(sql).await.map_err(|e| {
            format!("❌ [{}] 쿼리 실패: {}", table_name, e)
        })?;

        let items: Vec<T> = response.take(0).map_err(|e| {
            format!("❌ [{}] 파싱 실패: {}", table_name, e)
        })?;

        println!("   ✅ [{}] 조회 성공: {} 건", table_name, items.len());
        Ok(items)
    }

    // -------------------------------------------------------------
    // 1. 노드 조회 (RawRecord 사용)
    // -------------------------------------------------------------
    
    // (1) Event
    let events: Vec<RawRecord> = exec_query(db, "SELECT * FROM event", "Event").await?;
    for r in events {
        let id_str = r.id.to_string();
        valid_node_ids.insert(id_str.clone());
        nodes.push(GraphNode {
            id: id_str,
            group: "event".to_string(),
            label: "Session".to_string(),
            val: 20,
        });
    }

    // (2) Document
    let docs: Vec<RawRecord> = exec_query(db, "SELECT * FROM document", "Document").await?;
    for r in docs {
        let id_str = r.id.to_string();
        let filename = r.content.get("filename").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
        
        valid_node_ids.insert(id_str.clone());
        nodes.push(GraphNode {
            id: id_str,
            group: "document".to_string(),
            label: filename,
            val: 10,
        });
    }

    // (3) Chunk
    let chunks: Vec<RawRecord> = exec_query(db, "SELECT id, page_index FROM chunk", "Chunk").await?;
    for r in chunks {
        let id_str = r.id.to_string();
        let page = r.content.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0);
        
        valid_node_ids.insert(id_str.clone());
        nodes.push(GraphNode {
            id: id_str,
            group: "chunk".to_string(),
            label: format!("p.{}", page),
            val: 5,
        });
    }

    // -------------------------------------------------------------
    // 2. 엣지 조회 (EdgeRecord 사용 !!!)
    // -------------------------------------------------------------
    // 여기서 제네릭 타입을 <EdgeRecord>로 지정합니다.
    
    // (4) Imported Edges
    let imported_edges: Vec<EdgeRecord> = exec_query(db, "SELECT * FROM imported", "Imported").await?;
    for edge in imported_edges {
        // Thing 타입을 바로 문자열로 변환 (.to_string())
        let source = edge.in_.to_string();
        let target = edge.out.to_string();
        
        raw_links.push(GraphLink { source, target });
    }

    // (5) Contains Edges
    let contains_edges: Vec<EdgeRecord> = exec_query(db, "SELECT * FROM contains", "Contains").await?;
    for edge in contains_edges {
        let source = edge.in_.to_string();
        let target = edge.out.to_string();
        
        raw_links.push(GraphLink { source, target });
    }

    // -------------------------------------------------------------
    // 3. 필터링 및 반환
    // -------------------------------------------------------------
    let links: Vec<GraphLink> = raw_links
        .into_iter()
        .filter(|link| valid_node_ids.contains(&link.source) && valid_node_ids.contains(&link.target))
        .collect();

    println!("🏁 [Graph] 최종 반환: Nodes {}, Links {}", nodes.len(), links.len());
    Ok(GraphData { nodes, links })
}

// ---------------------------------------------------------
// 2. AppState 정의
// ---------------------------------------------------------
struct AppState {
    db: Surreal<Db>,
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
        
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }
        if end == chars.len() { break; }
        start += chunk_size - overlap;
    }
    chunks
}
// ---------------------------------------------------------
// Command: PDF 처리 및 임베딩
// ---------------------------------------------------------
#[tauri::command]
async fn process_pdfs_graph(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    println!("📂 Graph Indexing 시작: {}", path);
    let db = &state.db;

    // 1. 임베딩 모델 준비
    let embedding_model = state.openai_client.embedding_model("text-embedding-3-small");

    // 2. 파일 목록 읽기
    let directory_path = Path::new(&path);
    let entries = fs::read_dir(directory_path).map_err(|e| e.to_string())?;
    
    // 3. [Graph Layer 3] Event 노드 생성 ("Study Session")
    let session_id = Uuid::new_v4().to_string();
    let event_record: Option<EventNode> = db
        .create(("event", &session_id))
        .content(EventNode {
            id: None,
            summary: format!("PDF Import Session from {}", path),
            created_at: Utc::now(),
        })
        .await
        .map_err(|e: surrealdb::Error| e.to_string())?;
    
    let event_id = event_record.unwrap().id.unwrap(); // 생성된 Event의 ID (event:uuid)

    let mut processed_count = 0;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();

        if file_path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
            println!("📄 처리 중: {}", filename);

            // 텍스트 추출
            let content = match extract_text(&file_path) {
                Ok(text) => {
                    println!("  ✅ 텍스트 추출 성공 (길이: {})", text.len());
                    text
                },
                Err(e) => {
                    // 에러 로그를 출력하도록 수정
                    println!("  ❌ 텍스트 추출 실패: {} (건너뜀)", e);
                    continue; 
                }
            };

            if content.trim().is_empty() { continue; }

            // 4. [Graph Layer 1] Document 노드 생성
            println!("  💾 DB에 Document 노드 생성 시도...");
            let doc_uuid = Uuid::new_v4().to_string();
            let doc_record: Option<DocumentNode> = db
                .create(("document", &doc_uuid))
                .content(DocumentNode {
                    id: None,
                    filename: filename.clone(),
                    created_at: Utc::now(),
                })
                .await
                .map_err(|e| format!("DB Document 생성 실패: {}", e))?; // 에러 메시지 구체화
            
            // unwrap 안전장치
            let doc_id = match doc_record {
                Some(rec) => rec.id.unwrap(),
                None => {
                    println!("  ❌ DB 레코드가 반환되지 않았습니다.");
                    return Err("DB Record is None".to_string());
                }
            };
            println!("  ✅ Document 노드 생성 완료: {}", doc_id);

            // 5. [Edge] Event -> Document 연결 (RELATE 구문 사용)
            println!("  🔗 관계 연결 시도: {} -> imported -> {}", event_id, doc_id);
            
            db.query("RELATE $event->imported->$doc SET time = time::now()")
                .bind(("event", event_id.clone()))
                .bind(("doc", doc_id.clone()))
                .await
                .map_err(|e| e.to_string())?
                .check() // 여기서 결과를 파싱하지 않고 에러 유무만 체크합니다.
                .map_err(|e| e.to_string())?;

            println!("  ✅ 관계 연결 성공: Event -> Document");

            // 청킹 및 임베딩
            let chunks = chunk_text(&content, 1000, 100); // 청크 사이즈 조절
            println!("  🧩 청킹 완료: {}개", chunks.len());

            // Rig를 사용하여 임베딩 생성 (일괄 처리)
            // Rig의 Document 타입을 맞춰줘야 함
            let rig_docs: Vec<RigDoc> = chunks.iter().map(|c| {
                RigDoc {
                    id: "temp".to_string(), // 임베딩만 뽑을거라 ID는 무관
                    content: c.clone(),
                }
            }).collect();

            if rig_docs.is_empty() { continue; }

            let embeddings_result = EmbeddingsBuilder::new(embedding_model.clone())
                .documents(rig_docs)
                .map_err(|e| format!("Rig 문서 빌드 실패: {}", e))?
                .build()
                .await;

            let embeddings = match embeddings_result {
                Ok(emb) => {
                    println!("  ✅ 임베딩 생성 완료");
                    emb
                },
                Err(e) => {
                    println!("  ❌ OpenAI 임베딩 호출 실패: {}", e);
                    return Err(e.to_string());
                }
            };
            
            println!("  💾 Chunk 저장 및 연결 완료");

            // 6. [Graph Layer 1-2] Chunk 노드 생성 및 연결
            for (i, (chunk_text, embedding_tuple)) in chunks.iter().zip(embeddings).enumerate() {
                let chunk_uuid = Uuid::new_v4().to_string();
                let vector: Vec<f32> = embedding_tuple.1.first().vec.iter().map(|&x| x as f32).collect();
                // Chunk 생성
                let chunk_record: Option<ChunkNode> = db
                    .create(("chunk", &chunk_uuid))
                    .content(ChunkNode {
                        id: None,
                        content: chunk_text.clone(),
                        embedding: vector, // rig의 Embedding 구조체에서 벡터 추출
                        page_index: i,
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                
                let chunk_id = chunk_record.unwrap().id.unwrap();

                // [Edge] Document -> contains -> Chunk
                db.query("RELATE $doc->contains->$chunk")
                    .bind(("doc", doc_id.clone()))
                    .bind(("chunk", chunk_id))
                    .await
                    .map_err(|e| e.to_string())?
                    .check() // 수정됨
                    .map_err(|e| e.to_string())?;
            }

            processed_count += 1;
        }
    }

    Ok(format!("{}개의 PDF 파일이 그래프 데이터베이스(SurrealDB)에 저장되었습니다.", processed_count))
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
// Main
// ---------------------------------------------------------
#[tokio::main]
async fn main() {
    dotenv().ok();

    // 1. SurrealDB 초기화 (로컬 파일 rocksdb 사용)
    // 앱 실행 경로의 'crisper.db' 폴더에 저장됨
    let db = Surreal::new::<RocksDb>("../data/crisper_db").await.expect("DB 생성 실패");
    
    // 네임스페이스와 DB 선택
    db.use_ns("crisper_ns").use_db("crisper_db").await.expect("DB 선택 실패");

    let openai_client = Client::from_env();

    let app_state = AppState {
        db,
        openai_client,
    };

    let llama_child = Arc::new(Mutex::new(None::<CommandChild>));
    let llama_child_clone = llama_child.clone();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            process_pdfs_graph, // 그래프 생성 함수
            fetch_graph_data,   // 그래프 시각화 함수
        ])
        .setup(move |app| {
            // --- 사이드카 실행 로직 ---
            let resource_path = app.path().resource_dir().unwrap().join("binaries");
            
            // 모델 경로 (실제 경로에 맞게 수정 필요)
            let model_path = "C:/eoraha/crisper_app/crisper-app/src-tauri/models/ggml-model-Q4_K_M.gguf";

            let sidecar_command = app.shell().sidecar("llama-server").unwrap()
                .current_dir(resource_path)
                .args([
                    "--model", model_path,
                    "--port", "8080",
                    "--host", "127.0.0.1",
                    "--ctx-size", "4096",
                    "--parallel", "1",
                    "--n-gpu-layers", "99"
                ]);

            // [변경] spawn 시 child 프로세스 핸들을 가져옵니다.
            let (mut rx, child) = sidecar_command.spawn().expect("사이드카 실행 실패");

            // [추가] 핸들을 공유 변수에 저장
            *llama_child_clone.lock().unwrap() = Some(child);

            // 로그 출력용 비동기 태스크
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    if let tauri_plugin_shell::process::CommandEvent::Stderr(line) = event {
                         if let Ok(text) = String::from_utf8(line) {
                             // 로그가 너무 많으면 주석 처리
                             // println!("LLAMA: {}", text.trim());
                         }
                    }
                }
            });
            
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("앱 빌드 오류");

    // 3. 앱 실행 및 종료 이벤트 핸들링 (Run Loop)
    app.run(move |_app_handle, event| {
        match event {
            // 앱이 완전히 종료될 때 (창을 닫거나 Quit 했을 때)
            RunEvent::Exit => {
                println!("🛑 앱 종료 감지. Llama Server를 정리합니다...");
                
                // 공유 변수에서 프로세스 핸들을 꺼내서 kill() 호출
                let mut child_guard = llama_child.lock().unwrap();
                if let Some(child) = child_guard.take() {
                    // kill()을 호출하여 프로세스 종료
                    if let Err(e) = child.kill() {
                        eprintln!("⚠️ Llama Server 종료 실패: {}", e);
                    } else {
                        println!("✅ Llama Server가 안전하게 종료되었습니다.");
                    }
                }
            }
            _ => {}
        }
    });
}