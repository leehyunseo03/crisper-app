use tauri::State;
use std::fs;
use uuid::Uuid;
use chrono::Utc;
use surrealdb::sql::Thing;
use std::collections::{HashMap, HashSet};
use serde_json::json;
use std::time::Instant;

use crate::models::{EventNode, DocumentNode, ChunkNode, EntityNode, DocumentWithChunks, CoreAnalysisResult};
use crate::utils::extract_pages_from_pdf;
use crate::llm::extractor::analyze_content;
use crate::AppState;

// --- 1단계: PDF 파일 Ingest 및 구조 분석 (LLM) ---
#[tauri::command]
pub async fn ingest_documents(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = &state.db;
    let gen_url = "[http://127.0.0.1:8081/v1](http://127.0.0.1:8081/v1)"; // 로컬 LLM 서버 주소

    println!("\n📂 [Step 1] Ingest Process Started (1 Page = 1 Chunk)");
    println!("    Target Directory: {}", path);

    // 1. 파일 목록 수집
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut pdf_files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            pdf_files.push(path);
        }
    }
    
    let total_files = pdf_files.len();
    if total_files == 0 { return Err("No PDF files found.".to_string()); }

    // 2. 세션 생성 (작업 기록용 Event)
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
        
        // A. 페이지별 텍스트 추출
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

        // B. Document(부모) 요약 생성 (앞 2페이지만 사용)
        let summary_context = pages.iter().take(2).cloned().collect::<Vec<String>>().join("\n");
        
        println!("    🤖 Summarizing Document (Parent)...");
        let parent_analysis = analyze_content(gen_url, &summary_context).await.unwrap_or_else(|_| {
             CoreAnalysisResult {
                topic: original_filename.clone(),
                summary: "분석 실패".to_string(),
                key_entities: vec![],
                detailed_data: json!({}),
            }
        });

        // Document 저장
        let doc_id = Uuid::new_v4().to_string();
        let mut doc_meta = HashMap::new();
        doc_meta.insert("analysis".to_string(), json!(parent_analysis));

        let _doc: DocumentNode = db.create(("document", &doc_id))
            .content(DocumentNode { 
                id: None, filename: original_filename.clone(), created_at: Utc::now(), metadata: doc_meta 
            }).await.map_err(|e| e.to_string())?.expect("Failed to create doc");

        // Event -> Document 연결
        let _ = db.query("RELATE $e->imported->$d").bind(("e", session_id.clone())).bind(("d", format!("document:{}", doc_id))).await.ok();

        // C. 청크 처리 (페이지 단위)
        let chunks = pages; 
        for (i, txt) in chunks.iter().enumerate() {
            let chunk_uuid = Uuid::new_v4().to_string();
            
            print!("      Running LLM Analysis on Page #{} (Len: {})... ", i + 1, txt.len());
            
            // 페이지별 분석 실행
            let chunk_res = match analyze_content(gen_url, txt).await {
                Ok(res) => {
                    println!("✅ Done");
                    res
                },
                Err(e) => {
                    println!("\n      ❌ ERROR: {:?}", e);
                    CoreAnalysisResult {
                        topic: format!("Page {}", i+1),
                        summary: "분석 실패".to_string(),
                        key_entities: vec![],
                        detailed_data: json!({ "error": format!("{:?}", e) }),
                    }
                }
            };

            // Chunk 메타데이터 구성
            let mut chunk_meta = HashMap::new();
            chunk_meta.insert("page_number".to_string(), json!(i + 1));
            // Step 2(Graph)를 위해 분석 데이터를 통째로 저장
            chunk_meta.insert("analysis".to_string(), json!(chunk_res)); 

            // Chunk 저장
            let _chunk: ChunkNode = db.create(("chunk", &chunk_uuid))
                .content(ChunkNode {
                    id: None, 
                    content: txt.clone(), 
                    page_index: i, 
                    embedding: vec![], // 임베딩은 필요 시 나중에 추가
                    metadata: chunk_meta 
                }).await.map_err(|e| e.to_string())?.expect("Chunk create failed");

            // Document -> Chunk 연결
            let _ = db.query("RELATE $d->contains->$c")
                .bind(("d", format!("document:{}", doc_id)))
                .bind(("c", format!("chunk:{}", chunk_uuid)))
                .await.ok();
        }
        success_count += 1;
    }
    
    Ok(format!("✅ Processed {} files with Structural Analysis.", success_count))
}

