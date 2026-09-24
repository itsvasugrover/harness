// Settings: daemon connection only. Keys stay on this machine:
// the bearer lives in localStorage, never in code or logs.
import { CheckCircle2, PlugZap, XCircle } from "lucide-react";
import { useState } from "react";
import { Badge, Button, Field, TextInput } from "../components/ui";
import { CONTRACT, getIdentity, type ApiConfig } from "../lib/api";

export function Settings({ cfg, onSave }: { cfg: ApiConfig; onSave: (c: ApiConfig) => void }) {
  const [baseUrl, setBaseUrl] = useState(cfg.baseUrl);
  const [bearer, setBearer] = useState(cfg.bearer);
  const [probe, setProbe] = useState<
    { ok: true; host: string; contract: number } | { ok: false; error: string } | null
  >(null);
  const [testing, setTesting] = useState(false);

  async function test(candidate: ApiConfig) {
    setTesting(true);
    setProbe(null);
    try {
      const id = await getIdentity(candidate);
      setProbe({ ok: true, host: id.host_id, contract: id.contract });
    } catch (e) {
      setProbe({ ok: false, error: e instanceof Error ? e.message : "probe failed" });
    } finally {
      setTesting(false);
    }
  }

  return (
    <div className="max-w-xl">
      <h1 className="mb-4 font-display text-2xl font-bold">Settings</h1>
      <div className="space-y-4">
        <Field label="Daemon URL" hint="Loopback for this machine; LAN address when paired remotely.">
          <TextInput
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder="http://127.0.0.1:4317"
            inputMode="url"
          />
        </Field>
        <Field
          label="Bearer token"
          hint="Required on LAN. Empty on loopback. Rotatable from the daemon side."
        >
          <TextInput
            type="password"
            value={bearer}
            onChange={(e) => setBearer(e.target.value)}
            placeholder="leave empty on this machine"
            autoComplete="off"
          />
        </Field>
        <div className="flex gap-2">
          <Button
            onClick={() => {
              const next = { baseUrl: baseUrl.trim(), bearer: bearer.trim() };
              onSave(next);
              void test(next);
            }}
          >
            <PlugZap size={15} /> Save &amp; test
          </Button>
          <Button
            variant="ghost"
            disabled={testing}
            onClick={() => void test({ baseUrl: baseUrl.trim(), bearer: bearer.trim() })}
          >
            Test without saving
          </Button>
        </div>
        {probe && probe.ok && (
          <p className="flex items-center gap-2 text-sm">
            <CheckCircle2 size={16} className="text-ok" />
            <span className="font-mono">{probe.host}</span>
            <Badge tone={probe.contract === CONTRACT ? "emerald" : "amber"}>
              contract {probe.contract}
            </Badge>
          </p>
        )}
        {probe && !probe.ok && (
          <p className="flex items-center gap-2 text-sm text-bad">
            <XCircle size={16} /> {probe.error}
          </p>
        )}
      </div>
    </div>
  );
}
