# Feature: AI Passage

> Slug: `ai-passage`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

Generate one daily Chinese AI passage from the learner's current-day wrong words so the learner reviews difficult vocabulary in context instead of only as isolated cards.

## UX Contract

- The Today page shows an `AI 短文总结` entry after daily learning tasks are complete.
- If today's passage does not exist, the Today entry shows a generate action.
- If today's passage already exists, the Today entry shows the passage content directly instead of requiring a jump to the AI page.
- Only one AI passage can be generated per day.
- The passage is only mounted on Today for the current day. On the next day, the Today entry returns to the task-complete generate state after the user completes the new day's work.
- The AI page shows today's/latest passage and passage history inside the chat/workbench flow.
- The passage body highlights target English words and shows Chinese glosses inline.
- The body should contain only the top ten wrong words by priority. Remaining wrong words should be appended after the body as `其他错词：...`.
- Users can express style preferences in the AI passage mode. These preferences should affect the next or future generation instead of merely redisplaying the current passage.
- If the user asks to generate a passage before finishing today's tasks, the AI passage mode should answer that today's learning must be completed first.

## Shared Domain/Data Contract

The Rust bridge owns AI passage generation, validation, storage, and parsing. Flutter should render the returned structure instead of reconstructing passage semantics.

Core payload fields:

- `id`
- `date`
- `title`
- `blocks`
- `segments`
- `entryId` references for highlighted words
- `validationStatus`
- `style`
- `generatedAt`
- `preview`
- `otherWrongWords` or equivalent trailing block for words not used in the body

Priority rule for target wrong words:

1. Select only current-day wrong words.
2. Exclude mastered words and root/affix-only items from passage targets.
3. Sort by today's wrong count descending.
4. If tied, sort by total wrong count descending.
5. Use the top ten in the passage body.
6. List the remaining words directly after the passage as `其他错词：`.

Marker contract:

- The passage tool/text must use `[[word:ENTRY_ID]]` for target markers.
- Model-generated labels such as `[[coalition:5543]]` are invalid as source format, but the mobile client should normalize older saved passages where possible.

## Flutter Mobile Route

- Owner screen/widget: `TodayShellScreen`, `AiScreen`, `MobileRootShell` reload/cache callbacks.
- SDK/bridge calls: get Today AI context, generate passage, get passage history, get passage detail, save/load style preference.
- Loading/cache/reload behavior: after generation, invalidate Today and AI cache scopes and force-refresh Today AI context so the Today card changes from button to passage immediately.
- Orientation/gesture constraints: normal portrait; passage cards must dynamically size to content and preserve highlight/gloss wrapping.
- First implementation slice: daily generation, direct Today rendering, AI page workbench view, history viewing.
- Current status: mobile in progress with known verification gaps caused by unrelated workspace encoding/build blockers.

## Tauri Desktop Route

- Owner view/window: desktop Today/dashboard and AI workspace.
- Shared APIs to reuse: Rust passage generation, validation, history, style preference, and marker parsing APIs.
- Desktop-specific layout: use a larger reading pane with history side list or inspector; do not copy the mobile bottom composer or Today card size constraints.
- Mobile assumptions to avoid: mobile bottom navigation, small card truncation, keyboard overlap workarounds, and phone-only Today card layout.
- First parity slice: read existing passage history, render highlighted passage segments, and generate today's passage through the same Rust contract.
- Current status: not started.

## Sync And Storage

- Passage history should restore from Supabase/cloud for the active account.
- Today should only show the passage whose `date` matches the local Today date.
- Historical passages remain available from the AI page/history flow.
- Style preference is user setting state and should eventually sync if settings sync is enabled.
- Cache invalidation matters: generating a passage changes AI history and Today state.

## AI Or Provider Implications

The passage format should be enforced by a dedicated agent tool:

- Tool name: `write_ai_passage`.
- Tool arguments: `{ "title": string, "paragraphs": string[] }`.
- Paragraph markers: `[[word:ENTRY_ID]]` only.
- Anthropic route should read `tool_use.input`.
- OpenAI Responses route should read `function_call.arguments`.
- If a middle station or provider does not support tools, text JSON fallback can be used, but it must parse to the same payload shape.

Provider/fallback lessons from mobile debugging:

