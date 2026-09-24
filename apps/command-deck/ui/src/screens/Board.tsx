// Board: derived Kanban rendered from the live daemon API.
// Columns arrive server-derived; the UI never computes placement.
import {
  Bot,
  CheckCircle2,
  CircleDot,
  GitPullRequest,
  Inbox,
  MessageSquareWarning,
  RefreshCw,
  Unplug,
  X,
  XCircle,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Badge, Button, Card, EmptyState, Skeleton, cn } from "../components/ui";
import {
  COLUMNS,
  getBoard,
  type ApiConfig,
  type Board as BoardData,
  type Column,
  type PrCardView,
  type WorkerCard,
} from "../lib/api";

const COLUMN_TONE: Record<Column, string> = {
  working: "sky",
  needs_you: "amber",
  in_review: "blue",
  ready_to_merge: "emerald",
  done: "zinc",
};

const POLL_MS = 15_000;

function useBoard(cfg: ApiConfig) {
  const [board, setBoard] = useState<BoardData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [updatedAt, setUpdatedAt] = useState<Date | null>(null);
  const refresh = useCallback(async () => {
    try {
      setBoard(await getBoard(cfg));
      setError(null);
      setUpdatedAt(new Date());
    } catch (e) {
      setError(e instanceof Error ? e.message : "board fetch failed");
    }
  }, [cfg.baseUrl, cfg.bearer]);
  useEffect(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), POLL_MS);
    return () => clearInterval(timer);
  }, [refresh]);
  return { board, error, updatedAt, refresh };
}

function WorkerRow({ card, onOpen }: { card: WorkerCard; onOpen: () => void }) {
  return (
    <button
      type="button"
      onClick={onOpen}
      className="w-full cursor-pointer rounded-lg border border-edge bg-raised p-3 text-left transition-colors duration-150 hover:border-faint"
    >
      <div className="flex items-center gap-2">
        <Bot size={14} className="shrink-0 text-dim" />
        <span className="truncate font-mono text-sm">{card.worker_id}</span>
      </div>
      <p className="mt-1 truncate text-xs text-dim">
        {card.completed ? "finished" : card.blocked ?? (card.alive ? "running" : "idle")}
      </p>
    </button>
  );
}

function PrRow({ card }: { card: PrCardView }) {
  return (
    <div className="rounded-lg border border-edge bg-raised p-3">
      <div className="flex items-center gap-2">
        <GitPullRequest size={14} className="shrink-0 text-dim" />
        <span className="truncate font-mono text-sm">
          {card.repo ? `${card.repo}#${card.number}` : `#${card.number}`}
        </span>
      </div>
      <p className="mt-1 line-clamp-2 text-xs text-dim">{card.title}</p>
      <div className="mt-2 flex flex-wrap gap-1">
        {card.checks_green ? (
          <Badge tone="emerald">
            <CheckCircle2 size={12} className="mr-1" /> checks
          </Badge>
        ) : (
          <Badge tone="rose">
            <XCircle size={12} className="mr-1" /> checks
          </Badge>
        )}
        {card.unresolved > 0 && (
          <Badge tone="amber">
            <MessageSquareWarning size={12} className="mr-1" />
            {card.unresolved} threads
          </Badge>
        )}
        {!card.mergeable && <Badge tone="zinc">not mergeable</Badge>}
      </div>
    </div>
  );
}

