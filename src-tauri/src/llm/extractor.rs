// src-tauri/src/llm/extractor.rs
use crate::models::{LlmExtractionResult, LlmEntity, LlmRelation};
use serde_json::{json, Value};
use std::error::Error;
use reqwest::{Client, Response};

// 직접 HTTP 요청을 보내기 위해 reqwest 사용
pub async fn extract_knowledge(
    base_url: &str, 
    text: &str
) -> Result<LlmExtractionResult, Box<dyn Error>> {

    let client = reqwest::Client::new();
    
    // main.rs에서 설정한 alias "gpt-3.5-turbo"를 사용해야 llama-server가 인식합니다.
    let model_name = "gpt-3.5-turbo"; 

    let system_instruction = r#"
    You are a Knowledge Graph Extractor.
    Extract entities and relationships from the text into JSON.
    RULES:
    1. Output ONLY valid JSON.
    2. Extract entities (Person, Topic, Tech, Event).
    3. Extract relations (actions, descriptions).
    4. If text is Korean, you can use Korean values.
    JSON SCHEMA:
    {
      "entities": [{"name": "...", "category": "...", "summary": "..."}],
      "relations": [{"head": "...", "relation": "...", "tail": "...", "reason": "..."}]
    }
    "#;

    // 1. 요청 페이로드 구성 (OpenAI API 포맷)
    let payload = json!({
        "model": model_name,
        "messages": [
            { "role": "system", "content": system_instruction },
            { "role": "user", "content": text }
        ],
        "temperature": 0.0, // 정보 추출이므로 창의성 제한
        "max_tokens": 4096,
        "stream": false,
        // llama-server 최신 버전은 json_object 모드를 지원하므로 힌트를 줍니다.
        "response_format": { "type": "json_object" } 
    });

    // 2. URL 정리 (끝에 슬래시 제거 및 경로 결합)
    // main.rs나 ingest.rs에서 "http://127.0.0.1:8081/v1" 형태로 들어온다고 가정
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    println!("🤖 [Extractor] Requesting to: {}", endpoint);

    // 3. POST 요청 전송
    let res:Response = client.post(&endpoint)
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await?;
        return Err(format!("LLM Server Error: {}", err_text).into());
    }

    // 4. 응답 파싱
    let resp_json: Value = res.json().await?;
    
    // OpenAI 포맷: choices[0].message.content
    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    // 5. 후처리 및 JSON 변환
    let cleaned = clean_json_response(content);
    
    // 디버깅용 로그 (필요시 주석 처리)
    // println!("🔍 Raw LLM Response: {}", cleaned);

    match serde_json::from_str::<LlmExtractionResult>(&cleaned) {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("❌ JSON Parsing Failed. Raw Content:\n{}", content);
            Err(Box::new(e))
        }
    }
}

// 마크다운 코드 블록 제거 헬퍼 함수
fn clean_json_response(response: &str) -> String {
    let mut clean = response.trim().to_string();
    
    // ```json ... ``` 제거 로직
    if let Some(start) = clean.find("```json") { 
        clean = clean[start+7..].to_string(); 
    } else if let Some(start) = clean.find("```") { 
        clean = clean[start+3..].to_string(); 
    }
    
    if let Some(end) = clean.rfind("```") { 
        clean = clean[..end].to_string(); 
    }
    
    clean.trim().to_string()
}