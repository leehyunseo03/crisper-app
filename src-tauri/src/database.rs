// src-tauri/src/database.rs

use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

// 🚨 pub 추가
pub async fn init_db() -> surrealdb::Result<Surreal<Db>> {
    // 경로 수정: 실행 파일 기준 상위 폴더 등 적절히
    let db = Surreal::new::<RocksDb>("../data/crisper_db").await?;
    
    db.use_ns("crisper_ns").use_db("crisper_db").await?;
    
    Ok(db)
}