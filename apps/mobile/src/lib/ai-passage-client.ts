import {
  generateAiPassage as bridgeGenerateAiPassage,
  getAiProviderConfig,
  getAiPassage,
  getAiPassageHistory,
  getTodayAiPassageContext,
  type AiProviderConfig,
  type AIPassage,
  type AIPassageHistoryItem,
  type PassageBlock,
  type TodayAiPassageContext,
  type WrongWordInput,
} from './mobile-bridge';

export type {
  AiProviderConfig,
  AIPassage,
  AIPassageHistoryItem,
  PassageBlock,
  TodayAiPassageContext,
};

let cachedTodayContext: TodayAiPassageContext | null = null;
const cachedLatestPassagesByDate = new Map<string, AIPassage>();

export async function fetchTodayPassageContext(): Promise<TodayAiPassageContext> {
  const next = await getTodayAiPassageContext();
  cachedTodayContext = next;
  return next;
}

export function getCachedTodayPassageContext(): TodayAiPassageContext | null {
  return cachedTodayContext;
}

export async function generateTodayPassage(level: string): Promise<AIPassage> {
  const context = await getTodayAiPassageContext();

  if (!context.tasksComplete) {
    throw new Error('请先完成今日任务，再生成 AI 短文。');
  }

  if (context.wrongWords.length === 0) {
    throw new Error('今天还没有可用于生成短文的错词。');
  }

  const passage = await bridgeGenerateAiPassage({
    targetWords: context.wrongWords.map(item => item.word),
    wrongWords: context.wrongWords,
    date: context.date,
    level,
  } as {
    targetWords: string[];
    wrongWords: WrongWordInput[];
    date: string;
    level: string;
  });
  rememberPassage(context.date, passage);
  return passage;
}

export async function fetchPassageHistory(): Promise<AIPassageHistoryItem[]> {
  return getAiPassageHistory();
}

export async function fetchAiProviderConfig(): Promise<AiProviderConfig> {
  return getAiProviderConfig();
}

export async function fetchPassage(passageId: string): Promise<AIPassage | null> {
  const passage = await getAiPassage(passageId);
  if (passage?.generatedAt) {
    rememberPassage(passage.generatedAt.slice(0, 10), passage);
  }
  return passage;
}

export async function fetchLatestPassageForDate(
  date: string,
): Promise<AIPassage | null> {
  const cached = cachedLatestPassagesByDate.get(date);
  if (cached) {
    return cached;
  }
  const history = await getAiPassageHistory();
  const latest = history.find(item => item.generatedAt.slice(0, 10) === date);
  if (!latest) {
    return null;
  }
  const passage = await getAiPassage(latest.passageId);
  if (passage) {
    rememberPassage(date, passage);
  }
  return passage;
}

export function getCachedLatestPassageForDate(date: string): AIPassage | null {
  return cachedLatestPassagesByDate.get(date) ?? null;
}

function rememberPassage(date: string, passage: AIPassage): void {
  cachedLatestPassagesByDate.set(date, passage);
}
