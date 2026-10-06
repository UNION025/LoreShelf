import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./App.css";

interface LoreSummary {
  path: string;
  id: string;
  date: string | null;
  source: string | null;
  topic: string | null;
  tags: string[];
  kind: string | null;
  value: string | null;
  trails: string[];
  error: string | null;
  issues: Issue[];
  accepted: boolean;
}

interface Issue {
  line: number;
  severity: "error" | "warning";
  message: string;
}

const LIBRARY_KEY = "loreshelf.libraryDir";
const IMPORTED_KEY = "loreshelf.importedFiles";

function loadSavedDir(): string | null {
  try {
    return localStorage.getItem(LIBRARY_KEY);
  } catch {
    return null;
  }
}

function saveDir(dir: string) {
  try {
    localStorage.setItem(LIBRARY_KEY, dir);
  } catch {
    // Remembering the folder is a convenience; ignore storage failures.
  }
}

function loadImported(): string[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(IMPORTED_KEY) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((p) => typeof p === "string") : [];
  } catch {
    return [];
  }
}

function saveImported(paths: string[]) {
  try {
    localStorage.setItem(IMPORTED_KEY, JSON.stringify(paths));
  } catch {
    // Convenience only; ignore storage failures.
  }
}

function relativePath(dir: string, path: string): string {
  const rel = path.startsWith(dir) ? path.slice(dir.length) : path;
  return rel.replace(/^[\\/]+/, "").replace(/[\\/]LORE\.md$/, "");
}

function IssueList({ issues }: { issues: Issue[] }) {
  if (issues.length === 0) return null;
  return (
    <ul className="issues">
      {issues.map((issue, i) => (
        <li key={i} className={`issue ${issue.severity}`}>
          <span className="issue-line">{issue.line}行目</span>
          {issue.message}
        </li>
      ))}
    </ul>
  );
}

function LoreDetail({ item, onBack }: { item: LoreSummary; onBack: () => void }) {
  const [body, setBody] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setBody(null);
    setError(null);
    invoke<string>("read_lore", { path: item.path })
      .then((text) => !cancelled && setBody(text))
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, [item.path]);

  return (
    <>
      <header className="toolbar">
        <button onClick={onBack}>← 一覧に戻る</button>
      </header>
      <h2 className="detail-title">{item.topic ?? item.id}</h2>
      <div className="lore-head">
        <span className="lore-date">{item.date ?? "日付なし"}</span>
        {item.kind && <span className="badge">{item.kind}</span>}
        {item.value && <span className="badge">価値: {item.value}</span>}
      </div>
      {item.issues.length > 0 && (
        <details className="issue-panel">
          <summary>LoreSpec の警告 {item.issues.length} 件</summary>
          <IssueList issues={item.issues} />
        </details>
      )}
      {error && <p className="error">{error}</p>}
      {body === null && !error && <p className="count">読み込み中…</p>}
      {body !== null && (
        <article className="markdown">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>
        </article>
      )}
    </>
  );
}

function LoreCard({
  item,
  label,
  onOpen,
}: {
  item: LoreSummary;
  label: string;
  onOpen: () => void;
}) {
  return (
    <li className="lore-item clickable" onClick={onOpen}>
      <div className="lore-head">
        <span className="lore-date">{item.date ?? "日付なし"}</span>
        {item.kind && <span className="badge">{item.kind}</span>}
        {item.value && <span className="badge">価値: {item.value}</span>}
        {item.issues.length > 0 && <span className="badge warn">警告 {item.issues.length}</span>}
      </div>
      <div className="lore-topic">{item.topic ?? item.id}</div>
      <div className="lore-path">{label}</div>
      <div className="tags">
        {item.tags.map((t) => (
          <span key={t} className="tag">
            #{t}
          </span>
        ))}
      </div>
    </li>
  );
}

function RejectedCard({ item, label }: { item: LoreSummary; label: string }) {
  return (
    <li className="lore-item rejected-item">
      <div className="lore-path">{label}</div>
      {item.error && <div className="error">{item.error}</div>}
      <IssueList issues={item.issues.filter((i) => i.severity === "error")} />
    </li>
  );
}

