import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const SOURCE_META = {
  kaoyanJson: {
    repo: 'XixiGod7/kaoyan-english',
    url: 'https://github.com/XixiGod7/kaoyan-english',
    exam: 'kaoyan-english-1',
    label: '鑰冪爺鑻辫涓€',
  },
  cet6Txt: {
    repo: 'AndrewYuZhenYu/cet6_exam_questions_txt_collection',
    url: 'https://github.com/AndrewYuZhenYu/cet6_exam_questions_txt_collection',
    exam: 'cet6',
    label: '澶у鑻辫鍏骇',
  },
  cet4Parsed: {
    repo: 'ShepiTT/CET_practice_questions',
    url: 'https://github.com/ShepiTT/CET_practice_questions',
    exam: 'cet4',
    label: '澶у鑻辫鍥涚骇',
  },
  kaoyansouEnglish: {
    repo: 'kaoyansou.cn English API',
    url: 'https://english.kaoyansou.cn/',
    label: 'Kaoyansou English',
  },
};

function parseArgs(argv = process.argv.slice(2)) {
  const args = new Map();
  for (const arg of argv) {
    const [key, value] = arg.split('=', 2);
    if (key?.startsWith('--')) args.set(key.slice(2), value ?? '');
  }
  return args;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/u, ''));
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function cleanText(value) {
  return String(value ?? '')
    .replace(/\r\n?/gu, '\n')
    .replace(/[ \t]+/gu, ' ')
    .replace(/\n{3,}/gu, '\n\n')
    .trim();
}

function compactId(value) {
  return String(value ?? '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/gu, '-')
    .replace(/^-|-$/gu, '');
}

function normalizeChoices(choices) {
  if (!Array.isArray(choices)) return [];
  return choices
    .map((choice) => ({
      label: String(choice?.label ?? '').trim().toUpperCase(),
      text: cleanText(choice?.text),
    }))
    .filter((choice) => choice.label && choice.text);
}

function questionId(paperId, number, suffix = '') {
  return `${paperId}-q${String(number).padStart(3, '0')}${suffix}`;
}

function normalizeQuestion(paperId, raw, fallbackNumber, sourcePath) {
  const number = Number(raw?.number ?? raw?.q_number ?? fallbackNumber);
  const choices = normalizeChoices(raw?.choices ?? raw?.options);
  const answer = raw?.answer ? String(raw.answer).trim().toUpperCase() : null;
  return {
    id: questionId(paperId, number || fallbackNumber),
    number: number || fallbackNumber,
    kind: choices.length > 0 || /^[A-Z]$/u.test(answer ?? '') ? 'objective' : 'subjective',
    stem: cleanText(raw?.stem ?? raw?.content ?? raw?.prompt ?? ''),
    choices,
    answer,
    explanation: cleanText(raw?.explanation ?? raw?.analysis ?? ''),
    source: {
      path: sourcePath,
    },
  };
}

function applyAnswersToPapers(papers, answerMaps, answerSource) {
  let matched = 0;
  for (const paper of papers) {
    const key = `${paper.year}-${String(paper.month ?? 0).padStart(2, '0')}-${paper.set ?? 1}`;
    const answerMap = answerMaps.get(key) ?? answerMaps.get(String(paper.year));
    if (!answerMap) continue;
    for (const section of paper.sections) {
      for (const question of section.questions) {
        const answer = answerMap.get(Number(question.number));
        if (!answer) continue;
        question.answer = answer.answer;
        if (question.kind !== 'translation' && question.kind !== 'writing') question.kind = 'objective';
        if (answer.explanation && !question.explanation) question.explanation = answer.explanation;
        question.answerSource = answerSource;
        matched += 1;
      }
    }
  }
  return matched;
}

function answerStats(papers) {
  return papers.reduce(
    (sum, paper) =>
      sum +
      paper.sections.reduce(
        (sectionSum, section) =>
          sectionSum + section.questions.filter((question) => Boolean(question.answer)).length,
        0,
      ),
    0,
  );
}

function decodeHtmlEntities(value) {
  const named = new Map([
    ['nbsp', ' '],
    ['amp', '&'],
    ['quot', '"'],
    ['apos', "'"],
    ['lt', '<'],
    ['gt', '>'],
  ]);
  return String(value ?? '')
    .replace(/&([a-z]+);/giu, (match, name) => named.get(name.toLowerCase()) ?? match)
    .replace(/&#(x[0-9a-f]+|\d+);/giu, (match, code) => {
      const value = code[0].toLowerCase() === 'x'
        ? Number.parseInt(code.slice(1), 16)
        : Number.parseInt(code, 10);
      return Number.isFinite(value) ? String.fromCodePoint(value) : match;
    });
}

