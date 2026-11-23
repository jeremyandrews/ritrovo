# AI in Ritrovo

Ritrovo uses the AI features the Trovato kernel provides, configured rather than
rebuilt: the provider registry, the `ai_request` host call, token budgets, the AI
permissions, the administration screens and the usage log. Nothing in this
repository talks to a model provider directly, and no key is stored anywhere in
it.

Rows 31.1 to 31.11, and the search AI rows 30.4 and 30.5, of
[`STATUS.md`](ritrovo/STATUS.md) are what this page describes: the providers,
budgets and permissions, the chat tile, the editors' conference assistant, AI
Assist on the edit form, the MCP server, similar conferences, and what is
switched off and why.

## Turning it on

Two environment variables, and nothing else:

| Variable | Provider | Used for |
| :- | :- | :- |
| `RITROVO_ANTHROPIC_API_KEY` | Anthropic, `claude-haiku-5-5` | chat: every text answer the site asks for |
| `RITROVO_OPENAI_API_KEY` | OpenAI, `text-embedding-3-small` | embeddings: similar conferences |

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
| `variable.trovato_ai.field_rules.yml` | A summary of each conference's description, written into `field_summary` on a save; inert at the pinned kernel (see below) |
| `gather_query.ritrovo.similar_conferences.yml` | Similar conferences, at `/conferences/similar` (see below) |
| `tile.0193a5a0-0003-7000-8000-000000000009.yml` | The Ask Ritrovo chat tile, in the sidebar for signed-in members (see below) |
| `variable.ai_chat_config.yml` | The chat's system prompt and retrieval settings, Live content only (see below) |
| `variable.ai_assistant_config.yml` | The kernel's assistant, on, with Ritrovo's limits; its one scope is `ritrovo_cfp`'s (see below) |
| `role.*.yml` | `use ai` and `use ai chat` for every signed-in member; `view ai usage` and `use ai assistant` for editors and publishers; nothing for anonymous visitors |

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
| Chat (the Ask Ritrovo tile, `POST /api/v1/chat`) | one per message, at most 20 an hour per member | the member's daily budget | yes, for signed-in members |
| The conference assistant | one per model turn, up to six tool calls a message; at most 20,000 tokens a conversation and 30 messages an hour | the editor's daily budget | yes, for editors and publishers |
| The MCP server (`trovato-mcp`) | none: it reads and writes content and calls no model | nothing | run by hand, below |
| AI Assist on the edit form | one per button press | nobody: the kernel checks no budget on it (`G-AI-ASSIST-CHECKS-NO-BUDGET`); logged as `trovato_ai` | yes, for whoever can open an item form: editors and publishers |
| Search expansion and summary | one each per search | nobody: the kernel meters neither | **no**, see below |
| Similar conferences (`/conferences/similar?q=`) | one embedding call per page view with a `q` | nobody: the kernel meters no embedding call | yes, for everyone |
| Embedding backfill | one embedding call per item | nobody, as above | by hand, below |
| Conference summaries (`trovato_ai` field rule) | one per save through the edit form, by a member holding `use ai` | the member's daily budget | configured, but never runs at the pinned kernel |

### Why the search AI stages are off

The kernel's `/api/v1/search/expand`, `/summarize` and `/followup` check no
permission and no token budget, and record no usage. With a key set and a stage
on, every visitor's search, signed in or not, spends the site's tokens without a
limit or a record. `variable.search.yml` switches all three off; each then
answers 404 before any provider work. `FRICTION.md`,
`G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET`.

A change to `variable.search.yml` imported into a running site takes up to five
minutes to apply: the kernel caches the search settings and `config import`
does not drop the cached copy (`G-CONFIG-IMPORT-LEAVES-SEARCH-SETTINGS-CACHED`).
Saving the search settings form at `/admin/config/search` applies it at once.
The demo's bootstrap keeps every outside request away until its imports are
done, so the stages are off from the first request.

