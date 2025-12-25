//src/components/ChatRoom.tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core"; // Rust 호출용

const ChatRoom = () => {
  const [input, setInput] = useState("");
  const [chat, setChat] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const askAI = async () => {
    if (!input.trim()) return;
    setIsLoading(true);
    setChat("");
    
    try {
      // 1. Rust에게 벡터 검색 요청 (관련 문서 가져오기)
      let context = "";
      try {
        setChat("🧬 지식 베이스 검색 중...");
        context = await invoke<string>("search_docs", { query: input });
        console.log("🔍 [RAG 검색 결과]:\n", context);
      } catch (e) {
        console.error("검색 실패:", e);
        // 검색 실패해도 대화는 가능하게 빈 문자열 처리
      }

      // 2. 프롬프트 구성 (Context + Question)
      // 시스템 프롬프트 느낌으로 구성합니다.
      const augmentedPrompt = `
        당신은 사용자가 제공한 문서를 기반으로 답변하는 AI 비서입니다.
        아래의 [참고 문서]를 바탕으로 질문에 대해 명확하고 정확하게 답변하세요.
        만약 참고 문서에 내용이 없다면, 일반적인 지식으로 답변하되 문서에 없다고 언급해주세요.

        [참고 문서]
        ${context}

        [질문]
        ${input}
      `.trim();

      // UI에는 검색 완료 메시지 잠깐 표시 후 답변 시작
      setChat("🤔 답변 생성 중...");

      // 3. 로컬 LLM에게 전송
      const res = await fetch("http://localhost:8080/v1/chat/completions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: "ggml-model-Q4_K_M", // 실행시킬 때 쓴 모델명과 일치하지 않아도 llama.cpp는 보통 동작함
          messages: [
            { role: "system", content: "당신은 도움이 되는 AI 어시스턴트입니다." },
            { role: "user", content: augmentedPrompt } 
          ],
          stream: true,
        }),
      });

      if (!res.ok || !res.body) throw new Error("서버 에러");

      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let fullResponse = "";

      // 4. 스트리밍 응답 처리
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split("\n");
        for (const line of lines) {
          if (line.startsWith("data: ") && line !== "data: [DONE]") {
            try {
              const data = JSON.parse(line.replace("data: ", ""));
              const content = data.choices[0]?.delta?.content || "";
              fullResponse += content;
              setChat(fullResponse); // 화면 갱신
            } catch (e) {}
          }
        }
      }
    } catch (error) {
      setChat("오류가 발생했습니다: " + String(error));
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "30px" }}>
      <header style={{ marginBottom: "20px" }}>
        <h2 style={{ margin: 0 }}>💬 AI 어시스턴트 (RAG)</h2>
        <p style={{ color: "#666" }}>PDF 문서 내용을 바탕으로 답변합니다.</p>
      </header>

      {/* 답변창 */}
      <div style={{
        flex: 1,
        backgroundColor: "white",
        borderRadius: "12px",
        padding: "25px",
        boxShadow: "0 2px 10px rgba(0,0,0,0.05)",
        overflowY: "auto",
        whiteSpace: "pre-wrap",
        lineHeight: "1.7",
        fontSize: "1.1rem"
      }}>
        {chat || <span style={{ color: "#aaa" }}>질문을 입력하세요. (예: "무어의 법칙이 뭐야?")</span>}
      </div>

      {/* 입력창 */}
      <div style={{ marginTop: "20px", display: "flex", gap: "10px" }}>
        <input
          style={{
            flex: 1,
            padding: "15px",
            borderRadius: "8px",
            border: "1px solid #ddd",
            outline: "none"
          }}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !isLoading && askAI()}
          placeholder="메시지를 입력하세요..."
        />
        <button
          onClick={askAI}
          disabled={isLoading}
          style={{
            padding: "0 25px",
            backgroundColor: isLoading ? "#ccc" : "#89b4fa",
            color: "white",
            border: "none",
            borderRadius: "8px",
            cursor: isLoading ? "not-allowed" : "pointer",
            fontWeight: "bold"
          }}
        >
          {isLoading ? "..." : "전송"}
        </button>
      </div>
    </div>
  );
};

export default ChatRoom;