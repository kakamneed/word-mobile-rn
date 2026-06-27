# Feature: Wrong Words Page

> Slug: `wrong-words-page`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

Help learners review real weak points from study activity without mixing unrelated learned words, duplicate book entries, or root/affix cards into one confusing list.

The page should make wrong answers actionable:

- show only items with actual mistakes in the visible wrong list
- preserve learned records in the background for review scheduling
- expose error count, priority, risk by question type, history, meanings, and examples
- separate ordinary wrong words from wrong root/affix items

## UX Contract

- The Wrong Words tab shows a wrong-word notebook with filter chips, reinforcement entry, and grouped lists.
- Visible wrong list excludes entries with `errorCount == 0`.
- A selected item expands inline near the selected card, with the detail section top visible.
- Selecting an item should reliably scroll to the detail label area, not to page bottom.
- Detail includes:
  - word/root text
  - phonetic when available
  - part of speech or `root`
  - meanings
  - question-type risk breakdown with Chinese labels
  - recent error history with Chinese labels
  - examples, with the wrong word highlighted inside English example sentences
  - return button to scroll back to the selected card
- Ordinary wrong words and wrong root/affix items are displayed in separate sections.
- Root/affix wrong items must show their examples and meanings by resolving the original virtual card data.

## Shared Domain/Data Contract

Primary read contracts:

- Flutter SDK: `WrongWordsClient.getWrongWords(filter)`
- Flutter SDK: `WrongWordsClient.getWrongWordDetail(entryId)`
- Bridge methods: `getWrongWords`, `getWrongWordDetail`
- Rust domain service: `word_app_core::services::wrong_words_service::build_wrong_words`
- Mobile bridge loaders: `load_wrong_word_entries`, `load_wrong_word_detail_payload_with_bundle`

List item shape:

```json
{
  "entryId": 123,
  "sourceEntryKey": "root_affix_shared_abs",
  "word": "abs",
  "phoneticUs": null,
  "phoneticUk": null,
  "partOfSpeech": "root",
  "meanings": ["离开"],
  "errorCount": 1,
  "lastWrongAt": "2026-04-30T16:01:00Z",
  "priorityScore": 6.7,
  "entryKind": "rootAffix",
  "isActive": true
}
```

Detail item shape:

```json
{
  "entryId": 123,
  "word": "abs",
  "lemma": "abs",
  "partOfSpeech": "root",
  "meanings": [
    {
      "pos": "root",
      "meaningCn": "离开",
      "meaningEn": null
    }
  ],
  "examples": [
    {
      "sentenceEn": "absorb",
      "sentenceCn": "吸收"
    }
  ],
  "riskBreakdown": [
    {
      "questionType": "rootToGlossInput",
      "attempts": 1,
      "incorrect": 1,
      "skipped": 0,
      "fuzzyCorrect": 0,
      "correct": 0
    }
  ],
  "errorHistory": [
    {
      "date": "2026-04-30T16:01:00Z",
      "context": "rootToGlossInput / incorrect"
    }
  ],
  "relatedWords": []
}
```

Contract rules:

- Visible wrong entries must have `errorCount > 0`.
- Duplicate entries with the same lowercased word merge for display only when they share the same `entryKind`.
- `entryKind` is currently `word` or `rootAffix`.
- Root/affix classification is based on root/affix question types and/or part of speech values such as `root`, `prefix`, `suffix`, `affix`.
- Imported wrong words default to `entryKind: "word"`.
- Root/affix virtual entries may not have normal `entry_meanings` or `entry_examples`; detail and list payloads must enrich from bundled root/affix assets by `sourceEntryKey`.
- Question type and outcome strings may be JSON-encoded and must be normalized before display labels.

## Flutter Mobile Route

- Owner screen/widget: `apps/flutter_mobile/lib/features/wrong_words_screen.dart`
- SDK/bridge calls:
  - `WordSdk.wrongWords.getWrongWords(filter)`
  - `WordSdk.wrongWords.getWrongWordDetail(entryId)`
- Loading/cache/reload behavior:
  - load list on screen init and filter changes
  - load detail on selection
  - clear selected detail if selected entry disappears after reload
  - reload after reinforcement flow returns
- Orientation/gesture constraints:
  - phone-first vertical list
  - detail expansion must work for upper-list cards as well as lower-list cards
  - scroll-to-detail uses a stable detail anchor near the detail card top
  - return button scrolls back to the selected card
- First implementation slice: list, filters, inline detail, reinforcement entry.
- Current status: mobile implemented and actively corrected from user screenshots.

Flutter presentation rules:

- `WrongWordEntry.isRootAffix` drives section split.
- `_buildWrongEntrySection` renders ordinary words and root/affix sections with the same tile/detail behavior.
- `_questionTypeLabel`, `_outcomeLabel`, and `_historyContextLabel` translate stored enum values to Chinese UI labels.
- Example highlighting is presentation-only and should not affect stored text.

## Tauri Desktop Route

- Owner view/window: future desktop Wrong Words view.
- Shared APIs to reuse:
  - same wrong words list/detail contracts
  - same `entryKind` semantics
  - same Rust priority and merge logic
- Desktop-specific layout:
  - use a split view: left table/list with filters, right detail inspector
  - show ordinary wrong words and root/affix wrong items as tabs or grouped table sections
  - support keyboard navigation and wider risk/history tables
- Mobile assumptions to avoid:
  - no need for auto-scroll-to-inline-detail as the primary interaction
  - do not copy compact phone card spacing directly
  - do not hide detailed history behind a long vertical scroll when desktop has room for panes
- First parity slice:
  - render wrong words and root/affix wrong items separately
  - show detail inspector with meanings, examples, risk breakdown, and recent errors
  - verify duplicate merging and zero-error filtering
