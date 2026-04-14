# Word Mobile Cross-Platform Contract

**Version:** 0.1.0  
**Status:** Draft  
**Owner:** Rust shared core (`word-storage-core`, `word-app-core`)

This document defines the cross-platform contract between the Rust backend and TypeScript frontend clients (desktop and mobile).

## Principles

1. **Rust owns the truth** - DTOs are defined in Rust and serialized via serde
2. **camelCase JSON** - All JSON fields use camelCase naming
3. **Explicit nulls** - Optional fields are `T | null`, not `T | undefined`
4. **ISO 8601 timestamps** - All timestamps are ISO 8601 strings

---

## Common Types

### AnswerOutcome

```typescript
type AnswerOutcome = "correct" | "fuzzyCorrect" | "incorrect" | "skipped";
```

### SessionMode

```typescript
type SessionMode = "newWord" | "review" | "mixedTest" | "wrongWordReinforcement";
```

### QuestionType

```typescript
type QuestionType = "enToCnChoice" | "exampleToCnChoice" | "cnToEnChoice" | "enToCnInput";
```

---

## Bootstrap API

### Request

None - bootstrap is called automatically on app start.

### Response: BootstrapState

```typescript
interface BootstrapState {
  appReady: boolean;
  firstRunRequired: boolean;
  databaseStatus: "ready" | "init_failed";
  snapshotStatus: "ready" | "missing_required" | "empty";
  connectivityStatus: "offline" | "online";
  aiConfigStatus: "configured" | "missing";
  settingsEntryAvailable: boolean;
  blockingReason: string | null;
}
```

### Errors

- Database initialization failure: `appReady: false`, `databaseStatus: "init_failed"`
- Missing required snapshot on first run: `appReady: false`, `blockingReason: string`

---

## Today Home API

### Request

None - today home is read automatically after bootstrap.

### Response: TodayHomeState

```typescript
interface TodayHomeState {
  todayDate: string; // ISO 8601 date
  activePlan: PlanSummary | null;
  todaySnapshot: DailySnapshot | null;
  wordbooks: WordbookSummary[];
  dailyProgress: DailyProgress;
}

interface PlanSummary {
  id: number;
  name: string;
  newWordsPerDay: number;
  reviewWordsPerDay: number;
  mixedTestPerDay: number;
  wrongWordTestPerDay: number;
  growthIntervalDays: number;
  growthIncrement: number;
}

interface DailySnapshot {
  date: string;
  newWordsTarget: number;
  newWordsCompleted: number;
  reviewWordsTarget: number;
  reviewWordsCompleted: number;
  mixedTestTarget: number;
  mixedTestCompleted: number;
  wrongWordTestTarget: number;
  wrongWordTestCompleted: number;
}

interface WordbookSummary {
  id: number;
  code: string;
  name: string;
  category: string;
  totalEntries: number;
  isActive: boolean;
}

interface DailyProgress {
  totalTasks: number;
  completedTasks: number;
  nextRecommendedAction: string;
}
```

---

## Study Session API

### Start Session

#### Request: StartSessionRequest

```typescript
interface StartSessionRequest {
  mode: SessionMode;
  wordbookId: number | null;
  entrySourceIds: string[];
}
```

#### Response: StartSessionResponse

```typescript
interface StartSessionResponse {
  session: StudySession;
  currentQuestion: StudyQuestion;
  progress: SessionProgress;
}

interface StudySession {
  sessionId: string;
  mode: SessionMode;
  totalWords: number;
  wordbookId: number | null;
  startedAt: string; // ISO 8601
}

interface StudyQuestion {
  questionId: string;
  questionType: QuestionType;
  entrySourceId: string;
  word: string;
  prompt: string;
  acceptedMeanings: string[];
  exampleSentence: string | null;
  exampleTranslation: string | null;
  choices: ChoiceOption[] | null;
  correctChoiceLabel: string | null;
  questionIndex: number;
  totalQuestions: number;
}

interface ChoiceOption {
  text: string;
  label: string;
}

interface SessionProgress {
  current: number;
  total: number;
}
```

### Submit Answer

#### Request: SubmitAnswerRequest

```typescript
interface SubmitAnswerRequest {
  questionId: string;
  response: string; // choice label or text input
  responseTimeMs: number;
}
```

#### Response: SubmitAnswerResponse

```typescript
interface SubmitAnswerResponse {
  result: StudyResult;
  isComplete: boolean;
  currentQuestion: StudyQuestion | null;
  summary: SessionSummary | null;
  nextAction: string | null;
  progress: SessionProgress;
}

interface StudyResult {
  questionId: string;
  entrySourceId: string;
  questionType: QuestionType;
  userResponse: string;
  normalizedResponse: string | null;
  correctAnswer: string;
  outcome: AnswerOutcome;
  responseTimeMs: number;
  answeredAt: string; // ISO 8601
}

interface SessionSummary {
  sessionId: string;
  totalQuestions: number;
  correctCount: number;
  fuzzyCorrectCount: number;
  incorrectCount: number;
  skippedCount: number;
  totalWords: number;
  wrongWordCount: number;
  accuracyPercent: number;
  totalTimeMs: number;
  completedAt: string; // ISO 8601
}
```

### Complete Session

#### Request

None - complete is called when isComplete is true, or explicitly.

#### Response: CompleteSessionResponse

```typescript
interface CompleteSessionResponse {
  summary: SessionSummary;
  nextAction: string;
}
```

### Cancel Session

#### Request

None - cancel is called explicitly.

#### Response

Empty response on success.

---

## Reports API

### Request

None - reports are read with optional date range filters.

### Response: ReportsState

```typescript
interface ReportsState {
  dailyStats: DailyStat[];
  weeklyStats: WeeklyStat[];
  monthlyStats: MonthlyStat[];
  streakInfo: StreakInfo;
}

interface DailyStat {
  date: string;
  totalQuestions: number;
  correctCount: number;
  accuracyPercent: number;
  studyTimeMs: number;
}

interface WeeklyStat {
  weekStart: string;
  totalQuestions: number;
  studyDays: number;
}

interface MonthlyStat {
  month: string; // YYYY-MM
  totalQuestions: number;
  studyDays: number;
}

interface StreakInfo {
  currentStreak: number;
  longestStreak: number;
  lastStudyDate: string | null;
}
```

---

## Wrong Words API

### Request

None - wrong words list is read automatically.

### Response: WrongWordsState

```typescript
interface WrongWordsState {
  totalWrongWords: number;
  activeWrongWords: number;
  words: WrongWordEntry[];
}

interface WrongWordEntry {
  entryId: number;
  word: string;
  errorCount: number;
  lastWrongAt: string; // ISO 8601
  priorityScore: number;
}
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-04-09 | Initial contract definition |

---

## TypeScript Generation

TypeScript types are generated from Rust DTOs using `ts-rs` or similar tooling. The generated types should match this document exactly.

To regenerate types:

```bash
cd word-mobile-rn
./scripts/generate-ts-types.sh
```