export function extractQuestionStemFromExplanation(explanation) {
  const lines = decodeHtmlEntities(
    String(explanation ?? '')
      .replace(/<br\s*\/?>/giu, '\n')
      .replace(/<\/(?:p|div|li)>/giu, '\n')
      .replace(/<[^>]+>/gu, ' '),
  )
    .split(/\n/gu)
    .map((line) => cleanText(line))
    .filter(Boolean);
  const overviewIndex = lines.findIndex((line) =>
    /(?:题目.*选项.*概览|选项.*(?:概览|翻译))/u.test(line),
  );
  if (overviewIndex < 0) return '';
  return lines.slice(overviewIndex + 1).find((line) =>
    /[A-Za-z]{3}/u.test(line) &&
    !/[\u3400-\u9fff]/u.test(line) &&
    !/^[A-D][.．\s]/iu.test(line)
  ) ?? '';
}

function isPlaceholderQuestionStem(stem) {
  return /^Question\s+\d+$/iu.test(cleanText(stem));
}

export function repairPlaceholderQuestionStems(papers) {
  let updatedQuestions = 0;
  for (const paper of papers ?? []) {
    for (const section of paper.sections ?? []) {
      if (!/(?:阅读\s*Text\s*\d+|Reading\s*Text\s*\d+)/iu.test(section.title ?? '')) continue;
      for (const question of section.questions ?? []) {
        if (!isPlaceholderQuestionStem(question.stem) || !question.choices?.length) continue;
        const recovered = extractQuestionStemFromExplanation(question.explanation);
        if (!recovered) continue;
        question.stem = recovered;
        updatedQuestions += 1;
      }
    }
  }
  return { updatedQuestions };
}

function questionChoiceSignature(question) {
  const choices = question?.choices ?? [];
  if (choices.length < 2) return '';
  return choices
    .map((choice) => {
      const text = cleanText(choice.text)
        .toLowerCase()
        .replace(/[^\p{L}\p{N}]+/gu, '');
      return `${cleanText(choice.label).toUpperCase()}:${text}`;
    })
    .join('|');
}

export function mergePlaceholderQuestionStems(papers) {
  const stemsByPaper = new Map();
  for (const paper of papers ?? []) {
    if (!stemsByPaper.has(paper.id)) stemsByPaper.set(paper.id, new Map());
    const stems = stemsByPaper.get(paper.id);
    for (const section of paper.sections ?? []) {
      for (const question of section.questions ?? []) {
        const signature = questionChoiceSignature(question);
        if (!signature || isPlaceholderQuestionStem(question.stem)) continue;
        stems.set(signature, cleanText(question.stem));
      }
    }
  }
  let updatedQuestions = 0;
  for (const paper of papers ?? []) {
    const stems = stemsByPaper.get(paper.id);
    if (!stems) continue;
    for (const section of paper.sections ?? []) {
      for (const question of section.questions ?? []) {
        if (!isPlaceholderQuestionStem(question.stem)) continue;
        const recovered = stems.get(questionChoiceSignature(question));
        if (!recovered) continue;
        question.stem = recovered;
        updatedQuestions += 1;
      }
    }
  }
  return { updatedQuestions };
}

function paperAnswerCount(paper) {
  return paper.sections.reduce(
    (sum, section) => sum + section.questions.filter((question) => Boolean(question.answer)).length,
    0,
  );
}

function paperQuestionCount(paper) {
  return paper.sections.reduce((sum, section) => sum + section.questions.length, 0);
}

function dedupePapers(papers) {
  const byId = new Map();
  for (const paper of papers) {
    const existing = byId.get(paper.id);
    if (!existing) {
      byId.set(paper.id, paper);
      continue;
    }
    const existingScore = paperAnswerCount(existing) * 1000 + paperQuestionCount(existing);
    const nextScore = paperAnswerCount(paper) * 1000 + paperQuestionCount(paper);
    if (nextScore >= existingScore) byId.set(paper.id, paper);
  }
  return [...byId.values()];
}

export function applyParagraphTranslationOverrides(papers, overrides = {}) {
  for (const paper of papers) {
    for (const section of paper.sections ?? []) {
      const translations = overrides[section.id];
      if (!Array.isArray(translations)) continue;
      section.paragraphTranslations = translations.map(cleanText).filter(Boolean);
    }
  }
  return papers;
}

function paperBase(meta, fields) {
  const paperId = compactId([meta.exam, fields.year, fields.month, fields.set].filter(Boolean).join('-'));
  return {
    schemaVersion: 1,
    id: paperId,
    exam: meta.exam,
    title: fields.title,
    year: fields.year,
    month: fields.month ?? null,
    set: fields.set ?? null,
    source: {
      repo: meta.repo,
      url: meta.url,
      path: fields.sourcePath,
      license: fields.license ?? null,
    },
    sections: [],
  };
}