They would not work as they stand either. The kernel's search AI reads only the
first block of a model's answer, and Claude Haiku 5.5 often thinks first, so the
first block has no text. Live on the demo, with the stages switched on locally
for three calls, expansion returned four terms and the summary came back empty
twice, after spending the tokens
(`G-SEARCH-AI-READS-ONLY-THE-FIRST-CONTENT-BLOCK`).

## The chat tile

Ask Ritrovo is the kernel's chat widget, placed in the sidebar of every page but
the admin pages for signed-in members
(`demo/config/tile.0193a5a0-0003-7000-8000-000000000009.yml`). An anonymous
visitor gets no tile at all. A question goes to `POST /api/v1/chat`, which needs
`use ai` and `use ai chat` (every member holds both), and the answer streams back
over server-sent events.

What the model is told is `variable.ai_chat_config.yml`: Ritrovo's system prompt,
then the conferences the kernel's retrieval found for the question. Retrieval is
the kernel's full-text search over the Live stage only, published items or the
member's own, with every hit the member may not view dropped, so an Incoming or
Curated conference is never in the context. `the_chat_context_carries_live_conferences_and_never_an_incoming_or_curated_one`
reads the request the kernel sends the provider and checks exactly that.

Two things the kernel decides that shape the answers:

- **Every word of the question must match** (`G-CHAT-RAG-REQUIRES-EVERY-WORD`).
  The search joins the question's words with AND, so "What Rust conferences are
  in Europe?" needs "rust", "conferences" and "europe" in one conference's
  indexed text, which has its city and country but neither of the last two.
  Live on the demo (2026-11-23) that question, and "Rust conferences", got no
  context and an honest "Ritrovo has none"; "Rust" got both Live Rust
  conferences, RustLab Firenze 2026 and betterCode() Rust, each with its dates
  and website.
- **The context has no Ritrovo links.** It names each conference and its text
  fields, but not its address on the site (`G-CHAT-CONTEXT-HAS-NO-ITEM-LINKS`).
  The prompt asks for each conference's own website instead, which is a text
  field, in plain text: the widget renders no Markdown.

The chat cannot call a tool (`G-VISITOR-CHAT-HAS-NO-TOOLS`); the editors'
assistant can.

## The conference assistant

The kernel's AI assistant with one scope, `ritrovo_conference`, declared by
`ritrovo_cfp` (`plugins/ritrovo_cfp/src/assistant.rs`). It needs `use ai`,
`use ai assistant` and `edit any content`, so editors and publishers have it and
members do not. On a conference page the kernel shows an editor a launcher to
`/ai/assistant/ritrovo_conference/{conference id}`. The conversation starts from
a snapshot of that conference: its dates, city, website, CFP page and CFP end
date.

The model can call three tools:

| Tool | Kind | What it does |
| :- | :- | :- |
| `find_conferences` | read | upcoming Live conferences by topic, city or both, soonest first, up to ten, each with its address |
| `cfps_closing_soon` | read | open calls for papers closing within some days (default 14, at most 90), soonest first |
| `propose_cfp_end_date` | write | proposes a corrected CFP end date for this conference |

A read runs as soon as the model calls it, through the kernel's gather pipeline,
as the editor and on the Live stage. A write never runs on the model's say-so.
The kernel asks the plugin to describe the change and shows it as a proposal
card ("Change the CFP end date of ... from ... to ..."). Only the editor's Apply
saves it, through the kernel's item service, as a revision with the editor's
reason in its log. A date after the conference ends is refused, and the
proposal is then marked failed when applied
(`G-ASSISTANT-WRITE-CANNOT-REFUSE-A-PROPOSAL`).

`ritrovo_cfp`'s suite drives all of this through the kernel's own routes with
the kernel's scripted provider playing the model: the read runs and returns only
the Live conference, the write proposes and changes nothing until applied, and a
member is refused.

