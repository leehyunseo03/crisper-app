// src/components/Genifier.tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import GraphVisualizer from './GraphVisualizer';

interface SelectedNode {
  id: string;
  group: string;
  label: string;
  info?: string;
}

export default function Genifier() {
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [log, setLog] = useState<string>("");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [refreshGraph, setRefreshGraph] = useState(0);
  const [isPanelOpen, setIsPanelOpen] = useState(true);
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null);
  const [useGpu, setUseGpu] = useState(false);
  const [kakaoPath, setKakaoPath] = useState<string | null>(null);

  const handleToggleGpu = async () => {
    const nextState = !useGpu;
    setUseGpu(nextState); // UI 즉시 반영
    
    setLog(prev => prev + `\n🔄 ${nextState ? "GPU" : "CPU"} 모드로 전환 중... (서버 재시작)`);
    setStatus("loading"); // 잠시 로딩 표시

    try {
      const msg = await invoke<string>("toggle_gpu", { enable: nextState });
      setLog(prev => prev + `\n✅ 완료: ${msg}`);
      setStatus("idle");
    } catch (e) {
      setLog(prev => prev + `\n❌ 전환 실패: ${String(e)}`);
      setStatus("error");
      setUseGpu(!nextState); // 실패 시 스위치 원상복구
    }
  };

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

  const handleSelectKakaoFile = async () => {
    try {
      const path = await open({
        directory: false, // 파일 선택
        multiple: false,
        filters: [{ name: 'Text', extensions: ['txt'] }] // .txt 필터
      });
      if (path && typeof path === 'string') { // string 체크
        setKakaoPath(path);
        setLog(prev => prev + `\n💬 카톡 파일 선택됨: ${path}`);
        setStatus("idle");
      }
    } catch (error) {
      console.error(error);
      setLog(prev => prev + `\n❌ 파일 선택 에러: ${String(error)}`);
    }
  };

  // 🆕 카카오톡 처리 시작 핸들러
  const handleStartKakaoProcess = async () => {
    if (!kakaoPath) return;
    try {
      setStatus("loading");
      setLog((prev) => prev + `\n🚀 카카오톡 분석 시작...`);
      
      // Rust 커맨드 호출
      const result = await invoke<string>("process_kakao_log", {
        filePath: kakaoPath, // Rust 인자 이름 snake_case 주의 (여기서는 Rust에서 file_path로 받음, 타우리는 자동 변환해주지만 확실하게 하려면 rename_all 확인 필요. 보통 camelCase -> snake_case 자동 매핑됨)
      });

      setLog((prev) => prev + `\n✅ 완료: ${result}`);
      setStatus("success");
      setRefreshGraph(prev => prev + 1);
    } catch (error) {
      console.error(error);
      setLog((prev) => prev + `\n❌ 실패: ${String(error)}`);
      setStatus("error");
    }
  };

  const handleStartEmbedding = async () => {
    if (!selectedPath) return;
    try {
      setStatus("loading");
      setLog((prev) => prev + `\n🚀 분석 시작... (PDF 텍스트 추출 및 임베딩)`);
      
      // 🚨 [수정됨] 백엔드 함수명 'process_pdfs'와 일치시킴
      const result = await invoke<string>("process_pdfs", {
        path: selectedPath,
      });

      setLog((prev) => prev + `\n✅ 완료: ${result}`);
      setStatus("success");
      // 그래프 갱신 트리거
      setRefreshGraph(prev => prev + 1);
    } catch (error) {
      console.error(error);
      setLog((prev) => prev + `\n❌ 실패: ${String(error)}`);
      setStatus("error");
    }
  };

  const handleNodeClick = (node: any) => {
    setSelectedNode({
      id: node.id,
      group: node.group,
      label: node.label,
      info: node.info
    });
    
    // 백엔드 로그 호출 (기존에 작성하신 rust command 호출)
    invoke("log_node_click", {
      nodeId: node.id,
      group: node.group,
      label: node.label,
      info: node.info || null
    }).catch(console.error);
  };

  return (
    <div style={{ position: "relative", width: "100%", height: "100%", backgroundColor: "#1e1e2e" }}>
      
      {/* --- Layer 1: 배경 그래프 --- */}
      <div style={{ position: "absolute", inset: 0, zIndex: 0 }}>
        {/* viewMode를 "all"로 전달하여 모든 노드 조회 */}
        <GraphVisualizer 
        refreshKey={refreshGraph} 
        viewMode="all" 
        onNodeClick={handleNodeClick}
        />
      </div>

      {selectedNode && (
        <div style={{
          position: "absolute",
          bottom: "20px",
          left: "20px",
          width: "300px",
          backgroundColor: "rgba(30, 30, 46, 0.9)",
          backdropFilter: "blur(10px)",
          borderRadius: "12px",
          border: "1px solid #89b4fa",
          padding: "15px",
          color: "#cdd6f4",
          zIndex: 20,
          boxShadow: "0 4px 20px rgba(0,0,0,0.5)"
        }}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "10px" }}>
            <span style={{ 
              fontSize: "0.7rem", 
              textTransform: "uppercase", 
              backgroundColor: "#45475a", 
              padding: "2px 6px", 
              borderRadius: "4px",
              color: "#89b4fa"
            }}>
              {selectedNode.group}
            </span>
            <button onClick={() => setSelectedNode(null)} style={{ background: 'none', border: 'none', color: '#f38ba8', cursor: 'pointer' }}>✕</button>
          </div>
          <h4 style={{ margin: "0 0 10px 0", color: "#f9e2af" }}>{selectedNode.label}</h4>
          <p style={{ fontSize: "0.85rem", color: "#a6adc8", margin: 0 }}>
            {selectedNode.info || "추가 정보가 없습니다."}
          </p>
          <div style={{ marginTop: "10px", fontSize: "0.7rem", color: "#585b70" }}>
            ID: {selectedNode.id}
          </div>
        </div>
      )}

      {/* --- Layer 2: 컨트롤 패널 --- */}
      <div 
        style={{
          position: "absolute",
          top: "20px",
          right: "20px",
          width: "320px",
          backgroundColor: "rgba(30, 30, 46, 0.85)",
          backdropFilter: "blur(10px)",
          borderRadius: "12px",
          border: "1px solid #45475a",
          boxShadow: "0 8px 32px rgba(0, 0, 0, 0.3)",
          zIndex: 10,
          display: "flex",
          flexDirection: "column",
          transition: "transform 0.3s ease",
          transform: isPanelOpen ? "translateX(0)" : "translateX(340px)",
          maxHeight: "calc(100vh - 40px)",
        }}
      >
        {/* 헤더 */}
        <div style={{ 
          padding: "15px 20px", 
          borderBottom: "1px solid #313244", 
          display: "flex", 
          justifyContent: "space-between", 
          alignItems: "center" 
        }}>
          <h3 style={{ margin: 0, color: "#89b4fa", fontSize: "1rem" }}>🛠️ Knowledge Graph</h3>
          <button 
            onClick={() => setIsPanelOpen(false)}
            style={{ background: "none", border: "none", color: "#a6adc8", cursor: "pointer" }}
          >
            ✕
          </button>
        </div>

        {/* 컨텐츠 */}
        <div style={{ padding: "20px", overflowY: "auto" }}>
          {/* ⚡ GPU 스위치 UI 추가 */}
          <div style={{ 
            marginBottom: "20px", 
            padding: "10px", 
            backgroundColor: "#313244", 
            borderRadius: "8px",
            border: "1px solid #45475a",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center"
          }}>
            <div>
              <div style={{ color: "#cdd6f4", fontWeight: "bold", fontSize: "0.9rem" }}>
                🚀 Hardware Accel
              </div>
              <div style={{ color: "#a6adc8", fontSize: "0.75rem" }}>
                {useGpu ? "NVIDIA GPU (CUDA)" : "Intel CPU Only"}
              </div>
            </div>
            
            <button
              onClick={handleToggleGpu}
              style={{
                padding: "6px 12px",
                borderRadius: "20px",
                border: "none",
                fontWeight: "bold",
                cursor: "pointer",
                transition: "0.3s",
                backgroundColor: useGpu ? "#a6e3a1" : "#45475a", // 켜지면 초록, 꺼지면 회색
                color: useGpu ? "#1e1e2e" : "#bac2de"
              }}
            >
              {useGpu ? "ON" : "OFF"}
            </button>
          </div>
          
          <div style={{ marginBottom: "20px" }}>
            <label style={{ display: "block", color: "#fab387", marginBottom: "8px", fontSize: "0.9rem" }}>PDF Source</label>
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
              {selectedPath || "📂 PDF 폴더 선택하기..."}
            </button>
          </div>
          
          <div style={{ marginBottom: "20px" }}>
            <label style={{ display: "block", color: "#f9e2af", marginBottom: "8px", fontSize: "0.9rem" }}>KakaoTalk Log (.txt)</label>
            <button
              onClick={handleSelectKakaoFile}
              style={{
                width: "100%", padding: "10px", borderRadius: "6px", border: "1px solid #45475a",
                backgroundColor: "#313244", color: kakaoPath ? "#a6e3a1" : "#cdd6f4",
                cursor: "pointer", textAlign: "left", whiteSpace: "nowrap", overflow: "hidden", 
                textOverflow: "ellipsis", fontSize: "0.85rem", marginBottom: "10px"
              }}
            >
              {kakaoPath ? `📄 ...${kakaoPath.slice(-20)}` : "💬 대화 내역 선택 (.txt)"}
            </button>

            <button
              onClick={handleStartKakaoProcess}
              disabled={!kakaoPath || status === "loading"}
              style={{
                width: "100%", padding: "10px", borderRadius: "8px", border: "none",
                backgroundColor: (!kakaoPath || status === "loading") ? "#45475a" : "#f9e2af", // 카톡은 노란색 테마
                color: (!kakaoPath || status === "loading") ? "#a6adc8" : "#1e1e2e",
                fontWeight: "bold", cursor: (!kakaoPath || status === "loading") ? "not-allowed" : "pointer"
              }}
            >
               {status === "loading" && kakaoPath ? "⏳ 대화 분석 중..." : "🚀 카톡 분석"}
            </button>
          </div>
          
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
            {status === "loading" ? "⏳ 지식 추출 중..." : "🚀 그래프 생성 / 업데이트"}
          </button>

          <div style={{ marginTop: "20px" }}>
            <label style={{ display: "block", color: "#bac2de", marginBottom: "8px", fontSize: "0.9rem" }}>Process Log</label>
            <div style={{
              backgroundColor: "#11111b",
              padding: "10px",
              borderRadius: "6px",
              height: "150px",  
              overflowY: "auto",
              fontSize: "0.75rem",
              fontFamily: "monospace",
              color: "#a6adc8",
              border: "1px solid #313244",
              whiteSpace: "pre-wrap"
            }}>
              {log || "대기 중..."}
            </div>
          </div>
        </div>
      </div>

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
          ⚙️ 설정
        </button>
      )}
    </div>
  );
}