function App() {
  const [dir, setDir] = useState<string | null>(loadSavedDir);
  const [folderItems, setFolderItems] = useState<LoreSummary[]>([]);
  const [importedPaths, setImportedPaths] = useState<string[]>(loadImported);
  const [importedItems, setImportedItems] = useState<LoreSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<LoreSummary | null>(null);

  const scan = useCallback(async (target: string) => {
    try {
      setFolderItems(await invoke<LoreSummary[]>("scan_library", { dir: target }));
      setError(null);
    } catch (e) {
      setFolderItems([]);
      setError(String(e));
    }
  }, []);

  const inspect = useCallback(async (paths: string[]) => {
    try {
      setImportedItems(
        paths.length > 0 ? await invoke<LoreSummary[]>("inspect_files", { paths }) : [],
      );
    } catch (e) {
      setImportedItems([]);
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    if (dir) scan(dir);
  }, [dir, scan]);

  useEffect(() => {
    inspect(importedPaths);
  }, [importedPaths, inspect]);

  async function chooseFolder() {
    const chosen = await open({ directory: true, title: "Lore フォルダを選択" });
    if (typeof chosen === "string") {
      saveDir(chosen);
      setSelected(null);
      setDir(chosen);
    }
  }

  async function importFiles() {
    const chosen = await open({
      multiple: true,
      title: "Lore ファイルをインポート",
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (!chosen) return;
    const paths = Array.isArray(chosen) ? chosen : [chosen];
    const next = Array.from(new Set([...importedPaths, ...paths]));
    saveImported(next);
    setImportedPaths(next);
  }

  function clearImported() {
    saveImported([]);
    setImportedPaths([]);
    setSelected(null);
  }

  function reload() {
    setSelected(null);
    if (dir) scan(dir);
    inspect(importedPaths);
  }

  if (selected) {
    return (
      <main className="container">
        <LoreDetail item={selected} onBack={() => setSelected(null)} />
      </main>
    );
  }

  // Files found in the folder are listed first; hand-imported ones are marked.
  const entries = [
    ...folderItems.map((item) => ({ item, label: relativePath(dir ?? "", item.path) })),
    ...importedItems
      .filter((item) => !folderItems.some((f) => f.path === item.path))
      .map((item) => ({ item, label: `インポート: ${item.path}` })),
  ];
  const accepted = entries.filter((e) => e.item.accepted);
  const rejected = entries.filter((e) => !e.item.accepted);

  return (
    <main className="container">
      <header className="toolbar">
        <h1>LoreShelf</h1>
        <button onClick={chooseFolder}>フォルダを選択</button>
        <button onClick={importFiles}>ファイルをインポート</button>
        {(dir || importedPaths.length > 0) && <button onClick={reload}>再読み込み</button>}
      </header>

      {dir && <p className="dir">{dir}</p>}
      {importedPaths.length > 0 && (
        <p className="dir">
          個別インポート {importedPaths.length} 件{" "}
          <button className="link" onClick={clearImported}>
            すべて解除
          </button>
        </p>
      )}
      {error && <p className="error">{error}</p>}

      {!dir && importedPaths.length === 0 ? (
        <p className="empty">Lore のフォルダを選択するか、ファイルをインポートしてください。</p>
      ) : (
        <>
          <p className="count">
            {accepted.length} 件
            {rejected.length > 0 && ` ・ 読み込み拒否 ${rejected.length} 件`}
          </p>
          {rejected.length > 0 && (
            <section className="rejected">
              <h2>読み込みを拒否したファイル(LoreSpec 違反)</h2>
              <ul className="lore-list">
                {rejected.map(({ item, label }) => (
                  <RejectedCard key={item.path} item={item} label={label} />
                ))}
              </ul>
            </section>
          )}
          <ul className="lore-list">
            {accepted.map(({ item, label }) => (
              <LoreCard key={item.path} item={item} label={label} onOpen={() => setSelected(item)} />
            ))}
          </ul>
        </>
      )}
    </main>
  );
}

export default App;
