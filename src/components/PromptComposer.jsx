import React, { useEffect, useState } from "react";
import { invoke } from "../tauri";

export default function PromptComposer({ systemPrompt, setSystemPrompt }) {
  const [categories, setCategories] = useState([]); const [thinking, setThinking] = useState([]); const [outputs, setOutputs] = useState([]);
  const [category, setCategory] = useState("image"); const [item, setItem] = useState("midjourney"); const [thinkingId, setThinkingId] = useState("thinking"); const [outputId, setOutputId] = useState("midjourney_prompt");
  useEffect(() => { Promise.all([invoke("get_prompt_categories_safe"), invoke("get_thinking_methods_safe"), invoke("get_output_formats_safe")]).then(([c,t,o]) => { setCategories(c); setThinking(t); setOutputs(o); }).catch(() => {}); }, []);
  const current = categories.find((c) => c.id === category); const items = current?.items || [];
  useEffect(() => { if (items[0] && !items.some((i) => i.id === item)) setItem(items[0].id); }, [category, categories]);
  async function compose() { try { const value = await invoke("compose_system_prompt", { categoryId: category, itemId: item, thinkingId, outputId }); setSystemPrompt(value); } catch (e) { setSystemPrompt(String(e)); } }
  return <section className="tool-page"><header className="page-header"><div><h1>提示词组合</h1><span className="muted">选择模板，生成可复用的系统提示词</span></div></header><div className="form-grid"><label>提示词类别<select value={category} onChange={(e) => setCategory(e.target.value)}>{categories.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select></label><label>提示词条目<select value={item} onChange={(e) => setItem(e.target.value)}>{items.map((i) => <option key={i.id} value={i.id}>{i.name || i.id}</option>)}</select></label><label>思考方法<select value={thinkingId} onChange={(e) => setThinkingId(e.target.value)}>{thinking.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}</select></label><label>输出格式<select value={outputId} onChange={(e) => setOutputId(e.target.value)}>{outputs.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}</select></label></div><button className="primary" onClick={compose}>组合提示词</button><textarea className="preview" value={systemPrompt} onChange={(e) => setSystemPrompt(e.target.value)} placeholder="组合结果会显示在这里" /></section>;
}
