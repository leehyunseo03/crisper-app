import { useState, KeyboardEvent } from "react";

const ChatRoom = () => {
  const [input, setInput] = useState("");
  const [chat, setChat] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const askAI = async () => {
    if (!input.trim()) return;
    setIsLoading(true);
    setChat("");
    let fullResponse = "";

    try {
      const res = await fetch("http://localhost:8080/v1/chat/completions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model: "ggml-model-Q4_K_M",
          messages: [{ role: "user", content: input }],
          stream: true,
        }),
      });

      if (!res.ok || !res.body) throw new Error("서버 에러");

      const reader = res.body.getReader();
      const decoder = new TextDecoder();

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split("\n");
        for (const line of lines) {
          if (line.startsWith("data: ") && line !== "data: [DONE]") {
            try {
              const data = JSON.parse(line.replace("data: ", ""));
              fullResponse += data.choices[0]?.delta?.content || "";
              setChat(fullResponse);
            } catch (e) {}
          }
        }
      }
    } catch (error) {
      setChat("오류가 발생했습니다.");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "30px" }}>
      <header style={{ marginBottom: "20px" }}>
        <h2 style={{ margin: 0 }}>💬 AI 어시스턴트</h2>
        <p style={{ color: "#666" }}>로컬 모델과 대화하며 나만의 지식 베이스를 구축하세요.</p>
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
        lineHeight: "1.7"
      }}>
        {chat || <span style={{ color: "#aaa" }}>질문을 입력하면 AI가 응답을 시작합니다...</span>}
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
            backgroundColor: "#89b4fa",
            color: "white",
            border: "none",
            borderRadius: "8px",
            cursor: "pointer",
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