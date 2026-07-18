import fs from 'node:fs';
import path from 'node:path';

const bookDir = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const examDir = 'apps/flutter_mobile/assets/exam-papers';
const reportPath = 'docs/seed-vocab-real-exam-examples-report.json';
const requiredBooks = ['CET4_3.json', 'CET6_3.json', 'KaoYan_3.json', 'MEDICAL_RESP.json'];
const allowedExamsByBook = new Map([
  ['CET4_3.json', new Set(['cet4'])],
  ['CET6_3.json', new Set(['cet6'])],
  ['KaoYan_3.json', new Set(['kaoyan-english-1', 'kaoyan-english-2'])],
  ['MEDICAL_RESP.json', new Set()],
]);
const maxExamplesPerWord = 8;
const minSentenceChars = 24;
const maxSentenceChars = 260;

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}

function writeJson(filePath, value) {
  const payload = `${JSON.stringify(value)}\n`;
  const tmp = `${filePath}.${process.pid}.tmp`;
  let lastError;
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      fs.writeFileSync(tmp, payload, 'utf8');
      fs.renameSync(tmp, filePath);
      return;
    } catch (error) {
      lastError = error;
      try {
        if (fs.existsSync(tmp)) fs.unlinkSync(tmp);
      } catch {}
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 120 * (attempt + 1));
    }
  }
  throw lastError;
}

function cleanText(value) {
  return String(value ?? '')
    .replace(/\uFFFD/gu, '')
    .replace(/[“”]/gu, '"')
    .replace(/[‘’]/gu, "'")
    .replace(/\s+/gu, ' ')
    .trim();
}

function displayWord(item) {
  return String(item?.displayWord ?? item?.headWord ?? item?.content?.word?.wordHead ?? '').trim();
}

function wordId(item, fallback) {
  return String(item?.content?.word?.wordId ?? item?.sourceId ?? fallback).trim();
}

function wordContent(item) {
  return item?.content?.word?.content ?? {};
}

function normalizeWord(word) {
  return cleanText(word).toLowerCase().replace(/[^a-z0-9 -]/gu, '').replace(/\s+/gu, ' ').trim();
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
}

function inflectedForms(word) {
  const normalized = normalizeWord(word);
  if (!normalized) return [];
  if (normalized.includes(' ')) return [normalized];
  const forms = new Set([normalized]);
  if (normalized.length > 2) forms.add(`${normalized}s`);
  if (normalized.endsWith('y') && normalized.length > 2) forms.add(`${normalized.slice(0, -1)}ies`);
  if (normalized.endsWith('e') && normalized.length > 3) {
    forms.add(`${normalized}d`);
    forms.add(`${normalized.slice(0, -1)}ing`);
  } else if (normalized.length > 2) {
    forms.add(`${normalized}ed`);
    forms.add(`${normalized}ing`);
  }
  return [...forms].sort((a, b) => b.length - a.length);
}

function patternFor(word) {
  const forms = inflectedForms(word);
  if (forms.length === 0) return null;
  const source = forms.map(escapeRegExp).join('|');
  return new RegExp(`(?<![A-Za-z])(?:${source})(?![A-Za-z])`, 'iu');
}

function tokenForms(sentence) {
  const tokens = sentence.toLowerCase().match(/\b[a-z][a-z-]*\b/gu) ?? [];
  const forms = new Set(tokens);
  const maxPhraseTokens = 5;
  for (let start = 0; start < tokens.length; start += 1) {
    for (let size = 2; size <= maxPhraseTokens && start + size <= tokens.length; size += 1) {
      forms.add(tokens.slice(start, start + size).join(' '));
    }
  }
  return forms;
}

function splitSentences(text) {
  const normalized = cleanText(text);
  if (!normalized) return [];
  const pieces = [];
  const sentenceRe = /[^.!?\n。！？]+(?:[.!?。！？]+|$)/gu;
  for (const match of normalized.matchAll(sentenceRe)) {
    const sentence = cleanText(match[0]);
    if (sentence.length >= minSentenceChars && sentence.length <= maxSentenceChars && /[A-Za-z]/u.test(sentence)) {
      pieces.push(sentence);
    }
  }
  return pieces;
}