// --- 2단계: Chunk 메타데이터 -> 키워드 Graph 연결 ---
#[tauri::command]
pub async fn construct_graph(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = &state.db;

    println!("\n🕸️ [Step 2] Building Keyword Graph (No LLM)...");

    // 1. 아직 처리되지 않은 Chunk 조회
    let sql = "SELECT * FROM chunk WHERE metadata.step2_processed != true LIMIT 500";
    
    let mut chunks_to_process: Vec<ChunkNode> = db.query(sql)
        .await.map_err(|e| e.to_string())?
        .take(0).map_err(|e| e.to_string())?;

    if chunks_to_process.is_empty() {
        return Ok("✨ 처리할 새로운 Chunk가 없습니다.".to_string());
    }

    let total = chunks_to_process.len();
    println!(" 🚀 Linking {} chunks based on tags/keywords...", total);

    let mut success_count = 0;

    for chunk in chunks_to_process.iter() {
        let chunk_thing = match &chunk.id {
            Some(t) => t.clone(),
            None => continue,
        };

        // 2. 메타데이터에서 키워드 수집 (중복 제거)
        let mut topics: HashSet<String> = HashSet::new();

        // 참고: analyze_content의 결과가 metadata["analysis"]에 들어있다고 가정
        // 직접적인 "tags"나 "keywords" 필드가 없다면 아래 로직은 빈 동작을 할 수 있음.
        // 필요 시 chunk.metadata["analysis"]["key_entities"] 등을 파싱하도록 수정 가능.
        
        // (1) Tags 탐색
        if let Some(tags_val) = chunk.metadata.get("tags") {
            if let Some(arr) = tags_val.as_array() {
                for t in arr {
                    if let Some(s) = t.as_str() { topics.insert(s.trim().to_string()); }
                }
            }
        }

        // (2) Keywords 탐색
        if let Some(kws_val) = chunk.metadata.get("keywords") {
            if let Some(arr) = kws_val.as_array() {
                for k in arr {
                    if let Some(s) = k.as_str() { topics.insert(s.trim().to_string()); }
                }
            }
        }
        
        // (3) Analysis 결과 내 key_entities 탐색 (추가 보완)
        if let Some(analysis_val) = chunk.metadata.get("analysis") {
            if let Some(entities) = analysis_val.get("key_entities").and_then(|v| v.as_array()) {
                for e in entities {
                    if let Some(s) = e.as_str() { topics.insert(s.trim().to_string()); }
                }
            }
        }

        // 3. Entity 생성 및 연결
        for topic in topics {
            if topic.is_empty() { continue; }

            let safe_name = crate::utils::sanitize_id(&topic);
            let entity_id = Thing::from(("entity", safe_name.as_str()));

            // Entity Upsert
            let _: Option<EntityNode> = db
                .upsert(("entity", &safe_name))
                .content(EntityNode {
                    id: Some(entity_id.clone()),
                    name: topic.clone(),
                    category: "Keyword".to_string(),
                    description: format!("Extracted keyword: {}", topic),
                    embedding: vec![],
                    created_at: Utc::now(),
                })
                .await.ok().flatten();

            // Chunk -> mentions -> Entity 연결
            let sql = "RELATE $c -> mentions -> $e";
            let _ = db.query(sql)
                .bind(("c", chunk_thing.clone()))
                .bind(("e", entity_id))
                .await.ok();
        }

        // 4. 처리 완료 마킹
        let _: Option<ChunkNode> = db.update(("chunk", chunk_thing.id.to_string()))
            .merge(json!({
                "metadata": { "step2_processed": true }
            }))
            .await.ok().flatten();

        success_count += 1;
    }

    Ok(format!("✅ {}/{} 개의 청크 연결 완료 (고속 모드)", success_count, total))
}

// --- 문서 조회 (계층 구조 포함) ---
#[tauri::command]
pub async fn get_documents(state: State<'_, AppState>) -> Result<Vec<DocumentWithChunks>, String> {
    let db = &state.db;
    
    // 서브쿼리를 사용하여 Document와 연관된 Chunk들을 한 번에 조회
    let sql = "
        SELECT 
            *, 
            (SELECT * FROM ->contains->chunk ORDER BY page_index ASC) AS chunks 
        FROM document 
        ORDER BY created_at DESC
    ";
    
    let mut response = db.query(sql).await.map_err(|e| e.to_string())?;
    let documents: Vec<DocumentWithChunks> = response.take(0).map_err(|e| e.to_string())?;
    
    Ok(documents)
}