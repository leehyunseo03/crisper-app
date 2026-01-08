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
use serde::Deserialize;
use std::collections::HashSet;

// 엣지 조회용 임시 구조체 (DB에서 in/out만 쏙 빼올 때 사용)
#[derive(Debug, Deserialize)]
struct RawEdge {
    #[serde(rename = "in")]
    in_: surrealdb::sql::Thing,
    out: surrealdb::sql::Thing,
}

/// 그래프 데이터를 조회하는 메인 함수
/// view_mode: 나중에 "지식만 보기", "파일만 보기" 등 필터링을 위해 남겨둔 파라미터 (현재는 "all"로 동작)
#[tauri::command]
pub async fn fetch_graph_data(
    state: State<'_, AppState>,
    view_mode: Option<String>, 
) -> Result<GraphData, String> {
    let db = &state.db;
    println!("🚀 [Query] 그래프 데이터 조회 시작 (Mode: {:?})", view_mode);

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut links: Vec<GraphLink> = Vec::new();
    
    // 유효한 노드 ID를 추적하기 위한 집합 (없는 노드를 가리키는 엣지 방지)
    let mut valid_node_ids: HashSet<String> = HashSet::new();

    // ================================================================
    // 1. 노드(Vertex) 조회
    // ================================================================

    // (1) Events (import 세션)
    let events: Vec<EventNode> = db.select("event").await.map_err(|e| e.to_string())?;
    for e in events {
        let id = e.id.unwrap().to_string();
        valid_node_ids.insert(id.clone());
        nodes.push(GraphNode {
            id,
            group: "event".to_string(), // 색상 구분용
            label: "Import Session".to_string(),
            val: 20, // 노드 크기
        });
    }

    // (2) Documents (파일)
    let docs: Vec<DocumentNode> = db.select("document").await.map_err(|e| e.to_string())?;
    for d in docs {
        let id = d.id.unwrap().to_string();
        valid_node_ids.insert(id.clone());
        nodes.push(GraphNode {
            id,
            group: "document".to_string(),
            label: d.filename,
            val: 15,
        });
    }

    // (3) Entities (지식 - 사람, 주제 등)
    // 🌟 여기가 새로 추가된 부분입니다!
    let entities: Vec<EntityNode> = db.select("entity").await.map_err(|e| e.to_string())?;
    for ent in entities {
        let id = ent.id.unwrap().to_string();
        valid_node_ids.insert(id.clone());
        nodes.push(GraphNode {
            id,
            group: "entity".to_string(),
            label: ent.name, // "김철수", "인공지능" 등
            val: 12, // 문서보다는 작고 청크보다는 크게
        });
    }

    // (4) Chunks (텍스트 조각)
    // ※ 노드가 너무 많으면 브라우저가 느려질 수 있으므로, 나중에는 limit을 걸거나 숨겨야 합니다.
    let chunks: Vec<ChunkNode> = db.query("SELECT * FROM chunk LIMIT 500").await
        .map_err(|e| e.to_string())?
        .take(0).map_err(|e| e.to_string())?;
        
    for c in chunks {
        let id = c.id.unwrap().to_string();
        valid_node_ids.insert(id.clone());
        nodes.push(GraphNode {
            id,
            group: "chunk".to_string(),
            label: format!("p.{}", c.page_index),
            val: 5,
        });
    }

    // ================================================================
    // 2. 엣지(Edge) 조회
    // ================================================================

    // 헬퍼: 특정 테이블의 모든 엣지를 가져와서 links 벡터에 추가
    async fn fetch_edges(
        db: &Surreal<Db>, 
        table: &str, 
        valid_ids: &HashSet<String>, 
        links: &mut Vec<GraphLink>
    ) -> Result<(), String> {
        // SELECT in, out FROM table 구문
        let edges: Vec<RawEdge> = db.query(format!("SELECT in, out FROM {}", table))
            .await.map_err(|e| e.to_string())?
            .take(0).map_err(|e| e.to_string())?;

        for edge in edges {
            let source = edge.in_.to_string();
            let target = edge.out.to_string();

            // 양쪽 노드가 모두 존재할 때만 링크 추가 (데이터 무결성)
            if valid_ids.contains(&source) && valid_ids.contains(&target) {
                links.push(GraphLink { source, target });
            }
        }
        Ok(())
    }

    // (1) System Edges
    fetch_edges(db, "imported", &valid_node_ids, &mut links).await?; // Event -> Doc
    fetch_edges(db, "contains", &valid_node_ids, &mut links).await?; // Doc -> Chunk

    // (2) Knowledge Edges 🌟 (새로 추가됨)
    // Chunk -> Entity (언급 관계)
    fetch_edges(db, "mentions", &valid_node_ids, &mut links).await?; 
    // Entity -> Entity (지식 관계) - 아직 데이터 생성 로직은 없지만 조회는 준비해둠
    fetch_edges(db, "related_to", &valid_node_ids, &mut links).await?; 

    println!("🏁 [Query] 반환: 노드 {}개, 링크 {}개", nodes.len(), links.len());
    
    Ok(GraphData { nodes, links })
}