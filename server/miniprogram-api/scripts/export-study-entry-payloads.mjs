import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const args = parseArgs(process.argv.slice(2));
const source = args.source ?? args.input;
const outputJson = args.outputJson ?? 'server/miniprogram-api/out/study-entry-payloads.json';
const outputSql = args.outputSql ?? 'server/miniprogram-api/out/study-entry-payloads.sql';
const limit = Number(args.limit ?? 0);

if (!source) {
  fail('Usage: node scripts/export-study-entry-payloads.mjs --source=<payloads.json|seed-vocab/book|word-mobile.sqlite> [--outputJson=...] [--outputSql=...] [--limit=100]');
}

const rows = readRows(source);
const limitedRows = limit > 0 ? rows.slice(0, limit) : rows;

if (limitedRows.length === 0) {
  fail(`No study payload rows exported from ${source}`);
}

writeJson(outputJson, limitedRows);
writeSql(outputSql, limitedRows);

console.log(
  JSON.stringify(
    {
      source,
      rows: limitedRows.length,
      outputJson,
      outputSql,
      firstSourceId: limitedRows[0].source_id,
    },
    null,
    2,
  ),
);

function readPayloadJson(filePath) {
  const raw = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  const items = Array.isArray(raw) ? raw : raw.rows;
  if (!Array.isArray(items)) {
    fail('JSON source must be an array or an object with rows[]');
  }
  return looksLikeKajwebEntries(items)
    ? readKajwebBookRows(filePath, items)
    : items.map(normalizePayloadRow);
}

function readRows(sourcePath) {
  const stat = fs.existsSync(sourcePath) ? fs.statSync(sourcePath) : null;
  if (stat?.isDirectory()) {
    return readKajwebBookDirectory(sourcePath);
  }
  if (sourcePath.endsWith('.json')) {
    return readPayloadJson(sourcePath);
  }
  return readPayloadsFromSqlite(sourcePath);
}

function readKajwebBookDirectory(directoryPath) {
  const bookDir = resolveKajwebBookDir(directoryPath);
  const bundleRoot = resolveBundleRoot(directoryPath, bookDir);
  const files = fs
    .readdirSync(bookDir, { withFileTypes: true })
    .filter((item) => item.isFile() && item.name.toLowerCase().endsWith('.json'))
    .map((item) => path.join(bookDir, item.name))
    .sort((a, b) => path.basename(a).localeCompare(path.basename(b)));

  if (files.length === 0) {
    fail(`No kajweb book JSON files found under ${directoryPath}`);
  }

  const entryRows = files.flatMap((filePath) => {
    const raw = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    if (!Array.isArray(raw)) {
      fail(`Kajweb book JSON must be an array: ${filePath}`);
    }
    return readKajwebBookRows(filePath, raw);
  });
  return [...entryRows, ...readRootAffixRows(bundleRoot, files)];
}

function resolveKajwebBookDir(directoryPath) {
  for (const candidate of [
    path.join(directoryPath, 'seed-vocab', 'book'),
    path.join(directoryPath, 'book'),
    directoryPath,
  ]) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isDirectory()) {
      return candidate;
    }
  }
  return directoryPath;
}

function resolveBundleRoot(sourcePath, bookDir) {
  const normalized = path.normalize(bookDir);
  if (normalized.endsWith(path.normalize(path.join('seed-vocab', 'book')))) {
    return path.dirname(path.dirname(bookDir));
  }
  if (path.basename(bookDir) === 'book' && path.basename(path.dirname(bookDir)) === 'seed-vocab') {
    return path.dirname(path.dirname(bookDir));
  }
  return sourcePath;
}

function looksLikeKajwebEntries(items) {
  const sample = items.find((item) => item && typeof item === 'object');
  return Boolean(sample?.headWord || sample?.content?.word?.content?.wordId);
}

function readKajwebBookRows(filePath, items) {
  const bookId = path.basename(filePath, '.json');
  return items
    .map((item, index) => normalizeKajwebEntry(item, bookId, index + 1))
    .filter(Boolean);
}

