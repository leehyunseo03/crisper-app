use tauri::State;
use crate::AppState;

#[tauri::command(rename_all = "snake_case")]
pub fn log_node_click(
    node_id: String,
    group: String,
    label: String,
    info: Option<String>,
    state: State<'_, AppState>,
) {
    let msg = match group.as_str() {
        "entity" => format!(
            "🧠 Entity 클릭\n- id: {}\n- name: {}\n- category: {}",
            node_id,
            label,
            info.unwrap_or("unknown".into())
        ),
        "document" => format!(
            "📄 Document 클릭\n- id: {}\n- filename: {}",
            node_id,
            label
        ),
        "chunk" => format!(
            "📌 Chunk 클릭\n- id: {}\n- page: {}",
            node_id,
            label
        ),
        _ => format!(
            "🔹 Node 클릭\n- id: {}\n- label: {}",
            node_id,
            label
        ),
    };

    println!("{}", msg);

    // 👉 나중에 여기서 state.process_log.push(msg) 같은 것도 가능
}
