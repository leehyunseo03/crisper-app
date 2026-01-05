// src/components/Genifier.tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import GraphVisualizer from './GraphVisualizer';

export default function Genifier() {
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [log, setLog] = useState<string>("");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [refreshGraph, setRefreshGraph] = useState(0);
  const [isPanelOpen, setIsPanelOpen] = useState(true); // 패널 토글 상태

  const handleSelectFolder = async () => {
    try {
      const path = await open({
        directory: true,
        multiple: false,
      });
      if (path) {
        setSelectedPath(path);
        setLog(`📂 선택됨: ${path}`);
        setStatus("idle");
      }
    } catch (error) {
      console.error(error);
      setLog(`에러: ${String(error)}`);
    }
  };

  const handleStartEmbedding = async () => {
    if (!selectedPath) return;
    try {
      setStatus("loading");
      setLog((prev) => prev + `\n🚀 분석 시작...`);
      
      const result = await invoke<string>("process_pdfs_graph", {
        path: selectedPath,
      });

      setLog((prev) => prev + `\n✅ 완료: ${result}`);
      setStatus("success");
      setRefreshGraph(prev => prev + 1);
    } catch (error) {
      setLog((prev) => prev + `\n❌ 실패: ${String(error)}`);
      setStatus("error");
    }
  };

  return (
    <div style={{ position: "relative", width: "100%", height: "100%", backgroundColor: "#1e1e2e" }}>
      
      {/* --- Layer 1: 배경 그래프 (항상 표시) --- */}
      <div style={{ position: "absolute", inset: 0, zIndex: 0 }}>
        <GraphVisualizer refreshKey={refreshGraph} />
      </div>

      {/* --- Layer 2: 컨트롤 패널 (플로팅) --- */}
      <div 
        style={{
          position: "absolute",
          top: "20px",
          right: "20px",
          width: "320px",
          backgroundColor: "rgba(30, 30, 46, 0.85)", // 반투명 배경
          backdropFilter: "blur(10px)", // 블러 효과
          borderRadius: "12px",
          border: "1px solid #45475a",
          boxShadow: "0 8px 32px rgba(0, 0, 0, 0.3)",
          zIndex: 10,
          display: "flex",
          flexDirection: "column",
          transition: "transform 0.3s ease",
          transform: isPanelOpen ? "translateX(0)" : "translateX(340px)", // 패널 숨김 처리
          maxHeight: "calc(100vh - 40px)",
        }}
      >
        {/* 패널 헤더 */}
        <div style={{ 
          padding: "15px 20px", 
          borderBottom: "1px solid #313244", 
          display: "flex", 
          justifyContent: "space-between", 
          alignItems: "center" 
        }}>
          <h3 style={{ margin: 0, color: "#89b4fa", fontSize: "1rem" }}>🛠️ Control Panel</h3>
          <button 
            onClick={() => setIsPanelOpen(false)}
            style={{ background: "none", border: "none", color: "#a6adc8", cursor: "pointer" }}
          >
            ✕
          </button>
        </div>

        {/* 패널 내용 */}
        <div style={{ padding: "20px", overflowY: "auto" }}>
          
          {/* 폴더 선택 */}
          <div style={{ marginBottom: "20px" }}>
            <label style={{ display: "block", color: "#fab387", marginBottom: "8px", fontSize: "0.9rem" }}>Data Source</label>
            <button
              onClick={handleSelectFolder}
              style={{
                width: "100%",
                padding: "10px",
                borderRadius: "6px",
                border: "1px solid #45475a",
                backgroundColor: "#313244",
                color: selectedPath ? "#a6e3a1" : "#cdd6f4",
                cursor: "pointer",
                textAlign: "left",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
                fontSize: "0.85rem"
              }}
            >
              {selectedPath || "📂 폴더 선택하기..."}
            </button>
          </div>

          {/* 실행 버튼 */}
          <button
            onClick={handleStartEmbedding}
            disabled={!selectedPath || status === "loading"}
            style={{
              width: "100%",
              padding: "12px",
              borderRadius: "8px",
              border: "none",
              backgroundColor: (!selectedPath || status === "loading") ? "#45475a" : "#89b4fa",
              color: (!selectedPath || status === "loading") ? "#a6adc8" : "#1e1e2e",
              fontWeight: "bold",
              cursor: (!selectedPath || status === "loading") ? "not-allowed" : "pointer",
              transition: "0.2s"
            }}
          >
            {status === "loading" ? "⏳ 처리 중..." : "🚀 그래프 생성 / 업데이트"}
          </button>

          {/* 로그 영역 (간소화) */}
          <div style={{ marginTop: "20px" }}>
            <label style={{ display: "block", color: "#bac2de", marginBottom: "8px", fontSize: "0.9rem" }}>System Log</label>
            <div style={{
              backgroundColor: "#11111b",
              padding: "10px",
              borderRadius: "6px",
              height: "150px",
              overflowY: "auto",
              fontSize: "0.75rem",
              fontFamily: "monospace",
              color: "#a6adc8",
              border: "1px solid #313244"
            }}>
              {log || "대기 중..."}
            </div>
          </div>
        </div>
      </div>

      {/* 패널 열기 버튼 (패널이 닫혔을 때 표시) */}
      {!isPanelOpen && (
        <button
          onClick={() => setIsPanelOpen(true)}
          style={{
            position: "absolute",
            top: "20px",
            right: "20px",
            zIndex: 10,
            padding: "10px 15px",
            backgroundColor: "#89b4fa",
            color: "#1e1e2e",
            border: "none",
            borderRadius: "8px",
            fontWeight: "bold",
            cursor: "pointer",
            boxShadow: "0 4px 12px rgba(0,0,0,0.3)"
          }}
        >
          ⚙️ 옵션
        </button>
      )}
    </div>
  );
}