Live on the demo (2026-11-23), as `editor_alice` on Big Data Europe, one
message asked for upcoming Rust conferences and reported a two-day CFP
extension. `find_conferences` returned betterCode() Rust, and the extension
became the card "Change the CFP end date of Big Data Europe from 2026-11-23 to
2026-11-25" with Apply and Discard. Unapplied, the stored date stayed
2026-11-23. Two model calls, 4,089 tokens.

## The MCP server

`trovato-mcp` is the kernel's MCP server: it lets an MCP client, Claude Desktop
for one, list, read, search, create and edit content on the site as one Trovato
user, with exactly that user's permissions. It calls no model and spends no
tokens. It runs on your machine, beside the demo, and reads the demo's database
and Redis directly. These are the steps against `scripts/serve-demo.sh`'s demo,
from the repository root.

1. **Install it from the pinned kernel revision.** The revision is
   `kernel-release.toml`'s, the one the demo's image was built from, so read it
   from there rather than typing it:

   ```bash
   rev="$(sed -n 's/^rev = "\(.*\)"$/\1/p' kernel-release.toml)"
   cargo install --git https://github.com/jeremyandrews/trovato.git --rev "$rev" --locked trovato-mcp
   ```

2. **Give it the database and Redis.** The demo publishes neither; the override
   publishes both, on loopback only. It recreates the two containers, which
   signs everyone out:

   ```bash
   docker compose -f docker-compose.demo.yml -f docker-compose.mcp.yml up -d postgres redis
   ```

   | Variable | Value |
   | :- | :- |
   | `DATABASE_URL` | `postgres://trovato:trovato@127.0.0.1:55433/ritrovo` |
   | `REDIS_URL` | `redis://127.0.0.1:56380/0` |

3. **Give it the demo's plugins and templates.** The server builds the same
   kernel state the site runs, so it loads the same plugins (`ritrovo_access`
   among them, which decides who reads Incoming and Curated conferences) and
   refuses to start without the kernel's templates. Copy them out of the running
   demo:

   ```bash
   mkdir -p ~/.ritrovo-mcp
   docker compose -f docker-compose.demo.yml cp trovato:/app/templates ~/.ritrovo-mcp/kernel-templates
   docker compose -f docker-compose.demo.yml cp trovato:/app/plugins ~/.ritrovo-mcp/kernel-plugins
   docker compose -f docker-compose.demo.yml cp trovato:/ritrovo/overlay/plugins ~/.ritrovo-mcp/ritrovo-plugins
   ```

   | Variable | Value |
   | :- | :- |
   | `TEMPLATES_DIR` | `~/.ritrovo-mcp/kernel-templates:<this checkout>/docs/tutorial/templates` |
   | `PLUGINS_DIR` | `~/.ritrovo-mcp/kernel-plugins:~/.ritrovo-mcp/ritrovo-plugins` |

4. **Get an API token** for the user the client should act as. Signed in, open
   `/user/tokens` and create one; it is shown once. Or from a script, with the
   session's CSRF token from the `csrf-token` meta tag of any page:

   ```bash
   curl -s -b cookies.txt -X POST http://localhost:3000/api/v1/tokens \
     -H 'Content-Type: application/json' -H "X-CSRF-Token: $csrf" \
     -d '{"name":"claude-desktop","expires_in_days":30}'
   # {"id":"...","name":"claude-desktop","token":"<64 hex characters>","expires_at":...}
   ```

   Put the token in a file only you can read, `~/.ritrovo-mcp/token`
   (`chmod 600`). The server takes it with `--token-file`, or `--token`, or the
   `TROVATO_API_TOKEN` variable, and refuses to start on a token that is not
   one. Revoke it on `/user/tokens` when you are done.

