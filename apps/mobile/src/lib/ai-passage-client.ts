import {
  getTodayAiPassageContext,
  type AIPassage,
  type AIPassageHistoryItem,
  type PassageBlock,
  type TodayAiPassageContext,
  type WrongWordInput,
} from './mobile-bridge';

export type {AIPassage, AIPassageHistoryItem, PassageBlock, TodayAiPassageContext};

const PRIMARY_URL = 'http://103.38.81.122:8080/v1/messages';
const PRIMARY_MODEL = 'claude';
const PRIMARY_KEY =
  'sk-181b95262e1eaebe50313d416f5ed81e38164b7d71494f3e241f3149e63a3e31';

const BACKUP_URL = 'http://107.182.173.201:8080/v1/responses';
const BACKUP_MODEL = 'gpt-5.4';
const BACKUP_KEY =
  'sk-d0fea41ec127dc71bbeb14da6a2507cc0bd7d92c740c512db67242d64358567d';

const PROMPT_VERSION = '2.1';
const DEFAULT_STYLE = 'default';

const PROMPT_TEMPLATE = `You are a Chinese language learning assistant. Your task is to write a short, readable Chinese passage around specific English vocabulary words provided by the user.

Important: the backend will insert the actual English words and Chinese glosses. You should only decide where each word belongs inside the Chinese passage.

You will receive:
- word_list: a JSON array of objects with word, primary_gloss, part_of_speech, entry_id
- style: the desired writing style
- length_target: approximate character count for the Chinese body text

You MUST return exactly one JSON object with this structure:
{"title":"Optional contextual title","paragraphs":["Chinese paragraph with markers such as [[word:101]] inside the text."]}

Rules:
1. Return exactly one JSON object and nothing else.
2. Use [[word:ENTRY_ID]] exactly once per target word.
3. Do not output the English target words or glosses directly.
4. Write natural Chinese paragraphs, not a word list.
5. A loose coherent scene is enough; do not force dramatic plot twists.`;

const DEFAULT_STYLE_TEMPLATE = `Writing tone: Informative and accessible, similar to a short textbook note
Sentence length: Short to medium
Vocabulary level: Common Chinese vocabulary
Topic connection: A loose coherent scene is enough
Paragraph structure: 2-3 paragraphs
Character count target: 120-240 Chinese characters`;

const sessionHistory: AIPassageHistoryItem[] = [];
const sessionPassages = new Map<string, AIPassage>();

export async function fetchTodayPassageContext(): Promise<TodayAiPassageContext> {
  return getTodayAiPassageContext();
}

export async function generateTodayPassage(level: string): Promise<AIPassage> {
  const context = await getTodayAiPassageContext();

  if (!context.tasksComplete) {
    throw new Error('请先完成今日任务，再生成 AI 短文。');
  }

  if (context.wrongWords.length === 0) {
    throw new Error('今天还没有可用于生成短文的错词。');
  }

  return generatePassageFromWrongWords(context.wrongWords, context.date, level);
}

export async function fetchPassageHistory(): Promise<AIPassageHistoryItem[]> {
  return [...sessionHistory];
}

export async function fetchPassage(
  passageId: string,
): Promise<AIPassage | null> {
  return sessionPassages.get(passageId) ?? null;
}

export async function fetchLatestPassageForDate(
  date: string,
): Promise<AIPassage | null> {
  const latest = sessionHistory.find(item => item.generatedAt.slice(0, 10) === date);
  if (!latest) {
    return null;
  }
  return sessionPassages.get(latest.passageId) ?? null;
}

