// src/App.tsx
import { useState } from "react";
import ChatRoom from "./components/ChatRoom";
import ModelStore from "./components/ModelStore";
import Genifier from "./components/Genifier";

type Menu = "채팅" | "디지털 유전자" | "프로젝트 탐색" | "모델 다운로드";

function App() {
  // 1. 기본 메뉴를 '디지털 유전자'로 설정하여 앱 실행 시 바로 그래프가 보이게 함
  const [activeMenu, setActiveMenu] = useState<Menu>("디지털 유전자");

  const menuItems = [
    { name: "채팅", icon: "💬" },
    { name: "디지털 유전자", icon: "🧬" },
    { name: "프로젝트 탐색", icon: "🌐" },
    { name: "모델 다운로드", icon: "📥" },
  ];

  return (
    <div style={{ display: "flex", height: "100vh", width: "100vw", backgroundColor: "#1e1e2e" }}>
      {/* --- 사이드바 --- */}
      <nav style={{
        width: "260px",
        backgroundColor: "#11111b", // 더 어두운 톤으로 변경
        color: "white",
        display: "flex",
        flexDirection: "column",
        padding: "20px 0",
        borderRight: "1px solid #313244",
        zIndex: 20 // 그래프보다 위에 오도록
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
              gap: "15px",
              color: activeMenu === item.name ? "#cdd6f4" : "#a6adc8"
            }}
          >
            <span>{item.icon}</span>
            <span style={{ fontSize: "1rem" }}>{item.name}</span>
          </div>
        ))}
      </nav>

      {/* --- 메인 컨텐츠 영역 --- */}
      {/* padding을 제거하고 relative로 설정하여 내부 컴포넌트가 꽉 차게 함 */}
      <main style={{ flex: 1, position: "relative", overflow: "hidden", display: "flex", flexDirection: "column" }}>
        {activeMenu === "채팅" && <ChatRoom />}
        
        {/* Genifier는 이제 자체적으로 전체 화면을 씁니다 */}
        {activeMenu === "디지털 유전자" && <Genifier />}

        {activeMenu === "프로젝트 탐색" && (
          <div style={{ padding: "40px", textAlign: "center", color: "#cdd6f4" }}>
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