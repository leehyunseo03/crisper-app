// src/components/ModelStore.tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HFModel {
  id: string;
  downloads: number;
  likes: number;
  size?: number; // 허깅페이스 API에서 제공하는 바이트 단위 용량
}

const ModelStore = () => {
  const [models, setModels] = useState<HFModel[]>([]);
  const [loading, setLoading] = useState(true);
  const [downloading, setDownloading] = useState<string | null>(null);

  // 바이트 단위를 읽기 좋은 단위로 변환하는 함수
  const formatBytes = (bytes?: number) => {
    if (!bytes || bytes === 0) return "용량 정보 없음";
    const k = 1024;
    const sizes = ["Bytes", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  useEffect(() => {
    // GGUF 모델 검색 시 용량 정보를 포함하기 위해 정렬 및 필터링
    fetch("https://huggingface.co/api/models?search=gguf&sort=downloads&direction=-1&limit=12")
      .then(res => res.json())
      .then(data => {
        setModels(data);
        setLoading(false);
      });
  }, []);

  const handleDownload = async (modelId: string) => {
    setDownloading(modelId);
    const filename = `${modelId.split("/")[1]}.gguf`;
    const downloadUrl = `https://huggingface.co/${modelId}/resolve/main/${filename}`;

    try {
      await invoke("download_model", { url: downloadUrl, filename });
      alert("다운로드 완료!");
    } catch (e) {
      alert("다운로드 실패: " + e);
    } finally {
      setDownloading(null);
    }
  };

  return (
    <div style={{ padding: "30px", backgroundColor: "#f0f2f5", minHeight: "100%" }}>
      <header style={{ marginBottom: "30px" }}>
        <h2 style={{ margin: 0, color: "#1e1e2e" }}>📥 모델 스토어</h2>
        <p style={{ color: "#666" }}>허깅페이스의 인기 GGUF 모델을 확인하고 내 PC에 설치하세요.</p>
      </header>

      {loading ? (
        <div style={{ textAlign: "center", padding: "50px" }}>모델 목록을 불러오는 중...</div>
      ) : (
        <div style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))",
          gap: "20px"
        }}>
          {models.map(model => (
            <div key={model.id} style={{
              backgroundColor: "white",
              padding: "24px",
              borderRadius: "16px",
              boxShadow: "0 4px 12px rgba(0,0,0,0.05)",
              display: "flex",
              flexDirection: "column",
              transition: "transform 0.2s",
              border: "1px solid #eef0f2"
            }}>
              <div style={{ flex: 1 }}>
                <h4 style={{ margin: "0 0 8px 0", color: "#333", wordBreak: "break-all", fontSize: "1.1rem" }}>
                  {model.id.split("/")[1]}
                </h4>
                <p style={{ fontSize: "0.85rem", color: "#888", marginBottom: "16px" }}>{model.id}</p>
                
                {/* 모델 정보 태그 영역 */}
                <div style={{ display: "flex", gap: "10px", marginBottom: "20px", flexWrap: "wrap" }}>
                  <span style={{ backgroundColor: "#f1f3f9", padding: "4px 10px", borderRadius: "20px", fontSize: "0.8rem", color: "#555" }}>
                    ⚖️ {formatBytes(model.size)}
                  </span>
                  <span style={{ backgroundColor: "#f1f3f9", padding: "4px 10px", borderRadius: "20px", fontSize: "0.8rem", color: "#555" }}>
                    📥 {model.downloads > 1000 ? (model.downloads / 1000).toFixed(1) + "k" : model.downloads}
                  </span>
                </div>
              </div>
              
              <button
                onClick={() => handleDownload(model.id)}
                disabled={downloading === model.id}
                style={{
                  width: "100%",
                  padding: "12px",
                  backgroundColor: downloading === model.id ? "#ccc" : "#89b4fa",
                  color: "white",
                  border: "none",
                  borderRadius: "8px",
                  cursor: downloading === model.id ? "not-allowed" : "pointer",
                  fontWeight: "bold",
                  fontSize: "0.95rem",
                  transition: "background-color 0.2s"
                }}
              >
                {downloading === model.id ? "설치 중..." : "모델 설치"}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default ModelStore;