async function generatePassageFromWrongWords(
  wrongWords: WrongWordInput[],
  date: string,
  level: string,
): Promise<AIPassage> {
  const systemMessage = `${PROMPT_TEMPLATE}\n\n## Active Style\n\n${DEFAULT_STYLE_TEMPLATE}`;
  const userMessage = buildUserPrompt(wrongWords, DEFAULT_STYLE, date);

  let rawContent: string;
  try {
    rawContent = await callAnthropic(systemMessage, userMessage);
  } catch (primaryError) {
    rawContent = await callOpenAI(systemMessage, userMessage, primaryError);
  }

  const parsed = parseModelOutput(rawContent, wrongWords);
  const passage: AIPassage = {
    passageId: `passage_${Date.now()}`,
    title: parsed.title || 'AI 情境短文',
    blocks: parsed.blocks,
    wrongWords,
    coveredWordIds: parsed.coveredWordIds,
    missingWordIds: parsed.missingWordIds,
    validationStatus: parsed.missingWordIds.length === 0 ? 'passed' : 'failed',
    failureReason:
      parsed.missingWordIds.length === 0
        ? null
        : `仍有 ${parsed.missingWordIds.length} 个目标词未被覆盖。`,
    wordCount: estimateWordCount(parsed.blocks),
    targetLevel: level,
    generatedAt: new Date().toISOString(),
    preview: buildPreview(parsed.blocks),
  };

  sessionPassages.set(passage.passageId, passage);
  sessionHistory.unshift({
    passageId: passage.passageId,
    title: passage.title,
    preview: passage.preview,
    wordCount: passage.wordCount,
    generatedAt: passage.generatedAt,
    validationStatus: passage.validationStatus,
  });

  return passage;
}

function buildUserPrompt(
  wrongWords: WrongWordInput[],
  style: string,
  date: string,
): string {
  const wordListJson = JSON.stringify(
    wrongWords.map(word => ({
      word: word.word,
      primary_gloss: word.primaryGloss,
      part_of_speech: word.partOfSpeech,
      entry_id: word.entryId,
    })),
    null,
    2,
  );

  const lengthTarget =
    wrongWords.length <= 3
      ? '90-140 characters'
      : wrongWords.length <= 6
      ? '120-180 characters'
      : wrongWords.length <= 10
      ? '150-240 characters'
      : '180-280 characters';

  return `Generate a Chinese passage plan for the following English vocabulary words.

## Word List
\`\`\`json
${wordListJson}
\`\`\`

## Parameters
- Style: ${style}
- Prompt version: ${PROMPT_VERSION}
- Length target: ${lengthTarget}
- Date: ${date}

Remember: write natural Chinese paragraphs and place each word exactly once using [[word:entry_id]] markers. Do not output the English words or glosses directly; the backend will inject them. Return only the JSON structure specified in the prompt template.`;
}

async function callAnthropic(
  systemMessage: string,
  userMessage: string,
): Promise<string> {
  const response = await fetch(PRIMARY_URL, {
    method: 'POST',
    headers: {
      'x-api-key': PRIMARY_KEY,
      'anthropic-version': '2023-06-01',
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model: PRIMARY_MODEL,
      max_tokens: 2048,
      system: systemMessage,
      messages: [{role: 'user', content: userMessage}],
    }),
  });

  if (!response.ok) {
    throw new Error(`主 AI 请求失败：${response.status}`);
  }

  const json = (await response.json()) as {
    content?: Array<{type?: string; text?: string}>;
  };
  const text = json.content?.find(item => item.type === 'text')?.text;
  if (!text) {
    throw new Error('主 AI 响应缺少文本内容');
  }
  return text;
}

async function callOpenAI(
  systemMessage: string,
  userMessage: string,
  primaryError: unknown,
): Promise<string> {
  const response = await fetch(BACKUP_URL, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${BACKUP_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model: BACKUP_MODEL,
      instructions: systemMessage,
      reasoning: {effort: 'low'},
      input: userMessage,
      store: false,
      text: {
        format: {type: 'json_object'},
        verbosity: 'medium',
      },
    }),
  });

  if (!response.ok) {
    throw new Error(
      `备用 AI 请求失败。主请求错误：${String(primaryError)}；状态码：${response.status}`,
    );
  }

  const json = (await response.json()) as {
    output?: Array<{content?: Array<{type?: string; text?: string}>}>;
  };
  const text = json.output
    ?.flatMap(item => item.content ?? [])
    .find(item => item.type === 'output_text')?.text;
  if (!text) {
    throw new Error('备用 AI 响应缺少 output_text');
  }
  return text;
}