function normalizeKajwebEntry(item, bookId, rank) {
  const wordNode = item?.content?.word ?? {};
  const wordContent = wordNode.content ?? {};
  const sourceId = wordNode.wordId ?? wordContent.wordId ?? `${bookId}_${item?.headWord ?? rank}`;
  const word = item?.headWord ?? wordNode.wordHead ?? wordContent.wordHead;
  if (!sourceId || !word) return null;

  const meaningDetails = Array.isArray(wordContent.trans)
    ? wordContent.trans
        .map((meaning) => ({
          pos: String(meaning?.pos ?? ''),
          meaningCn: String(meaning?.tranCn ?? '').trim(),
          meaningEn: meaning?.tranOther ?? null,
        }))
        .filter((meaning) => meaning.meaningCn)
    : [];
  const meanings = meaningDetails.map((meaning) => meaning.meaningCn);
  const firstExample = Array.isArray(wordContent.sentence?.sentences)
    ? wordContent.sentence.sentences.find((sentence) => sentence?.sContent)
    : null;
  const wordbookId = stableEntryId(bookId);
  const rankInBook = Number(item.wordRank ?? rank);

  return normalizePayloadRow({
    entry_id: stableEntryId(sourceId),
    source_id: sourceId,
    word,
    part_of_speech: meaningDetails[0]?.pos || null,
    frequency: rankInBook > 0 ? 10000 / rankInBook : 0,
    phonetic_us: wordContent.usphone ?? null,
    phonetic_uk: wordContent.ukphone ?? null,
    meanings_json: meanings,
    meaning_details_json: meaningDetails,
    example_sentence: firstExample?.sContent ?? null,
    example_translation: firstExample?.sCn ?? null,
    wordbook_id: wordbookId,
    rank_in_book: rankInBook,
    is_active: true,
  });
}

function readRootAffixRows(bundleRoot, bookFiles) {
  const cards = new Map();
  for (const filePath of bookFiles) {
    if (path.basename(filePath) === 'MEDICAL_RESP.json') continue;
    const raw = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    if (!Array.isArray(raw)) continue;
    for (const item of raw) {
      for (const card of parseSharedRootAffixCards(item)) {
        mergeRootAffixCard(cards, card);
      }
    }
  }
  for (const card of readMedicalRootAffixCards(bundleRoot)) {
    if (!cards.has(card.id)) cards.set(card.id, card);
  }
  return [...cards.values()]
    .filter(isReliableRootAffixCard)
    .map(rootAffixCardToPayloadRow);
}

function parseSharedRootAffixCards(item) {
  const wordNode = item?.content?.word ?? {};
  const wordContent = wordNode.content ?? {};
  const word = String(wordNode.wordHead ?? item?.headWord ?? '').trim();
  const remMethod = String(wordContent.remMethod?.val ?? '').trim();
  if (!remMethod || word.length < 3) return [];
  const formula = remMethod.includes('->') ? remMethod.slice(0, remMethod.indexOf('->')) : remMethod;
  const matches = [...formula.matchAll(/([A-Za-z-]{2,12})\s*\(([^)]+)\)/g)]
    .map((match) => [match[1].trim(), match[2].trim()]);
  const lowerWord = word.toLowerCase();
  const gloss = sanitizeChineseMeaning(wordContent.trans?.[0]?.tranCn ?? '');
  const cards = [];
  matches.forEach(([rawForm, rawMeaning], index) => {
    const meaning = sanitizeChineseMeaning(rawMeaning);
    let normalized = normalizeRootAffixForm(rawForm);
    const promoted = promoteSharedRootAffixForm(lowerWord, normalized);
    const wasPromotedPrefix = Boolean(promoted);
    if (promoted) normalized = promoted;
    if (normalized.length < 2 || normalized.length >= lowerWord.length || !meaning) return;
    if (!sharedRootAffixFormMatchesWord(lowerWord, normalized, rawForm)) return;
    const displayForm = wasPromotedPrefix && lowerWord.startsWith(normalized)
      ? `${normalized}-`
      : rawForm.startsWith('-') || rawForm.endsWith('-')
        ? rawForm
        : formatSharedRootAffixForm(lowerWord, normalized, index, matches.length);
    cards.push({
      id: `root_affix_shared_${normalized}`,
      form: displayForm,
      meaningCn: meaning,
      examplePairs: [[word, compactRootExampleGloss(gloss)]],
      scope: 'shared',
    });
  });
  return cards;
}