5. **Tell Claude Desktop about it**, in `claude_desktop_config.json`, with every
   path absolute (Claude Desktop expands no `~`):

   ```json
   {
     "mcpServers": {
       "ritrovo": {
         "command": "/Users/you/.cargo/bin/trovato-mcp",
         "args": ["--token-file", "/Users/you/.ritrovo-mcp/token"],
         "env": {
           "DATABASE_URL": "postgres://trovato:trovato@127.0.0.1:55433/ritrovo",
           "REDIS_URL": "redis://127.0.0.1:56380/0",
           "TEMPLATES_DIR": "/Users/you/.ritrovo-mcp/kernel-templates:/Users/you/ritrovo/docs/tutorial/templates",
           "PLUGINS_DIR": "/Users/you/.ritrovo-mcp/kernel-plugins:/Users/you/.ritrovo-mcp/ritrovo-plugins"
         }
       }
     }
   }
   ```

The server offers ten tools: `list_items`, `get_item`, `create_item`,
`update_item`, `delete_item`, `search`, `list_content_types`,
`list_categories`, `list_tags` and `run_gather`. What each one may do is the
token's user's permissions, checked as the web checks them: an editor's token
reads Incoming and Curated conferences, a member's reads Live ones only.
`run_gather` cannot run any of Ritrovo's gathers: their ids carry a dot, and the
tool accepts letters, digits and underscores only
(`G-MCP-RUN-GATHER-REFUSES-DOTTED-QUERY-IDS`).

`ritrovo_access`'s suite serves the same server in process over a pipe, with
real tokens: an editor's token lists conference content, a member's cannot read
a Curated conference, a bad token opens no session, and `run_gather` refuses
`ritrovo.upcoming_conferences`.

## AI Assist on the edit form

The kernel's `trovato_ai` plugin adds an **AI Assist** button under every
plain-text field of an item form (`tap_form_alter`), for a user holding `use ai
chat`, and links its script. Nothing in this repository configures it: every
member holds the permission, so the buttons appear for whoever can open the
form, which in Ritrovo is an editor or a publisher. A member is refused the form
(403), and so is a visitor (401).

A press opens a popover with five operations: Rewrite, Expand, Shorten,
Translate and Adjust Tone. The field's text goes to `POST /api/v1/ai/assist` with
Ritrovo's chat model; the answer is shown with **Accept**, which replaces the
field's text, and **Reject**, which leaves it. Nothing is saved until the editor
saves the form.

Live, as editor_alice on Italian Free Software Conference 2026 (2026-11-23):
Shorten on the description's text, in the summary field, answered "The Italian
Free Software Conference celebrates open source and free software, with talks on
licensing, contributing to OSS projects, and open code culture in the Italian
public administration." Reject kept the field; a second run, accepted, replaced
it. Two calls, 361 tokens. The form was not saved.

What to know before relying on it:

- **One press per page.** The second press on the same page answers "Invalid or
  missing CSRF token": the script reuses the page's token and the route spends
  it. Save, or reload, before the next one; a reload loses unsaved edits
  (`G-AI-ASSIST-WORKS-ONCE-PER-PAGE`).
- **No button on the description.** It is a `Blocks` field, and the buttons go
  on plain-text fields only (`G-AI-ASSIST-SKIPS-BLOCK-FIELDS`).
- **No budget.** The route checks no token budget and applies no AI rate limit,
  and logs every token as output (`G-AI-ASSIST-CHECKS-NO-BUDGET`).
- **An empty answer is possible.** The route reads the model's first content
  block only; when the model thinks first, the preview is empty. Reject it
  (`G-AI-ASSIST-READS-ONLY-THE-FIRST-CONTENT-BLOCK`).

`verify-demo.sh` checks the buttons and their script on editor_alice's
conference edit form, that the description has none, and that viewer_carol, a
visitor and the conference page get none. The route is covered by
`ai_assist_shortens_an_editor_s_text_with_ritrovo_s_model_and_refuses_a_visitor`,
`ai_assist_answers_an_empty_result_when_the_model_thinks_first` and
`ai_assist_spends_past_an_exhausted_budget_and_logs_every_token_as_output`,
against the kernel's scripted provider.

## Similar conferences

`/conferences/similar?q=rust+conferences+in+europe` lists Live conferences
ranked by how close their text is to the query. Every conference page links to
it with its own title as the query, and so does the main menu ("Similar"). It is
the kernel's `semantic_similarity` gather operator over pgvector, configured in
`gather_query.ritrovo.similar_conferences.yml`:

