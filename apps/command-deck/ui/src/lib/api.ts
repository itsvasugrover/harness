// Thin daemon client. No logic lives here beyond fetch + types:
// columns are derived server-side, this file only renders them.
export type Column =
  | "working"
  | "needs_you"
  | "in_review"
  | "ready_to_merge"
  | "done";

export interface Identity {
  host_id: string;
  contract: number;
}

/// Daemon contract version this UI speaks.
export const CONTRACT = 2;

export interface WorkerCard {
  worker_id: string;
  column: Column;
  alive: boolean;
  blocked: string | null;
  completed: boolean;
}

export interface PrCardView {
  repo: string;
  number: number;
  title: string;
  state: string;
  column: Column;
  checks_green: boolean;
  unresolved: number;
  mergeable: boolean;
}

export interface Board {
  workers: WorkerCard[];
  prs: PrCardView[];
}

export interface Attribution {
  actor: string;
  agent: string;
  skill: string | null;
  mcp_server: string | null;
}

export interface AuditEvent {
  seq: number;
  id: string;
  time: string;
  attribution: Attribution;
  repo: string;
  kind: string;
  summary: string;
  verdict: string;
  refs: string[];
  hash_prev: string;
}

export interface ApiConfig {
  baseUrl: string;
  bearer: string;
}

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function fetchJson<T>(cfg: ApiConfig, path: string): Promise<T> {
  const headers: Record<string, string> = {};
  if (cfg.bearer) headers["Authorization"] = `Bearer ${cfg.bearer}`;
  const res = await fetch(`${cfg.baseUrl}${path}`, { headers });
  if (!res.ok) {
    if (res.status === 401) throw new ApiError(401, "bearer rejected — check Settings");
    throw new ApiError(res.status, `${path} failed (${res.status})`);
  }
  return (await res.json()) as T;
}

/** Unauthenticated host probe. Use it to test pairing in Settings. */
export function getIdentity(cfg: ApiConfig): Promise<Identity> {
  return fetchJson<Identity>(cfg, "/api/v1/identity");
}

export function getBoard(cfg: ApiConfig): Promise<Board> {
  return fetchJson<Board>(cfg, "/api/v1/board");
}

export interface AuditFilter {
  actor?: string;
  repo?: string;
  kind?: string;
  limit?: number;
}

export function getAudit(cfg: ApiConfig, f: AuditFilter): Promise<AuditEvent[]> {
  const q = new URLSearchParams();
  if (f.actor) q.set("actor", f.actor);
  if (f.repo) q.set("repo", f.repo);
  if (f.kind) q.set("kind", f.kind);
  q.set("limit", String(f.limit ?? 50));
  return fetchJson<AuditEvent[]>(cfg, `/api/v1/audit?${q.toString()}`);
}

/** Display metadata only — the column itself always comes from the daemon. */
export const COLUMNS: { id: Column; label: string }[] = [
  { id: "working", label: "Working" },
  { id: "needs_you", label: "Needs you" },
  { id: "in_review", label: "In review" },
  { id: "ready_to_merge", label: "Ready to merge" },
  { id: "done", label: "Done" },
];

const SETTINGS_KEY = "command-deck.api";

export function loadConfig(): ApiConfig {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<ApiConfig>;
      return {
        baseUrl: parsed.baseUrl || "http://127.0.0.1:4317",
        bearer: parsed.bearer || "",
      };
    }
  } catch {
    // Corrupt settings fall back to loopback defaults.
  }
  return { baseUrl: "http://127.0.0.1:4317", bearer: "" };
}

export function saveConfig(cfg: ApiConfig): void {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(cfg));
}
