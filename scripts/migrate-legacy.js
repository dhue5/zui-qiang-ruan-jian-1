import fs from 'fs';
import path from 'path';
import { DatabaseSync } from 'node:sqlite';

const usage = 'usage: node scripts/migrate-legacy.js <dump.json> [chats.db]';
const dumpPath = process.argv[2];
if (!dumpPath) { console.error(usage); process.exit(1); }
const dbPath = process.argv[3] || path.join(process.env.APPDATA || '.', 'com.zuiqiangruanjian.desktop', 'chats.db');

const dump = JSON.parse(fs.readFileSync(dumpPath, 'utf8'));

fs.mkdirSync(path.dirname(dbPath), { recursive: true });
const db = new DatabaseSync(dbPath);
db.exec(`CREATE TABLE IF NOT EXISTS conversations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  created_at TEXT,
  updated_at TEXT,
  model TEXT
);`);
db.exec(`CREATE TABLE IF NOT EXISTS messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  conversation_id INTEGER,
  role TEXT,
  content TEXT,
  created_at TEXT,
  model TEXT,
  deleted BOOLEAN DEFAULT 0,
  media_type TEXT,
  media_url TEXT,
  media_metadata TEXT
);`);
db.exec(`CREATE TABLE IF NOT EXISTS generation_tasks (
  id TEXT PRIMARY KEY,
  message_id TEXT,
  conversation_id TEXT,
  status TEXT,
  progress TEXT,
  error TEXT,
  timestamp INTEGER,
  completed_at INTEGER,
  model_id TEXT,
  model_provider TEXT,
  model_type TEXT,
  provider TEXT,
  request_data TEXT,
  result_url TEXT,
  result_data TEXT
);`);
db.exec(`CREATE TABLE IF NOT EXISTS canvases (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT,
  content TEXT,
  source_type TEXT,
  source_id INTEGER,
  created_at TEXT,
  updated_at TEXT,
  tags TEXT
);`);
db.exec(`CREATE TABLE IF NOT EXISTS notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT,
  content TEXT,
  source_type TEXT,
  source_id INTEGER,
  created_at TEXT,
  updated_at TEXT,
  tags TEXT
);`);

const insertConv = db.prepare('INSERT INTO conversations (title, created_at, updated_at, model) VALUES (?, ?, ?, ?)');
const insertMsg = db.prepare(`INSERT INTO messages
  (conversation_id, role, content, created_at, model, deleted, media_type, media_url, media_metadata)
  VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`);
const insertCanvas = db.prepare(`INSERT INTO canvases
  (title, content, source_type, source_id, created_at, updated_at, tags)
  VALUES (?, ?, ?, ?, ?, ?, ?)`);
const insertNote = db.prepare(`INSERT INTO notes
  (title, content, source_type, source_id, created_at, updated_at, tags)
  VALUES (?, ?, ?, ?, ?, ?, ?)`);
const insertTask = db.prepare(`INSERT INTO generation_tasks
  (id, message_id, conversation_id, status, progress, error, timestamp, completed_at,
   model_id, model_provider, model_type, provider, request_data, result_url, result_data)
  VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`);

const idMap = new Map();
for (const c of dump.conversations || []) {
  const oldId = c.id;
  const info = insertConv.run(c.title ?? '(migrated)', c.created_at ?? null, c.updated_at ?? c.created_at ?? null, c.model ?? null);
  idMap.set(oldId, Number(info.lastInsertRowid));
}
let m = 0;
for (const msg of dump.messages || []) {
  const convId = idMap.get(msg.conversation_id);
  if (convId === undefined) continue;
  insertMsg.run(
    convId, msg.role ?? 'user', msg.content ?? '', msg.created_at ?? null, msg.model ?? null,
    msg.deleted ? 1 : 0, msg.media_type ?? null, msg.media_url ?? null, msg.media_metadata ?? null
  );
  m++;
}
let c = 0;
for (const item of dump.canvases || []) {
  let sourceId = item.source_id != null ? Number(item.source_id) : null;
  if (item.source_type === 'conversation' && idMap.has(Number(item.source_id))) {
    sourceId = idMap.get(Number(item.source_id));
  }
  insertCanvas.run(item.title ?? '', item.content ?? '', item.source_type ?? null, sourceId, item.created_at ?? null, item.updated_at ?? item.created_at ?? null, item.tags ?? null);
  c++;
}
let n = 0;
for (const item of dump.notes || []) {
  let sourceId = item.source_id != null ? Number(item.source_id) : null;
  if (item.source_type === 'conversation' && idMap.has(Number(item.source_id))) {
    sourceId = idMap.get(Number(item.source_id));
  }
  insertNote.run(item.title ?? '', item.content ?? '', item.source_type ?? null, sourceId, item.created_at ?? null, item.updated_at ?? item.created_at ?? null, item.tags ?? null);
  n++;
}
let t = 0;
for (const item of dump.generation_tasks || []) {
  insertTask.run(
    String(item.id), item.message_id ?? null, item.conversation_id != null ? String(item.conversation_id) : null,
    item.status ?? 'pending', item.progress ?? null, item.error ?? null,
    item.timestamp ?? null, item.completed_at ?? null,
    item.model_id ?? null, item.model_provider ?? null, item.model_type ?? null,
    item.provider ?? null, item.request_data ? JSON.stringify(item.request_data) : null,
    item.result_url ?? null, item.result_data ? JSON.stringify(item.result_data) : null
  );
  t++;
}
console.log(`migrated conversations=${idMap.size} messages=${m} canvases=${c} notes=${n} tasks=${t} -> ${dbPath}`);
db.close();