1. The demo's Postgres is `pgvector/pgvector:pg16`, and CI's is
   `pgvector/pgvector:pg17`. On a Postgres without the extension the kernel
   starts, keeps no embeddings, and the page lists nothing.
2. Each page view with a `q` embeds `q` once, with the embedding provider, and
   asks pgvector for the nearest items.
3. The items' own embeddings come from the kernel's backfill. The importer writes
   conferences by raw SQL, so no save queues one. As the administrator:

   ```bash
   curl -b <admin session> -X POST -H "X-CSRF-Token: <token from any admin page>" \
     "http://localhost:3000/admin/embed/backfill?limit=5000"
   curl -b <admin session> http://localhost:3000/admin/embed/status
   ```

   The backfill only queues work; cron embeds at most 100 items a run, so the
   demo's cron container gets through it in its own time.

What leaves the site, and what it costs:
- **Every item is sent.** The backfill takes every item in every stage, not
  only Live conferences, and the kernel sends every text field of each, editors'
  notes included (`G-EMBEDDINGS-BYPASS-BUDGETS-AND-THE-USAGE-LOG`).
- **The full backfill is cheap.** On the demo, 5,698 items (every stage, 5,635 of them Incoming) came to 2.48 million characters, at most about 620,000 tokens: under 0.013 USD. Cron took about 40 minutes to work through the queue.
- **A query costs almost nothing:** a few tokens, about two millionths of a dollar.
- **None of it is metered.** No embedding call counts against a budget or appears
  on the usage dashboard (same entry).

With no key, the page answers 200 and says no similar conferences were found.

## Conference summaries

`variable.trovato_ai.field_rules.yml` gives the kernel's `trovato_ai` plugin one
rule: on a save through the edit form, by a member holding `use ai`, write a
two-sentence summary of the description into an empty `field_summary`, which the
conference page shows when present. At the pinned kernel the rule never runs.
`trovato_ai` reads its rules from a column `site_config` does not have, so it
finds none (`G-AI-FIELD-RULES-NEVER-LOAD`). A save as `editor_alice` with both
keys set left the summary empty and made no provider call.

Conferences the importer writes would get no summary anyway: they never pass
through a save (`G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE`).

## With no key

Which is CI, the public demo workflow, the nightly run against the kernel's
`main`, and any install that has not set the two variables. Every AI surface
degrades without an error page and without a word of the provider's answer:

| Surface | Answer |
| :- | :- |
| A visitor's chat request | 401, sign in (with or without a key) |
| The chat tile | shown to members; a question answers "Error: AI is temporarily unavailable" |
| A member's chat request | 503, "AI is temporarily unavailable" |
| The conference assistant | the page opens for an editor; a message ends "The model provider refused the request." and changes nothing |
| AI Assist | the buttons render; a press answers 503 and the popover reads "AI is temporarily unavailable" |
| Search AI | 404, switched off (with or without a key); the Pagefind results are unchanged |
| Similar conferences | 200 with no rows and "No similar conferences found." |
| Translation drafts | none stored; the cron result counts the failure |
| The provider screen | each variable `(not set)` |

The kernel still sends a request with no key, and the provider refuses it
(`FRICTION.md`, `G-AI-REQUESTS-SENT-WITHOUT-A-KEY`). That costs no money, but
the request goes out whole: a member's chat message reaches the provider
even though the site has no account with it. In this demo, with translation
drafts and the search stages off, chat, the assistant and AI Assist are the
surfaces that send anything.
An install that will run without keys should set `enabled: false` on both
providers in `variable.ai_providers.yml`. A disabled provider is never
resolved, so every AI surface gives its "not available" answer without sending
anything.

These are asserted by the host tests in `plugins/ritrovo_access/tests/` and
`plugins/ritrovo_translate/tests/` and, against the running demo, by the "AI"
section of `scripts/verify-demo.sh`, which makes no model call.
