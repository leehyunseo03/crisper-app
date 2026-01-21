// src-tauri/src/commands/ingest.rs
use tauri::State;
use std::path::Path;
use std::fs;
use uuid::Uuid;
use chrono::Utc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use surrealdb::sql::Thing;
use rig::embeddings::EmbeddingsBuilder;
use rig::client::EmbeddingsClient;
use std::collections::HashMap;
use serde_json::json;
use std::time::Instant;
use serde::{Serialize, Deserialize};

use crate::models::{EventNode, DocumentNode, ChunkNode, EntityNode, LlmExtractionResult};
use crate::utils::sanitize_id;
use crate::utils::{extract_pages_from_pdf, chunk_text, RigDoc};
use crate::llm::extractor::{extract_knowledge, summarize_document};
use crate::AppState;
use crate::utils::parse_kakao_talk_log;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentWithChunks {
    pub id: Thing,
    pub filename: String,
    pub created_at: chrono::DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
    // 🌟 여기가 핵심: SurrealDB가 연결된 청크들을 이 필드에 채워줍니다.
    #[serde(default)] 
    pub chunks: Vec<ChunkNode>, 
}

#[tauri::command]
pub async fn ingest_documents(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = &state.db;
    let gen_url = "http://127.0.0.1:8081/v1";

    println!("\n📂 [Step 1] Ingest Process Started (1 Page = 1 Chunk)");
    println!("   Target Directory: {}", path);

    // 1. 파일 목록 수집 (기존 동일)
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut pdf_files = Vec::new();
    for entry in entries { /* ... */ 
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            pdf_files.push(path);
        }
    }
    
    let total_files = pdf_files.len();
    if total_files == 0 { return Err("No PDF files found.".to_string()); }

    // 2. 세션 생성 (기존 동일)
    let session_id = Uuid::new_v4().to_string();
    let _: EventNode = db.create(("event", &session_id))
        .content(EventNode {
            id: None, summary: format!("PDF Ingest: {}", path), created_at: Utc::now(),
        }).await.map_err(|e| e.to_string())?.ok_or("Event create failed")?;

    let mut success_count = 0;

    // 3. 파일 처리 루프
    for (idx, file_path) in pdf_files.iter().enumerate() {
        let current_num = idx + 1;
        let original_filename = file_path.file_name().unwrap().to_string_lossy().to_string();
        
        println!("\n---------------------------------------------------");
        println!("▶️  [{}/{}] Processing: {}", current_num, total_files, original_filename);
        let file_start = Instant::now();

        // A. 🌟 [핵심 변경] 페이지별 텍스트 추출 (Vec<String>)
        print!("    📖 Extracting pages... ");
        let pages = match extract_pages_from_pdf(file_path) {
            Ok(p) => {
                println!("Done ({} pages)", p.len());
                p
            },
            Err(e) => {
                println!("❌ Failed: {}", e);
                continue;
            }
        };

        if pages.is_empty() { 
            println!("    ⚠️ Skipped (Empty PDF)");
            continue; 
        }

        // B. Document(부모) 요약 생성
        // 전체 텍스트가 없으므로, 앞쪽 1~2페이지를 합쳐서 부모 문서의 요약용으로 씁니다.
        let summary_context = pages.iter().take(2).cloned().collect::<Vec<String>>().join("\n");
        
        println!("    🤖 Summarizing Document (Parent)...");
        let parent_summary = summarize_document(gen_url, &summary_context).await.unwrap_or_else(|_| {
             crate::llm::extractor::DocSummaryResult {
                title: original_filename.clone(), summary: "Parent Summary Failed".to_string(), tags: vec![]
            }
        });

        // Document 저장
        let doc_id = Uuid::new_v4().to_string();
        let mut doc_meta = HashMap::new();
        doc_meta.insert("title".to_string(), json!(parent_summary.title));
        doc_meta.insert("summary".to_string(), json!(parent_summary.summary));
        
        let _doc: DocumentNode = db.create(("document", &doc_id))
            .content(DocumentNode { 
                id: None, filename: original_filename.clone(), created_at: Utc::now(), metadata: doc_meta 
            }).await.map_err(|e| e.to_string())?.expect("Failed to create doc");

        // Event 연결
        let _ = db.query("RELATE $e->imported->$d").bind(("e", session_id.clone())).bind(("d", format!("document:{}", doc_id))).await.ok();

        // C. 청킹 (이미 페이지별로 나눠져 있으므로 chunk_text 함수 호출 안 함!)
        // let chunks = chunk_text(...) -> 삭제!
        // pages 변수 자체가 청크 리스트입니다.
        let chunks = pages; 

        println!("    Process {} Pages as Chunks...", chunks.len());

        // D. 각 페이지별 LLM 요약 실행
        for (i, txt) in chunks.iter().enumerate() {
            let chunk_uuid = Uuid::new_v4().to_string();
            
            // 페이지가 너무 길 수 있으니 요약용으로는 앞부분만 자를 수도 있습니다.
            // 여기선 그대로 넣습니다.
            print!("       Running LLM on Page #{}... ", i + 1);
            
            // 페이지별 요약 (제목에 페이지 번호 자동 부여)
            let chunk_res = summarize_document(gen_url, txt).await.unwrap_or_else(|_| {
                 crate::llm::extractor::DocSummaryResult {
                    title: format!("Page {}", i+1), // LLM 실패시 "Page 1" 등으로 제목 설정
                    summary: "요약 실패".to_string(),
                    tags: vec![]
                }
            });
            println!("Done");

            let mut chunk_meta = HashMap::new();
            chunk_meta.insert("title".to_string(), json!(chunk_res.title)); // "서론", "결론" 등 페이지 내용을 반영한 제목
            chunk_meta.insert("summary".to_string(), json!(chunk_res.summary));
            chunk_meta.insert("tags".to_string(), json!(chunk_res.tags));
            chunk_meta.insert("page_number".to_string(), json!(i + 1)); // 🌟 몇 페이지인지 메타데이터에 추가

            // Chunk 저장
            let _chunk: ChunkNode = db.create(("chunk", &chunk_uuid))
                .content(ChunkNode {
                    id: None, 
                    content: txt.clone(), 
                    page_index: i, 
                    embedding: vec![],
                    metadata: chunk_meta 
                }).await.map_err(|e| e.to_string())?.expect("Chunk create failed");
            println!("       ----------------------------------------");
            println!("       📄 Title:   {}", chunk_res.title);
            println!("       📝 Summary: {}", chunk_res.summary);
            println!("       🏷️ Tags:    {:?}", chunk_res.tags);
            println!("       ----------------------------------------");
            // Document -> Chunk 연결
            let doc_thing = Thing::from(("document", doc_id.as_str()));
            let chunk_thing = Thing::from(("chunk", chunk_uuid.as_str()));

            db.query("RELATE $d->contains->$c")
                .bind(("d", doc_thing))
                .bind(("c", chunk_thing))
                .await
                .ok();
        }

        println!("    ✨ File completed in {:.2?}", file_start.elapsed());
        success_count += 1;
    }

    Ok(format!("✅ Processed {} files.", success_count))
}
// --- 2단계: Document(Chunk) -> Graph (오래 걸림) ---
#[tauri::command]
pub async fn construct_graph(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = &state.db;
    let gen_url = "http://127.0.0.1:8081/v1";

    println!("🕸️ [Step 2] Building Knowledge Graph...");

    // 1. 아직 처리되지 않은(mentions 관계가 없는) Chunk들을 조회
    //    주의: surrealql 문법에 따라 `count(->mentions) = 0` 사용
    //    성능을 위해 한 번에 10개씩만 처리하거나 루프를 돕니다. 여기선 예시로 20개 제한.
    let mut chunks_to_process: Vec<ChunkNode> = db.query("SELECT * FROM chunk WHERE count(->mentions) = 0 LIMIT 20")
        .await.map_err(|e| e.to_string())?
        .take(0).map_err(|e| e.to_string())?;

    if chunks_to_process.is_empty() {
        return Ok("✨ 처리할 새로운 Chunk가 없습니다.".to_string());
    }

    let total = chunks_to_process.len();
    println!("   🚀 Processing {} chunks...", total);

    for (idx, chunk) in chunks_to_process.iter().enumerate() {
        let chunk_id_raw = chunk.id.as_ref().unwrap().id.to_string(); // thing에서 id 부분만 추출 필요할 수 있음
        // SurrealDB Rust SDK의 Thing.id는 Id 타입이므로 to_string()하면 괄호 등이 포함될 수 있음.
        // 안전하게 Thing 자체를 사용하거나 String으로 변환. 여기선 String 변환 가정.
        
        // 2. LLM 추출
        match extract_knowledge(gen_url, &chunk.content).await {
            Ok(result) => {
                // 3. 그래프 데이터 저장 (재사용)
                // chunk_id_raw가 "chunk:uuid" 형태인지 "uuid" 형태인지 확인 필요.
                // save_graph_data는 "uuid" 문자열을 기대하도록 작성되었음.
                let simple_id = chunk_id_raw.replace("chunk:", "").replace("⟨", "").replace("⟩", "");
                
                if let Err(e) = save_graph_data(db, simple_id, &result).await {
                    eprintln!("❌ Save Error: {}", e);
                } else {
                    println!("   ✅ [{}/{}] Graph extracted for chunk", idx + 1, total);
                }
            },
            Err(e) => eprintln!("❌ Extraction Error: {}", e),
        }
    }

    Ok(format!("✅ {}개 Chunk에 대한 그래프 생성 완료", total))
}