function pushText(corpus, text, meta) {
  for (const sentence of splitSentences(text)) {
    corpus.push({ sentence, meta });
  }
}

function collectExamCorpus() {
  const manifest = readJson(path.join(examDir, 'manifest.json'));
  const files = (manifest.files ?? [])
    .map((file) => path.basename(file))
    .filter((file) => file.endsWith('.json') && file !== 'manifest.json');
  const corpus = [];
  const stats = { files: files.length, papers: 0, sections: 0, questions: 0, sentences: 0 };
  for (const fileName of files) {
    const examFile = readJson(path.join(examDir, fileName));
    const exam = examFile.exam ?? path.basename(fileName, '.json');
    for (const paper of examFile.papers ?? []) {
      stats.papers += 1;
      for (const section of paper.sections ?? []) {
        stats.sections += 1;
        const baseMeta = {
          exam,
          paperId: paper.id,
          paperTitle: paper.title,
          year: paper.year,
          month: paper.month,
          set: paper.set,
          sectionId: section.id,
          sectionTitle: section.title,
          sourcePath: section?.source?.path ?? paper?.source?.path ?? '',
        };
        pushText(corpus, section.instructions, { ...baseMeta, textKind: 'instructions' });
        pushText(corpus, section.passage, { ...baseMeta, textKind: 'passage' });
        for (const question of section.questions ?? []) {
          stats.questions += 1;
          const questionMeta = {
            ...baseMeta,
            questionId: question.id,
            questionNumber: question.number,
            questionKind: question.kind,
          };
          pushText(corpus, question.stem, { ...questionMeta, textKind: 'stem' });
          for (const choice of question.choices ?? []) {
            pushText(corpus, choice.text, {
              ...questionMeta,
              textKind: 'choice',
              choiceLabel: choice.label,
            });
          }
        }
      }
    }
  }
  const seen = new Set();
  const deduped = [];
  for (const item of corpus) {
    const key = `${item.sentence}\u0000${item.meta.paperId}\u0000${item.meta.sectionId}\u0000${item.meta.questionId ?? ''}\u0000${item.meta.choiceLabel ?? ''}`;
    if (seen.has(key)) continue;
    seen.add(key);
    deduped.push(item);
  }
  stats.sentences = deduped.length;
  return { corpus: deduped, stats };
}

function sourceLabel(meta) {
  const parts = [String(meta.exam ?? '').toUpperCase(), meta.paperTitle, meta.sectionTitle];
  if (meta.questionNumber != null) parts.push(`Q${meta.questionNumber}`);
  if (meta.choiceLabel) parts.push(`Option ${meta.choiceLabel}`);
  return parts.filter(Boolean).join(' / ');
}

function scoreMatch(word, sentence, meta, existingKeys) {
  const normalized = normalizeWord(word);
  const lower = sentence.toLowerCase();
  let score = 0;
  if (lower.includes(` ${normalized} `)) score += 30;
  if (meta.textKind === 'passage') score += 20;
  if (meta.textKind === 'choice') score += 12;
  if (meta.questionKind === 'objective') score += 8;
  const len = sentence.length;
  if (len >= 45 && len <= 160) score += 12;
  if (existingKeys.has(sentence.toLowerCase())) score -= 80;
  return score;
}

function buildEntryIndex() {
  const books = [];
  const entries = [];
  for (const fileName of requiredBooks) {
    const filePath = path.join(bookDir, fileName);
    const items = readJson(filePath);
    const book = path.basename(fileName, '.json');
    books.push({ book, fileName, filePath, items });
    for (const item of items) {
      const word = displayWord(item);
      const normalized = normalizeWord(word);
      if (!normalized) continue;
      entries.push({
        item,
        book,
        fileName,
        word,
        wordId: wordId(item, word),
        normalized,
        pattern: patternFor(word),
      });
    }
  }
  return { books, entries };
}

