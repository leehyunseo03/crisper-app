use crate::models::CoreAnalysisResult;
use serde_json::{json, Value};
use std::error::Error;
use reqwest::Client;
use regex::Regex;

/// 텍스트를 분석하여 구조화된 JSON(CoreAnalysisResult)으로 반환합니다.
/// (Ingest Step 1에서 사용)
pub async fn analyze_content(
    base_url: &str,
    text: &str
) -> Result<CoreAnalysisResult, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let model_name = "gpt-3.5-turbo"; 

    // 프롬프트: 단순 요약이 아닌 "구조화된 정보" 추출 요구
    let system_instruction = r#"
    You are a Data Analyst preparing data for a Knowledge Graph.
    Analyze the given text and extract core information into JSON.

    ### JSON Output Format ###
    {
        "topic": "A short, descriptive title for this segment",
        "summary": "Contextual summary in Korean (1-2 sentences)",
        "key_entities": ["List", "of", "important", "nouns", "or", "names"],
        "detailed_data": {
            "type": "Identify the text type (e.g., Code, Meeting, News, Paper)",
            "facts": ["List of key facts"],
            "sentiment": "Neutral/Positive/Negative (Optional)"
        }
    }

    ### RULES ###
    1. Output MUST be valid JSON.
    2. 'summary' and 'facts' MUST be in **Korean**.
    3. 'key_entities' should be potential nodes for a graph (Person, Tech, Location).
    "#;

    // 텍스트 길이 제한 (속도 및 토큰 비용 최적화)
    let truncated_text = if text.len() > 3000 { &text[0..3000] } else { text };

    let payload = json!({
        "model": model_name,
        "messages": [
            { "role": "system", "content": system_instruction },
            { "role": "user", "content": truncated_text }
        ],
        "temperature": 0.2, // 구조화 정확성을 위해 낮음
        "response_format": { "type": "json_object" }
    });

    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    
    let res = client.post(&endpoint).json(&payload).send().await?;
    if !res.status().is_success() {
        return Err(format!("LLM Request Failed: {}", res.status()).into());
    }

    let resp_json: Value = res.json().await?;
    let content = resp_json["choices"][0]["message"]["content"].as_str().unwrap_or("{}");

    // JSON 수리 (LLM이 가끔 깨진 JSON을 줄 때를 대비)
    let cleaned = clean_and_repair_json(content);
    
    // 파싱 시도, 실패 시 에러 정보를 담은 기본 객체 반환
    let result: CoreAnalysisResult = serde_json::from_str(&cleaned).unwrap_or(CoreAnalysisResult {
        topic: "Analysis Failed".to_string(),
        summary: "분석 실패".to_string(),
        key_entities: vec![],
        detailed_data: json!({ "error": "Parsing failed", "raw": content }),
    });

    Ok(result)
}

/// LLM 응답 문자열에서 깨진 JSON을 수리합니다.
/// (Markdown 제거, Trailing Comma 제거, 닫히지 않은 괄호 수리)
fn clean_and_repair_json(input: &str) -> String {
    let mut clean = input.trim().to_string();

    // 1. 마크다운 코드 블록 제거
    if let Some(start) = clean.find("```json") { clean = clean[start+7..].to_string(); }
    else if let Some(start) = clean.find("```") { clean = clean[start+3..].to_string(); }
    if let Some(end) = clean.rfind("```") { clean = clean[..end].to_string(); }
    
    clean = clean.trim().to_string();

    // 2. Trailing Comma 제거 (", ]" -> "]")
    let re_trailing = Regex::new(r",(\s*[\]}])").unwrap();
    clean = re_trailing.replace_all(&clean, "$1").to_string();

    // 3. 빈 키 제거 ("": "",)
    let re_empty_key = Regex::new(r#"\s*""\s*:\s*".*?",?"#).unwrap();
    clean = re_empty_key.replace_all(&clean, "").to_string();
    
    // 4. 닫히지 않은 괄호 수리 (Truncated JSON 응급처치)
    if !clean.ends_with('}') {
        clean = clean.trim_end_matches(',').trim().to_string();
        
        let open_braces = clean.chars().filter(|&c| c == '{').count();
        let close_braces = clean.chars().filter(|&c| c == '}').count();
        let open_brackets = clean.chars().filter(|&c| c == '[').count();
        let close_brackets = clean.chars().filter(|&c| c == ']').count();

        if open_brackets > close_brackets { clean.push_str("]"); }
        if open_braces > close_braces { clean.push_str("}"); }
        
        // 최후의 수단
        if !clean.ends_with('}') { clean.push_str("}"); }
    }

    clean
}