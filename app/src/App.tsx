import { useCallback, useEffect, useMemo, useState } from "react";
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
  readable: boolean;
}

interface Hit {
  path: string;
  id: string;
  topic: string;
  snippet: string;
}

interface IndexReport {
  indexed: number;
  skipped: number;
  db_path: string;
}

interface Issue {
  line: number;
  severity: "error" | "warning";
  message: string;
}

const LIBRARY_KEY = "loreshelf.libraryDir";
const IMPORTED_KEY = "loreshelf.importedFiles";
const ALLOWED_KEY = "loreshelf.allowedViolations";
const HIDDEN_KEY = "loreshelf.hiddenFiles";

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

/** Files the user chose to load despite errors: path -> the errors they accepted. */
function loadAllowed(): Record<string, string> {
  try {
    const parsed = JSON.parse(localStorage.getItem(ALLOWED_KEY) ?? "{}");
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

function saveAllowed(allowed: Record<string, string>) {
  try {
    localStorage.setItem(ALLOWED_KEY, JSON.stringify(allowed));
  } catch {
    // Convenience only; the question is simply asked again next time.
  }
}

/** Files the user chose to hide, typically the other versions of one session. */
function loadHidden(): Set<string> {
  try {
    const parsed = JSON.parse(localStorage.getItem(HIDDEN_KEY) ?? "[]");
    return new Set(Array.isArray(parsed) ? parsed.filter((p) => typeof p === "string") : []);
  } catch {
    return new Set();
  }
}

function saveHidden(hidden: Set<string>) {
  try {
    localStorage.setItem(HIDDEN_KEY, JSON.stringify([...hidden]));
  } catch {
    // Convenience only; hidden files simply show again next time.
  }
}

function relativePath(dir: string, path: string): string {
  const rel = path.startsWith(dir) ? path.slice(dir.length) : path;
  return rel.replace(/^[\\/]+/, "").replace(/[\\/]LORE\.md$/, "");
}

/** Renders a search snippet; the Rust side wraps matches in \u0001 ... \u0002. */
function Snippet({ text }: { text: string }) {
  const parts = text.split(/(\u0001[^\u0002]*\u0002)/);
  return (
    <>
      {parts.map((part, i) =>
        part.startsWith("\u0001") ? (
          <mark key={i}>{part.slice(1, -1)}</mark>
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </>
  );
}

function countIssues(issues: Issue[]) {
  const errors = issues.filter((i) => i.severity === "error").length;
  return { errors, warnings: issues.length - errors };
}

/** Identifies the errors of a file, so a decision holds until the errors change. */
function errorSignature(item: LoreSummary): string {
  return item.issues
    .filter((i) => i.severity === "error")
    .map((i) => `${i.line}:${i.message}`)
    .join("|");
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
        <details className="issue-panel" open={countIssues(item.issues).errors > 0}>
          <summary>
            構造チェック: エラー {countIssues(item.issues).errors} 件 / 警告{" "}
            {countIssues(item.issues).warnings} 件
          </summary>
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
  duplicates,
  onOpen,
  onShowDuplicates,
}: {
  item: LoreSummary;
  label: string;
  /** How many loaded files share this file's id (1 means it is unique). */
  duplicates: number;
  onOpen: () => void;
  onShowDuplicates: () => void;
}) {
  return (
    <li className="lore-item clickable" onClick={onOpen}>
      <div className="lore-head">
        <span className="lore-date">{item.date ?? "日付なし"}</span>
        {item.kind && <span className="badge">{item.kind}</span>}
        {item.value && <span className="badge">価値: {item.value}</span>}
        {duplicates > 1 && (
          <button
            className="badge dup"
            title="同じIDのファイルを見る"
            onClick={(e) => {
              e.stopPropagation();
              onShowDuplicates();
            }}
          >
            同じID ×{duplicates}
          </button>
        )}
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

function UnreadableCard({ item, label }: { item: LoreSummary; label: string }) {
  return (
    <li className="lore-item rejected-item">
      <div className="lore-path">{label}</div>
      {item.error && <div className="error">{item.error}</div>}
    </li>
  );
}

/** A file held back because of errors: why, and a way to load it anyway. */
function HeldCard({
  item,
  label,
  onLoad,
}: {
  item: LoreSummary;
  label: string;
  onLoad: () => void;
}) {
  return (
    <li className="lore-item rejected-item">
      <div className="held-head">
        <div className="lore-path">{label}</div>
        <button onClick={onLoad}>読み込む</button>
      </div>
      <IssueList issues={item.issues.filter((i) => i.severity === "error")} />
    </li>
  );
}

/** Asked before files with errors are loaded; nothing is loaded until it is answered. */
function ViolationDialog({
  entries,
  onLoadAll,
  onHold,
}: {
  entries: { item: LoreSummary; label: string }[];
  onLoadAll: () => void;
  onHold: () => void;
}) {
  const SHOWN = 3;
  return (
    <div className="modal-overlay">
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby="violation-title">
        <h2 id="violation-title">LoreSpec からずれているファイルがあります</h2>
        <p>
          {entries.length} 件のファイルにエラーがあります。このまま読み込むと、構造を使う機能
          (項目での絞り込みなど)で、正しく扱えない部分が出ることがあります。読み込みますか?
        </p>
        <ul className="modal-list">
          {entries.map(({ item, label }) => {
            const errors = item.issues.filter((i) => i.severity === "error");
            return (
              <li key={item.path}>
                <div className="lore-path">{label}</div>
                <IssueList issues={errors.slice(0, SHOWN)} />
                {errors.length > SHOWN && (
                  <div className="more">ほか {errors.length - SHOWN} 件</div>
                )}
              </li>
            );
          })}
        </ul>
        <div className="modal-actions">
          <button onClick={onHold}>読み込まない(保留にする)</button>
          <button className="primary" onClick={onLoadAll}>
            すべて読み込む
          </button>
        </div>
      </div>
    </div>
  );
}

type FileState = "shown" | "hidden" | "waiting";

/** Lists every file that shares an id, so the user can open one or hide the others. */
function DuplicateDialog({
  id,
  files,
  onOpen,
  onToggleHidden,
  onClose,
}: {
  id: string;
  files: { item: LoreSummary; label: string; state: FileState }[];
  onOpen: (item: LoreSummary) => void;
  onToggleHidden: (item: LoreSummary) => void;
  onClose: () => void;
}) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dup-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="dup-title">同じIDのファイル({files.length} 件)</h2>
        <p className="dup-id">{id}</p>
        <p>
          同じセッションを、別の版(別のAIで作り直した、日本語と英語、修正の前後など)で持っている
          可能性があります。使わない版は隠せます。ファイルは削除されず、一覧と検索から外れるだけです。
        </p>
        <ul className="modal-list dup-list">
          {files.map(({ item, label, state }) => (
            <li key={item.path}>
              <div className="held-head">
                <div>
                  <div className="lore-path">{label}</div>
                  <div className="lore-head">
                    <span className="lore-date">{item.date ?? "日付なし"}</span>
                    {state === "hidden" && <span className="badge">非表示</span>}
                    {state === "waiting" && <span className="badge warn">保留中</span>}
                  </div>
                </div>
                <div className="dup-actions">
                  {state !== "waiting" && <button onClick={() => onOpen(item)}>開く</button>}
                  <button onClick={() => onToggleHidden(item)}>
                    {state === "hidden" ? "表示する" : "隠す"}
                  </button>
                </div>
              </div>
            </li>
          ))}
        </ul>
        <div className="modal-actions">
          <button onClick={onClose}>閉じる</button>
        </div>
      </div>
    </div>
  );
}

function App() {
  const [dir, setDir] = useState<string | null>(loadSavedDir);
  const [folderItems, setFolderItems] = useState<LoreSummary[]>([]);
  const [importedPaths, setImportedPaths] = useState<string[]>(loadImported);
  const [importedItems, setImportedItems] = useState<LoreSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<LoreSummary | null>(null);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<Hit[]>([]);
  const [indexReport, setIndexReport] = useState<IndexReport | null>(null);
  // Files the user agreed to load despite errors, and files held back this session.
  const [allowed, setAllowed] = useState<Record<string, string>>(loadAllowed);
  const [held, setHeld] = useState<Set<string>>(new Set());
  // Files the user hid, and the id whose files are being compared.
  const [hidden, setHidden] = useState<Set<string>>(loadHidden);
  const [duplicateId, setDuplicateId] = useState<string | null>(null);

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

  // Files found in the folder are listed first; hand-imported ones are marked.
  const entries = useMemo(
    () => [
      ...folderItems.map((item) => ({ item, label: relativePath(dir ?? "", item.path) })),
      ...importedItems
        .filter((item) => !folderItems.some((f) => f.path === item.path))
        .map((item) => ({ item, label: `インポート: ${item.path}` })),
    ],
    [folderItems, importedItems, dir],
  );
  const unreadable = entries.filter((e) => !e.item.readable);
  // A file with errors waits for the user's answer; the answer holds until its errors change.
  const withErrors = entries.filter((e) => e.item.readable && countIssues(e.item.issues).errors > 0);
  const waiting = withErrors.filter((e) => allowed[e.item.path] !== errorSignature(e.item));
  const shownEntries = entries.filter((e) => e.item.readable && !waiting.includes(e));
  const loaded = shownEntries.filter((e) => !hidden.has(e.item.path));
  const hiddenEntries = shownEntries.filter((e) => hidden.has(e.item.path));
  const idCounts = new Map<string, number>();
  for (const e of loaded) idCounts.set(e.item.id, (idCounts.get(e.item.id) ?? 0) + 1);
  const undecided = waiting.filter((e) => !held.has(e.item.path));

  useEffect(() => {
    if (dir) scan(dir);
  }, [dir, scan]);

  useEffect(() => {
    inspect(importedPaths);
  }, [importedPaths, inspect]);

  // Rebuild the search index from the files that are loaded. Files still
  // waiting for the user's answer are not indexed.
  const loadedKey = loaded.map((e) => e.item.path).join("\n");
  useEffect(() => {
    const paths = loadedKey ? loadedKey.split("\n") : [];
    let cancelled = false;
    invoke<IndexReport>("index_library", { paths })
      .then((report) => !cancelled && setIndexReport(report))
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, [loadedKey]);

  // Search as the user types (debounced), and again after the index changes.
  useEffect(() => {
    if (!query.trim()) {
      setHits([]);
      return;
    }
    let cancelled = false;
    const timer = setTimeout(() => {
      invoke<Hit[]>("search_lore", { query })
        .then((found) => !cancelled && setHits(found))
        .catch((e) => !cancelled && setError(String(e)));
    }, 200);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query, indexReport]);

  function allow(items: LoreSummary[]) {
    const next = { ...allowed };
    for (const item of items) next[item.path] = errorSignature(item);
    saveAllowed(next);
    setAllowed(next);
    setHeld((prev) => {
      const rest = new Set(prev);
      items.forEach((i) => rest.delete(i.path));
      return rest;
    });
  }

  function toggleHidden(item: LoreSummary) {
    const next = new Set(hidden);
    if (!next.delete(item.path)) next.add(item.path);
    saveHidden(next);
    setHidden(next);
  }

  function holdBack(items: LoreSummary[]) {
    setHeld((prev) => new Set([...prev, ...items.map((i) => i.path)]));
  }

  async function chooseFolder() {
    const chosen = await open({ directory: true, title: "Lore フォルダを選択" });
    if (typeof chosen === "string") {
      saveDir(chosen);
      setSelected(null);
      setHeld(new Set());
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

  return (
    <main className="container">
      {duplicateId !== null && (
        <DuplicateDialog
          id={duplicateId}
          files={entries
            .filter((e) => e.item.readable && e.item.id === duplicateId)
            .map((e) => ({
              ...e,
              state: (hidden.has(e.item.path)
                ? "hidden"
                : waiting.includes(e)
                  ? "waiting"
                  : "shown") as FileState,
            }))}
          onOpen={(item) => {
            setDuplicateId(null);
            setSelected(item);
          }}
          onToggleHidden={toggleHidden}
          onClose={() => setDuplicateId(null)}
        />
      )}
      {undecided.length > 0 && (
        <ViolationDialog
          entries={undecided}
          onLoadAll={() => allow(undecided.map((e) => e.item))}
          onHold={() => holdBack(undecided.map((e) => e.item))}
        />
      )}
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
          <input
            className="search"
            type="search"
            placeholder="全文検索(日本語・英語)"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <p className="count">
            {query.trim() ? `${hits.length} 件ヒット / ` : ""}
            {loaded.length} 件
            {waiting.length > 0 && ` ・ 保留 ${waiting.length} 件`}
            {hiddenEntries.length > 0 && ` ・ 非表示 ${hiddenEntries.length} 件`}
            {unreadable.length > 0 && ` ・ 読めなかったファイル ${unreadable.length} 件`}
          </p>
          {waiting.length > 0 && undecided.length === 0 && (
            <section className="rejected">
              <h2>読み込みを保留したファイル(LoreSpec からのずれ)</h2>
              <ul className="lore-list">
                {waiting.map(({ item, label }) => (
                  <HeldCard key={item.path} item={item} label={label} onLoad={() => allow([item])} />
                ))}
              </ul>
            </section>
          )}
          {unreadable.length > 0 && (
            <section className="rejected">
              <h2>読めなかったファイル</h2>
              <ul className="lore-list">
                {unreadable.map(({ item, label }) => (
                  <UnreadableCard key={item.path} item={item} label={label} />
                ))}
              </ul>
            </section>
          )}
          {query.trim() ? (
            <ul className="lore-list">
              {hits.map((hit) => {
                const entry = loaded.find((e) => e.item.path === hit.path);
                return (
                  <li
                    key={hit.path}
                    className="lore-item clickable"
                    onClick={() => entry && setSelected(entry.item)}
                  >
                    <div className="lore-topic">{hit.topic}</div>
                    <div className="hit-snippet">
                      <Snippet text={hit.snippet} />
                    </div>
                    <div className="lore-path">{entry?.label ?? hit.path}</div>
                  </li>
                );
              })}
            </ul>
          ) : (
            <ul className="lore-list">
              {loaded.map(({ item, label }) => (
                <LoreCard
                  key={item.path}
                  item={item}
                  label={label}
                  duplicates={idCounts.get(item.id) ?? 1}
                  onOpen={() => setSelected(item)}
                  onShowDuplicates={() => setDuplicateId(item.id)}
                />
              ))}
            </ul>
          )}
          {hiddenEntries.length > 0 && (
            <details className="hidden-files">
              <summary>隠したファイル({hiddenEntries.length} 件)</summary>
              <ul className="lore-list">
                {hiddenEntries.map(({ item, label }) => (
                  <li key={item.path} className="lore-item">
                    <div className="held-head">
                      <div>
                        <div className="lore-topic">{item.topic ?? item.id}</div>
                        <div className="lore-path">{label}</div>
                      </div>
                      <button onClick={() => toggleHidden(item)}>表示する</button>
                    </div>
                  </li>
                ))}
              </ul>
            </details>
          )}
        </>
      )}
    </main>
  );
}

export default App;
