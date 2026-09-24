// Command Deck shell: sidebar nav, connection status, screens.
// The shell renders daemon facts; it owns no agent or forge logic.
import { KanbanSquare, ScrollText, Settings as SettingsIcon } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { cn } from "./components/ui";
import { getIdentity, loadConfig, saveConfig, type ApiConfig } from "./lib/api";
import { Audit } from "./screens/Audit";
import { Board } from "./screens/Board";
import { Settings } from "./screens/Settings";

type Tab = "board" | "audit" | "settings";

const TABS: { id: Tab; label: string; icon: typeof KanbanSquare }[] = [
  { id: "board", label: "Board", icon: KanbanSquare },
  { id: "audit", label: "Audit", icon: ScrollText },
  { id: "settings", label: "Settings", icon: SettingsIcon },
];

function useConnection(cfg: ApiConfig) {
  const [online, setOnline] = useState<boolean | null>(null);
  const probe = useCallback(async () => {
    try {
      await getIdentity(cfg);
      setOnline(true);
    } catch {
      setOnline(false);
    }
  }, [cfg.baseUrl, cfg.bearer]);
  useEffect(() => {
    void probe();
    const timer = setInterval(() => void probe(), 30_000);
    return () => clearInterval(timer);
  }, [probe]);
  return online;
}

export function App() {
  const [cfg, setCfg] = useState<ApiConfig>(loadConfig);
  const [tab, setTab] = useState<Tab>("board");
  const online = useConnection(cfg);

  return (
    <div className="flex min-h-[100dvh]">
      <nav
        aria-label="Primary"
        className="flex w-16 shrink-0 flex-col items-center gap-1 border-r border-edge bg-panel py-4 sm:w-52 sm:items-stretch sm:px-3"
      >
        <p className="mb-3 hidden px-2 font-display text-lg font-bold sm:block">
          Command<span className="text-accent">Deck</span>
        </p>
        {TABS.map((t) => {
          const Icon = t.icon;
          const active = tab === t.id;
          return (
            <button
              key={t.id}
              type="button"
              onClick={() => setTab(t.id)}
              aria-current={active ? "page" : undefined}
              className={cn(
                "flex min-h-[44px] cursor-pointer items-center justify-center gap-2 rounded-lg px-2 text-sm transition-colors duration-150 sm:justify-start sm:px-3",
                active ? "bg-raised text-ink" : "text-dim hover:bg-raised/60 hover:text-ink",
              )}
            >
              <Icon size={17} />
              <span className="hidden sm:inline">{t.label}</span>
            </button>
          );
        })}
        <span className="flex-1" />
        <p
          className="flex items-center gap-2 px-2 text-xs text-faint"
          title={online === null ? "probing daemon" : online ? "daemon reachable" : "daemon unreachable"}
        >
          <span
            aria-hidden
            className={cn(
              "inline-block h-2 w-2 rounded-full",
              online === null ? "bg-faint" : online ? "bg-ok" : "bg-bad",
            )}
          />
          <span className="hidden sm:inline">{online === null ? "…" : online ? "live" : "offline"}</span>
        </p>
      </nav>
      <main className="min-w-0 flex-1 px-4 py-5 sm:px-6">
        <div className="mx-auto max-w-[1400px]">
          {tab === "board" && <Board cfg={cfg} />}
          {tab === "audit" && <Audit cfg={cfg} />}
          {tab === "settings" && (
            <Settings
              cfg={cfg}
              onSave={(next) => {
                setCfg(next);
                saveConfig(next);
              }}
            />
          )}
        </div>
      </main>
    </div>
  );
}
