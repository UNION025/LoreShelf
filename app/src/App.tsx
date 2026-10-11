import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  domains: string[];
  trails: string[];
  error: string | null;
  issues: Issue[];
  readable: boolean;
}

type ContextMode = "full" | "digest";

interface ContextBundle {
  text: string;
  files: number;
  chars: number;
  skipped: number;
}

type ImportOutcome = "imported" | "duplicate" | "already" | "new-version" | "failed";

interface ImportResult {
  source: string;
  id: string;
  outcome: ImportOutcome;
  dest: string | null;
  message: string;
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
const REJECTED_KEY = "loreshelf.rejectedFiles";

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

/** Files the user declined to load despite errors: path -> the errors they declined. */
function loadRejected(): Record<string, string> {
  try {
    const parsed = JSON.parse(localStorage.getItem(REJECTED_KEY) ?? "{}");
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

function saveRejected(rejected: Record<string, string>) {
  try {
    localStorage.setItem(REJECTED_KEY, JSON.stringify(rejected));
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

function LoreDetail({
  item,
  onBack,
  onCopy,
}: {
  item: LoreSummary;
  onBack: () => void;
  onCopy: (mode: ContextMode) => void;
}) {
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
        <span className="spacer" />
        <button onClick={() => onCopy("digest")} title="決定、未解決の問い、次の一手だけを、AIに貼る形でコピー">
          決定と未解決だけをコピー
        </button>
        <button onClick={() => onCopy("full")} title="全文を、AIに貼る形でコピー">
          全文をコピー
        </button>
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

/** A file the user rejected because of errors: why, and a way to load it after all. */
function RejectedCard({
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
  onReject,
}: {
  entries: { item: LoreSummary; label: string }[];
  onLoadAll: () => void;
  onReject: () => void;
}) {
  const SHOWN = 3;
  return (
    <div className="modal-overlay">
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby="violation-title">
        <h2 id="violation-title">LoreSpec からずれているファイルがあります</h2>
        <p>
          {entries.length} 件のファイルにエラーがあります。このまま読み込むと、構造を使う機能
          (項目での絞り込みなど)で、正しく扱えない部分が出ることがあります。読み込みますか?
          却下すると一覧に出ず、あとから「却下したファイル」で、読み込むこともできます(ファイルは消えません)。
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
          <button onClick={onReject}>却下する</button>
          <button className="primary" onClick={onLoadAll}>
            すべて読み込む
          </button>
        </div>
      </div>
    </div>
  );
}

const OUTCOME_LABEL: Record<ImportOutcome, string> = {
  imported: "取り込みました",
  "new-version": "別の版として取り込みました",
  duplicate: "取り込みませんでした(同じ内容がすでにあります)",
  already: "取り込みませんでした(すでにライブラリの中にあります)",
  failed: "取り込めませんでした",
};

/** What happened to each file the user chose to import. */
function ImportResultDialog({
  results,
  library,
  onClose,
}: {
  results: ImportResult[];
  library: string;
  onClose: () => void;
}) {
  const imported = results.filter((r) => r.outcome === "imported" || r.outcome === "new-version").length;
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="import-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="import-title">取り込み結果(新しく保存したもの: {imported} 件)</h2>
        <p>
          ライブラリ(<span className="dup-id">{library}</span>)に、<code>〔ID〕/LORE.md</code>{" "}
          の形で保存します。元のファイルは、そのまま残ります。上書きや削除はしません。
        </p>
        <ul className="modal-list dup-list">
          {results.map((r) => (
            <li key={r.source}>
              <div className="lore-path">{r.source}</div>
              <div className={r.outcome === "failed" ? "error" : "import-outcome"}>
                {OUTCOME_LABEL[r.outcome]}
              </div>
              <div className="more">{r.message}</div>
              {r.dest && r.outcome !== "failed" && <div className="more">{relativePath(library, r.dest)}</div>}
            </li>
          ))}
        </ul>
        <div className="modal-actions">
          <button className="primary" onClick={onClose}>
            閉じる
          </button>
        </div>
      </div>
    </div>
  );
}

/** Shown when the clipboard cannot be written: the text is there to copy by hand. */
function CopyFallbackDialog({ text, onClose }: { text: string; onClose: () => void }) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="copy-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="copy-title">コピーできませんでした</h2>
        <p>クリップボードに書き込めませんでした。下の文を、全部選んで、手でコピーしてください。</p>
        <textarea
          className="fallback-text"
          readOnly
          value={text}
          onFocus={(e) => e.currentTarget.select()}
        />
        <div className="modal-actions">
          <button className="primary" onClick={onClose}>
            閉じる
          </button>
        </div>
      </div>
    </div>
  );
}

type FileState = "shown" | "hidden" | "waiting";  // waiting: not loaded (rejected, or not yet answered)

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
                    {state === "waiting" && <span className="badge warn">読み込んでいません</span>}
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
  // Files the user agreed to load despite errors, and files they rejected.
  const [allowed, setAllowed] = useState<Record<string, string>>(loadAllowed);
  const [rejected, setRejected] = useState<Record<string, string>>(loadRejected);
  // Files the user hid, and the id whose files are being compared.
  const [hidden, setHidden] = useState<Set<string>>(loadHidden);
  const [duplicateId, setDuplicateId] = useState<string | null>(null);
  const [importResults, setImportResults] = useState<ImportResult[] | null>(null);
  // The shelf (domain) the list is narrowed to, and the state of "copy for an AI".
  const [domain, setDomain] = useState<string | null>(null);
  const [copyStatus, setCopyStatus] = useState<string | null>(null);
  const [copyFallback, setCopyFallback] = useState<string | null>(null);
  // Index updates run one after another, in the order they were asked for.
  const indexQueue = useRef<Promise<void>>(Promise.resolve());

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
  // Shelves: the domains found in the loaded Lore, most used first.
  const domainCounts = new Map<string, number>();
  for (const e of loaded) {
    for (const d of e.item.domains) domainCounts.set(d, (domainCounts.get(d) ?? 0) + 1);
  }
  const domainList = [...domainCounts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  const activeDomain = domain !== null && domainCounts.has(domain) ? domain : null;
  const inShelf = (e: { item: LoreSummary }) => activeDomain === null || e.item.domains.includes(activeDomain);
  const shown = loaded.filter(inShelf);
  const shownHits = hits.filter((h) => {
    const entry = loaded.find((e) => e.item.path === h.path);
    return entry !== undefined && inShelf(entry);
  });
  // What "copy for an AI" would take: the search results, or else the shelf in view.
  const copyPaths = query.trim() ? shownHits.map((h) => h.path) : shown.map((e) => e.item.path);
  const idCounts = new Map<string, number>();
  for (const e of loaded) idCounts.set(e.item.id, (idCounts.get(e.item.id) ?? 0) + 1);
  // A rejection holds until the file's errors change, like an agreement does.
  const rejectedNow = waiting.filter((e) => rejected[e.item.path] === errorSignature(e.item));
  const undecided = waiting.filter((e) => rejected[e.item.path] !== errorSignature(e.item));

  useEffect(() => {
    if (dir) scan(dir);
  }, [dir, scan]);

  useEffect(() => {
    inspect(importedPaths);
  }, [importedPaths, inspect]);

  // Rebuild the search index from the files that are loaded. Files still
  // waiting for the user's answer are not indexed. The updates are queued so
  // that a slow, older one can never finish after a newer one and overwrite
  // it, and an update that has been superseded before it starts is skipped.
  const loadedKey = loaded.map((e) => e.item.path).join("\n");
  useEffect(() => {
    const paths = loadedKey ? loadedKey.split("\n") : [];
    let superseded = false;
    indexQueue.current = indexQueue.current.then(async () => {
      if (superseded) return;
      try {
        const report = await invoke<IndexReport>("index_library", { paths });
        if (!superseded) setIndexReport(report);
      } catch (e) {
        if (!superseded) setError(String(e));
      }
    });
    return () => {
      superseded = true;
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
    // Loading a file after all lifts its rejection.
    const rest = { ...rejected };
    for (const item of items) delete rest[item.path];
    saveRejected(rest);
    setRejected(rest);
  }

  function toggleHidden(item: LoreSummary) {
    const next = new Set(hidden);
    if (!next.delete(item.path)) next.add(item.path);
    saveHidden(next);
    setHidden(next);
  }

  function reject(items: LoreSummary[]) {
    const next = { ...rejected };
    for (const item of items) next[item.path] = errorSignature(item);
    saveRejected(next);
    setRejected(next);
  }

  useEffect(() => {
    if (!copyStatus) return;
    const timer = setTimeout(() => setCopyStatus(null), 10000);
    return () => clearTimeout(timer);
  }, [copyStatus]);

  /** Builds the text for an AI and puts it on the clipboard (or shows it to copy by hand). */
  async function copyContext(paths: string[], mode: ContextMode) {
    if (paths.length === 0) return;
    try {
      const bundle = await invoke<ContextBundle>("build_context", { paths, mode, withPreface: true });
      try {
        await navigator.clipboard.writeText(bundle.text);
        const long = mode === "full" && bundle.chars > 40000;
        setCopyStatus(
          `${bundle.files} 件(約 ${bundle.chars.toLocaleString()} 文字)をコピーしました。AIの会話に貼り付けてください。` +
            (long ? "長いので、AIによっては入りきりません。「決定と未解決だけをコピー」も試してください。" : "") +
            (bundle.skipped > 0 ? `(読めなかったファイル ${bundle.skipped} 件は、含まれていません)` : ""),
        );
      } catch {
        setCopyFallback(bundle.text);
      }
    } catch (e) {
      setError(String(e));
    }
  }

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
    if (dir) {
      // With a library chosen, each file is filed there as <id>/LORE.md.
      try {
        setImportResults(await invoke<ImportResult[]>("import_files", { library: dir, paths }));
        setError(null);
        await scan(dir);
      } catch (e) {
        setError(String(e));
      }
      return;
    }
    // Without a library, the files are only listed from where they are.
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
        <LoreDetail
          item={selected}
          onBack={() => setSelected(null)}
          onCopy={(mode) => copyContext([selected.path], mode)}
        />
        {copyStatus && (
          <p className="copy-status" role="status">
            {copyStatus}
          </p>
        )}
        {copyFallback !== null && <CopyFallbackDialog text={copyFallback} onClose={() => setCopyFallback(null)} />}
      </main>
    );
  }

  return (
    <main className="container">
      {copyFallback !== null && <CopyFallbackDialog text={copyFallback} onClose={() => setCopyFallback(null)} />}
      {importResults !== null && dir && (
        <ImportResultDialog results={importResults} library={dir} onClose={() => setImportResults(null)} />
      )}
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
          onReject={() => reject(undecided.map((e) => e.item))}
        />
      )}
      <header className="toolbar">
        <h1>LoreShelf</h1>
        <button onClick={chooseFolder}>フォルダを選択</button>
        <button
          onClick={importFiles}
          title={dir ? "選んだファイルを、ライブラリに ID/LORE.md の形で保存します" : "フォルダを選ぶと、ライブラリに整理して保存します"}
        >
          ファイルをインポート
        </button>
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
        <p className="empty">
          Lore のフォルダ(ライブラリ)を選択してください。そのあと「ファイルをインポート」で、AIが出したファイルを、
          名前に関係なく <code>ID/LORE.md</code> の形に整えて保存できます。
        </p>
      ) : (
        <>
          <input
            className="search"
            type="search"
            placeholder="全文検索(日本語・英語)"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {domainList.length > 0 && (
            <div className="shelves" role="group" aria-label="分野(棚)で絞り込む">
              <button
                className={activeDomain === null ? "chip active" : "chip"}
                onClick={() => setDomain(null)}
              >
                すべて ({loaded.length})
              </button>
              {domainList.map(([d, n]) => (
                <button
                  key={d}
                  className={activeDomain === d ? "chip active" : "chip"}
                  onClick={() => setDomain(activeDomain === d ? null : d)}
                >
                  {d} ({n})
                </button>
              ))}
            </div>
          )}
          <div className="copybar">
            <span>AIに渡す({copyPaths.length} 件):</span>
            <button
              disabled={copyPaths.length === 0}
              onClick={() => copyContext(copyPaths, "digest")}
              title="決定、未解決の問い、次の一手だけを、AIに貼る形でコピー"
            >
              決定と未解決だけをコピー
            </button>
            <button
              disabled={copyPaths.length === 0}
              onClick={() => copyContext(copyPaths, "full")}
              title="全文を、AIに貼る形でコピー"
            >
              全文をコピー
            </button>
          </div>
          {copyStatus && (
            <p className="copy-status" role="status">
              {copyStatus}
            </p>
          )}
          <p className="count">
            {query.trim() ? `${shownHits.length} 件ヒット / ` : ""}
            {activeDomain ? `${activeDomain}: ` : ""}
            {shown.length} 件
            {rejectedNow.length > 0 && ` ・ 却下 ${rejectedNow.length} 件`}
            {hiddenEntries.length > 0 && ` ・ 非表示 ${hiddenEntries.length} 件`}
            {unreadable.length > 0 && ` ・ 読めなかったファイル ${unreadable.length} 件`}
          </p>
          {rejectedNow.length > 0 && (
            <details className="hidden-files">
              <summary>却下したファイル({rejectedNow.length} 件)</summary>
              <ul className="lore-list">
                {rejectedNow.map(({ item, label }) => (
                  <RejectedCard key={item.path} item={item} label={label} onLoad={() => allow([item])} />
                ))}
              </ul>
            </details>
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
              {shownHits.map((hit) => {
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
              {shown.map(({ item, label }) => (
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