- Current status: not started.

## Sync And Storage

Local source of truth:

- `study_results` for wrong outcomes, timestamps, question types, and history
- `entries`, `entry_meanings`, `entry_examples` for normal word details
- bundled root/affix assets for virtual root/affix card meanings and examples
- `imported_wrong_words` for AI/OCR-assisted imported items

Storage implications:

- Learned-but-not-wrong entries can remain in persistence for review scheduling but must not show in the visible wrong list with 0 errors.
- Wordbook duplicate entries should not create duplicate cards when they represent the same visible word and same entry kind.
- Root/affix virtual entries need `sourceEntryKey` retained so detail can resolve original asset data.
- Sync should preserve enough answer-level history to rebuild wrong list/detail after device migration.

Offline behavior:

- Wrong list and detail should work from local persistence and bundled assets.
- AI/imported wrong words should degrade gracefully if cloud sync is unavailable.

## AI Or Provider Implications

Related but not required for core wrong-word list:

- imported wrong words from AI/OCR sources use `imported_wrong_words`
- AI passage generation may select target wrong words from the wrong pool

AI must not invent wrong-word priority, error count, or risk breakdown. Those stay Rust/domain owned.

## Implementation Log

- `2026-04-29`: User reported wrong words page showing 0-error entries. Mobile-visible list was changed to filter `errorCount > 0` while keeping learned records available in storage.
- `2026-04-29`: User reported needing to manually scroll to detail. Inline detail expansion and return-to-selected-card behavior were added.
- `2026-04-29`: User refined scroll behavior: scroll should reveal the detail label top, not page bottom.
- `2026-04-29`: User reported unstable scroll for upper-list items. Detail placement was moved inline after the selected tile instead of relying on a page-bottom detail target.
- `2026-04-29`: User reported duplicate wrong words. Display merge logic was added for duplicate lowercased words.
- `2026-04-29`: User requested examples highlight the wrong word. Flutter detail examples now render highlighted occurrences.
- `2026-05-01`: User requested ordinary wrong words and wrong root/affix items be separated. Added `entryKind` and split mobile sections.
- `2026-05-01`: User reported root/affix wrong items lacked examples and meanings. Bridge enrichment now resolves root/affix assets by `sourceEntryKey`.
- `2026-06-25`: Ledger created from conversation history using `cross-platform-feature-ledger`.

## Mobile Lessons Learned

- A bottom-only detail card creates fragile scroll behavior, especially for items near the top of a long list.
- Inline detail after the selected tile is more stable and makes the return button meaningful.
- `Scrollable.ensureVisible` must target a stable detail anchor near the label/top of the detail section.
- Zero-error learned entries are useful for review logic but visually noisy and misleading in a wrong-word notebook.
- Duplicate book entries need domain-level display merging; UI-only dedup is not enough.
- Merge keys must include `entryKind`, otherwise a root like `abs` can merge with a normal word-like item.
- Root/affix study cards are virtual. If detail reads only normal entry tables, examples and meanings disappear.
- Question type strings like `enToCnChoice` and outcomes like `incorrect` must be translated for users; leaking enum names looks broken.
- Highlighting wrong words in examples helps users connect the detail card to the mistake context.

## Desktop Follow-Up Notes

- Start with a two-pane desktop layout instead of reproducing mobile inline expansion.
- Preserve `entryKind` grouping visibly; tabs `Words` and `Roots/Affixes` are likely clearer than one long grouped list.
- Desktop can show risk breakdown as a table with localized labels and raw enum debug text hidden behind a developer affordance if needed.
- Desktop should reuse Rust detail enrichment for root/affix entries rather than loading assets directly in UI code.
- Add search and sort only after parity with filtering, grouping, detail, and reinforcement entry.

## Route Changes

- `2026-04-29`: Detail route changed from page-bottom target to inline selected-card expansion.
- `2026-04-29`: Wrong-word visible list changed from all learned/seen candidates to actual wrong entries only.
- `2026-05-01`: Wrong-word display identity changed from `word` to `(entryKind, lower(word))` for duplicate merging.
- `2026-05-01`: Root/affix wrong items split from ordinary wrong words through `entryKind`.
- `2026-05-01`: Root/affix details changed to enrich from bundled virtual-card assets when normal entry tables are empty.

## Known Pitfalls

- Do not show 0-error entries in the visible wrong-word list.
- Do not remove learned records from storage just because they are hidden from the wrong page.
- Do not merge ordinary words and root/affix entries solely by surface text.
- Do not place detail only at the end of the page.
- Do not target page bottom when scrolling to detail; target the detail section top.
- Do not leak enum keys such as `cnToEnChoice`, `rootToGlossInput`, `incorrect`, or `skipped` in user-facing text.
- Do not assume root/affix entries have normal `entry_meanings` or `entry_examples`.
- Do not make Flutter the source of wrong-word score or risk truth.

## Verification

- Mobile:
  - `flutter analyze --no-pub`
  - Manual checks from screenshots:
    - no visible 0-error wrong cards
    - duplicate `cancel`/similar duplicate cards merge when same kind
    - selecting top-list items reliably reveals detail top
    - return button scrolls back to selected card
    - ordinary wrong words and root/affix wrong items are separated
    - example sentence highlights the wrong word
    - root/affix detail shows meaning and examples
- Desktop:
  - Not implemented.
- Shared/domain:
  - `cargo test -p word-app-core wrong_words_service`
  - `cargo test -p word-platform-mobile wrong_word_entries_mark_root_affix_items`
  - Regression: duplicate word entries merge by kind.
  - Regression: word and root/affix entries with same text stay separate.
  - Regression: root/affix wrong list and detail are enriched from asset-backed card data.
