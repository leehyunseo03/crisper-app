import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css"; // 필요 시 유지

// root 요소를 찾고, 없을 경우를 대비해 타입 단언(as HTMLElement) 또는 조건문을 사용합니다.
const rootElement = document.getElementById("root");

if (rootElement) {
  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
} else {
  console.error("루트 요소를 찾을 수 없습니다. index.html에 id='root'인 div가 있는지 확인하세요.");
}