export function Board({ cfg }: { cfg: ApiConfig }) {
  const { board, error, updatedAt, refresh } = useBoard(cfg);
  const [openId, setOpenId] = useState<string | null>(null);
  const open = board?.workers.find((w) => w.worker_id === openId) ?? null;

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <h1 className="font-display text-2xl font-bold">Board</h1>
        <span className="text-xs text-faint">
          {updatedAt ? `updated ${updatedAt.toLocaleTimeString()}` : "connecting…"}
        </span>
        <span className="flex-1" />
        <Button variant="ghost" onClick={() => void refresh()} title="Refresh now">
          <RefreshCw size={15} /> Refresh
        </Button>
      </div>

      {error && (
        <div className="mb-4 flex items-center gap-2 rounded-xl border border-bad/40 bg-bad/10 px-4 py-3 text-sm">
          <Unplug size={16} className="text-bad" />
          <span>{error}</span>
          <span className="flex-1" />
          <Button variant="ghost" onClick={() => void refresh()}>
            Retry
          </Button>
        </div>
      )}

      {!board && !error && (
        <div className="grid gap-3 md:grid-cols-3 xl:grid-cols-5">
          {COLUMNS.map((c) => (
            <div key={c.id} className="space-y-2">
              <Skeleton className="h-5 w-24" />
              <Skeleton className="h-20" />
              <Skeleton className="h-20" />
            </div>
          ))}
        </div>
      )}

      {board && board.workers.length === 0 && board.prs.length === 0 && (
        <EmptyState
          icon={<Inbox size={28} />}
          title="Nothing on the board"
          hint="Run a goal with harnessd run and workers will appear here as their facts land."
        />
      )}

      {board && (board.workers.length > 0 || board.prs.length > 0) && (
        <div className="rail grid auto-cols-[260px] grid-flow-col gap-3 overflow-x-auto pb-2 xl:auto-cols-auto xl:grid-flow-row xl:grid-cols-5 xl:overflow-visible">
          {COLUMNS.map((col) => {
            const workers = board.workers.filter((w) => w.column === col.id);
            const prs = board.prs.filter((p) => p.column === col.id);
            return (
              <section key={col.id} aria-label={col.label}>
                <header className="mb-2 flex items-center gap-2">
                  <CircleDot size={13} className="text-faint" />
                  <h2 className="text-sm font-semibold">{col.label}</h2>
                  <Badge tone={COLUMN_TONE[col.id]}>{workers.length + prs.length}</Badge>
                </header>
                <div className="space-y-2">
                  {workers.map((w) => (
                    <WorkerRow key={w.worker_id} card={w} onOpen={() => setOpenId(w.worker_id)} />
                  ))}
                  {prs.map((p) => (
                    <PrRow key={`${p.repo}#${p.number}`} card={p} />
                  ))}
                </div>
              </section>
            );
          })}
        </div>
      )}

      {open && (
        <div
          className="fixed inset-0 z-50 bg-black/60"
          onClick={() => setOpenId(null)}
          role="presentation"
        >
          <aside
            className={cn("absolute right-0 top-0 h-full w-full max-w-md overflow-y-auto bg-panel p-6")}
            onClick={(e) => e.stopPropagation()}
            aria-label={`Worker ${open.worker_id}`}
          >
            <div className="mb-4 flex items-center gap-2">
              <h2 className="truncate font-mono text-base">{open.worker_id}</h2>
              <span className="flex-1" />
              <Button variant="ghost" onClick={() => setOpenId(null)} title="Close">
                <X size={16} />
              </Button>
            </div>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between gap-4 border-b border-edge py-2">
                <dt className="text-dim">Status</dt>
                <dd>
                  <Badge tone={COLUMN_TONE[open.column]}>
                    {COLUMNS.find((c) => c.id === open.column)?.label}
                  </Badge>
                </dd>
              </div>
              <div className="flex justify-between gap-4 border-b border-edge py-2">
                <dt className="text-dim">Liveness</dt>
                <dd>{open.alive ? "alive" : "stopped"}</dd>
              </div>
              <div className="flex justify-between gap-4 border-b border-edge py-2">
                <dt className="text-dim">Blocker</dt>
                <dd className="text-right">{open.blocked ?? "none"}</dd>
              </div>
              <div className="flex justify-between gap-4 border-b border-edge py-2">
                <dt className="text-dim">Completed</dt>
                <dd>{open.completed ? "yes" : "no"}</dd>
              </div>
            </dl>
            <Card className="mt-4 p-3">
              <p className="mb-1 text-xs text-faint">Raw facts</p>
              <pre className="overflow-x-auto font-mono text-xs text-dim">
                {JSON.stringify(open, null, 2)}
              </pre>
            </Card>
          </aside>
        </div>
      )}
    </div>
  );
}