#[tauri::command]
pub async fn get_documents(state: State<'_, AppState>) -> Result<Vec<DocumentWithChunks>, String> {
    let db = &state.db;
    
    // 🌟 [수정 핵심] 서브쿼리를 사용해 연결된 데이터를 중첩 구조로 가져옵니다.
    // 의미: "document를 가져오는데, 'chunks'라는 필드에는 
    //      나(document)와 'contains'로 연결된 'chunk'들을 페이지 순서대로 담아라"
    let sql = "
        SELECT 
            *, 
            (SELECT * FROM ->contains->chunk ORDER BY page_index ASC) AS chunks 
        FROM document 
        ORDER BY created_at DESC
    ";
    
    // 쿼리 실행
    let mut response = db.query(sql).await.map_err(|e| e.to_string())?;
    
    // 결과를 새로 만든 구조체(DocumentWithChunks) 리스트로 변환
    let documents: Vec<DocumentWithChunks> = response.take(0).map_err(|e| e.to_string())?;
    
    Ok(documents)
}

async fn save_graph_data(
    db: &Surreal<Db>,
    chunk_id: String,
    data: &LlmExtractionResult,
) -> Result<(), String> {
    
    // 1. Entity 저장 및 Chunk와 연결
    for entity in &data.entities {
        let safe_name = sanitize_id(&entity.name);
        
        // 🌟 [수정 핵심] Rust에서 직접 Thing(ID) 객체 생성
        let chunk_thing = Thing::from(("chunk", chunk_id.as_str()));
        let entity_thing = Thing::from(("entity", safe_name.as_str()));

        // 1-1. Entity 생성/업데이트
        let _: Option<EntityNode> = db
            .upsert(("entity", &safe_name))
            .content(EntityNode {
                id: None,
                name: entity.name.clone(),
                category: entity.category.clone(),
                description: entity.summary.clone(),
                embedding: vec![],
                created_at: Utc::now(),
            })
            .await
            .map_err(|e| format!("Entity Upsert Error: {}", e))?;

        // 1-2. Chunk -> Entity 연결 (SQL이 훨씬 깔끔해집니다)
        // 기존: RELATE type::thing(...) -> ...
        // 변경: RELATE $c -> mentions -> $e
        let sql = "RELATE $c -> mentions -> $e";
        
        let _ = db.query(sql)
            // 🌟 String 대신 Thing 객체를 바인딩합니다.
            // DB는 이걸 받아서 "아, 이건 문자열이 아니라 레코드 ID구나"라고 바로 인식합니다.
            .bind(("c", chunk_thing)) 
            .bind(("e", entity_thing))
            .await
            .map_err(|e| format!("Relate Chunk-Entity Error: {}", e))?;
    }

    // 2. Relation (Entity -> Entity) 저장
    for rel in &data.relations {
        let head_safe = sanitize_id(&rel.head);
        let tail_safe = sanitize_id(&rel.tail);

        // 🌟 여기도 Thing 객체 생성
        let head_thing = Thing::from(("entity", head_safe.as_str()));
        let tail_thing = Thing::from(("entity", tail_safe.as_str()));

        // Head/Tail 노드 이름 보장 (빈 껍데기 생성)
        let _ = db.query("UPDATE type::thing('entity', $id) SET name = $name RETURN NONE")
            .bind(("id", head_safe.clone()))
            .bind(("name", rel.head.clone()))
            .await;
            
        let _ = db.query("UPDATE type::thing('entity', $id) SET name = $name RETURN NONE")
            .bind(("id", tail_safe.clone()))
            .bind(("name", rel.tail.clone()))
            .await;

        // 2-1. 관계 생성
        let sql = "
            RELATE $h -> related_to -> $t
            CONTENT {
                relation: $rel,
                reason: $reason,
                created_at: time::now()
            }
        ";
        
        let _ = db.query(sql)
            .bind(("h", head_thing)) // Thing 객체 바인딩
            .bind(("t", tail_thing)) // Thing 객체 바인딩
            .bind(("rel", rel.relation.clone()))
            .bind(("reason", rel.reason.clone()))
            .await
            .map_err(|e| format!("Relate Entity-Entity Error: {}", e))?;
    }

    Ok(())
}