function parseKaoyanAnswerDir(answerDir) {
  const maps = new Map();
  if (!answerDir || !fs.existsSync(answerDir)) return maps;
  const solutionDir = fs.existsSync(path.join(answerDir, 'solutions'))
    ? path.join(answerDir, 'solutions')
    : answerDir;
  for (const yearName of fs.readdirSync(solutionDir).sort()) {
    if (!/^\d{4}$/u.test(yearName)) continue;
    const filePath = path.join(solutionDir, yearName, `english1_${yearName}.md`);
    if (!fs.existsSync(filePath)) continue;
    const text = fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/u, '');
    const answers = new Map();
    for (const line of text.split(/\r?\n/u)) {
      const match = line.match(/^\s*(\d{1,2})\.\s*\[([A-Z])\]\s*([^\[]*)\s*$/u);
      if (!match) continue;
      answers.set(Number(match[1]), { answer: match[2], explanation: cleanText(match[3]) });
    }
    const firstTwenty = [...answers.entries()]
      .filter(([number]) => number >= 1 && number <= 20)
      .map(([, answer]) => answer.answer);
    const diversity = new Set(firstTwenty).size;
    if (answers.size >= 25 && diversity >= 3) {
      maps.set(yearName, answers);
    }
  }
  return maps;
}

function parseCet6AnswerFile(filePath, targetMap) {
  if (!fs.existsSync(filePath)) return;
  const text = fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/u, '');
  const lines = text.split(/\r?\n/u);
  const setCounters = new Map();
  let current = null;

  for (const line of lines) {
    const heading = line.match(/^##\s*(20\d{2})-(\d{2})/u);
    if (heading) {
      const yearMonth = `${heading[1]}-${heading[2]}`;
      const set = (setCounters.get(yearMonth) ?? 0) + 1;
      setCounters.set(yearMonth, set);
      const key = `${yearMonth}-${set}`;
      if (!targetMap.has(key)) targetMap.set(key, new Map());
      current = targetMap.get(key);
      continue;
    }
    if (!current) continue;
    const answer = line.match(/^\s*(\d{1,2})\.\s*([A-Z])(?:\)|\s|$)/u);
    if (!answer) continue;
    current.set(Number(answer[1]), { answer: answer[2], explanation: '' });
  }
}

function parseCet6AnswerDir(answerDir) {
  const maps = new Map();
  if (!answerDir || !fs.existsSync(answerDir)) return maps;
  parseCet6AnswerFile(path.join(answerDir, 'Answers_Listen_Answer.md'), maps);
  parseCet6AnswerFile(path.join(answerDir, 'Answers_Answers.md'), maps);
  parseCet6AnswerFile(path.join(answerDir, 'Answers', 'Listen_Answer.md'), maps);
  parseCet6AnswerFile(path.join(answerDir, 'Answers', 'Answers.md'), maps);
  return maps;
}

function normalizeKaoyanPaper(filePath, dataRoot) {
  const raw = readJson(filePath);
  const year = Number(raw.year ?? path.basename(filePath, '.json'));
  const sourcePath = path.relative(dataRoot, filePath).replace(/\\/gu, '/');
  const paper = paperBase(SOURCE_META.kaoyanJson, {
    year,
    title: `${SOURCE_META.kaoyanJson.label} ${year}`,
    sourcePath,
  });
  const sections = raw.sections && typeof raw.sections === 'object' ? raw.sections : {};
  for (const [sectionKey, section] of Object.entries(sections)) {
    const questions = Array.isArray(section?.questions) ? section.questions : [];
    paper.sections.push({
      id: `${paper.id}-${compactId(sectionKey)}`,
      type: String(section?.type ?? sectionKey),
      title: cleanText(section?.name ?? sectionKey),
      instructions: cleanText(section?.instructions ?? ''),
      passage: cleanText(section?.article ?? section?.text ?? ''),
      questions: questions.map((question, index) =>
        normalizeQuestion(paper.id, question, index + 1, sourcePath),
      ),
    });
  }
  return paper;
}

function normalizeCet4Parsed(filePath) {
  const raw = readJson(filePath);
  const papers = [];
  const skippedEmpty = [];
  for (const [fileName, questions] of Object.entries(raw)) {
    const match = fileName.match(/(?<year>20\d{2})-(?<month>\d{2})-CET4-(?<set>\d+)/u);
    if (!match || !Array.isArray(questions)) continue;
    if (questions.length === 0) {
      skippedEmpty.push(fileName);
      continue;
    }
    const { year, month, set } = match.groups;
    const sourcePath = `parsed_data.json#${fileName}`;
    const paper = paperBase(SOURCE_META.cet4Parsed, {
      year: Number(year),
      month: Number(month),
      set: Number(set),
      title: `CET4 ${year}-${month} Set ${set}`,
      sourcePath,
      license: 'Apache-2.0 code; exam content rights unverified',
    });
    const sectionMap = new Map();
    for (const rawQuestion of questions) {
      const sectionName = cleanText(rawQuestion?.section || 'Unknown Section');
      if (!sectionMap.has(sectionName)) {
        sectionMap.set(sectionName, {
          id: `${paper.id}-${compactId(sectionName) || 'section'}`,
          type: compactId(sectionName) || 'section',
          title: sectionName,
          instructions: '',
          passage: '',
          questions: [],
        });
      }
      const section = sectionMap.get(sectionName);
      section.questions.push(
        normalizeQuestion(paper.id, rawQuestion, section.questions.length + 1, sourcePath),
      );
    }
    paper.sections = [...sectionMap.values()];
    papers.push(paper);
  }
  return {
    papers: papers.sort((left, right) => left.id.localeCompare(right.id)),
    skippedEmpty,
  };
}

