// src/components/Genifier.tsx
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

// 2. 🆕 청크(Chunk) 데이터 인터페이스 추가
interface ChunkData {
  id: any;
  content: string;
  page_index: number;
  metadata?: DocMetadata; // 청크별 요약 정보
}

// 3. 🚨 DocumentData 인터페이스 수정 (chunks 추가)
interface DocumentData {
  id: { tb: string, id: { String: string } } | any;
  filename: string;
  created_at: string;
  metadata: DocMetadata;
  chunks: ChunkData[]; // 👈 이 줄이 없어서 에러가 났던 것입니다.
}

const DocumentItem = ({ doc }: { doc: DocumentData }) => {
  const [isOpen, setIsOpen] = useState(false);
  
  // ID 처리 (Rust의 Thing 구조체가 JSON으로 넘어올 때의 처리)
  const docId = typeof doc.id === 'object' ? doc.id.id.String || JSON.stringify(doc.id) : doc.id;
  const meta = doc.metadata || {};
  const tags = Array.isArray(meta.tags) ? meta.tags : [];

  return (
    <div style={{ backgroundColor: "#1e1e2e", borderRadius: "10px", border: "1px solid #313244", marginBottom: "10px", overflow: "hidden" }}>
      {/* 헤더 (클릭 시 토글) */}
      <div 
        onClick={() => setIsOpen(!isOpen)}
        style={{ padding: "15px 20px", display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer", backgroundColor: isOpen ? "#313244" : "transparent", transition: "0.2s" }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <span style={{ fontSize: "1.5rem" }}>📄</span>
          <div>
            {/* 메타데이터의 title이 있으면 쓰고, 없으면 파일명 사용 */}
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
            // 🌟 청크 메타데이터 가져오기
            const cMeta = chunk.metadata || {};
            const cTitle = cMeta.title || `Chunk #${index + 1}`;
            const cSummary = cMeta.summary || "No summary available.";
            const cTags = cMeta.tags || [];

            return (
              <div key={index} style={{ padding: "15px", borderBottom: "1px solid #313244", marginBottom: "5px" }}>
                {/* 청크 헤더: 제목 및 태그 */}
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

                {/* 청크 요약 */}
                <p style={{ fontSize: "0.85rem", color: "#cdd6f4", margin: "0 0 10px 0", lineHeight: "1.4" }}>
                  {cSummary}
                </p>

                {/* 원본 텍스트 (더보기로 숨기거나 작게 표시) */}
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
  const [isPanelOpen, setIsPanelOpen] = useState(true);
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null);
  const [useGpu, setUseGpu] = useState(false);
  const [kakaoPath, setKakaoPath] = useState<string | null>(null);
  const [uiMode, setUiMode] = useState<"graph" | "list">("graph");
  
  // 🆕 문서 목록 State
  const [documents, setDocuments] = useState<DocumentData[]>([]);

  // 🆕 문서 목록 불러오기 함수
  const fetchDocuments = async () => {
    try {
      const docs = await invoke<DocumentData[]>("get_documents");
      setDocuments(docs);
    } catch (e) {
      console.error("Failed to fetch documents:", e);
    }
  };

  // 🆕 컴포넌트 마운트 시 최초 로드
  useEffect(() => {
    fetchDocuments();
  }, []);

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

  const handleIngestDocs = async () => {
    if (!selectedPath) return;
    try {
      setStatus("loading");
      setLog(prev => prev + `\n📥 [Step 1] 문서 저장 및 요약 시작...`);
      
      const result = await invoke<string>("ingest_documents", { path: selectedPath });

      setLog(prev => prev + `\n✅ 1단계 완료: ${result}`);
      setStatus("success");
      
      // 🌟 [핵심] 완료 후 리스트 즉시 갱신 및 리스트 뷰로 전환
      await fetchDocuments(); 
      setUiMode("list"); // 작업 끝나면 결과를 보라고 리스트 뷰로 보내줌 (옵션)
      
    } catch (error) {
      setLog(prev => prev + `\n❌ 1단계 실패: ${String(error)}`);
      setStatus("error");
    }
  };

  const handleBuildGraph = async () => {
    try {
      setStatus("loading");
      setLog(prev => prev + `\n🕸️ [Step 2] 지식 그래프 생성 시작... (시간이 걸릴 수 있습니다)`);
      
      const result = await invoke<string>("construct_graph"); // 인자 없음 (DB 전체 스캔)

      setLog(prev => prev + `\n✅ 2단계 완료: ${result}`);
      setStatus("success");
      setRefreshGraph(prev => prev + 1); // 그래프 뷰 갱신
    } catch (error) {
      console.error(error);
      setLog(prev => prev + `\n❌ 2단계 실패: ${String(error)}`);
      setStatus("error");
    }
  };

  const ControlPanelSection = () => (
    <div style={{ 
      display: "flex", 
      gap: "10px", 
      padding: "10px 15px", 
      backgroundColor: "#181825", // 더 진한 배경으로 헤더 느낌
      borderBottom: "1px solid #313244",
      alignItems: "stretch", // 높이 통일
      height: "80px", // 고정 높이 (작게)
      flexShrink: 0 // 리스트 스크롤 시 줄어들지 않도록 고정
    }}>
      
      {/* 1. 좌측: 설정 및 파일 선택 (수직 스택으로 좁게 배치) */}
      <div style={{ display: "flex", flexDirection: "column", justifyContent: "space-between", width: "240px" }}>
        {/* GPU 토글 (작게) */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", backgroundColor: "#313244", padding: "4px 10px", borderRadius: "6px" }}>
          <span style={{ fontSize: "0.75rem", color: "#cdd6f4", fontWeight: "bold" }}>⚡ HW Accel</span>
          <button 
            onClick={handleToggleGpu}
            style={{ 
              fontSize: "0.7rem", padding: "2px 8px", borderRadius: "4px", border: "none", cursor: "pointer", 
              backgroundColor: useGpu ? "#a6e3a1" : "#45475a", color: useGpu ? "#1e1e2e" : "#bac2de", fontWeight: "bold"
            }}
          >
            {useGpu ? "ON" : "OFF"}
          </button>
        </div>

        {/* 파일 선택 버튼 (Input 스타일) */}
        <button 
          onClick={handleSelectFolder} 
          title={selectedPath || "폴더 선택"}
          style={{ 
            width: "100%", padding: "6px 10px", borderRadius: "6px", border: "1px solid #45475a", 
            backgroundColor: "#313244", color: selectedPath ? "#a6e3a1" : "#cdd6f4", 
            cursor: "pointer", textAlign: "left", textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap", fontSize: "0.8rem" 
          }}
        >
          {selectedPath ? `📂 ...${selectedPath.slice(-20)}` : "📂 PDF 폴더 선택"}
        </button>
      </div>

      {/* 2. 중앙: 액션 버튼 (가로 배치) */}
      <div style={{ display: "flex", gap: "8px" }}>
        <button 
          onClick={handleIngestDocs} 
          disabled={!selectedPath || status === "loading"} 
          style={{ 
            width: "100px", borderRadius: "8px", border: "none", 
            backgroundColor: (!selectedPath || status === "loading") ? "#45475a" : "#fab387", 
            color: "#1e1e2e", fontWeight: "bold", cursor: "pointer", 
            display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "2px"
          }}
        >
          <span style={{ fontSize: "1.2rem" }}>📥</span>
          <span style={{ fontSize: "0.75rem" }}>Step 1</span>
        </button>

        <button 
          onClick={handleBuildGraph} 
          disabled={status === "loading"} 
          style={{ 
            width: "100px", borderRadius: "8px", border: "none", 
            backgroundColor: (status === "loading") ? "#45475a" : "#89b4fa", 
            color: "#1e1e2e", fontWeight: "bold", cursor: "pointer", 
            display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "2px"
          }}
        >
          <span style={{ fontSize: "1.2rem" }}>🕸️</span>
          <span style={{ fontSize: "0.75rem" }}>Step 2</span>
        </button>
      </div>

      {/* 3. 우측: 로그 콘솔 (남는 공간 전부 차지) */}
      <div style={{ 
        flex: 1, backgroundColor: "#11111b", padding: "8px", borderRadius: "6px", 
        border: "1px solid #313244", overflowY: "auto", fontFamily: "monospace", 
        fontSize: "0.7rem", color: "#a6adc8", whiteSpace: "pre-wrap"
      }}>
        {log || "Process Log Ready..."}
      </div>
    </div>
  );

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
               /* 노드 상세 팝업 (기존 유지) */
               <div style={{ position: "absolute", bottom: "20px", left: "20px", width: "300px", backgroundColor: "rgba(30, 30, 46, 0.95)", backdropFilter: "blur(10px)", borderRadius: "12px", border: "1px solid #89b4fa", padding: "15px", color: "#cdd6f4", zIndex: 40 }}>
                <h4 style={{ margin: "0 0 10px 0", color: "#f9e2af" }}>{selectedNode.label}</h4>
                <p style={{ fontSize: "0.85rem", color: "#a6adc8" }}>{selectedNode.info}</p>
              </div>
            )}
          </>
        ) : (
          /* 📜 List Mode: 실제 데이터 연동됨 */
          <div style={{ display: "flex", flexDirection: "column", height: "100%", boxSizing: "border-box" }}>
            <ControlPanelSection />
            <div style={{ flex: 1, backgroundColor: "#11111b", display: "flex", flexDirection: "column", overflow: "hidden" }}>
              <div style={{ flex: 1, backgroundColor: "#11111b", borderRadius: "12px", border: "1px solid #313244", display: "flex", flexDirection: "column", overflow: "hidden" }}>
                <div style={{ padding: "15px 20px", borderBottom: "1px solid #313244", color: "#f9e2af", fontWeight: "bold", display: "flex", justifyContent: "space-between" }}>
                  <span>📜 Knowledge List ({documents.length})</span>
                  <button onClick={fetchDocuments} style={{ background: "none", border: "none", cursor: "pointer", fontSize: "1.2rem" }}>🔄</button>
                </div>
                <div style={{ flex: 1, overflowY: "auto", padding: "20px" }}>
                  {/* 데이터 렌더링 */}
                  {documents.length === 0 ? (
                    <div style={{ color: "#585b70", textAlign: "center", marginTop: "50px" }}>
                      아직 저장된 문서가 없습니다. <br /> 상단에서 PDF를 선택하고 Step 1을 실행해주세요.
                    </div>
                  ) : (
                    documents.map((doc, i) => (
                      <DocumentItem key={i} doc={doc} />
                    ))
                  )}
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}