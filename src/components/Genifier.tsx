import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import GraphVisualizer from './GraphVisualizer';

interface SelectedNode {
  id: string;
  group: string;
  label: string;
  info?: string;
}

interface DocMetadata {
  title?: string;
  summary?: string;
  tags?: string[];
}

// 🆕 청크(Chunk) 데이터 인터페이스
interface ChunkData {
  id: any;
  content: string;
  page_index: number;
  metadata?: DocMetadata;
}

// 🆕 DocumentData 인터페이스
interface DocumentData {
  id: { tb: string, id: { String: string } } | any;
  filename: string;
  created_at: string;
  metadata: DocMetadata;
  chunks: ChunkData[]; 
}

const DocumentItem = ({ doc }: { doc: DocumentData }) => {
  const [isOpen, setIsOpen] = useState(false);
  
  // ID 처리 (Rust의 Thing 구조체 호환)
  const docId = typeof doc.id === 'object' ? doc.id.id.String || JSON.stringify(doc.id) : doc.id;
  const meta = doc.metadata || {};

  return (
    <div style={{ backgroundColor: "#1e1e2e", borderRadius: "10px", border: "1px solid #313244", marginBottom: "10px", overflow: "hidden" }}>
      {/* 헤더 */}
      <div 
        onClick={() => setIsOpen(!isOpen)}
        style={{ padding: "15px 20px", display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer", backgroundColor: isOpen ? "#313244" : "transparent", transition: "0.2s" }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <span style={{ fontSize: "1.5rem" }}>📄</span>
          <div>
            <div style={{ color: "#cdd6f4", fontWeight: "bold", fontSize: "1rem" }}>
              {meta.title || doc.filename}
            </div>
            <div style={{ color: "#6c7086", fontSize: "0.75rem", marginTop: "2px" }}>
              {new Date(doc.created_at).toLocaleString()}
            </div>
          </div>
        </div>
        <div style={{ color: "#a6adc8", transform: isOpen ? "rotate(180deg)" : "rotate(0deg)", transition: "0.3s" }}>▼</div>
      </div>

      {/* 바디 (상세 내용) */}
      {isOpen && (
        <div style={{ backgroundColor: "#11111b", padding: "10px", borderTop: "1px solid #313244" }}>
          {doc.chunks.map((chunk: any, index: number) => {
            const cMeta = chunk.metadata || {};
            const cTitle = cMeta.title || `Chunk #${index + 1}`;
            const cSummary = cMeta.summary || "No summary available.";
            const cTags = cMeta.tags || [];

            return (
              <div key={index} style={{ padding: "15px", borderBottom: "1px solid #313244", marginBottom: "5px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "8px" }}>
                  <span style={{ color: "#fab387", fontWeight: "bold", fontSize: "0.9rem" }}>
                    {cTitle}
                  </span>
                  <div style={{ display: "flex", gap: "4px" }}>
                    {cTags.map((tag: string, tIdx: number) => (
                      <span key={tIdx} style={{ fontSize: "0.65rem", padding: "2px 6px", borderRadius: "4px", backgroundColor: "#313244", color: "#a6adc8" }}>
                        #{tag}
                      </span>
                    ))}
                  </div>
                </div>
                <p style={{ fontSize: "0.85rem", color: "#cdd6f4", margin: "0 0 10px 0", lineHeight: "1.4" }}>
                  {cSummary}
                </p>
                <details style={{ fontSize: "0.75rem", color: "#585b70", cursor: "pointer" }}>
                  <summary>원본 텍스트 보기</summary>
                  <p style={{ marginTop: "5px", whiteSpace: "pre-wrap" }}>{chunk.content}</p>
                </details>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default function Genifier() {
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [log, setLog] = useState<string>("");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [refreshGraph, setRefreshGraph] = useState(0);
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null);
  const [useGpu, setUseGpu] = useState(false);
  const [uiMode, setUiMode] = useState<"graph" | "list">("graph");
  const [documents, setDocuments] = useState<DocumentData[]>([]);

  // 🔄 문서 목록 불러오기
  const fetchDocuments = async () => {
    try {
      const docs = await invoke<DocumentData[]>("get_documents");
      setDocuments(docs);
    } catch (e) {
      console.error("Failed to fetch documents:", e);
    }
  };

  useEffect(() => {
    fetchDocuments();
  }, []);

  const handleToggleGpu = async () => {
    const nextState = !useGpu;
    setUseGpu(nextState);
    setLog(prev => prev + `\n🔄 ${nextState ? "GPU" : "CPU"} 모드로 전환 중...`);
    setStatus("loading");
    try {
      const msg = await invoke<string>("toggle_gpu", { enable: nextState });
      setLog(prev => prev + `\n✅ 완료: ${msg}`);
      setStatus("idle");
    } catch (e) {
      setLog(prev => prev + `\n❌ 실패: ${String(e)}`);
      setStatus("error");
      setUseGpu(!nextState);
    }
  };

  const handleSelectFolder = async () => {
    try {
      const path = await open({ directory: true, multiple: false });
      if (path) {
        setSelectedPath(path);
        setLog(`📂 선택됨: ${path}`);
        setStatus("idle");
      }
    } catch (error) {
      setLog(`에러: ${String(error)}`);
    }
  };

  // Step 1: 문서 저장 (Ingest)
  const handleIngestDocs = async () => {
    if (!selectedPath) return;
    try {
      setStatus("loading");
      setLog(prev => prev + `\n📥 [Step 1] 문서 저장 및 요약 시작...`);
      
      const result = await invoke<string>("ingest_documents", { path: selectedPath });

      setLog(prev => prev + `\n✅ 1단계 완료: ${result}`);
      setStatus("success");
      
      await fetchDocuments(); 
      setUiMode("list"); // 완료 후 리스트 뷰로 이동
    } catch (error) {
      setLog(prev => prev + `\n❌ 1단계 실패: ${String(error)}`);
      setStatus("error");
    }
  };

  // Step 2: 그래프 생성 (Graph Build)
  const handleBuildGraph = async () => {
    try {
      setStatus("loading");
      setLog(prev => prev + `\n🕸️ [Step 2] 지식 그래프 생성 시작... (시간이 걸릴 수 있습니다)`);
      
      const result = await invoke<string>("construct_graph"); // Rust Backend 호출

      setLog(prev => prev + `\n✅ 2단계 완료: ${result}`);
      setStatus("success");
      
      setRefreshGraph(prev => prev + 1); // 그래프 뷰 갱신 트리거
      setUiMode("graph"); // 그래프 뷰로 이동
    } catch (error) {
      console.error(error);
      setLog(prev => prev + `\n❌ 2단계 실패: ${String(error)}`);
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
  };

  return (
    <div style={{ position: "relative", width: "100%", height: "100vh", backgroundColor: "#1e1e2e", overflow: "hidden", display: "flex", flexDirection: "column" }}>
      {/* 상단 탭 */}
      <div style={{ padding: "15px 20px", display: "flex", gap: "10px", zIndex: 30, backgroundColor: "#11111b", borderBottom: "1px solid #313244" }}>
        <button onClick={() => setUiMode("graph")} style={{ padding: "8px 16px", borderRadius: "8px", border: "none", backgroundColor: uiMode === "graph" ? "#89b4fa" : "#313244", color: uiMode === "graph" ? "#11111b" : "#cdd6f4", cursor: "pointer", fontWeight: "bold" }}>🌐 Graph View</button>
        <button onClick={() => setUiMode("list")} style={{ padding: "8px 16px", borderRadius: "8px", border: "none", backgroundColor: uiMode === "list" ? "#89b4fa" : "#313244", color: uiMode === "list" ? "#11111b" : "#cdd6f4", cursor: "pointer", fontWeight: "bold" }}>📜 List View</button>
      </div>

      <div style={{ flex: 1, position: "relative", overflow: "hidden" }}>
        {uiMode === "graph" ? (
          <>
            <GraphVisualizer refreshKey={refreshGraph} viewMode="all" onNodeClick={handleNodeClick} />
            {selectedNode && (
               <div style={{ position: "absolute", bottom: "20px", left: "20px", width: "300px", backgroundColor: "rgba(30, 30, 46, 0.95)", backdropFilter: "blur(10px)", borderRadius: "12px", border: "1px solid #89b4fa", padding: "15px", color: "#cdd6f4", zIndex: 40, boxShadow: "0 4px 12px rgba(0,0,0,0.5)" }}>
                <h4 style={{ margin: "0 0 10px 0", color: "#f9e2af" }}>{selectedNode.label}</h4>
                <p style={{ fontSize: "0.85rem", color: "#a6adc8", maxHeight: "150px", overflowY: "auto" }}>{selectedNode.info}</p>
              </div>
            )}
          </>
        ) : (
          /* 📜 List Mode */
          <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
            {/* 컨트롤 패널 */}
            <div style={{ display: "flex", gap: "10px", padding: "10px 15px", backgroundColor: "#181825", borderBottom: "1px solid #313244", height: "80px", flexShrink: 0 }}>
              <div style={{ display: "flex", flexDirection: "column", justifyContent: "space-between", width: "240px" }}>
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", backgroundColor: "#313244", padding: "4px 10px", borderRadius: "6px" }}>
                  <span style={{ fontSize: "0.75rem", color: "#cdd6f4", fontWeight: "bold" }}>⚡ HW Accel</span>
                  <button onClick={handleToggleGpu} style={{ fontSize: "0.7rem", padding: "2px 8px", borderRadius: "4px", border: "none", cursor: "pointer", backgroundColor: useGpu ? "#a6e3a1" : "#45475a", color: useGpu ? "#1e1e2e" : "#bac2de", fontWeight: "bold" }}>
                    {useGpu ? "ON" : "OFF"}
                  </button>
                </div>
                <button onClick={handleSelectFolder} title={selectedPath || "폴더 선택"} style={{ width: "100%", padding: "6px 10px", borderRadius: "6px", border: "1px solid #45475a", backgroundColor: "#313244", color: selectedPath ? "#a6e3a1" : "#cdd6f4", cursor: "pointer", textAlign: "left", overflow: "hidden", whiteSpace: "nowrap", fontSize: "0.8rem" }}>
                  {selectedPath ? `📂 ...${selectedPath.slice(-20)}` : "📂 PDF 폴더 선택"}
                </button>
              </div>

              <div style={{ display: "flex", gap: "8px" }}>
                <button onClick={handleIngestDocs} disabled={!selectedPath || status === "loading"} style={{ width: "100px", borderRadius: "8px", border: "none", backgroundColor: (!selectedPath || status === "loading") ? "#45475a" : "#fab387", color: "#1e1e2e", fontWeight: "bold", cursor: "pointer", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "2px" }}>
                  <span style={{ fontSize: "1.2rem" }}>📥</span><span style={{ fontSize: "0.75rem" }}>Step 1</span>
                </button>
                <button onClick={handleBuildGraph} disabled={status === "loading"} style={{ width: "100px", borderRadius: "8px", border: "none", backgroundColor: (status === "loading") ? "#45475a" : "#89b4fa", color: "#1e1e2e", fontWeight: "bold", cursor: "pointer", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "2px" }}>
                  <span style={{ fontSize: "1.2rem" }}>🕸️</span><span style={{ fontSize: "0.75rem" }}>Step 2</span>
                </button>
              </div>

              <div style={{ flex: 1, backgroundColor: "#11111b", padding: "8px", borderRadius: "6px", border: "1px solid #313244", overflowY: "auto", fontFamily: "monospace", fontSize: "0.7rem", color: "#a6adc8", whiteSpace: "pre-wrap" }}>
                {log || "Ready..."}
              </div>
            </div>

            {/* 문서 리스트 */}
            <div style={{ flex: 1, backgroundColor: "#11111b", display: "flex", flexDirection: "column", overflow: "hidden" }}>
              <div style={{ padding: "15px 20px", borderBottom: "1px solid #313244", color: "#f9e2af", fontWeight: "bold", display: "flex", justifyContent: "space-between" }}>
                <span>📜 Knowledge List ({documents.length})</span>
                <button onClick={fetchDocuments} style={{ background: "none", border: "none", cursor: "pointer", fontSize: "1.2rem" }}>🔄</button>
              </div>
              <div style={{ flex: 1, overflowY: "auto", padding: "20px" }}>
                {documents.length === 0 ? (
                  <div style={{ color: "#585b70", textAlign: "center", marginTop: "50px" }}>
                    아직 저장된 문서가 없습니다. <br /> 상단에서 PDF를 선택하고 Step 1을 실행해주세요.
                  </div>
                ) : (
                  documents.map((doc, i) => <DocumentItem key={i} doc={doc} />)
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}