// Audit: append-only Sentinel ledger with actor/repo/kind filters.
// Read-only by rule; verdicts render as tone badges, never as actions.
import { FileSearch, RefreshCw, ScrollText, Unplug } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Badge, Button, EmptyState, Skeleton, TextInput } from "../components/ui";
import { getAudit, type ApiConfig, type AuditEvent } from "../lib/api";

function verdictTone(v: string): string {
  const lower = v.toLowerCase();
  if (lower === "pass") return "emerald";
  if (lower === "warn") return "amber";
  if (lower === "block") return "rose";
  return "zinc";
}

export function Audit({ cfg }: { cfg: ApiConfig }) {
  const [actor, setActor] = useState("");
  const [repo, setRepo] = useState("");
  const [kind, setKind] = useState("");
  const [rows, setRows] = useState<AuditEvent[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setRows(
        await getAudit(cfg, {
          actor: actor || undefined,
          repo: repo || undefined,
          kind: kind || undefined,
          limit: 50,
        }),
      );
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "audit fetch failed");
    }
  }, [cfg.baseUrl, cfg.bearer, actor, repo, kind]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <h1 className="font-display text-2xl font-bold">Audit</h1>
        <span className="flex-1" />
        <Button variant="ghost" onClick={() => void refresh()} title="Refresh now">
          <RefreshCw size={15} /> Refresh
        </Button>
      </div>

      <form
        className="mb-4 grid gap-2 sm:grid-cols-3"
        onSubmit={(e) => {
          e.preventDefault();
          void refresh();
        }}
      >
        <TextInput
          aria-label="Filter by actor"
          placeholder="actor"
          value={actor}
          onChange={(e) => setActor(e.target.value)}
        />
        <TextInput
          aria-label="Filter by repo"
          placeholder="owner/repo"
          value={repo}
          onChange={(e) => setRepo(e.target.value)}
        />
        <TextInput
          aria-label="Filter by kind"
          placeholder="kind"
          value={kind}
          onChange={(e) => setKind(e.target.value)}
        />
      </form>

      {error && (
        <div className="mb-4 flex items-center gap-2 rounded-xl border border-bad/40 bg-bad/10 px-4 py-3 text-sm">
          <Unplug size={16} className="text-bad" />
          <span>{error}</span>
        </div>
      )}

      {!rows && !error && (
        <div className="space-y-2">
          <Skeleton className="h-16" />
          <Skeleton className="h-16" />
          <Skeleton className="h-16" />
        </div>
      )}

      {rows && rows.length === 0 && (
        <EmptyState
          icon={<FileSearch size={28} />}
          title="No audit events"
          hint="Every consequential action lands here with its actor and verdict. Loosen the filters to see more."
        />
      )}

      {rows && rows.length > 0 && (
        <ol className="space-y-2">
          {rows.map((r) => (
            <li key={r.id} className="rounded-xl border border-edge bg-panel p-3">
              <div className="flex flex-wrap items-center gap-2">
                <ScrollText size={14} className="text-faint" />
                <span className="font-mono text-xs text-faint">#{r.seq}</span>
                <Badge tone={verdictTone(r.verdict)}>{r.verdict || "pending"}</Badge>
                <span className="rounded bg-raised px-1.5 py-0.5 font-mono text-xs">{r.kind}</span>
                <span className="flex-1" />
                <time className="font-mono text-xs text-faint">
                  {new Date(r.time).toLocaleString()}
                </time>
              </div>
              <p className="mt-1 text-sm">{r.summary}</p>
              <p className="mt-1 font-mono text-xs text-faint">
                {r.attribution.actor} · {r.attribution.agent}
                {r.attribution.skill ? ` · skill:${r.attribution.skill}` : ""}
                {r.repo ? ` · ${r.repo}` : ""}
              </p>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}