function existingExampleKeys(content) {
  const out = new Set();
  for (const sentence of content?.sentence?.sentences ?? []) {
    const text = cleanText(sentence?.sContent).toLowerCase();
    if (text) out.add(text);
  }
  return out;
}

function enrich() {
  const { corpus, stats: corpusStats } = collectExamCorpus();
  const { books, entries } = buildEntryIndex();
  const byForm = new Map();
  for (const entry of entries) {
    for (const form of inflectedForms(entry.word)) {
      if (!byForm.has(form)) byForm.set(form, []);
      byForm.get(form).push(entry);
    }
  }

  const matches = new Map();
  for (const corpusItem of corpus) {
    const candidates = new Set();
    for (const form of tokenForms(corpusItem.sentence)) {
      for (const entry of byForm.get(form) ?? []) {
        candidates.add(entry);
      }
    }
    for (const entry of candidates) {
      const allowedExams = allowedExamsByBook.get(entry.fileName) ?? new Set();
      if (!allowedExams.has(corpusItem.meta.exam)) continue;
      if (entry.pattern && !entry.pattern.test(corpusItem.sentence)) continue;
      const key = `${entry.fileName}\u0000${entry.wordId}`;
      if (!matches.has(key)) matches.set(key, []);
      matches.get(key).push(corpusItem);
    }
  }

  const report = {
    generatedAt: new Date().toISOString(),
    maxExamplesPerWord,
    corpus: corpusStats,
    books: {},
    unmatchedSamples: {},
  };

  for (const book of books) {
    const allowedExams = [...(allowedExamsByBook.get(book.fileName) ?? new Set())].sort();
    let matchedEntries = 0;
    let totalRawMatches = 0;
    let cappedEntries = 0;
    let writtenExamples = 0;
    const unmatched = [];
    for (const item of book.items) {
      const word = displayWord(item);
      const id = wordId(item, word);
      const key = `${book.fileName}\u0000${id}`;
      const content = wordContent(item);
      const existingKeys = existingExampleKeys(content);
      const rawMatches = matches.get(key) ?? [];
      totalRawMatches += rawMatches.length;
      const deduped = [];
      const seenSentence = new Set();
      for (const match of rawMatches) {
        const sentenceKey = match.sentence.toLowerCase();
        if (seenSentence.has(sentenceKey)) continue;
        seenSentence.add(sentenceKey);
        deduped.push(match);
      }
      deduped.sort((a, b) => scoreMatch(word, b.sentence, b.meta, existingKeys) - scoreMatch(word, a.sentence, a.meta, existingKeys));
      const selected = deduped.slice(0, maxExamplesPerWord);
      if (deduped.length > maxExamplesPerWord) cappedEntries += 1;
      if (selected.length > 0) {
        matchedEntries += 1;
        content.realExamSentence = {
          desc: '真题例句',
          source: 'exam-papers',
          maxExamplesPerWord,
          sentences: selected.map((match, index) => ({
            sContent: match.sentence,
            sCn: `真题来源：${sourceLabel(match.meta)}`,
            source: match.meta,
            matchRank: index + 1,
          })),
        };
        writtenExamples += selected.length;
      } else {
        delete content.realExamSentence;
        if (unmatched.length < 30) unmatched.push(word);
      }
    }
    report.books[book.fileName] = {
      entries: book.items.length,
      allowedExams,
      matchedEntries,
      unmatchedEntries: book.items.length - matchedEntries,
      coverage: Number((matchedEntries / book.items.length).toFixed(4)),
      rawMatches: totalRawMatches,
      writtenExamples,
      cappedEntries,
    };
    report.unmatchedSamples[book.fileName] = unmatched;
    writeJson(book.filePath, book.items);
  }

  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  writeJson(reportPath, report);
  console.log(JSON.stringify(report.books, null, 2));
  console.log(`sentences=${corpusStats.sentences} report=${reportPath}`);
}

enrich();