function dumpCet4Sqlite(dbPath) {
  const script = String.raw`
import json
import sqlite3
import sys

db_path = sys.argv[1]
con = sqlite3.connect(db_path)
con.row_factory = sqlite3.Row
cur = con.cursor()

groups = {}
for row in cur.execute("select id, source_file, title, passage, group_type from question_group order by id"):
    groups[row["id"]] = {
        "id": row["id"],
        "source_file": row["source_file"],
        "title": row["title"],
        "passage": row["passage"],
        "group_type": row["group_type"],
        "questions": [],
    }

question_ids = []
for row in cur.execute("select id, group_id, source_file, section, content, q_number, correct_answer, explanation from question order by source_file, q_number, id"):
    question = dict(row)
    question["options"] = []
    question_ids.append(row["id"])
    if row["group_id"] in groups:
        groups[row["group_id"]]["questions"].append(question)

for row in cur.execute("select question_id, label, text from option order by question_id, label"):
    for group in groups.values():
        found = False
        for question in group["questions"]:
            if question["id"] == row["question_id"]:
                question["options"].append({"label": row["label"], "text": row["text"]})
                found = True
                break
        if found:
            break

print(json.dumps(list(groups.values()), ensure_ascii=False))
con.close()
`;
  const result = spawnSync('python', ['-', dbPath], {
    input: script,
    encoding: 'utf8',
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.status !== 0) {
    throw new Error(`failed to read ${dbPath}: ${result.stderr || result.stdout}`);
  }
  return JSON.parse(result.stdout);
}

function normalizeCet4Sqlite(dbPath) {
  const groups = dumpCet4Sqlite(dbPath);
  const bySource = new Map();
  for (const group of groups) {
    if (!Array.isArray(group.questions) || group.questions.length === 0) continue;
    if (!bySource.has(group.source_file)) bySource.set(group.source_file, []);
    bySource.get(group.source_file).push(group);
  }

  const papers = [];
  for (const [fileName, sourceGroups] of [...bySource.entries()].sort()) {
    const match = fileName.match(/(?<year>20\d{2})-(?<month>\d{2})-CET4-(?<set>\d+)/u);
    if (!match) continue;
    const { year, month, set } = match.groups;
    const sourcePath = `instance/${path.basename(dbPath)}#${fileName}`;
    const paper = paperBase(SOURCE_META.cet4Parsed, {
      year: Number(year),
      month: Number(month),
      set: Number(set),
      title: `CET4 ${year}-${month} Set ${set}`,
      sourcePath,
      license: 'Apache-2.0 code; exam content rights unverified',
    });
    paper.sections = sourceGroups.map((group) => ({
      id: `${paper.id}-${compactId(group.title) || group.id}`,
      type: cleanText(group.group_type || group.title || 'section'),
      title: cleanText(group.title || group.group_type || 'Section'),
      instructions: '',
      passage: cleanText(group.passage),
      questions: group.questions.map((question, index) => ({
        ...normalizeQuestion(paper.id, question, index + 1, sourcePath),
        answer: question.correct_answer ? String(question.correct_answer).trim().toUpperCase() : null,
        kind: question.correct_answer || question.options?.length ? 'objective' : 'subjective',
        answerSource: question.correct_answer ? 'ShepiTT/CET_practice_questions instance/cet4_v2.db' : undefined,
        explanation: cleanText(question.explanation),
      })),
    }));
    papers.push(paper);
  }
  return papers.sort((left, right) => left.id.localeCompare(right.id));
}

function safeParseJson(value, fallback = null) {
  if (!value) return fallback;
  try {
    return JSON.parse(value);
  } catch {
    return fallback;
  }
}

function tokenListToText(tokens) {
  if (!Array.isArray(tokens)) return cleanText(tokens ?? '');
  return cleanText(
    tokens
      .map((token) => String(token?.name ?? ''))
      .filter((part) => part !== '')
      .join(' ')
      .replace(/\s+([,.;:!?])/gu, '$1')
      .replace(/\(\s+/gu, '(')
      .replace(/\s+\)/gu, ')'),
  );
}

function kaoyansouExamFields(record) {
  const englishType = cleanText(record.englishType);
  const year = Number(record.year);
  if (englishType === '英一') {
    return {
      exam: 'kaoyan-english-1',
      year,
      month: null,
      set: null,
      title: `Kaoyan English I ${year}`,
    };
  }
  if (englishType === '英二') {
    return {
      exam: 'kaoyan-english-2',
      year,
      month: null,
      set: null,
      title: `Kaoyan English II ${year}`,
    };
  }
  const cetMatch = englishType.match(/^(四|六)级-(\d+)月(\d+)套$/u);
  if (cetMatch) {
    const exam = cetMatch[1] === '四' ? 'cet4' : 'cet6';
    const month = Number(cetMatch[2]);
    const set = Number(cetMatch[3]);
    return {
      exam,
      year,
      month,
      set,
      title: `${exam.toUpperCase()} ${year}-${String(month).padStart(2, '0')} Set ${set}`,
    };
  }
  return null;
}

function kaoyansouMetaForExam(exam) {
  return {
    ...SOURCE_META.kaoyansouEnglish,
    exam,
  };
}

function kaoyansouPassageFromContent(content) {
  const data = content?.data;
  if (Array.isArray(data?.conts)) {
    return cleanText(
      data.conts
        .map((entry) => {
          if (Array.isArray(entry?.econt)) return tokenListToText(entry.econt);
          if (Array.isArray(entry?.econt?.cont)) return tokenListToText(entry.econt.cont);
          return cleanText(entry?.econt ?? entry?.jzyfs ?? entry?.zcont ?? '');
        })
        .filter(Boolean)
        .join('\n'),
    );
  }
  if (content?.hearingOriginalText?.data) {
    return cleanText(
      content.hearingOriginalText.data
        .flatMap((group) => (Array.isArray(group?.data) ? group.data : [group]))
        .map((entry) => cleanText(entry?.econt ?? tokenListToText(entry?.cont) ?? entry?.zcont ?? ''))
        .filter(Boolean)
        .join('\n'),
    );
  }
  return cleanText(data?.econt ?? data?.source ?? data?.title ?? '');
}

function kaoyansouParagraphTranslationsFromContent(content) {
  const conts = content?.data?.conts;
  if (!Array.isArray(conts) || conts.length === 0) return [];
  const translations = conts.map((entry) => cleanText(entry?.zcont ?? ''));
  return translations.every(Boolean) ? translations : [];
}

function kaoyansouAnswerLetter(rawQuestion) {
  const explicit = cleanText(rawQuestion?.zhengque).toUpperCase();
  if (/^[A-Z]$/u.test(explicit)) return explicit;
  const numeric = Number(rawQuestion?.daan?.zhengque ?? rawQuestion?.zhengque);
  if (Number.isFinite(numeric) && numeric >= 1 && numeric <= 26) {
    return String.fromCharCode(64 + numeric);
  }
  return null;
}

function kaoyansouChoices(rawQuestion) {
  const rawChoices = rawQuestion?.daan?.xx;
  if (!Array.isArray(rawChoices)) return [];
  return rawChoices
    .map((choice, index) => {
      if (Array.isArray(choice)) {
        const label = cleanText(choice.find((token) => token?.pos)?.pos ?? String.fromCharCode(65 + index)).toUpperCase();
        return {
          label,
          text: tokenListToText(choice),
        };
      }
      const label = cleanText(choice?.name ?? String.fromCharCode(65 + index)).toUpperCase();
      return {
        label,
        text: cleanText(choice?.subject ?? choice?.text ?? ''),
      };
    })
    .filter((choice) => /^[A-Z]$/u.test(choice.label) && choice.text);
}

function kaoyansouSectionKind(sectionTitle) {
  const title = cleanText(sectionTitle).toLowerCase();
  if (title.includes('translation') || title.includes('翻译')) return 'translation';
  if (title.includes('writing') || title.includes('写作')) return 'writing';
  return null;
}

function kaoyansouTranslationAnswer(rawQuestion) {
  const rawAnswer = cleanText(rawQuestion?.daan?.zhengque ?? rawQuestion?.zhengque ?? '');
  if (rawAnswer && !/^[A-Z]$/iu.test(rawAnswer) && !/^\d+$/u.test(rawAnswer)) return rawAnswer;
  return '';
}

function kaoyansouTranslationPairFromContent(content) {
  const conts = content?.data?.conts;
  if (!Array.isArray(conts)) return { prompt: '', answer: '' };
  const prompt = cleanText(conts.map((entry) => cleanText(entry?.zcont ?? '')).filter(Boolean).join('\n'));
  const answer = cleanText(
    conts
      .map((entry) => {
        if (Array.isArray(entry?.econt)) return tokenListToText(entry.econt);
        return cleanText(entry?.econt ?? entry?.jzyfs ?? '');
      })
      .filter(Boolean)
      .join('\n'),
  );
  return { prompt, answer };
}

function normalizeKaoyansouQuestion(
  paperId,
  sectionSourceId,
  sectionTitle,
  rawQuestion,
  index,
  sourcePath,
  sectionFallback = {},
) {
  const number = Number(rawQuestion?.num ?? rawQuestion?.number ?? index + 1);
  const sectionKind = kaoyansouSectionKind(sectionTitle);
  const choices = sectionKind === 'translation' ? [] : kaoyansouChoices(rawQuestion);
  const answer = sectionKind === 'translation'
    ? kaoyansouTranslationAnswer(rawQuestion) || cleanText(sectionFallback.answer)
    : kaoyansouAnswerLetter(rawQuestion);
  const kind = sectionKind ?? (choices.length > 0 || /^[A-Z]$/u.test(answer ?? '') ? 'objective' : 'subjective');
  return {
    id: questionId(paperId, number || index + 1, `-s${sectionSourceId}`),
    number: number || index + 1,
    kind,
    stem: cleanText(rawQuestion?.title ?? rawQuestion?.tigan?.text ?? sectionFallback.prompt ?? `Question ${number || index + 1}`),
    choices,
    answer,
    explanation: cleanText(rawQuestion?.jiexi_text ?? rawQuestion?.jiexi ?? ''),
    answerSource: answer ? `${SOURCE_META.kaoyansouEnglish.repo} /paper/${sectionSourceId}` : undefined,
    source: {
      path: sourcePath,
    },
  };
}

function kaoyansouQuestionsFromRecord(record, content) {
  const contentQuestions = Array.isArray(content?.data?.tm) ? content.data.tm : [];
  const sectionTitle = cleanText(record.sectionName);
  if (!sectionTitle.includes('听力') && !sectionTitle.toLowerCase().includes('listening')) {
    return contentQuestions.filter(Boolean);
  }
  const tiMuPayload = safeParseJson(record.tiMuJson, {});
  const listeningQuestions = Array.isArray(tiMuPayload?.data) ? tiMuPayload.data.flat() : [];
  return listeningQuestions.length > 0 ? listeningQuestions.filter(Boolean) : contentQuestions.filter(Boolean);
}

function normalizeKaoyansouEnglishDir(apiDir) {
  const listPath = path.join(apiDir, 'paper-list.json');
  if (!apiDir || !fs.existsSync(listPath)) return [];
  const list = readJson(listPath);
  const records = Array.isArray(list?.data?.records) ? list.data.records : [];
  const grouped = new Map();

  for (const listedRecord of records) {
    const detailPath = path.join(apiDir, 'papers', `${listedRecord.id}.json`);
    if (!fs.existsSync(detailPath)) continue;
    const detail = readJson(detailPath);
    const record = detail.data ?? listedRecord;
    const fields = kaoyansouExamFields(record);
    if (!fields) continue;
    const key = [fields.exam, fields.year, fields.month ?? 0, fields.set ?? 0].join('|');
    if (!grouped.has(key)) grouped.set(key, { fields, records: [] });
    grouped.get(key).records.push({ record, detailPath });
  }

  const papers = [];
  for (const { fields, records: sectionRecords } of grouped.values()) {
    const paper = paperBase(kaoyansouMetaForExam(fields.exam), {
      year: fields.year,
      month: fields.month,
      set: fields.set,
      title: fields.title,
      sourcePath: `paper-list.json#${fields.exam}-${fields.year}-${fields.month ?? 0}-${fields.set ?? 0}`,
      license: 'Public API content rights unverified',
    });
    paper.sections = sectionRecords
      .sort((left, right) => {
        const leftName = cleanText(left.record.sectionName);
        const rightName = cleanText(right.record.sectionName);
        return leftName.localeCompare(rightName, 'zh-Hans-CN', { numeric: true });
      })
      .map(({ record, detailPath }) => {
        const sourcePath = path.relative(apiDir, detailPath).replace(/\\/gu, '/');
        const content = safeParseJson(record.contentJson, {});
        const questions = kaoyansouQuestionsFromRecord(record, content);
        const sectionTitle = cleanText(record.sectionName || content?.data?.title || 'Section');
        const sectionFallback = kaoyansouSectionKind(sectionTitle) === 'translation'
          ? kaoyansouTranslationPairFromContent(content)
          : {};
        const sectionId = `${paper.id}-${compactId(sectionTitle) || record.id}`;
        return {
          id: `${sectionId}-${record.id}`,
          type: compactId(sectionTitle) || 'section',
          title: sectionTitle,
          instructions: cleanText(content?.data?.title ?? ''),
          passage: kaoyansouPassageFromContent(content),
          paragraphTranslations: kaoyansouParagraphTranslationsFromContent(content),
          source: {
            path: sourcePath,
            api: `/api/paper/${record.id}`,
          },
          questions: questions.map((question, index) =>
            normalizeKaoyansouQuestion(paper.id, record.id, sectionTitle, question, index, sourcePath, sectionFallback),
          ),
        };
      });
    papers.push(paper);
  }
  return papers.sort((left, right) => left.id.localeCompare(right.id));
}

function parseCet6Txt(filePath, dataRoot) {
  const text = fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/u, '');
  const sourcePath = path.relative(dataRoot, filePath).replace(/\\/gu, '/');
  const fileName = path.basename(filePath, '.txt');
  const match = fileName.match(/(?<year>20\d{2})-(?<month>\d{2})_(?<set>\d+)/u);
  if (!match) return null;
  const { year, month, set } = match.groups;
  const paper = paperBase(SOURCE_META.cet6Txt, {
    year: Number(year),
    month: Number(month),
    set: Number(set),
    title: `CET6 ${year}-${month} Set ${Number(set)}`,
    sourcePath,
  });
  const lines = text.split(/\r?\n/u).map((line) => line.trim()).filter(Boolean);
  let currentSection = {
    id: `${paper.id}-body`,
    type: 'body',
    title: 'Body',
    instructions: '',
    passage: '',
    questions: [],
  };
  const sections = [currentSection];
  let activeQuestion = null;
  let passageBuffer = [];

  function flushPassage() {
    const passage = cleanText(passageBuffer.join('\n'));
    if (passage) currentSection.passage = cleanText([currentSection.passage, passage].filter(Boolean).join('\n\n'));
    passageBuffer = [];
  }

  for (const line of lines) {
    const partMatch = line.match(/^Part\s+[IVX]+/iu);
    const sectionMatch = line.match(/^Section\s+[A-Z]/u);
    if (partMatch || sectionMatch) {
      flushPassage();
      const title = line;
      currentSection = {
        id: `${paper.id}-${compactId(title)}`,
        type: compactId(title),
        title,
        instructions: '',
        passage: '',
        questions: [],
      };
      sections.push(currentSection);
      activeQuestion = null;
      continue;
    }

    const questionMatch = line.match(/^(?<number>\d{1,2})\.\s*(?<rest>.*)$/u);
    if (questionMatch) {
      const number = Number(questionMatch.groups.number);
      activeQuestion = {
        id: questionId(paper.id, number),
        number,
        kind: 'subjective',
        stem: cleanText(questionMatch.groups.rest),
        choices: [],
        answer: null,
        explanation: '',
        source: { path: sourcePath },
      };
      currentSection.questions.push(activeQuestion);
      continue;
    }

    const choiceMatch = line.match(/^(?<label>[A-D])\.\s*(?<text>.*)$/u);
    if (choiceMatch && activeQuestion) {
      activeQuestion.choices.push({
        label: choiceMatch.groups.label,
        text: cleanText(choiceMatch.groups.text),
      });
      activeQuestion.kind = 'objective';
      continue;
    }

    if (/^Directions:/iu.test(line)) {
      currentSection.instructions = cleanText([currentSection.instructions, line].filter(Boolean).join('\n'));
    } else if (activeQuestion && !activeQuestion.choices.length && activeQuestion.stem) {
      activeQuestion.stem = cleanText(`${activeQuestion.stem} ${line}`);
    } else {
      passageBuffer.push(line);
    }
  }
  flushPassage();
  paper.sections = sections.filter(
    (section) => section.questions.length > 0 || section.instructions || section.passage,
  );
  return paper;
}