function parseModelOutput(
  content: string,
  wrongWords: WrongWordInput[],
): {
  title: string | null;
  blocks: PassageBlock[];
  coveredWordIds: number[];
  missingWordIds: number[];
} {
  const cleaned = extractJsonPayload(stripCodeFences(content));
  const raw = JSON.parse(cleaned) as {
    title?: string | null;
    paragraphs?: string[];
    failed?: boolean;
    reason?: string;
  };

  if (raw.failed) {
    throw new Error(raw.reason || 'AI 短文生成失败');
  }

  if (!Array.isArray(raw.paragraphs)) {
    throw new Error('AI 返回缺少 paragraphs');
  }

  const lookup = new Map<number, WrongWordInput>(
    wrongWords.map(item => [item.entryId, item]),
  );
  const covered = new Set<number>();

  const blocks = raw.paragraphs.map(paragraph => {
    const segments: PassageBlock['segments'] = [];
    let remainder = paragraph;
    const markerRegex = /\[\[word:(\d+)\]\]/;

    while (true) {
      const match = markerRegex.exec(remainder);
      if (!match) {
        if (remainder) {
          segments.push({type: 'text', text: remainder});
        }
        break;
      }

      const [fullMatch, entryIdText] = match;
      const leading = remainder.slice(0, match.index);
      if (leading) {
        segments.push({type: 'text', text: leading});
      }

      const entryId = Number(entryIdText);
      const word = lookup.get(entryId);
      if (!word) {
        throw new Error(`AI 返回了未知 entryId：${entryId}`);
      }

      covered.add(entryId);
      segments.push({
        type: 'word',
        text: word.word,
        entryId,
        glossZh: word.primaryGloss,
        highlighted: true,
      });
      remainder = remainder.slice(match.index + fullMatch.length);
    }

    return {
      blockType: 'paragraph',
      segments,
    };
  });

  const coveredWordIds = [...covered];
  const missingWordIds = wrongWords
    .map(item => item.entryId)
    .filter(entryId => !covered.has(entryId));

  return {
    title: raw.title ?? null,
    blocks,
    coveredWordIds,
    missingWordIds,
  };
}

function stripCodeFences(content: string): string {
  const trimmed = content.trim();
  if (!trimmed.startsWith('```')) {
    return trimmed;
  }

  return trimmed
    .replace(/^```json/i, '')
    .replace(/^```/i, '')
    .replace(/```$/i, '')
    .trim();
}

function extractJsonPayload(content: string): string {
  const start = content.indexOf('{');
  if (start < 0) {
    return content;
  }

  let depth = 0;
  let inString = false;
  let escaped = false;

  for (let index = start; index < content.length; index += 1) {
    const char = content[index];

    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (char === '\\') {
        escaped = true;
      } else if (char === '"') {
        inString = false;
      }
      continue;
    }

    if (char === '"') {
      inString = true;
      continue;
    }
    if (char === '{') {
      depth += 1;
      continue;
    }
    if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return content.slice(start, index + 1);
      }
    }
  }

  return content;
}

function estimateWordCount(blocks: PassageBlock[]): number {
  return blocks
    .flatMap(block => block.segments)
    .map(segment => segment.text)
    .join('')
    .length;
}

function buildPreview(blocks: PassageBlock[]): string {
  const text = blocks
    .flatMap(block => block.segments)
    .map(segment =>
      segment.type === 'word'
        ? `${segment.text}${segment.glossZh ? `（${segment.glossZh}）` : ''}`
        : segment.text,
    )
    .join('');

  return text.length > 80 ? `${text.slice(0, 80)}...` : text;
}