function readMedicalRootAffixCards(bundleRoot) {
  const filePath = path.join(bundleRoot, 'seed-medical', 'medical-root-affix.txt');
  if (!fs.existsSync(filePath)) return builtinMedicalRootAffixCards();
  const reliableFileCards = new Map();
  const lines = fs.readFileSync(filePath, 'utf8').split(/\r?\n/);
  let lastCard;
  const parsedCards = [];
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    if (trimmed.startsWith('examples:')) {
      if (lastCard) lastCard.examplePairs = splitMedicalExamples(trimmed.slice('examples:'.length));
      continue;
    }
    const parts = trimmed.split(/\s+/);
    if (parts.length < 2) continue;
    const form = parts[0].trim();
    const meaning = sanitizeChineseMeaning(parts.slice(1).join(' '));
    const normalized = normalizeRootAffixForm(form);
    if (!normalized || !containsHan(meaning)) continue;
    lastCard = {
      id: `root_affix_medical_${normalized}`,
      form,
      meaningCn: meaning,
      examplePairs: [],
      scope: 'medical',
    };
    parsedCards.push(lastCard);
  }
  for (const card of parsedCards.filter(isReliableRootAffixCard)) {
    reliableFileCards.set(card.id, card);
  }
  for (const card of builtinMedicalRootAffixCards()) {
    if (!reliableFileCards.has(card.id)) reliableFileCards.set(card.id, card);
  }
  const cards = reliableFileCards;
  return [...cards.values()];
}

function rootAffixCardToPayloadRow(card) {
  const [exampleWords, exampleGlosses] = limitedRootExampleText(card.examplePairs, 3);
  return normalizePayloadRow({
    entry_id: stableEntryId(card.id),
    source_id: card.id,
    word: card.form,
    part_of_speech: 'root',
    frequency: 0,
    meanings_json: [card.meaningCn],
    meaning_details_json: [{ pos: 'root', meaningCn: card.meaningCn, meaningEn: null }],
    example_sentence: exampleWords || null,
    example_translation: exampleGlosses || null,
    wordbook_id: null,
    rank_in_book: card.scope === 'medical' ? 200000 : 100000,
    is_active: true,
  });
}

function mergeRootAffixCard(cards, incoming) {
  const existing = cards.get(incoming.id);
  if (!existing) {
    cards.set(incoming.id, incoming);
    return;
  }
  const seen = new Set(existing.examplePairs.map(([word]) => word.trim().toLowerCase()));
  for (const [word, gloss] of incoming.examplePairs) {
    const key = word.trim().toLowerCase();
    if (key && !seen.has(key)) {
      seen.add(key);
      existing.examplePairs.push([word.trim(), compactRootExampleGloss(gloss)]);
    }
  }
}

function promoteSharedRootAffixForm(word, normalized) {
  const longerPrefixes = [
    'anti', 'ante', 'auto', 'circum', 'contra', 'extra', 'inter', 'intra', 'intro', 'micro',
    'multi', 'post', 'semi', 'super', 'trans', 'ultra', 'abs', 'bio', 'dia', 'dis', 'fore',
    'mal', 'mis', 'non', 'pre', 'pro', 'sub', 'sur', 'sym', 'syn', 'tele', 'tri', 'con',
    'com', 'ab', 'ad', 'de', 'di', 'ex', 're', 'un',
  ];
  return longerPrefixes
    .filter((candidate) =>
      candidate.length > normalized.length &&
      candidate.startsWith(normalized) &&
      word.startsWith(candidate))
    .sort((left, right) => right.length - left.length)[0];
}

function sharedRootAffixFormMatchesWord(word, normalized, rawForm) {
  if (rawForm.startsWith('-')) return word.endsWith(normalized);
  if (rawForm.endsWith('-')) return word.startsWith(normalized);
  return word.startsWith(normalized) || word.endsWith(normalized);
}

function formatSharedRootAffixForm(word, normalized, index, total) {
  if (index === 0 && total > 1 && word.startsWith(normalized)) return `${normalized}-`;
  if (index + 1 === total && total > 1 && word.endsWith(normalized)) return `-${normalized}`;
  return normalized;
}

function isReliableRootAffixCard(card) {
  const normalized = normalizeRootAffixForm(card.form);
  if (normalized.length < 2 || normalized.length > 6) return false;
  if (card.scope === 'shared' && !isReliableSharedRootAffixForm(normalized, card)) return false;
  if (!containsHan(card.meaningCn) || !sanitizeChineseMeaning(card.meaningCn)) return false;
  const distinctExamples = new Set(card.examplePairs.map(([word]) => word.trim().toLowerCase()).filter(Boolean));
  return distinctExamples.size >= 2;
}

