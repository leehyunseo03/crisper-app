// src-tauri/src/llm/extractor.rs
use crate::models::{LlmExtractionResult, LlmEntity, LlmRelation};
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use std::error::Error;
use reqwest::{Client, Response};
use regex::Regex;

#[derive(Deserialize, Serialize, Debug, Clone)] // Clone, Serialize 추가
pub struct DocSummaryResult {
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    #[serde(default)] 
    pub keywords: Vec<String>, // 🆕 추가: 핵심 키워드 리스트
}

pub async fn summarize_document(
    base_url: &str,
    text: &str
) -> Result<DocSummaryResult, Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let model_name = "gpt-3.5-turbo"; // 혹은 사용 중인 모델명

    let system_instruction = r#"
    You are a Librarian AI. 
    Analyze the given text snippet.
    
    Output JSON format:
    {
        "title": "Concise title",
        "summary": "1-sentence summary in Korean",
        "tags": ["General Category"],
        "keywords": ["Entity1", "Entity2", "Technical Term"] 
    }
    
    Rules:
    1. 'keywords' must be specific nouns (e.g., 'Python', 'Transformer', 'Sam Altman').
    2. Extract 3~5 key entities.
    3. JSON only.
    "#;

    // 텍스트가 너무 길면 요약이 오래 걸리므로 앞부분 2000자만 사용
    let truncated_text = if text.len() > 2000 { &text[0..2000] } else { text };

    let payload = json!({
        "model": "gpt-3.5-turbo", // main.rs의 alias 확인
        "messages": [
            { "role": "system", "content": system_instruction },
            { "role": "user", "content": truncated_text }
        ],
        "temperature": 0.3,
        "response_format": { "type": "json_object" }
    });

    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    
    let res = client.post(&endpoint).json(&payload).send().await?;
    if !res.status().is_success() {
        return Err("Summary LLM Request Failed".into());
    }

    let resp_json: Value = res.json().await?;
    let content = resp_json["choices"][0]["message"]["content"].as_str().unwrap_or("{}");
    
    // 기존에 만든 clean_and_repair_json 재사용 (JSON 파싱 안전장치)
    let cleaned = clean_and_repair_json(content);
    
    let result: DocSummaryResult = serde_json::from_str(&cleaned).unwrap_or(DocSummaryResult {
        title: "Untitled".to_string(),
        summary: "요약 실패".to_string(),
        tags: vec![],
        keywords: vec![], // 실패 시 빈 배열
    });

    Ok(result)
}

