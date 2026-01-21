// src-tauri/src/utils.rs
use std::fs;
use std::path::Path;
use std::io::Read;
use pdf_extract::extract_text;
use anyhow::Context;
use rig::embeddings::{Embed, TextEmbedder, EmbedError};
use serde::{Serialize, Deserialize};
use regex::Regex;

// Rig용 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigDoc {
    pub id: String,
    pub content: String,
}

impl Embed for RigDoc {
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        embedder.embed(self.content.clone());
        Ok(())
    }
}

// 🚨 pub 추가
pub fn extract_text_from_pdf<P: AsRef<Path>>(file_path: P) -> anyhow::Result<String> {
    extract_text(file_path.as_ref())
        .with_context(|| format!("Failed to extract text from PDF: {:?}", file_path.as_ref()))
}

pub fn parse_kakao_talk_log<P: AsRef<Path>>(file_path: P) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(file_path)?;
    let mut content = String::new();
    std::io::Read::read_to_string(&mut file, &mut content)?;

    // 🌟 [수정] 정규식으로 카톡 패턴 정리
    // 패턴: [이름] [시간] 내용 -> 이름: 내용
    let re = Regex::new(r"\[(.*?)\] \[(.*?)\] (.*)").unwrap();
    
    let cleaned_lines: Vec<String> = content.lines()
        .map(|line| {
            if let Some(caps) = re.captures(line) {
                let name = &caps[1];
                // 시간(&caps[2])은 지식 그래프에 중요하지 않으니 제거
                let message = &caps[3];
                format!("{}: {}", name, message)
            } else {
                // 날짜 구분선 등은 그대로 둠
                line.to_string()
            }
        })
        .collect();

    Ok(cleaned_lines.join("\n"))
}

// 🚨 pub 추가
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        let end = std::cmp::min(start + chunk_size, chars.len());
        let chunk: String = chars[start..end].iter().collect();
        
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }
        if end == chars.len() { break; }
        start += chunk_size - overlap;
    }
    chunks
}

/// 텍스트를 SurrealDB ID safe한 문자열로 변환 (예: "Apple Inc." -> "apple_inc")
pub fn sanitize_id(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        // 연속된 언더스코어 제거 등은 선택 사항
}