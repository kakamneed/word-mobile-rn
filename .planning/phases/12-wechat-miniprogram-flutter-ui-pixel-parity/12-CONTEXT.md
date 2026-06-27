# Phase 12 Context - WeChat Mini Program Flutter UI Pixel Parity

## Source of Truth

The Flutter screenshots are canonical for this phase. The odd-numbered images show the current WeChat Mini Program state and are only used to identify gaps.

Reference images:

| Pair | Current Mini Program | Flutter Target | Surface |
| --- | --- | --- | --- |
| 1/2 | `D:/xwechat_files/wxid_ry4e83m0cson22_d13f/temp/RWTemp/2026-05/9e20f478899dc29eb19741386f9343c8/3ce98e5cbb5002c6f5918bf8c84fe375.jpg` | `D:/xwechat_files/wxid_ry4e83m0cson22_d13f/temp/RWTemp/2026-05/9e20f478899dc29eb19741386f9343c8/d94d402b62be1053ac9bab5f50fb8d67.jpg` | Today hero and task breakdown |
| 3/4 | `D:/xwechat_files/wxid_ry4e83m0cson22_d13f/temp/RWTemp/2026-05/9e20f478899dc29eb19741386f9343c8/64ab65c8f7e52cfa5fa8504528119c37.jpg` | `D:/xwechat_files/wxid_ry4e83m0cson22_d13f/temp/RWTemp/2026-05/9e20f478899dc29eb19741386f9343c8/fbc2b71fd7cecc22377d4d4787e7b8e6.jpg` | Today lower sections, reward, diagnostics; skip AI summary for mini program v1 |
| 5/6 | `D:/xwechat_files/wxid_ry4e83m0cson22_d13f/temp/RWTemp/2026-05/9e20f478899dc29eb19741386f9343c8/b2761e78a3354f65fe53936a2960af97.jpg` | `D:/xwechat_files/wxid_ry4e83m0cson22_d13f/temp/RWTemp/2026-05/9e20f478899dc29eb19741386f9343c8/fa6347924e33487cbb24cc7e64835263.jpg` | Plan, Wrong Words, Reports, account drawer |

## Locked Product Decisions

1. Flutter is the visual implementation reference; current mini program English cards are placeholders to replace.
2. Backend, study generation, plan semantics, wrong-word state, and report state remain sourced from the existing backend/domain work. This phase changes presentation and UI composition first.
3. Account is not a bottom tab. It is opened from the avatar/account button as a drawer overlay, matching Flutter.
4. Bottom navigation has four tabs for this mini program release: Today, Plan, Wrong, Reports. It needs icons and a selected lavender pill, so the mini program must use a custom tab bar or an equivalent custom in-page navigation component.
5. The default WeChat native navigation title bar must be removed for parity. The app shell owns safe-area padding, page titles, and avatar placement.
6. AI-related UI is deliberately omitted from the mini program first version: no AI tab, no AI page, no AI summary card, and no AI generation button.

## Visual Tokens

Use these as the first-pass mini program tokens, then tune against screenshots:

- Page background: `#fbf7ff`
- Primary purple card: `#6b5b95`
- Selected lavender: `#eee6ff`
- Text primary: `#211c28`
- Text secondary: `#6f6878`
- Muted rail: `#e4dee9`
- Card background: `#ffffff`
- Card border: `#eee7f2`
- Card radius: `16px`
- Metric chip radius: `10px`
- Page horizontal padding: `16px`
- Section gap: `16px`
- Major title font: `32px`, weight `600`
- Card title font: `22px`, weight `600`
- Hero title font: `28px`, weight `700`
- Body font: `16px`
- Caption font: `13px`

## Current Gaps to Close

- Native WeChat chrome and page titles are still visible, which pushes content down and makes the UI unlike Flutter.
- Current pages use English placeholder text and green cards; Flutter uses Chinese copy, a purple/lavender system, and denser cards.
- Current tab bar has four text-only tabs and still treats Account as a tab. The mini program target keeps four learning tabs but adds icons/selected-pill styling and moves account to a drawer.
- Current Today page lacks the large task-oriented hero, progress rails, task rows, reward visual, and diagnostics card. The Flutter AI summary card is intentionally skipped.
- Current Plan page has overlapping English text and lacks editor cards, stepper controls, wordbook radios, and sticky actions.
- Current Wrong Words page lacks the purple summary, filter/reinforcement cards, and wrong-word list card style.
- Current Reports page uses a bar-like placeholder instead of Flutter's line trend chart and mode cards.
