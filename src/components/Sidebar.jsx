import React from "react";

export default function Sidebar({ page, setPage, onNew }) {
  return <aside className="sidebar">
    <div className="brand"><span className="brand-mark">最</span><strong>最强软件2</strong></div>
    <button className="new-chat" onClick={onNew}>＋ 新建会话</button>
    <nav>
      <button className={page === "chat" ? "active" : ""} onClick={() => setPage("chat")}>对话</button>
      <button className={page === "prompt" ? "active" : ""} onClick={() => setPage("prompt")}>提示词组合</button>
      <button className={page === "settings" ? "active" : ""} onClick={() => setPage("settings")}>模型设置</button>
      <button className={page === "diagnostics" ? "active" : ""} onClick={() => setPage("diagnostics")}>诊断信息</button>
    </nav>
    <div className="sidebar-foot">Tauri 2 · Rust · React</div>
  </aside>;
}
