use std::path::Path;
use lopdf::Document;
use anyhow::Context;

/// PDF 파일에서 페이지별로 텍스트를 추출합니다.
/// 
/// # Arguments
/// * `file_path` - PDF 파일의 경로
/// 
/// # Returns
/// * `Ok(Vec<String>)` - 각 페이지의 텍스트가 담긴 리스트 (빈 페이지 제외)
pub fn extract_pages_from_pdf<P: AsRef<Path>>(file_path: P) -> anyhow::Result<Vec<String>> {
    // PDF 로드 (lopdf crate 사용)
    let doc = Document::load(file_path.as_ref())
        .with_context(|| format!("Failed to load PDF: {:?}", file_path.as_ref()))?;
    
    let mut pages = Vec::new();
    
    // 페이지 번호를 가져와서 순서대로 정렬 (1페이지부터)
    let mut page_numbers: Vec<u32> = doc.get_pages().keys().cloned().collect();
    page_numbers.sort();

    for page_num in page_numbers {
        // 해당 페이지의 텍스트 추출
        // 실패 시 에러를 내지 않고 빈 문자열 처리하여 진행
        let text = doc.extract_text(&[page_num]).unwrap_or_default();
        
        // 내용이 있는 페이지만 결과에 포함
        if !text.trim().is_empty() {
            pages.push(text);
        }
    }

    Ok(pages)
}

/// 텍스트를 SurrealDB의 ID로 사용하기 적합한 형태(소문자, 특수문자 제거)로 변환합니다.
/// 예: "Apple Inc." -> "apple_inc" (단, 여기서는 alphanumeric만 남기고 '_'로 치환)
pub fn sanitize_id(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
}