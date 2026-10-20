import React, { useEffect, useMemo, useState } from "react";
import { invoke } from "./tauri";
import Sidebar from "./components/Sidebar";
import ChatView from "./components/ChatView";
import PromptComposer from "./components/PromptComposer";
import SettingsPanel from "./components/SettingsPanel";
import DiagnosticsPanel from "./components/DiagnosticsPanel";

const defaultProvider = {
  id: "goaichat",
  provider: "Goaichat",
  baseUrl: "https://llm.goaichat.top/v1",
  apiKey: "",
  models: [
    { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", modelId: "deepseek-v4-pro", enabled: true },
    { id: "deepseek-v4-pro-0813", name: "DeepSeek V4 Pro (0813)", modelId: "deepseek-v4-pro-0813", enabled: true },
    { id: "glm-5.3", name: "GLM 5.3", modelId: "glm-5.3", enabled: true },
    { id: "kimi-k3", name: "Kimi K3", modelId: "kimi-k3", enabled: true },
  ],
};

function readProviders() {
  try {
    const parsed = JSON.parse(localStorage.getItem("provider-configs") || "[]");
    return Array.isArray(parsed) && parsed.length ? parsed : [defaultProvider];
  } catch {
    return [defaultProvider];
  }
}

export default function App() {
  const [page, setPage] = useState("chat");
  const [providers, setProviders] = useState(readProviders);
  const [selectedModel, setSelectedModel] = useState(
    () => localStorage.getItem("selected-model") || "deepseek-v4-pro"
  );
  const [messages, setMessages] = useState([]);
  const [prompt, setPrompt] = useState("");
  const [systemPrompt, setSystemPrompt] = useState("");
  const [conversationId, setConversationId] = useState(null);
  const [error, setError] = useState("");

  const provider = providers[0] || defaultProvider;
  const model = useMemo(
    () => provider.models.find((m) => m.modelId === selectedModel) || provider.models[0],
    [provider, selectedModel]
  );

  useEffect(() => {
    localStorage.setItem("provider-configs", JSON.stringify(providers));
  }, [providers]);
  useEffect(() => {
    localStorage.setItem("selected-model", selectedModel);
  }, [selectedModel]);
  useEffect(() => {
    const onDelta = (event) => setMessages((items) => items.map((m, i) => i === items.length - 1 ? { ...m, content: (m.content || "") + event.detail } : m));
    const onEnd = () => setMessages((items) => items.map((m, i) => i === items.length - 1 ? { ...m, streaming: false } : m));
    window.addEventListener("anime-chat-delta", onDelta);
    window.addEventListener("anime-chat-end", onEnd);
    return () => { window.removeEventListener("anime-chat-delta", onDelta); window.removeEventListener("anime-chat-end", onEnd); };
  }, []);

  async function newConversation() {
    try {
      const id = await invoke("create_conversation", { title: "新会话", model: selectedModel });
      setConversationId(id);
      setMessages([]);
      setPage("chat");
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }

  async function sendMessage(text) {
    const value = String(text || prompt).trim();
    if (!value) return;
    const history = [...messages, { role: "user", content: value }];
    setMessages([...history, { role: "assistant", content: "", streaming: true }]);
    setPrompt("");
    setError("");
    try {
      if (conversationId) {
        await invoke("save_message", {
          conversationId,
          role: "user",
          content: value,
          model: selectedModel,
          mediaType: null,
          mediaUrl: null,
          mediaMetadata: null,
        });
      }
      await invoke("stream_chat_raw", {
        request: {
          messages: history,
          apiKey: provider.apiKey || null,
          baseUrl: provider.baseUrl,
          model: selectedModel,
          temperature: 0.7,
          systemPrompt: systemPrompt || null,
          maxTokens: 32768,
          reasoningEffort: "high",
        },
      });
    } catch (e) {
      const message = formatError(e);
      setError(message);
      setMessages((items) =>
        items.map((m, i) =>
          i === items.length - 1 ? { ...m, content: message, streaming: false, error: true } : m
        )
      );
    }
  }

  function formatError(value) {
    const text = String(value || "未知错误");
    if (/401|unauthorized|invalid.*key/i.test(text)) return "认证失败：请检查 API Key。";
    if (/403|forbidden/i.test(text)) return "访问被拒绝：当前 Key 或模型没有权限。";
    if (/429|rate.?limit/i.test(text)) return "请求过于频繁：请稍后重试。";
    if (/timeout|timed out/i.test(text)) return "请求超时：请检查网络或代理。";
    if (/connect|dns|network/i.test(text)) return "网络连接失败：请确认 API 地址可访问。";
    return text;
  }

  return (
    <div className="app-shell">
      <Sidebar page={page} setPage={setPage} onNew={newConversation} />
      <main className="main-area">
        {page === "chat" && (
          <ChatView
            messages={messages}
            prompt={prompt}
            setPrompt={setPrompt}
            onSend={sendMessage}
            modelName={model?.name || selectedModel}
            error={error}
          />
        )}
        {page === "prompt" && (
          <PromptComposer
            systemPrompt={systemPrompt}
            setSystemPrompt={setSystemPrompt}
          />
        )}
        {page === "settings" && (
          <SettingsPanel
            providers={providers}
            setProviders={setProviders}
            selectedModel={selectedModel}
            setSelectedModel={setSelectedModel}
          />
        )}
        {page === "diagnostics" && <DiagnosticsPanel />}
      </main>
    </div>
  );
}
