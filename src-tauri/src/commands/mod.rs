// src/commands/mod.rs

// 같은 폴더에 있는 파일들을 공개 모듈로 등록
pub mod ingest;
pub mod query;
pub mod log;

// (선택) 밖에서 crate::commands::process_pdfs 처럼 바로 쓰게 하려면:
// pub use ingest::process_pdfs;