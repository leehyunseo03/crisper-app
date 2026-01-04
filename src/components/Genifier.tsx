import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import GraphVisualizer from './GraphVisualizer';

export default function Genifier() {
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [log, setLog] = useState<string>("");
  const [selectedPath, setSelectedPath] = useState<string | null>(null); // 선택된 경로 상태 저장
  const [refreshGraph, setRefreshGraph] = useState(0);

  // 1. 폴더 선택 핸들러
  const handleSelectFolder = async () => {
    try {
      const path = await open({
        directory: true,
        multiple: false,
      });

      if (path) {
        setSelectedPath(path);
        setLog(`📂 폴더가 선택되었습니다: ${path}`);
        setStatus("idle"); // 상태 초기화
      }
    } catch (error) {
      console.error(error);
      setLog(`경로 선택 중 에러: ${String(error)}`);
    }
  };

  // 2. 임베딩(그래프 생성) 시작 핸들러
  const handleStartEmbedding = async () => {
    if (!selectedPath) {
      setLog("⚠️ 먼저 폴더를 선택해주세요.");
      return;
    }

    try {
      setStatus("loading");
      setLog((prev) => prev + `\n\n🚀 [SurrealDB] 그래프 생성 시작...`);

      // Rust 백엔드 호출
      console.log("Value:", selectedPath);
      console.log("Type:", typeof selectedPath);
      const result = await invoke<string>("process_pdfs_graph", {
        path: selectedPath,
      });

      setLog((prev) => prev + `\n✅ 완료: ${result}`);
      setStatus("success");

      setRefreshGraph(prev => prev + 1);
    } catch (error) {
      console.error(error);
      setLog((prev) => prev + `\n❌ 에러 발생: ${String(error)}`);
      setStatus("error");
    }
  };

  return (
    <div style={{ padding: "40px", color: "#cdd6f4", maxWidth: "800px", margin: "0 auto" }}>
      <h2 style={{ color: "#89b4fa" }}>🧬 디지털 유전자 (Graph Index)</h2>
      <p style={{ marginBottom: "30px", color: "#a6adc8" }}>
        학습시킬 PDF 문서들이 들어있는 폴더를 선택하고, 그래프 생성을 시작하세요.
      </p>

      {/* --- 1단계: 폴더 선택 영역 --- */}
      <div style={{ marginBottom: "20px" }}>
        <h3 style={{ fontSize: "1.1rem", marginBottom: "10px", color: "#fab387" }}>Step 1. 폴더 선택</h3>
        <div style={{ display: "flex", gap: "10px", alignItems: "center" }}>
          <button
            onClick={handleSelectFolder}
            disabled={status === "loading"}
            style={{
              padding: "12px 20px",
              fontSize: "1rem",
              borderRadius: "8px",
              border: "1px solid #45475a",
              backgroundColor: "#313244",
              color: "#cdd6f4",
              cursor: status === "loading" ? "not-allowed" : "pointer",
              transition: "0.2s",
              flexShrink: 0,
            }}
          >
            📂 폴더 열기
          </button>
          
          <div style={{ 
            flex: 1, 
            padding: "12px", 
            backgroundColor: "#181825", 
            borderRadius: "8px", 
            border: "1px solid #313244",
            color: selectedPath ? "#a6e3a1" : "#585b70",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            fontFamily: "monospace"
          }}>
            {selectedPath || "선택된 폴더 없음"}
          </div>
        </div>
      </div>

      {/* --- 2단계: 실행 버튼 영역 --- */}
      <div style={{ marginBottom: "30px" }}>
        <h3 style={{ fontSize: "1.1rem", marginBottom: "10px", color: "#fab387" }}>Step 2. 그래프 생성</h3>
        <button
          onClick={handleStartEmbedding}
          disabled={!selectedPath || status === "loading"}
          style={{
            width: "100%",
            padding: "15px",
            fontSize: "1.1rem",
            borderRadius: "10px",
            border: "none",
            // 경로가 없으면 회색, 로딩중이면 노란색, 준비되면 파란색
            backgroundColor: !selectedPath ? "#45475a" : status === "loading" ? "#f9e2af" : "#89b4fa",
            color: !selectedPath ? "#a6adc8" : "#1e1e2e",
            fontWeight: "bold",
            cursor: (!selectedPath || status === "loading") ? "not-allowed" : "pointer",
            transition: "all 0.3s ease",
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            gap: "10px"
          }}
        >
          {status === "loading" ? (
            <>⏳ 분석 및 임베딩 진행 중...</>
          ) : (
            <>🚀 임베딩 시작 (Graph Indexing)</>
          )}
        </button>
      </div>

      {/* --- 로그 출력 영역 --- */}
      <div
        style={{
          marginTop: "20px",
          backgroundColor: "#11111b",
          padding: "20px",
          borderRadius: "10px",
          fontFamily: "monospace",
          fontSize: "0.9rem",
          whiteSpace: "pre-wrap",
          minHeight: "150px",
          maxHeight: "300px",
          overflowY: "auto",
          border: "1px solid #313244",
          color: status === "error" ? "#f38ba8" : "#bac2de",
          boxShadow: "inset 0 0 10px rgba(0,0,0,0.5)"
        }}
      >
        <div style={{ color: "#6c7086", marginBottom: "10px", borderBottom: "1px solid #313244", paddingBottom: "5px" }}>
          🖥️ System Logs
        </div>
        {log || "대기 중..."}
      </div>
      <div style={{ marginTop: "40px" }}>
        <h3 style={{ fontSize: "1.1rem", marginBottom: "15px", color: "#fab387" }}>
            Step 3. Knowledge Graph Visualization
        </h3>
        {/* 그래프 컴포넌트 배치 */}
        <GraphVisualizer refreshKey={refreshGraph} />
      </div>
    </div>
  );
}