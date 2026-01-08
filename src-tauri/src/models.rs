// src-tauri/src/models.rs
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use surrealdb::sql::Thing;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// =======================
// Graph Nodes
// =======================

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChunkNode {
    pub id: Option<Thing>,
    pub content: String,
    pub page_index: usize,
    pub embedding: Vec<f32>,
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
    pub entities: Vec<LlmEntity>,
    pub relations: Vec<LlmRelation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmEntity {
    pub name: String,
    pub category: String,
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmRelation {
    pub head: String,
    pub relation: String,
    pub tail: String,
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
}

#[derive(Serialize, Debug)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
}

#[derive(Serialize, Debug)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}