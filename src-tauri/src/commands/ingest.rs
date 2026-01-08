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

use crate::models::{EventNode, DocumentNode, ChunkNode, EntityNode, LlmExtractionResult};
use crate::utils::sanitize_id;
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
            if i < 20 { 
                println!("🤖 Extracting info from chunk {} of {}...", i, filename);
                
                match extract_knowledge(gen_url, txt).await {
                    Ok(result) => {
                        println!("\n========================================");
                        println!("📄 [Extraction Result] Chunk #{}", i);
                        println!("========================================");

                        // 1. Entities 출력
                        println!("🔹 Found {} Entities:", result.entities.len());
                        for (idx, entity) in result.entities.iter().enumerate() {
                            println!(
                                "   {}. [{}] {} - {}", 
                                idx + 1, 
                                entity.category, 
                                entity.name, 
                                entity.summary
                            );
                        }

                        println!("----------------------------------------");

                        // 2. Relations 출력
                        println!("🔸 Found {} Relations:", result.relations.len());
                        for (idx, rel) in result.relations.iter().enumerate() {
                            println!(
                                "   {}. {} --[{}]--> {} (Why: {})", 
                                idx + 1, 
                                rel.head, 
                                rel.relation, 
                                rel.tail, 
                                rel.reason
                            );
                        }
                        println!("========================================\n");
                        
                        // TODO: 여기서 DB 저장 로직 수행 (GraphRAG 구축)
                        println!("💾 Saving Graph Data to DB...");
                        if let Err(e) = save_graph_data(db, chunk_uuid.clone(), &result).await {
                             eprintln!("❌ Failed to save graph data: {}", e);
                        } else {
                             println!("✅ Graph Data Saved!");
                        }
                    },
                    Err(e) => {
                        println!("❌ Extraction failed: {}", e);
                    }
                }
            }
        }
    }

    Ok("Done".to_string())
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