- A 503 from one middle station does not necessarily mean the API key is bad.
- Prompt size can contribute to failures when many wrong words are included; the Top10 body rule reduces prompt and output complexity.
- Primary/secondary/backup provider failures should report which leg failed without exposing internal debug noise to normal users.

## Implementation Log

- `2026-04`: AI passage initially had a direct generation page and exposed provider/debug errors too visibly.
- `2026-04`: Today entry was changed from navigation-only to generation/display logic.
- `2026-05`: Daily generation was limited to one passage per day.
- `2026-05`: Passage generation was changed to use all current-day wrong words for counting, but only a bounded body set for readability.
- `2026-05`: Body target selection was changed to top ten by today's wrong count, then total wrong count; remaining words should appear under `其他错词：`.
- `2026-05`: Prompt/style direction was changed to avoid default classroom scenes and allow more imaginative contexts.
- `2026-05`: Root/affix-only wrong items were excluded from AI passage targets.
- `2026-05`: AI page was reshaped into a chat/workbench where AI passage viewing and wrong-word import are function modes.
- `2026-06-25`: Passage format enforcement moved from prompt-only toward the `write_ai_passage` agent tool contract.
- `2026-06-25`: Feature record rewritten from current conversation to preserve mobile lessons and desktop parity route.

## Mobile Lessons Learned

- Prompt-only format instructions are not enough. The model produced bad markers such as `[[coalition:5543]]`; use an agent tool and keep parser normalization for old saved content.
- Today must force-refresh after generation. Route reload alone can leave the Today card showing a stale generate button.
- Passage cards must size to full content. Fixed-height cards hide text and make highlights/glosses look broken.
- Highlight/gloss rendering can disappear if passage segment reconstruction falls back to plain text; client normalization must preserve word segments.
- History restoration must read account/cloud-backed passage history, not only the current local context.
- Limiting the body to top ten wrong words keeps the passage readable and reduces provider/prompt failure risk while still exposing all wrong words via `其他错词：`.
- Style requests in AI passage mode should update future generation behavior, not just redisplay the existing passage.
- Chinese UI strings are fragile in this workspace; avoid broad string rewrites and verify visible labels after edits.

## Desktop Follow-Up Notes

- Start from the Rust/domain passage contract, not from Flutter UI structure.
- Reuse `write_ai_passage` and parser/validation semantics exactly so desktop and mobile do not diverge on marker rules.
- Render passage segments with the same highlight/gloss information, but use desktop-friendly layout: reading pane, history list, relation/word inspector.
- Desktop can expose style preferences as a side panel or prompt field; it should make clear whether a style affects the existing passage or the next generation.
- Desktop should not copy the mobile one-card Today limitation; it can show passage summary and full passage in separate areas.

## Route Changes

- `2026-05`: AI passage stopped being a pure AI-tab destination; Today became a direct display surface.
- `2026-05`: The word selection strategy changed from a hard small visible count to current-day all wrong words with top-ten body priority.
- `2026-05`: AI passage style became conversational preference state rather than a one-off static view.
- `2026-06-25`: Format contract moved toward an agent tool so the model returns structured arguments rather than free-form JSON.

## Known Pitfalls

- Do not generate from non-current-day wrong words.
- Do not include root/affix-only mistakes as passage target words.
- Do not include more than ten words in the passage body when many wrong words exist.
- Do not lose the remaining wrong words; append them under `其他错词：`.
- Do not show yesterday's passage in Today as if it were today's generated passage.
- Do not expose raw bridge/provider errors as the main user-facing copy.
- Do not rely on prompt wording alone for marker format.
- Do not let mobile-only cache behavior define the desktop contract.

## Verification

- Mobile: verify task incomplete response, generate button after task completion, one passage per day, Today card changes to content after generation, next-day reset, AI page rendering, history restoration, highlight/gloss rendering, style preference save/use, Top10 plus `其他错词：` behavior.
- Desktop: pending; first parity slice should render history and current passage from the shared Rust contract.
- Shared/domain: verify current-day filtering, mastered/root-affix exclusion, top-ten sorting, marker parser, tool output parsing, text JSON fallback parsing, and cloud restore shape.

## Current Blockers Or Caveats

- Current workspace has unrelated Rust source encoding/parse errors in study-core files, which can block full `cargo check` or Rust tests until repaired.
- Some Flutter analyzer checks previously passed for focused AI files, but broader checks have been affected by unrelated workspace state and should be re-run before release.