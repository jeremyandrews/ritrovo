# AI in Ritrovo

Ritrovo uses the AI features the Trovato kernel provides, configured rather than
rebuilt: the provider registry, the `ai_request` host call, token budgets, the AI
permissions, the administration screens and the usage log. Nothing in this
repository talks to a model provider directly, and no key is stored anywhere in
it.

Rows 31.1 to 31.4 and 31.9 of [`STATUS.md`](ritrovo/STATUS.md) are what this page
describes. Search AI (30.4, 30.5), field enrichment (31.5), the chat tile (31.7),
the assistant's tools (31.8), the MCP server (31.10) and similarity search (31.11)
build on it and are not switched on yet; their rows say so.

## Turning it on

Two environment variables, and nothing else:

| Variable | Provider | Used for |
| :- | :- | :- |
| `RITROVO_ANTHROPIC_API_KEY` | Anthropic, `claude-haiku-5-5` | chat: every text answer the site asks for |
| `RITROVO_OPENAI_API_KEY` | OpenAI, `text-embedding-3-small` | embeddings (not used yet) |

Set them in the shell that starts the demo, and start it:

```bash
export RITROVO_ANTHROPIC_API_KEY="..."   # from your own secret store
export RITROVO_OPENAI_API_KEY="..."
scripts/serve-demo.sh
```

`docker-compose.demo.yml` passes both into the kernel container when the shell
has them, and leaves them out when it does not. On an install of your own, give
the kernel process the same two variables, or leave them out entirely: do not set
one to the empty string, because the kernel counts an empty variable as a key
(`FRICTION.md`, `G-AI-REQUESTS-SENT-WITHOUT-A-KEY`).

The kernel reads a key when it makes a call and stores none of it: the provider
screen at `/admin/system/ai-providers` shows each variable's name and
`(set)` or `(not set)`, never the value, and `trovato config export` writes the
name only.

Never put a key in a file in this repository, in a commit, in CI configuration or
in an issue. The repository is public.

## What is configured, and where

All of it is in `demo/config/`, imported by the kernel's own `config import`:

| File | What it sets |
| :- | :- |
| `variable.ai_providers.yml` | The two providers, their models, key variable names and request-a-minute limits |
| `variable.ai_defaults.yml` | Chat to Anthropic, embeddings to OpenAI |
| `variable.ai_token_budgets.yml` | Daily budgets in deny mode: 20,000 tokens a member, 100,000 an editor or publisher, per provider; 20,000 a day for `ritrovo_translate`'s background drafts |
| `variable.ai_pricing.yml` | Prices per 1,000 tokens, so the usage log carries a cost estimate |
| `variable.search.yml` | The search page's AI stages, all **off** (see below) |
| `role.*.yml` | `use ai` and `use ai chat` for every signed-in member, `view ai usage` for editors and publishers, nothing for anonymous visitors |

`scripts/demo-bootstrap.sh` also enables the kernel's `trovato_ai` plugin.

The administrator holds every AI permission through the administrator flag,
including `configure ai`, which opens the provider, chat and assistant screens.
The administration screens are:

| Screen | Permission | What it shows |
| :- | :- | :- |
| `/admin/system/ai-providers` | `configure ai` | the providers, the defaults, a connection test per provider |
| `/admin/system/ai-budgets` | `view ai usage` | tokens and requests per provider and per user for the period, and the budget grid (`configure ai` to save it) |
| `/admin/system/ai-chat` | `configure ai` | the chat system prompt and retrieval settings |
| `/admin/system/ai-assistant` | `configure ai` | the editors' assistant |

## What each feature costs

Every call is one request to the chat provider, logged with its tokens and an
estimated cost in `ai_usage_log`, and counted against the caller's daily budget.
At Claude Haiku 5.5's prices (0.10 USD a million input tokens, 0.50 a million
output), a call of 1,000 tokens in and 300 out costs about 0.00025 USD.

| Feature | Calls | Who pays | Switched on |
| :- | :- | :- | :- |
| Translation drafts (`ritrovo_translate`) | one per drafted conference, at most three per cron cycle | the plugin's background budget, 20,000 tokens a day | no: plugin variable `assist_enabled`, off |
| Chat (`POST /api/v1/chat`) | one per message | the member's daily budget | the route is live for members; no tile shows it yet (31.7) |
| AI Assist on the edit form | one per button press | the editor's daily budget | buttons render for members with `use ai chat`; not yet checked (31.6) |
| Search expansion and summary | one each per search | nobody: the kernel meters neither | **no**, see below |

### Why the search AI stages are off

The kernel's `/api/v1/search/expand`, `/summarize` and `/followup` check no
permission and no token budget, and record no usage. With a key set and a stage
on, every visitor's search, signed in or not, spends the site's tokens without a
limit or a record. `variable.search.yml` switches all three off; each then
answers 404 before any provider work. `FRICTION.md`,
`G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET`.

## With no key

Which is CI, the public demo workflow, the nightly run against the kernel's
`main`, and any install that has not set the two variables. Every AI surface
degrades without an error page and without a word of the provider's answer:

| Surface | Answer |
| :- | :- |
| A visitor's chat request | 401, sign in (with or without a key) |
| A member's chat request | 503, "AI is temporarily unavailable" |
| Search AI | 404, switched off (with or without a key); the Pagefind results are unchanged |
| Translation drafts | none stored; the cron result counts the failure |
| The provider screen | each variable `(not set)` |

The kernel still sends a request with no key, and the provider refuses it
(`FRICTION.md`, `G-AI-REQUESTS-SENT-WITHOUT-A-KEY`). That costs no money, but
the request goes out whole: a member's chat message reaches the provider
even though the site has no account with it. In this demo, with translation
drafts and the search stages off, chat is the only surface that sends anything.
An install that will run without keys should set `enabled: false` on both
providers in `variable.ai_providers.yml`. A disabled provider is never
resolved, so every AI surface gives its "not available" answer without sending
anything.

These are asserted by the host tests in `plugins/ritrovo_access/tests/` and
`plugins/ritrovo_translate/tests/` and, against the running demo, by the "AI"
section of `scripts/verify-demo.sh`, which makes no model call.
