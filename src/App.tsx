import { useState } from "react";
import ChatRoom from "./components/ChatRoom"; // 채팅 컴포넌트 분리
import ModelStore from "./components/ModelStore";

// 메뉴 타입 정의
type Menu = "채팅" | "디지털 유전자" | "프로젝트 탐색" | "모델 다운로드";

function App() {
  const [activeMenu, setActiveMenu] = useState<Menu>("채팅");

  const menuItems = [
    { name: "채팅", icon: "💬" },
    { name: "디지털 유전자", icon: "🧬" },
    { name: "프로젝트 탐색", icon: "🌐" },
    { name: "모델 다운로드", icon: "📥" },
  ];

  return (
    <div style={{ display: "flex", height: "100vh", width: "100vw", backgroundColor: "#f0f2f5" }}>
      {/* --- 사이드바 --- */}
      <nav style={{
        width: "260px",
        backgroundColor: "#1e1e2e",
        color: "white",
        display: "flex",
        flexDirection: "column",
        padding: "20px 0"
      }}>
        <div style={{ padding: "0 20px 30px", fontSize: "1.5rem", fontWeight: "bold", color: "#89b4fa" }}>
          Crisper
        </div>
        
        {menuItems.map((item) => (
          <div
            key={item.name}
            onClick={() => setActiveMenu(item.name as Menu)}
            style={{
              padding: "15px 25px",
              cursor: "pointer",
              backgroundColor: activeMenu === item.name ? "#313244" : "transparent",
              borderLeft: activeMenu === item.name ? "4px solid #89b4fa" : "4px solid transparent",
              transition: "0.2s",
              display: "flex",
              alignItems: "center",
              gap: "15px"
            }}
          >
            <span>{item.icon}</span>
            <span style={{ fontSize: "1rem" }}>{item.name}</span>
          </div>
        ))}
      </nav>

      {/* --- 메인 컨텐츠 영역 --- */}
      <main style={{ flex: 1, position: "relative", overflowY: "auto", display: "flex", flexDirection: "column" }}>
        {activeMenu === "채팅" && <ChatRoom />}
        
        {activeMenu === "디지털 유전자" && (
          <div style={{ padding: "40px", textAlign: "center" }}>
            <h2>🧬 디지털 유전자 (Graph Index)</h2>
            <p>사용자 데이터를 분석하여 관계형 그래프를 생성합니다. (준비 중)</p>
          </div>
        )}

        {activeMenu === "프로젝트 탐색" && (
          <div style={{ padding: "40px", textAlign: "center" }}>
            <h2>🌐 프로젝트 탐색</h2>
            <p>프로젝트 공유기능 들어갈 예정(준비 중)</p>
          </div>
        )}

        {activeMenu === "모델 다운로드" && <ModelStore />}
      </main>
    </div>
  );
}

export default App;