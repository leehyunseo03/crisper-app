use tauri::State;
use crate::AppState;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue; // 🌟 표준 JSON Value 사용

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNodeRes>,
    pub links: Vec<GraphLinkRes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNodeRes {
    pub id: String,
    pub group: String,
    pub label: String,
    pub info: Option<String>, 
    pub val: f32,             
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphLinkRes {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

// 🛠️ 헬퍼: JSON Value에서 문자열 안전하게 추출
fn get_str(val: &JsonValue, key: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

#[tauri::command]
pub async fn fetch_graph_data(
    state: State<'_, AppState>,
    view_mode: String, 
) -> Result<GraphResponse, String> {
    let db = &state.db;
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    println!("\n🔍 [Debug] Graph Fetch Started (JSON Mode)...");

    // 🌟 핵심 전략: SQL에서 미리 ID와 Edge를 문자열(<string>)로 변환합니다.
    // 이렇게 하면 Rust는 복잡한 Enum 처리를 할 필요 없이 단순 JSON으로 받을 수 있습니다.
    
    // 1. Documents 조회 (ID 변환)
    let sql_doc = "SELECT *, type::string(id) as id FROM document";
    let docs_res: Vec<JsonValue> = db.query(sql_doc)
        .await.map_err(|e| e.to_string())?
        .take(0).map_err(|e| e.to_string())?;

    for d in docs_res {
        let id = get_str(&d, "id");
        let filename = get_str(&d, "filename");
        
        if !id.is_empty() {
            nodes.push(GraphNodeRes {
                id,
                group: "document".into(),
                label: if filename.is_empty() { "Untitled".into() } else { filename },
                info: Some("Original PDF Document".into()),
                val: 20.0,
            });
        }
    }

    // 2. Chunks 조회
    if view_mode != "semantic" {
        let sql_chunk = "SELECT *, type::string(id) as id FROM chunk";
        let chunks_res: Vec<JsonValue> = db.query(sql_chunk)
            .await.map_err(|e| e.to_string())?
            .take(0).map_err(|e| e.to_string())?;

        for c in chunks_res {
            let id = get_str(&c, "id");
            if id.is_empty() { continue; }

            // Metadata 처리
            let mut page_num = 0;
            let mut title = "Page".to_string();
            
            if let Some(meta) = c.get("metadata") {
                page_num = meta.get("page_number").and_then(|v| v.as_i64()).unwrap_or(0);
                title = meta.get("title").and_then(|v| v.as_str()).unwrap_or("Page").to_string();
            }

            let content = get_str(&c, "content");
            let preview: String = content.chars().take(50).collect();

            nodes.push(GraphNodeRes {
                id,
                group: "chunk".into(),
                label: format!("p.{}: {}", page_num, title),
                info: Some(preview + "..."),
                val: 5.0,
            });
        }
    }

    // 3. Entities 조회
    let sql_entity = "SELECT *, type::string(id) as id FROM entity";
    let entities_res: Vec<JsonValue> = db.query(sql_entity)
        .await.map_err(|e| e.to_string())?
        .take(0).map_err(|e| e.to_string())?;

    for e in entities_res {
        let id = get_str(&e, "id");
        if id.is_empty() { continue; }

        let name = get_str(&e, "name");
        let category = get_str(&e, "category");
        let desc = get_str(&e, "description");

        nodes.push(GraphNodeRes {
            id,
            group: "entity".into(),
            label: name,
            info: Some(format!("[{}] {}", category, desc)),
            val: 10.0,
        });
    }

    // 4. Links 조회 (Edge 테이블의 in, out도 문자열로 변환)
    if view_mode != "semantic" {
        // Contains
        let sql_contains = "SELECT type::string(in) as source, type::string(out) as target FROM contains";
        let contains_res: Vec<JsonValue> = db.query(sql_contains)
            .await.map_err(|e| e.to_string())?
            .take(0).map_err(|e| e.to_string())?;

        for rel in contains_res {
            let s = get_str(&rel, "source");
            let t = get_str(&rel, "target");
            if !s.is_empty() && !t.is_empty() {
                links.push(GraphLinkRes { source: s, target: t, label: None });
            }
        }

        // Mentions
        let sql_mentions = "SELECT type::string(in) as source, type::string(out) as target FROM mentions";
        let mentions_res: Vec<JsonValue> = db.query(sql_mentions)
            .await.map_err(|e| e.to_string())?
            .take(0).map_err(|e| e.to_string())?;

        for rel in mentions_res {
            let s = get_str(&rel, "source");
            let t = get_str(&rel, "target");
            if !s.is_empty() && !t.is_empty() {
                links.push(GraphLinkRes { source: s, target: t, label: None });
            }
        }
    }

    // 5. Related_to Links
    let sql_related = "SELECT type::string(in) as source, type::string(out) as target, relation FROM related_to";
    let related_res: Vec<JsonValue> = db.query(sql_related)
        .await.map_err(|e| e.to_string())?
        .take(0).map_err(|e| e.to_string())?;

    for rel in related_res {
        let s = get_str(&rel, "source");
        let t = get_str(&rel, "target");
        let label = get_str(&rel, "relation");

        if !s.is_empty() && !t.is_empty() {
            links.push(GraphLinkRes { source: s, target: t, label: Some(label) });
        }
    }

    println!("✅ [Debug] Success! Nodes: {}, Links: {}", nodes.len(), links.len());
    Ok(GraphResponse { nodes, links })
}