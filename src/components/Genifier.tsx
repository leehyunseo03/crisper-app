// src/components/Genifier.tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog"; // 파일 선택 다이얼로그

export default function Genifier() {
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [log, setLog] = useState<string>("");

  // 폴더 선택 및 임베딩 시작 핸들러
  const handleSelectAndEmbed = async () => {
    try {
      // 1. 폴더 선택 다이얼로그 열기
      const selectedPath = await open({
        directory: true, // 폴더 선택 모드 (Rust 코드가 fs::read_dir를 쓰므로)
        multiple: false,
      });

      if (!selectedPath) return; // 취소함

      setStatus("loading");
      setLog(`선택된 경로: ${selectedPath}\n분석 및 임베딩 시작...`);

      // 2. Rust 백엔드 명령어 호출 (path 인자 전달)
      const result = await invoke<string>("process_pdfs", {
        path: selectedPath,
      });

      setLog((prev) => prev + `\n완료: ${result}`);
      setStatus("success");
    } catch (error) {
      console.error(error);
      setLog((prev) => prev + `\n에러 발생: ${String(error)}`);
      setStatus("error");
    }
  };

  return (
    <div style={{ padding: "40px", color: "#cdd6f4" }}>
      <h2 style={{ color: "#89b4fa" }}>🧬 디지털 유전자 (Graph Index)</h2>
      <p style={{ marginBottom: "20px" }}>
        PDF 문서가 있는 폴더를 선택하세요. 문서를 분석하여 지식 그래프를 생성합니다.
      </p>

      {/* 업로드 섹션 */}
      <div
        style={{
          border: "2px dashed #45475a",
          borderRadius: "10px",
          padding: "40px",
          textAlign: "center",
          backgroundColor: "#1e1e2e",
          cursor: status === "loading" ? "wait" : "default",
        }}
      >
        <div style={{ fontSize: "3rem", marginBottom: "10px" }}>📂</div>
        
        {status === "idle" || status === "success" || status === "error" ? (
          <button
            onClick={handleSelectAndEmbed}
            style={{
              padding: "10px 20px",
              fontSize: "1rem",
              borderRadius: "8px",
              border: "none",
              backgroundColor: "#89b4fa",
              color: "#1e1e2e",
              fontWeight: "bold",
              cursor: "pointer",
              transition: "0.2s",
            }}
          >
            폴더 선택 및 학습 시작
          </button>
        ) : (
          <div style={{ color: "#f9e2af" }}>
            🧬 DNA 생성 중... (잠시만 기다려주세요)
          </div>
        )}
      </div>

      {/* 로그 출력 영역 */}
      <div
        style={{
          marginTop: "20px",
          backgroundColor: "#11111b",
          padding: "15px",
          borderRadius: "8px",
          fontFamily: "monospace",
          fontSize: "0.9rem",
          whiteSpace: "pre-wrap",
          minHeight: "100px",
          border: "1px solid #313244",
          color: status === "error" ? "#f38ba8" : "#a6e3a1",
        }}
      >
        {log || "대기 중..."}
      </div>
    </div>
  );
}