function isReliableSharedRootAffixForm(normalized, card) {
  if (!isAllowedSharedRootAffix(normalized, card.meaningCn)) return false;
  if (normalized.length > 4 || normalized === 'ear' || normalized === 'exe') return false;
  if ([...normalized].every((ch) => 'aeiou'.includes(ch))) return false;
  if (!card.form.endsWith('-') && !card.form.startsWith('-')) {
    const prefixHits = card.examplePairs.filter(([word]) => word.toLowerCase().startsWith(normalized)).length;
    if (prefixHits * 2 < Math.max(1, card.examplePairs.length)) return false;
  }
  return isConciseRootMeaningClean(card.meaningCn);
}

function isAllowedSharedRootAffix(normalized, meaning) {
  const allowed = {
    ab: ['离开', '远离'], abs: ['离开', '远离'], ad: ['向', '朝向', '加强'],
    ante: ['前', '先'], anti: ['反', '抗', '相反'], auto: ['自己', '自动'],
    bio: ['生命', '生物'], circ: ['圆', '环'], circum: ['周围', '环绕'],
    com: ['共同', '一起'], con: ['共同', '一起'], contra: ['反对', '相反'],
    de: ['向下', '离开', '否定'], di: ['二', '分开'], dia: ['穿过', '通过'],
    dis: ['分开', '否定', '不'], ex: ['出', '向外'], extra: ['外', '超出'],
    fore: ['前', '预先'], inter: ['之间', '相互'], intra: ['内部', '内'],
    intro: ['向内', '内部'], mal: ['坏', '恶'], micro: ['小', '微'],
    mis: ['错误', '坏'], mono: ['单', '一'], multi: ['多'], non: ['不', '无'],
    post: ['后'], pre: ['前', '预先'], pro: ['向前', '支持'], re: ['再', '重新', '回'],
    semi: ['半'], sub: ['下', '次'], super: ['上', '超过'], sur: ['上', '超过'],
    sym: ['共同', '一起'], syn: ['共同', '一起'], tele: ['远'],
    trans: ['横过', '转移', '改变'], tri: ['三'], ultra: ['超', '极端'], un: ['不', '相反'],
  }[normalized];
  if (!allowed) return false;
  const meaningKey = normalizeRootAffixMeaningKey(meaning);
  return allowed.some((value) => {
    const allowedKey = normalizeRootAffixMeaningKey(value);
    return meaningKey.includes(allowedKey) || allowedKey.includes(meaningKey);
  });
}

function isConciseRootMeaningClean(value) {
  const meaning = sanitizeChineseMeaning(value);
  return meaning && [...meaning].length <= 8 && !meaning.includes('电脑') && !meaning.includes('执行文件') && !meaning.includes('可执行');
}

function splitMedicalExamples(payload) {
  const pairs = [];
  const seen = new Set();
  for (const item of payload.split(/[;,]/)) {
    const match = item.trim().match(/^([^()]+)\(([^)]*)\)/);
    if (!match) continue;
    const word = match[1].trim();
    const key = word.toLowerCase();
    if (!word || seen.has(key)) continue;
    seen.add(key);
    pairs.push([word, compactRootExampleGloss(match[2])]);
  }
  return pairs;
}

function limitedRootExampleText(pairs, limit) {
  const selected = pairs.filter(([word]) => word.trim()).slice(0, limit);
  return [
    selected.map(([word]) => word.trim()).join(', '),
    selected.map(([, gloss]) => compactRootExampleGloss(gloss)).join(', '),
  ];
}

function compactRootExampleGloss(value) {
  const parts = [];
  const seen = new Set();
  for (const raw of String(value).split(/[;,/]/)) {
    const cleaned = sanitizeChineseMeaning(raw);
    if (!cleaned || [...cleaned].length > 12 || seen.has(cleaned)) continue;
    seen.add(cleaned);
    parts.push(cleaned);
    if (parts.length >= 3) break;
  }
  return parts.length > 0 ? parts.join(', ') : [...sanitizeChineseMeaning(value)].slice(0, 18).join('');
}

function builtinMedicalRootAffixCards() {
  return [
    medicalRootAffixCard('cardi-', '心脏', [['cardiology', '心脏病学'], ['cardiopulmonary', '心肺的']]),
    medicalRootAffixCard('bronch-', '支气管', [['bronchitis', '支气管炎'], ['bronchoscopy', '支气管镜检查']]),
    medicalRootAffixCard('pneumo-', '肺', [['pneumonia', '肺炎'], ['pneumothorax', '气胸']]),
    medicalRootAffixCard('hypo-', '低', [['hypoxia', '缺氧'], ['hypoxemia', '低氧血症']]),
    medicalRootAffixCard('hyper-', '高', [['hypertension', '高血压'], ['hypercapnia', '高碳酸血症']]),
    medicalRootAffixCard('-itis', '炎症', [['bronchitis', '支气管炎'], ['rhinitis', '鼻炎']]),
    medicalRootAffixCard('-emia', '血症', [['hypoxemia', '低氧血症'], ['anemia', '贫血']]),
    medicalRootAffixCard('-scopy', '镜检', [['bronchoscopy', '支气管镜检查'], ['endoscopy', '内镜检查']]),
    medicalRootAffixCard('trache-', '气管', [['tracheal', '气管的'], ['tracheostomy', '气管造口术']]),
    medicalRootAffixCard('pulmon-', '肺', [['pulmonary', '肺的'], ['extrapulmonary', '肺外的']]),
  ];
}