// 직접 HTTP 요청을 보내기 위해 reqwest 사용
pub async fn extract_knowledge(
    base_url: &str, 
    text: &str
) -> Result<LlmExtractionResult, Box<dyn Error + Send + Sync>> {

    let client = reqwest::Client::new();
    
    // main.rs에서 설정한 alias "gpt-3.5-turbo"를 사용해야 llama-server가 인식합니다.
    let model_name = "gpt-3.5-turbo"; 

    let system_instruction = r#"
    You are an AI assistant that converts Chat Logs into a Knowledge Graph JSON.
    
    ### STRICT RULES ###
    1. **LANGUAGE:** ALL values (summary, reason) MUST be in **KOREAN (한국어)**.
    2. **FORBIDDEN:** Do NOT use Chinese characters (漢字). Do NOT use English unless the input is English.
    3. **FORMAT:** Output ONLY valid JSON matching the schema below.
    4. **CONTENT:** Extract clear entities and their interactions. Ignore trivial greetings (e.g., "ㅋㅋ", "안녕").

    ### JSON SCHEMA ###
    {
      "entities": [{"name": "User or Topic", "category": "Person/Tech/Issue", "summary": "Description in Korean"}],
      "relations": [{"head": "Subject", "relation": "Action", "tail": "Object", "reason": "Context in Korean"}]
    }

    ### ONE-SHOT EXAMPLE (Follow this pattern) ###
    Input:
    김철수: 이번주 서버 배포 일정 어떻게 돼?
    이영희: 내일 오후 2시에 진행할 예정이야. 근데 DB 마이그레이션이 좀 걱정되네.

    Output:
    {
      "entities": [
        {"name": "김철수", "category": "Person", "summary": "서버 배포 일정을 문의함"},
        {"name": "이영희", "category": "Person", "summary": "배포 일정 답변 및 DB 이슈 우려"},
        {"name": "서버 배포", "category": "Event", "summary": "내일 오후 2시 예정"},
        {"name": "DB 마이그레이션", "category": "Tech", "summary": "이영희가 우려하는 작업"}
      ],
      "relations": [
        {"head": "김철수", "relation": "asked_about", "tail": "서버 배포", "reason": "일정 문의"},
        {"head": "이영희", "relation": "scheduled", "tail": "서버 배포", "reason": "내일 오후 2시로 계획"},
        {"head": "이영희", "relation": "worried_about", "tail": "DB 마이그레이션", "reason": "잠재적 문제 예상"}
      ]
    }
    "#;

    // 1. 요청 페이로드 구성 (OpenAI API 포맷)
    let payload = json!({
        "model": model_name,
        "messages": [
            { "role": "system", "content": system_instruction },
            { "role": "user", "content": text } // 🌟 utils.rs에서 정제된 텍스트가 들어가야 함
        ],
        // 🌟 [중요] 파라미터 튜닝
        "temperature": 0.1,       // 0.0은 가끔 무한 루프에 빠지므로 0.1로 약간의 숨통 트기
        "top_p": 0.9,             // 엉뚱한 단어(염소 goat 등) 선택 방지
        "frequency_penalty": 1.1, // 반복 방지
        "max_tokens": 4096,
        "stream": false,
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
    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    // 🌟 [핵심] JSON 수리 및 파싱 시도
    let cleaned = clean_and_repair_json(content);
    
    // 디버깅용 로그 (필요시 주석 처리)
    println!("🔍 Raw LLM Response: {}", cleaned);

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
    
    // 마크다운 제거
    if let Some(start) = clean.find("```json") { 
        clean = clean[start+7..].to_string(); 
    } else if let Some(start) = clean.find("```") { 
        clean = clean[start+3..].to_string(); 
    }
    
    if let Some(end) = clean.rfind("```") { 
        clean = clean[..end].to_string(); 
    }
    
    clean = clean.trim().to_string();

    // 🚨 [추가] 끝이 '}' 나 ']' 로 끝나지 않으면 강제로 닫아주기 (응급처치)
    // 보통 relations 배열 내부에서 끊기므로, "}]}" 를 붙여서 복구를 시도해볼 수 있음.
    // 하지만 완벽하지 않으므로, 위 1, 2번 해결책이 우선입니다.
    if !clean.ends_with('}') {
        // 1. 마지막 쉼표 제거 시도
        clean = clean.trim_end_matches(',').to_string();
        
        // 2. 닫히지 않은 구조 닫기 (단순 무식한 방법)
        // 실제로는 스택을 써야 정확하지만, 여기선 relations 배열이 열려있다고 가정
        if !clean.ends_with("]}") {
             if clean.ends_with(']') {
                 clean.push('}');
             } else if clean.ends_with('}') {
                 // do nothing
             } else {
                 // 문자열 중간에 끊긴 경우 (ex: "reason": "...) -> 복구 불가능하므로 그냥 닫음
                 clean.push_str("\"}]}"); 
             }
        }
    }
    
    clean
}

// 🛠️ JSON 수리 함수 (가장 강력한 버전)
fn clean_and_repair_json(input: &str) -> String {
    let mut clean = input.trim().to_string();

    // 1. 마크다운 제거
    if let Some(start) = clean.find("```json") { clean = clean[start+7..].to_string(); }
    else if let Some(start) = clean.find("```") { clean = clean[start+3..].to_string(); }
    if let Some(end) = clean.rfind("```") { clean = clean[..end].to_string(); }
    
    clean = clean.trim().to_string();

    // 2. Trailing Comma 제거 (", ]" -> "]")
    // 정규식: ,(\s*[\]}]) -> $1
    let re_trailing = Regex::new(r",(\s*[\]}])").unwrap();
    clean = re_trailing.replace_all(&clean, "$1").to_string();

    // 3. 이상한 빈 키 제거 ("": "",) -> 정규식으로 삭제
    // 이 패턴이 로그에 자주 보임: "" : "",
    let re_empty_key = Regex::new(r#"\s*""\s*:\s*".*?",?"#).unwrap();
    clean = re_empty_key.replace_all(&clean, "").to_string();
    
    // 4. "$type$" 같은 이상한 키가 포함된 라인 제거 (선택 사항)
    // 리스크가 있으므로 일단은 스킵하거나, 특정 키워드만 삭제
    
    // 5. 닫히지 않은 괄호 수리 (Truncated JSON 응급처치)
    // relations 배열이 열려있는데 끝난 경우 등
    if !clean.ends_with('}') {
        // 마지막이 ','라면 제거
        clean = clean.trim_end_matches(',').trim().to_string();
        
        // 닫는 괄호 개수 계산 (간단 버전)
        let open_braces = clean.chars().filter(|&c| c == '{').count();
        let close_braces = clean.chars().filter(|&c| c == '}').count();
        let open_brackets = clean.chars().filter(|&c| c == '[').count();
        let close_brackets = clean.chars().filter(|&c| c == ']').count();

        // 배열이 덜 닫혔으면 닫아줌
        if open_brackets > close_brackets { clean.push_str("]"); }
        // 객체가 덜 닫혔으면 닫아줌
        if open_braces > close_braces { clean.push_str("}"); }
        
        // 그래도 안 맞으면 강제 종료
        if !clean.ends_with('}') { clean.push_str("}"); }
    }

    clean
}