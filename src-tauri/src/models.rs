use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use surrealdb::sql::Thing;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde_json::Value;

// =======================
// DB Nodes (SurrealDB)
// =======================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentNode {
    #[serde(skip_serializing_if = "Option::is_none")]
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
    
    /// 청크 분석 결과(CoreAnalysisResult 등)가 담기는 필드
    #[serde(default)] 
    pub metadata: HashMap<String, serde_json::Value>, 
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    pub name: String,
    pub category: String,
    pub description: String,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
}

// =======================
// API Response DTOs
// =======================

/// 클라이언트에 문서와 포함된 청크 정보를 함께 반환하기 위한 구조체
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentWithChunks {
    pub id: Thing,
    pub filename: String,
    pub created_at: chrono::DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// 서브쿼리를 통해 채워지는 청크 리스트
    #[serde(default)] 
    pub chunks: Vec<ChunkNode>, 
}

/// LLM 분석 결과 (Step 1)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoreAnalysisResult {
    /// 문서/청크의 주제
    pub topic: String, 
    /// 문맥 요약 (한국어)
    pub summary: String,
    /// 그래프 생성을 위한 핵심 키워드/엔티티 리스트
    pub key_entities: Vec<String>,
    /// 추가적인 상세 데이터 (Type, Facts 등)
    pub detailed_data: Value, 
}