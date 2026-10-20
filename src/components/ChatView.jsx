import React, { useEffect, useRef } from "react";
import { listen } from "../tauri";

export default function ChatView({ messages, prompt, setPrompt, onSend, modelName, error }) {
  const endRef = useRef(null);
  useEffect(() => {
    const unsubs = [];
    listen("chat-raw-chunk", (event) => {
      try {
        const raw = String(event.payload || "").replace(/^data:\s*/, "");
        const json = JSON.parse(raw); const delta = json?.choices?.[0]?.delta || {};
        const text = delta.content || delta.reasoning_content || "";
        if (text) window.dispatchEvent(new CustomEvent("anime-chat-delta", { detail: text }));
      } catch (_) {}
    }).then((u) => unsubs.push(u));
    listen("chat-raw-end", () => window.dispatchEvent(new CustomEvent("anime-chat-end"))).then((u) => unsubs.push(u));
    return () => unsubs.forEach((u) => u());
  }, []);
  useEffect(() => endRef.current?.scrollIntoView({ behavior: "smooth" }), [messages]);
  useEffect(() => {
    const onDelta = (event) => {
      window.__animeDelta = (window.__animeDelta || "") + event.detail;
    };
    window.addEventListener("anime-chat-delta", onDelta);
    return () => window.removeEventListener("anime-chat-delta", onDelta);
  }, []);
  return <section className="chat-page">
    <header className="page-header"><div><h1>对话</h1><span className="muted">{modelName}</span></div><span className="status-dot">在线</span></header>
    <div className="message-list">
      {!messages.length && <div className="empty-state"><div className="empty-logo">最</div><h2>开始一次新的创作</h2><p>描述你的想法，或使用提示词组合器获得更稳定的输出。</p></div>}
      {messages.map((m, i) => <div key={i} className={`message ${m.role} ${m.error ? "message-error" : ""}`}><div className="message-role">{m.role === "user" ? "你" : "助手"}</div><div className="message-content">{m.content || (m.streaming ? "正在思考…" : "")}</div></div>)}
      <div ref={endRef} />
    </div>
    {error && <div className="error-banner">{error}</div>}
    <form className="composer" onSubmit={(e) => { e.preventDefault(); onSend(); }}><textarea value={prompt} onChange={(e) => setPrompt(e.target.value)} placeholder="输入消息…" onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); onSend(); } }} /><button type="submit">发送</button></form>
  </section>;
}
