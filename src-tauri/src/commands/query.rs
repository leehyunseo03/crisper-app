// src-tauri/src/commands/query.rs
use tauri::State;
use crate::AppState;
// models.rs에 정의된 UI용 구조체와 DB용 구조체를 가져옵니다.
use crate::models::{
    GraphData, GraphNode, GraphLink, 
    EventNode, DocumentNode, ChunkNode, EntityNode
};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use surrealdb::sql::Thing;
use serde::Deserialize;
use std::collections::HashSet;

// 엣지 조회용 임시 구조체 (DB에서 in/out만 쏙 빼올 때 사용)
#[derive(Debug, Deserialize)]
struct RawEdge {
    #[serde(rename = "in")]
    in_: surrealdb::sql::Thing,
    out: surrealdb::sql::Thing,
    relation: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EntityRow {
    id: Thing,
    name: String,
    category: String,
}

/// 그래프 데이터를 조회하는 메인 함수
/// view_mode: 나중에 "지식만 보기", "파일만 보기" 등 필터링을 위해 남겨둔 파라미터 (현재는 "all"로 동작)
#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_graph_data(
    state: State<'_, AppState>,
    view_mode: Option<String>,
) -> Result<GraphData, String> {
    let db = &state.db;
    let mode = view_mode.unwrap_or("all".to_string()); // "all" 또는 "knowledge"
    
    println!("🚀 그래프 조회 Mode: {}", mode);

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut links: Vec<GraphLink> = Vec::new();
    let mut valid_ids: HashSet<String> = HashSet::new();

    // ==========================================
    // 1. 노드 조회 (모드에 따라 필터링)
    // ==========================================

    // [Entity]는 모든 모드에서 표시 (지식 그래프의 핵심)
    let entities: Vec<EntityRow> = match db
        .query("SELECT id, name, category FROM entity")
        .await
        .map_err(|e| e.to_string())
        .and_then(|mut r| r.take(0).map_err(|e| e.to_string()))
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("⚠️ Entity query failed: {}", e);
            Vec::new()
        }
    };


    println!("🔍 DB 조회 결과: Entity 개수 = {}, Mode = {:?}", entities.len(), mode);
    for e in entities {
        let id_str = e.id.to_string();
        valid_ids.insert(id_str.clone());

        nodes.push(GraphNode {
            id: id_str,
            group: "entity".into(),
            label: e.name,
            val: 6,
            info: Some(e.category),
        });
    }

    // [File & Chunk]는 "knowledge" 모드가 아닐 때만 표시
    if mode != "knowledge" {
        // Event
        let events: Vec<EventNode> = db.select("event").await.map_err(|e| e.to_string())?;
        for e in events {
            let id = e.id.unwrap().to_string();
            valid_ids.insert(id.clone());
            nodes.push(GraphNode {
                id, group: "event".to_string(), label: "Import".to_string(), val: 10, info: None 
            });
        }
        // Document
        let docs: Vec<DocumentNode> = db.select("document").await.map_err(|e| e.to_string())?;
        for d in docs {
            let id = d.id.unwrap().to_string();
            valid_ids.insert(id.clone());
            nodes.push(GraphNode {
                id, group: "document".to_string(), label: d.filename, val: 20, info: None
            });
        }
        // Chunk (너무 많으면 느려지니 제한)
        let mut response = db.query("SELECT * FROM chunk LIMIT 500").await.map_err(|e| e.to_string())?;
        let chunks: Vec<ChunkNode> = response.take(0).map_err(|e| e.to_string())?; // Query 결과의 첫번째 뭉치를 가져옴
        for c in chunks {
            let id = c.id.unwrap().to_string();
            valid_ids.insert(id.clone());
            nodes.push(GraphNode {
                id, group: "chunk".to_string(), label: format!("p.{}", c.page_index), val: 5, info: None
            });
        }
    }

    // ==========================================
    // 2. 엣지 조회 함수 (관계 이름 포함)
    // ==========================================
    async fn fetch_edges(
        db: &Surreal<Db>, 
        table: &str, 
        valid_ids: &HashSet<String>, 
        links: &mut Vec<GraphLink>
    ) -> Result<(), String> {
        // relation 필드도 같이 조회
        let sql = format!("SELECT in, out, relation FROM {}", table);
        let edges: Vec<RawEdge> = db.query(sql).await.map_err(|e| e.to_string())?.take(0).map_err(|e| e.to_string())?;

        for edge in edges {
            let s = edge.in_.to_string();
            let t = edge.out.to_string();
            if valid_ids.contains(&s) && valid_ids.contains(&t) {
                links.push(GraphLink { 
                    source: s, 
                    target: t,
                    label: edge.relation // 👈 DB에서 가져온 관계 이름 (예: "founded")
                });
            }
        }
        Ok(())
    }

    // ==========================================
    // 3. 엣지 추가
    // ==========================================
    
    // 지식 관계 (Entity -> Entity) : 핵심!
    fetch_edges(db, "related_to", &valid_ids, &mut links).await?;

    if mode != "knowledge" {
        // 파일 구조 관계
        fetch_edges(db, "imported", &valid_ids, &mut links).await?;
        fetch_edges(db, "contains", &valid_ids, &mut links).await?;
        fetch_edges(db, "mentions", &valid_ids, &mut links).await?;
    }

    Ok(GraphData { nodes, links })
}