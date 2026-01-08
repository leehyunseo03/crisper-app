// src-tauri/src/commands/ingest.rs
use tauri::State;
use std::path::Path;
use std::fs;
use uuid::Uuid;
use chrono::Utc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use rig::embeddings::EmbeddingsBuilder;
use rig::client::EmbeddingsClient;

use crate::models::{EventNode, DocumentNode, ChunkNode};
use crate::utils::{extract_text_from_pdf, chunk_text, RigDoc};
use crate::llm::extractor::extract_knowledge;
use crate::AppState;

#[tauri::command]
pub async fn process_pdfs(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = &state.db;
    // [중요] 임베딩용 클라이언트 (8080)
    let embed_model = state.embed_client.embedding_model("ggml-model-Q4_K_M");
    
    // [중요] 추출용 클라이언트 (8081)
    let gen_client = &state.gen_client;

    println!("📂 Ingesting from: {}", path);

    // 1. 세션(Event) 생성
    let session_id = Uuid::new_v4().to_string();
    let event: EventNode = db.create(("event", &session_id))
        .content(EventNode {
            id: None,
            summary: format!("Import from {}", path),
            created_at: Utc::now(),
        })
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Failed to create event")?;

    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;

    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("pdf") { continue; }
        
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        
        // 2. 텍스트 추출 & 청킹
        let text = extract_text_from_pdf(&path).map_err(|e| e.to_string())?;
        if text.trim().is_empty() { continue; }
        let chunks = chunk_text(&text, 1000, 100);

        // 3. Document 생성
        let doc_id = Uuid::new_v4().to_string();
        let _doc: DocumentNode = db.create(("document", &doc_id))
            .content(DocumentNode { 
                id: None, filename: filename.clone(), 
                created_at: Utc::now(), metadata: Default::default() 
            })
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Failed to create event")?;

        // Event -> Document 연결
        let _ = db.query("RELATE $e->imported->$d")
            .bind(("e", session_id.clone())).bind(("d", format!("document:{}", doc_id)))
            .await.map_err(|e| e.to_string())?;

        // 4. 임베딩 생성 (Batch)
        /*
        let rig_docs: Vec<RigDoc> = chunks.iter().map(|c| RigDoc { id: "x".into(), content: c.clone() }).collect();
        let embeddings = EmbeddingsBuilder::new(embed_model.clone())
            .documents(rig_docs).map_err(|e| e.to_string())?
            .build().await.map_err(|e| e.to_string())?;
        */

        // 5. Chunk 저장 및 지식 추출 루프
        //for (i, (txt, emb_res)) in chunks.iter().zip(embeddings).enumerate() {
        for (i, txt) in chunks.iter().enumerate(){
            let chunk_uuid = Uuid::new_v4().to_string();
            //let vec: Vec<f32> = emb_res.1.first().vec.iter().map(|&x| x as f32).collect();
            let dummy_embedding: Vec<f32> = vec![];

            let _chunk: ChunkNode = db.create(("chunk", &chunk_uuid))
                .content(ChunkNode {
                    id: None, 
                    content: txt.clone(), 
                    page_index: i, 
                    embedding: dummy_embedding//vec.clone()
                })
                .await
                .map_err(|e| e.to_string())?
                .ok_or("Failed to create event")?;

            // Document -> Chunk 연결
            db.query("RELATE $d->contains->$c")
                .bind(("d", format!("document:{}", doc_id)))
                .bind(("c", format!("chunk:{}", chunk_uuid)))
                .await.map_err(|e| e.to_string())?;
            
            let gen_url = "http://127.0.0.1:8081/v1"; 

            // 🧠 지식 추출
            if i < 10 { // 테스트를 위해 10개 청크만
                println!("🤖 Extracting info from chunk {} of {}...", i, filename);
                
                // 직접 호출한 extractor 함수 사용
                match extract_knowledge(gen_url, txt).await {
                    Ok(result) => {
                        println!("  ✅ Found {} entities, {} relations", result.entities.len(), result.relations.len());
                        
                        // TODO: 추출된 entity와 relation을 DB에 저장하는 로직 추가
                        // 예: save_graph_data(&db, doc_id, result).await;
                    },
                    Err(e) => {
                        println!("  ❌ Extraction failed: {}", e);
                        // 에러가 나도 전체 프로세스는 죽지 않도록 로그만 남기고 계속 진행
                    }
                }
            }
        }
    }

    Ok("Done".to_string())
}