function medicalRootAffixCard(form, meaningCn, examplePairs) {
  const normalized = normalizeRootAffixForm(form);
  return {
    id: `root_affix_medical_${normalized}`,
    form,
    meaningCn,
    examplePairs,
    scope: 'medical',
  };
}

function sanitizeChineseMeaning(value) {
  return String(value).trim().replace(/^[\[\],;/\s]+|[\[\],;/\s]+$/g, '');
}

function containsHan(value) {
  return /[\u4e00-\u9fff]/.test(String(value));
}

function normalizeRootAffixForm(form) {
  return String(form).replace(/[^A-Za-z]/g, '').toLowerCase();
}

function normalizeRootAffixMeaningKey(value) {
  return sanitizeChineseMeaning(value).replace(/[\uFF0C,;\uFF1B\s/]/g, '');
}

function readPayloadsFromSqlite(dbPath) {
  if (!fs.existsSync(dbPath)) {
    fail(`SQLite source does not exist: ${dbPath}`);
  }
  const query = `
with meaning_rows as (
  select
    e.id as entry_id,
    json_group_array(em.meaning_cn) as meanings_json,
    json_group_array(json_object('pos', em.pos, 'meaningCn', em.meaning_cn, 'meaningEn', em.meaning_en)) as meaning_details_json
  from entries e
  left join entry_meanings em on em.entry_id = e.id
  group by e.id
),
example_rows as (
  select
    entry_id,
    min(sentence_en) as example_sentence,
    min(sentence_cn) as example_translation
  from entry_examples
  group by entry_id
),
book_rows as (
  select
    entry_id,
    min(wordbook_id) as wordbook_id,
    min(rank_in_book) as rank_in_book
  from wordbook_entries
  group by entry_id
)
select json_object(
  'entry_id', e.id,
  'source_id', e.source_entry_key,
  'word', e.word,
  'part_of_speech', nullif(e.part_of_speech, ''),
  'frequency', coalesce(e.frequency, 0),
  'phonetic_us', nullif(e.phonetic_us, ''),
  'phonetic_uk', nullif(e.phonetic_uk, ''),
  'meanings_json', json(coalesce(m.meanings_json, '[]')),
  'meaning_details_json', json(coalesce(m.meaning_details_json, '[]')),
  'example_sentence', x.example_sentence,
  'example_translation', x.example_translation,
  'wordbook_id', b.wordbook_id,
  'rank_in_book', coalesce(b.rank_in_book, 0),
  'is_active', 1
)
from entries e
left join meaning_rows m on m.entry_id = e.id
left join example_rows x on x.entry_id = e.id
left join book_rows b on b.entry_id = e.id
order by coalesce(b.wordbook_id, 0), coalesce(b.rank_in_book, e.id), e.id;
`;
  const sqlite = spawnSync('sqlite3', ['-json', dbPath, query], {
    encoding: 'utf8',
    windowsHide: true,
    maxBuffer: 1024 * 1024 * 200,
  });
  if (sqlite.error) {
    fail(`Failed to run sqlite3. Install sqlite3 CLI or export JSON first. ${sqlite.error.message}`);
  }
  if (sqlite.status !== 0) {
    fail(`sqlite3 failed: ${sqlite.stderr}`);
  }
  const rawRows = JSON.parse(sqlite.stdout || '[]');
  return rawRows.map((row) => normalizePayloadRow(JSON.parse(row['json_object('] ?? Object.values(row)[0])));
}

