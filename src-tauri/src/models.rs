// src-tauri/src/models.rs
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use surrealdb::sql::Thing;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// =======================
// Graph Nodes
// =======================

fn default_string() -> String { "".to_string() }
fn default_category() -> String { "General".to_string() } // 카테고리 없으면 'General'로 자동 채움

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventNode {
    pub id: Option<Thing>,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentNode {
    pub id: Option<Thing>,
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    pub content: String,
    pub page_index: usize,
    pub embedding: Vec<f32>,
    
    // 🆕 청크별 요약 정보를 담을 필드 추가
    #[serde(default)] 
    pub metadata: HashMap<String, serde_json::Value>, 
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityNode {
    pub id: Option<Thing>,
    pub name: String,
    pub category: String,
    pub description: String,
    pub embedding: Vec<f32>, // 병합용 벡터
    pub created_at: DateTime<Utc>,
}

// =======================
// Graph Edges
// =======================

// 시스템 관계 (imported, contains 등)
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemEdge {
    pub id: Option<Thing>,
    #[serde(rename = "in")]
    pub in_: Thing,
    #[serde(rename = "out")]
    pub out: Thing,
    pub created_at: DateTime<Utc>,
}

// 지식 관계 (mentions, relates_to 등)
#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub id: Option<Thing>,
    #[serde(rename = "in")]
    pub in_: Thing,
    #[serde(rename = "out")]
    pub out: Thing,
    pub rel_type: String,
    pub details: HashMap<String, JsonValue>,
}

// =======================
// LLM DTOs
// =======================

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmExtractionResult {
    #[serde(default)]
    pub entities: Vec<LlmEntity>,
    #[serde(default)]
    pub relations: Vec<LlmRelation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmEntity {
    // 이름이 없으면 "Unknown" 처리
    #[serde(default = "default_string")]
    pub name: String,
    
    // 🚨 [핵심 수정] category 필드가 없으면 에러 내지 말고 "General"로 채워라
    #[serde(default = "default_category")] 
    pub category: String,
    
    // summary가 없으면 빈 문자열로 채워라
    #[serde(default = "default_string")] 
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmRelation {
    #[serde(default = "default_string")]
    pub head: String,
    
    #[serde(default = "default_string")]
    pub relation: String,
    
    #[serde(default = "default_string")]
    pub tail: String,
    
    #[serde(default = "default_string")]
    pub reason: String,
}
// =======================
// UI Visualization
// =======================
#[derive(Serialize, Debug)]
pub struct GraphNode {
    pub id: String,
    pub group: String,
    pub label: String,
    pub val: usize,
    pub info: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}