function latestByYear(papers) {
  return papers.reduce(
    (latest, paper) => (!latest || paper.year > latest.year ? paper : latest),
    null,
  );
}

export function importExamPapers(options) {
  const outDir = options.outDir;
  const reportPath = options.reportPath;
  const allPapers = [];
  const sourceReports = [];

  if (options.kaoyanDir) {
    const dataRoot = path.join(options.kaoyanDir, 'public', 'data');
    const files = fs
      .readdirSync(dataRoot)
      .filter((name) => /^\d{4}\.json$/u.test(name))
      .sort();
    const papers = files.map((name) => normalizeKaoyanPaper(path.join(dataRoot, name), dataRoot));
    const answerMaps = parseKaoyanAnswerDir(options.kaoyanAnswerDir);
    const matchedAnswers = applyAnswersToPapers(
      papers,
      answerMaps,
      SOURCE_META.kaoyanJson.repo === 'XixiGod7/kaoyan-english'
        ? 'TsekaLuk/Kaoyan-English1-Papers'
        : null,
    );
    allPapers.push(...papers);
    sourceReports.push({
      source: SOURCE_META.kaoyanJson.repo,
      papers: papers.length,
      latest: latestByYear(papers)?.year ?? null,
      answerBearingQuestions: matchedAnswers,
      answerSource: matchedAnswers > 0 ? 'TsekaLuk/Kaoyan-English1-Papers' : null,
    });
  }

  if (options.cet6TxtDir) {
    const dataRoot = path.join(options.cet6TxtDir, 'cet6_zhenti_cleaned');
    const files = fs.readdirSync(dataRoot).filter((name) => name.endsWith('.txt')).sort();
    const papers = files
      .map((name) => parseCet6Txt(path.join(dataRoot, name), dataRoot))
      .filter(Boolean);
    const answerMaps = parseCet6AnswerDir(options.cet6AnswerDir);
    const matchedAnswers = applyAnswersToPapers(papers, answerMaps, 'Drhm1224/cet6-all-in-one');
    allPapers.push(...papers);
    sourceReports.push({
      source: SOURCE_META.cet6Txt.repo,
      papers: papers.length,
      latest: latestByYear(papers)?.year ?? null,
      answerBearingQuestions: matchedAnswers,
      answerSource: matchedAnswers > 0 ? 'Drhm1224/cet6-all-in-one' : null,
    });
  }

  if (options.cet4Dir) {
    const parsedPath = path.join(options.cet4Dir, 'parsed_data.json');
    const sqlitePath = path.join(options.cet4Dir, 'instance', 'cet4_v2.db');
    const usedSqlite = fs.existsSync(sqlitePath);
    const result = usedSqlite ? { papers: normalizeCet4Sqlite(sqlitePath), skippedEmpty: [] } : normalizeCet4Parsed(parsedPath);
    const papers = result.papers;
    const pdfDir = path.join(options.cet4Dir, 'data');
    const pdfFiles = fs.existsSync(pdfDir)
      ? fs.readdirSync(pdfDir).filter((name) => name.toLowerCase().endsWith('.pdf')).sort()
      : [];
    const parsedNames = new Set(
      papers.map((paper) => {
        const month = String(paper.month).padStart(2, '0');
        return `${paper.year}-${month}-CET4-${paper.set}.pdf`;
      }),
    );
    const unparsedPdfFiles = pdfFiles.filter((fileName) => !parsedNames.has(fileName));
    allPapers.push(...papers);
    sourceReports.push({
      source: SOURCE_META.cet4Parsed.repo,
      papers: papers.length,
      latest: latestByYear(papers)?.year ?? null,
      skippedEmpty: result.skippedEmpty.length,
      unparsedPdfFiles,
      answerBearingQuestions: answerStats(papers),
      answerSource: usedSqlite ? 'instance/cet4_v2.db' : 'parsed_data.json',
    });
  }

  if (options.kaoyansouEnglishDir) {
    const papers = normalizeKaoyansouEnglishDir(options.kaoyansouEnglishDir);
    allPapers.push(...papers);
    sourceReports.push({
      source: SOURCE_META.kaoyansouEnglish.repo,
      papers: papers.length,
      latest: latestByYear(papers)?.year ?? null,
      answerBearingQuestions: answerStats(papers),
      answerSource: SOURCE_META.kaoyansouEnglish.url,
    });
  }

  mergePlaceholderQuestionStems(allPapers);
  const dedupedPapers = dedupePapers(allPapers).sort((left, right) => left.id.localeCompare(right.id));
  repairPlaceholderQuestionStems(dedupedPapers);
  const translationOverridesPath = options.paragraphTranslationsPath ?? path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    'exam-paper-paragraph-translations.json',
  );
  if (fs.existsSync(translationOverridesPath)) {
    applyParagraphTranslationOverrides(dedupedPapers, readJson(translationOverridesPath));
  }
  const droppedZeroAnswerPapers = dedupedPapers
    .filter((paper) => paperQuestionCount(paper) > 0 && paperAnswerCount(paper) === 0)
    .map((paper) => ({
      id: paper.id,
      exam: paper.exam,
      title: paper.title,
      year: paper.year,
      month: paper.month,
      set: paper.set,
      questions: paperQuestionCount(paper),
      source: paper.source?.repo ?? null,
    }));
  const uniquePapers = dedupedPapers.filter(
    (paper) => !(paperQuestionCount(paper) > 0 && paperAnswerCount(paper) === 0),
  );
  const byExam = new Map();
  for (const paper of uniquePapers) {
    if (!byExam.has(paper.exam)) byExam.set(paper.exam, []);
    byExam.get(paper.exam).push(paper);
  }

  fs.mkdirSync(outDir, { recursive: true });
  const writtenFiles = [];
  for (const [exam, papers] of [...byExam.entries()].sort()) {
    const filePath = path.join(outDir, `${exam}.json`);
    writeJson(filePath, { schemaVersion: 1, exam, papers });
    writtenFiles.push(path.relative(process.cwd(), filePath).replace(/\\/gu, '/'));
  }

  const manifest = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    sourcePolicy:
      'Imported from public GitHub repositories for local study data normalization; upstream exam-content rights remain unverified.',
    files: writtenFiles,
    sources: sourceReports,
    totalPapers: uniquePapers.length,
  };
  writeJson(path.join(outDir, 'manifest.json'), manifest);
  const report = {
    generatedAt: manifest.generatedAt,
    outDir: path.relative(process.cwd(), outDir).replace(/\\/gu, '/'),
    totalPapers: uniquePapers.length,
    totalQuestions: uniquePapers.reduce(
      (sum, paper) => sum + paper.sections.reduce((sectionSum, section) => sectionSum + section.questions.length, 0),
      0,
    ),
    answerBearingQuestions: answerStats(uniquePapers),
    droppedZeroAnswerPapers,
    sources: sourceReports,
    files: writtenFiles,
  };
  writeJson(reportPath, report);
  return report;
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);
if (isMain) {
  const args = parseArgs();
  const report = importExamPapers({
    kaoyanDir: args.get('kaoyanDir'),
    kaoyanAnswerDir: args.get('kaoyanAnswerDir'),
    cet6TxtDir: args.get('cet6TxtDir'),
    cet6AnswerDir: args.get('cet6AnswerDir'),
    cet4Dir: args.get('cet4Dir'),
    kaoyansouEnglishDir: args.get('kaoyansouEnglishDir'),
    paragraphTranslationsPath: args.get('paragraphTranslationsPath'),
    outDir: args.get('outDir') ?? 'apps/flutter_mobile/assets/exam-papers',
    reportPath: args.get('reportPath') ?? 'docs/exam-paper-import-report.json',
  });
  console.log(
    `papers=${report.totalPapers} questions=${report.totalQuestions} outDir=${report.outDir}`,
  );
  for (const source of report.sources) {
    console.log(`${source.source}: papers=${source.papers} latest=${source.latest}`);
  }
}
