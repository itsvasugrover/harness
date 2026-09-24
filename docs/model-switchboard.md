# Model Switchboard (multi-model, OpenAI-compatible)

One catalog, many providers, BYO keys. Any model string looks like
`provider/model@variant` (e.g. `glm/glm-5`, `deepseek/deepseek-chat`,
`muse-spark/muse-spark-1` — all defined in config, never in code).

## Catalog (models.dev pattern, our own fetch)

- Base listing fetched from `catalog_url` (default
  `https://models.dev/api.json` — the same upstream opencode uses),
  cached 24h to disk; stale cache survives network outages, and boot
  never fails on network. Verified live: 224 providers parsed.
- models.dev carries no base URLs — your GLM/DeepSeek/Muse-Spark
  endpoints attach purely through local config, which overlays and
  wins on every field it sets (verified: local base_url beats the
  upstream entry).
- `config.yaml` overlay adds custom providers/models; missing adapter
  falls back to the generic OpenAI-compatible adapter.

## Config shape (YAML, one example)

```yaml
default_model: glm/glm-5
providers:
  glm:
    base_url: https://YOUR-GLM-ENDPOINT/v1   # from your provider docs
    env: [GLM_API_KEY]
    models:
      glm-5:
        context: 200000
        output: 8000
```

Keys resolve in order: explicit `options.api_key` → listed `env` vars →
OS keychain → auth plugin. Keys never leave the machine except to the
provider itself.

## Gateway + Relay (both ship, per your choice)

| Mode | How | Use for |
|---|---|---|
| Gateway | `harnessd serve` → `POST /v1/chat/completions`, `/v1/responses`, `/v1/messages`, `GET /v1/models`, SSE streaming | Our decks, scripts, any OpenAI SDK |
| Relay | `harness relay wrap <tool>` → localhost proxy + env injection | Third-party CLIs with zero code change |

Both funnel through the same route → Press → adapter chain, so savings
and audit apply uniformly.

## Request transform (per-provider correctness)

Single `transform.rs` (< 300 lines, split into `cache.rs`,
`params.rs`, `variants.rs` when it grows):

- Normalize messages to the provider's shape (`max_tokens` →
  `max_completion_tokens` where required, Snowflake/Vertex quirks).
- Cache markers: first 2 system + last 2 messages get ephemeral markers
  (Anthropic/Bedrock message-level; others last-part-level).
- Session-scoped cache key (`prompt_cache_key = session_id`) for
  providers that support explicit prefix caching; gateway id header for
  Cloudflare-style gateways.
- Reasoning variants: effort toggles mapped per adapter
  (`reasoning_effort` / `thinking.budget_tokens` / `effort`), clamp-only.
- Tool-call repair: lowercase match, else return a typed `invalid`
  tool result so the model retries cleanly.

## Streaming and cost ledger

- Stream events: `text|reasoning|tool_start|tool_delta|tool_end|finish|error`.
- Every finish writes `{input, output, reasoning, cache_read, cache_write}`
  + computed cost into the session row; SSE pushes a live counter to the decks.
- Token estimate fallback is `bytes/4` (clearly labeled estimate);
  provider-reported usage wins when present.