function normalizePayloadRow(item) {
  const sourceId = item.source_id ?? item.sourceId;
  const word = item.word;
  if (!sourceId || !word) {
    fail(`Payload row missing source_id/sourceId or word: ${JSON.stringify(item)}`);
  }
  const meaningDetails = item.meaning_details_json ?? item.meaningDetails ?? [];
  const meanings = item.meanings_json ?? item.meanings ?? [];
  return {
    entry_id: Number(item.entry_id ?? item.entryId ?? stableEntryId(sourceId)),
    source_id: String(sourceId),
    word: String(word),
    part_of_speech: item.part_of_speech ?? item.partOfSpeech ?? null,
    frequency: Number(item.frequency ?? 0),
    phonetic_us: item.phonetic_us ?? item.phoneticUs ?? null,
    phonetic_uk: item.phonetic_uk ?? item.phoneticUk ?? null,
    meanings_json: normalizeMeanings(meanings, meaningDetails),
    meaning_details_json: normalizeMeaningDetails(meaningDetails, meanings),
    example_sentence: item.example_sentence ?? item.exampleSentence ?? null,
    example_translation: item.example_translation ?? item.exampleTranslation ?? null,
    wordbook_id: nullableNumber(item.wordbook_id ?? item.wordbookId),
    rank_in_book: Number(item.rank_in_book ?? item.rankInBook ?? 0),
    is_active: item.is_active ?? item.isActive ?? true,
  };
}

function normalizeMeanings(meanings, meaningDetails) {
  const source = Array.isArray(meanings) && meanings.length > 0
    ? meanings
    : Array.isArray(meaningDetails)
      ? meaningDetails.map((item) => item.meaningCn ?? item.meaning_cn).filter(Boolean)
      : [];
  return source.map(String).filter(Boolean);
}

function normalizeMeaningDetails(meaningDetails, meanings) {
  if (Array.isArray(meaningDetails) && meaningDetails.length > 0) {
    return meaningDetails.map((item) => ({
      pos: String(item.pos ?? ''),
      meaningCn: String(item.meaningCn ?? item.meaning_cn ?? ''),
      meaningEn: item.meaningEn ?? item.meaning_en ?? null,
    })).filter((item) => item.meaningCn);
  }
  return normalizeMeanings(meanings, []).map((meaning) => ({
    pos: '',
    meaningCn: meaning,
    meaningEn: null,
  }));
}

function writeJson(filePath, rows) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(rows, null, 2)}\n`, 'utf8');
}

function writeSql(filePath, rows) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const values = rows.map((row) => `(${[
    row.entry_id,
    sqlString(row.source_id),
    sqlString(row.word),
    sqlString(row.part_of_speech),
    row.frequency,
    sqlString(row.phonetic_us),
    sqlString(row.phonetic_uk),
    `${sqlString(JSON.stringify(row.meanings_json))}::jsonb`,
    `${sqlString(JSON.stringify(row.meaning_details_json))}::jsonb`,
    sqlString(row.example_sentence),
    sqlString(row.example_translation),
    row.wordbook_id ?? 'null',
    row.rank_in_book,
    row.is_active ? 'true' : 'false',
  ].join(', ')})`);
  const sql = `insert into public.study_entry_payloads (
  entry_id,
  source_id,
  word,
  part_of_speech,
  frequency,
  phonetic_us,
  phonetic_uk,
  meanings_json,
  meaning_details_json,
  example_sentence,
  example_translation,
  wordbook_id,
  rank_in_book,
  is_active
) values
${values.join(',\n')}
on conflict (entry_id) do update set
  source_id = excluded.source_id,
  word = excluded.word,
  part_of_speech = excluded.part_of_speech,
  frequency = excluded.frequency,
  phonetic_us = excluded.phonetic_us,
  phonetic_uk = excluded.phonetic_uk,
  meanings_json = excluded.meanings_json,
  meaning_details_json = excluded.meaning_details_json,
  example_sentence = excluded.example_sentence,
  example_translation = excluded.example_translation,
  wordbook_id = excluded.wordbook_id,
  rank_in_book = excluded.rank_in_book,
  is_active = excluded.is_active,
  updated_at = now();
`;
  fs.writeFileSync(filePath, sql, 'utf8');
}

function sqlString(value) {
  if (value == null) return 'null';
  return `'${String(value).replaceAll("'", "''")}'`;
}

function nullableNumber(value) {
  if (value == null || value === '') return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function stableEntryId(sourceId) {
  let hash = 0;
  for (const char of String(sourceId)) {
    hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  }
  return hash;
}

function parseArgs(argv) {
  const result = {};
  for (const arg of argv) {
    if (!arg.startsWith('--')) continue;
    const [key, ...rest] = arg.slice(2).split('=');
    result[key] = rest.join('=') || true;
  }
  return result;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
