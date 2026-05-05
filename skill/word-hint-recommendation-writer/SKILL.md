---
name: word-hint-recommendation-writer
description: Write and maintain pre-generated AI-looking vocabulary hint recommendations for the word-mobile-rn app, including homophone mnemonics, root-affix breakdowns, confusable-word distinctions, and short story hooks. Use when adding or reviewing entries in the word hint recommendation library for CET4, CET6, kaoyan, or overlapping wordbooks.
---

# Word Hint Recommendation Writer

Use this skill when creating or reviewing the pre-generated hint library shown to users as AI recommendations.

## Output Shape

Each recommendation item represents one word and one hint style:

```json
{
  "id": "ai-language-phonetic",
  "word": "language",
  "style": "phonetic",
  "label": "AI谐音",
  "text": "谐音：language南瓜哥——南瓜哥在说语言",
  "wordbookCodes": ["cet4", "cet6", "kaoyan"]
}
```

Use one canonical recommendation set per normalized English word. If a word appears in multiple wordbooks, keep one set and attach all known `wordbookCodes`.

## Allowed Styles

- `phonetic` / `AI谐音`: Chinese sound-alike or spelling-like hook. It does not need to be etymologically related to the definition, but the sentence must pull back to the meaning.
- `rootAffix` / `AI拆解`: Prefix, root, suffix, or visible spelling chunks that help recall the meaning.
- `similarWord` / `AI辨析`: Pair the target with a confusable word and make the difference memorable.
- `story` / `AI典故`: A short origin-like, image-like, or usage story. If the origin is only a mnemonic, do not imply it is true etymology.

## Text Formats

Keep the prefix exactly as below so the UI reads consistently:

- 谐音：`谐音：<word><中文钩子>——<中文钩子画面>，记“<核心释义>”`
- 拆解：`拆解：<word>=<chunk>+<chunk>——<chunk解释>，所以是“<核心释义>”`
- 辨析：`辨析：<word> vs <confusable>——<target钩子>；<confusable钩子>`
- 典故：`典故：<word><画面/来源>——<一句故事>，记“<核心释义>”`

Examples:

- `谐音：language南瓜哥——南瓜哥在说语言`
- `拆解：federal=feder+al——feder 表示联盟，-al 表示“……的”，所以是“联邦的”`
- `辨析：principle vs principal——principle 结尾 le 像 law，是“原则”；principal 结尾 pal，常指“校长/主要的”`

## Quality Rules

- Prefer one or two strong hints per word; add a third only for genuinely confusing or high-frequency words.
- For lookalike words, use different hooks for each word and cross-link both entries when both exist.
- Avoid hooks that can make users remember the wrong meaning.
- Keep each text short enough for a mobile card: usually under 36 Chinese characters after the dash, unless a distinction needs more.
- Use exam-relevant core meanings first. Add secondary meanings only when they are common traps.
- Use readable Chinese punctuation: `：`, `——`, `；`, `“”`.
- Do not duplicate the same Chinese hook across unrelated words.

## Review Checklist

Before saving a batch, check:

- Every item has `id`, `word`, `style`, `label`, `text`, and `wordbookCodes`.
- `id` is stable: `ai-<word>-<style-or-hook>`.
- `word` is lowercase and normalized.
- The text starts with one of `谐音：`, `拆解：`, `辨析：`, `典故：`.
- Overlapping CET4/CET6/kaoyan words are represented once, not once per wordbook.

## Regeneration

To rebuild the full bundled-vocabulary seed library from bundled vocabulary assets, run:

```bash
node skill/word-hint-recommendation-writer/scripts/generate_word_hint_recommendations.mjs
```

The script preserves existing hand-written recommendations, fills missing words from seed `remMethod`/translation data, and refreshes both `crates/platform-mobile/resources/wordbook_overlap_index.json` and `crates/platform-mobile/resources/wordbook_vocabulary_index.json`.
