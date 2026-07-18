import fs from 'node:fs';
import path from 'node:path';

const bookDir = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const reportPath = 'docs/seed-vocab-choice-conflict-repair-report.json';
const maxDistractors = 7;
const requiredBooks = ['CET4_3.json', 'CET6_3.json', 'KaoYan_3.json', 'MEDICAL_RESP.json'];

const groups = [
  '插入 嵌入 插进 插手 介入 注入 投入 塞入 放入 夹入 纳入 镶嵌 嵌进 刺入 戳入 输入 安放 安插 进入 透入 渗入 侵入 卷入 封入 加插图 潜入',
  '取消 撤销 删除 删去 抵消 废除 作废 解除 终止',
  '吸收 消化 同化 并入 合并 吸引 吸入 吸气',
  '爆炸 爆发 引爆 爆破 猛烈爆发',
  '取代 代替 替换 替代',
  '治疗 疗法 医治 诊治 医疗',
  '基础 根基 地基 根本 基本 基准',
  '帮助 协助 援助 支持 辅助',
  '减少 降低 下降 削减 缩小',
  '增加 增长 提高 扩大 增强 加强',
  '重要 重大 关键 要紧',
  '明显 显著 清楚 明白',
  '困难 艰难 难题 困境',
  '快速 迅速 立即 马上',
  '破坏 损坏 毁坏 毁灭 摧毁',
  '连接 联合 结合 联系 关联',
  '分离 分开 隔离 脱离',
  '选择 挑选 选举 选拔',
  '证明 证实 说明 表明 显示',
  '改变 变化 改造 转变',
  '之中 中间 围绕 其中 当中 之间',
  '然而 但是 尽管 尽管如此 依然 不过 可是',
].map((line) => line.split(/\s+/u));
const groupIndex = new Map();
for (const group of groups) for (const token of group) groupIndex.set(token, group);
const generic = new Set('使 被 把 给 对 和 与 及 或 在 有 无 不 非 某 一种 一个 东西 事情 进行 表示 用于 关于 产生 发生 成为 变成 的 地 得 者 物 人'.split(/\s+/u));
const splitRe = /[;,，；、。:：/?？\s]+/u;
const supplementalDistractors = [
  { word: 'although', pos: 'conj', meaning: '虽然' },
  { word: 'because', pos: 'conj', meaning: '因为' },
  { word: 'whether', pos: 'conj', meaning: '是否' },
  { word: 'once', pos: 'conj', meaning: '一旦' },
  { word: 'before', pos: 'conj', meaning: '在...之前' },
  { word: 'after', pos: 'conj', meaning: '在...之后' },
  { word: 'until', pos: 'conj', meaning: '直到' },
  { word: 'whenever', pos: 'conj', meaning: '无论何时' },
  { word: 'wherever', pos: 'conj', meaning: '无论哪里' },
  { word: 'above', pos: 'prep', meaning: '在...上方' },
  { word: 'below', pos: 'prep', meaning: '在...下方' },
  { word: 'beyond', pos: 'prep', meaning: '超出' },
  { word: 'beneath', pos: 'prep', meaning: '在...下面' },
  { word: 'beside', pos: 'prep', meaning: '在...旁边' },
  { word: 'throughout', pos: 'prep', meaning: '遍及' },
  { word: 'unlike', pos: 'prep', meaning: '不像' },
  { word: 'toward', pos: 'prep', meaning: '朝向' },
  { word: 'against', pos: 'prep', meaning: '反对' },
  { word: 'someone', pos: 'pron', meaning: '某人' },
  { word: 'nobody', pos: 'pron', meaning: '没有人' },
  { word: 'others', pos: 'pron', meaning: '其他人' },
  { word: 'either', pos: 'pron', meaning: '任一' },
  { word: 'neither', pos: 'pron', meaning: '两者都不' },
  { word: 'each', pos: 'pron', meaning: '每个' },
  { word: 'both', pos: 'pron', meaning: '两者都' },
  { word: 'hello', pos: 'int', meaning: '你好' },
  { word: 'alas', pos: 'int', meaning: '唉' },
  { word: 'bravo', pos: 'int', meaning: '好哇' },
  { word: 'ouch', pos: 'int', meaning: '哎哟' },
  { word: 'wow', pos: 'int', meaning: '哇' },
  { word: 'hey', pos: 'int', meaning: '嘿' },
  { word: 'cheers', pos: 'int', meaning: '干杯' },
  { word: 'hush', pos: 'int', meaning: '嘘' },
  { word: 'one', pos: 'num', meaning: '一' },
  { word: 'two', pos: 'num', meaning: '二' },
  { word: 'three', pos: 'num', meaning: '三' },
  { word: 'four', pos: 'num', meaning: '四' },
  { word: 'five', pos: 'num', meaning: '五' },
  { word: 'six', pos: 'num', meaning: '六' },
  { word: 'seven', pos: 'num', meaning: '七' },
  { word: 'eight', pos: 'num', meaning: '八' },
];
const meaningOverrides = new Map(Object.entries({
  reform: '改革；改良',
  tow: '拖曳；牵引',
  dread: '恐惧；畏惧',
  boycott: '抵制；联合抵制',
  curse: '诅咒；咒骂',
  nonetheless: '然而；尽管如此',
  twinkle: '闪烁；闪光',
  glide: '滑行；滑动',
  auction: '拍卖',
  revolt: '反抗；叛乱',
  slap: '掌掴；拍击',
  arrest: '逮捕；拘留',
  fore: '前面的；前部的',
  gaze: '凝视；注视',
  exile: '流放；流亡',
  compute: '计算',
  shiver: '颤抖；发抖',
  hug: '拥抱',
  envy: '嫉妒；羡慕',
  blush: '脸红；羞愧',
  collapse: '倒塌；崩溃',
  debate: '辩论；争论',
  assault: '攻击；袭击',
  giggle: '咯咯笑；傻笑',
  glimpse: '一瞥；瞥见',
  stroll: '散步；闲逛',
  cease: '停止；终止',
}));

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}
function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
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
      sleep(120 * (attempt + 1));
    }
  }
  throw lastError;
}
function getWordNode(item) {
  return item?.content?.word ?? {};
}
function getContent(item) {
  return getWordNode(item)?.content ?? {};
}
function clean(text) {
  return String(text ?? '')
    .replace(/[<>\uFFFD]/gu, '')
    .replace(/^[a-z.]+\s+/iu, '')
    .replace(/\s+/gu, ' ')
    .trim();
}
function displayWord(item) {
  return String(item?.displayWord ?? getWordNode(item)?.displayWord ?? getContent(item)?.displayWord ?? item?.headWord ?? getWordNode(item)?.wordHead ?? '').trim();
}
function sourceId(item, word) {
  return String(getWordNode(item)?.wordId ?? word).trim();
}
function meanings(item) {
  const values = (getContent(item).trans ?? []).map((entry) => clean(entry?.tranCn)).filter((value) => /[\p{Script=Han}]/u.test(value));
  if (values.length) return values;
  const override = meaningOverrides.get(normWord(displayWord(item)));
  return override ? [override] : [];
}
function primaryMeaning(item) {
  return meanings(item)[0] ?? '';
}
function allMeaningText(item) {
  return meanings(item).join('；');
}
function normalizePos(pos) {
  const value = String(pos ?? '').trim().replace(/\.+$/u, '').toLowerCase();
  if (!value) return '';
  if (value.startsWith('v')) return 'v';
  if (value.startsWith('n')) return 'n';
  if (value.startsWith('adj') || value === 'a') return 'adj';
  if (value.startsWith('adv')) return 'adv';
  if (value.startsWith('prep')) return 'prep';
  if (value.startsWith('conj')) return 'conj';
  if (value.startsWith('pron')) return 'pron';
  if (value === 'int' || value.startsWith('interj')) return 'int';
  if (['effect', 'dioxide', 'hormone', 'failure', 'dose', 'computed', 'magnetic', 'concentration', 'therapy', 'index'].includes(value)) return 'n';
  return value;
}
function broadPos(pos) {
  const p = normalizePos(pos);
  return ['n', 'v', 'adj', 'adv', 'prep', 'conj', 'pron', 'int'].includes(p) ? p : p || 'unknown';
}
function primaryPos(item) {
  const entry = (getContent(item).trans ?? []).find((candidate) => clean(candidate?.tranCn));
  const meaning = clean(entry?.tranCn);
  const embedded = meaning.match(/^(n|v|vt|vi|adj|adv|prep|conj|pron|int)\./iu)?.[1];
  return normalizePos(embedded ?? entry?.pos);
}
function normWord(word) {
  return String(word ?? '').toLowerCase().replace(/[^a-z0-9]/gu, '');
}
function meaningKey(text) {
  return clean(text).replace(/[\s,.;:，。；：、？]/gu, '').toLowerCase();
}
function units(text) {
  const parts = clean(text)
    .replace(/[()（）\[\]【】]/gu, '；')
    .split(splitRe)
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => part.replace(/^(使|被|把|给|将|对|与|和|以|为|向|从)/u, '').trim())
    .filter((part) => part.length >= 2 && !generic.has(part));
  const out = new Set();
  for (const part of parts) {
    out.add(part);
    const chunks = part.match(/[\p{Script=Han}]{2,}/gu) ?? [];
    for (const chunk of chunks) {
      if (chunk.length >= 2 && chunk.length <= 4 && !generic.has(chunk)) out.add(chunk);
      for (const group of groups) {
        if (group.some((token) => chunk.includes(token) || token.includes(chunk))) {
          for (const related of group) out.add(related);
        }
      }
    }
  }
  for (const unit of [...out]) {
    const group = groupIndex.get(unit);
    if (group) for (const related of group) out.add(related);
  }
  return out;
}
function sharedUnits(leftUnits, rightUnits) {
  return [...leftUnits].filter((unit) => rightUnits.has(unit));
}
function sameSemanticGroup(left, right) {
  const a = clean(left);
  const b = clean(right);
  if (!a || !b) return false;
  return groups.some((group) => group.some((token) => a.includes(token)) && group.some((token) => b.includes(token)));
}
function tooSimilarText(left, right) {
  const a = meaningKey(left);
  const b = meaningKey(right);
  if (!a || !b) return false;
  if (a === b) return true;
  const shorter = Math.min(a.length, b.length);
  return shorter >= 2 && (a.includes(b) || b.includes(a));
}
function wordTooSimilar(left, right) {
  const a = normWord(left);
  const b = normWord(right);
  if (!a || !b) return false;
  if (a === b) return true;
  const shorter = Math.min(a.length, b.length);
  return shorter >= 3 && (a.includes(b) || b.includes(a));
}
function conflicts(left, right) {
  if (sameSemanticGroup(left.text, right.text)) return true;
  if (tooSimilarText(left.text, right.text)) return true;
  return sharedUnits(left.units, right.units).length > 0;
}
function candidateConflicts(target, candidate) {
  if (wordTooSimilar(target.word, candidate.word)) return true;
  return conflicts(target.meaningProfile, candidate.meaningProfile);
}
function relationConflicts(profiles, candidateProfile) {
  return profiles.some((profile) => conflicts(profile, candidateProfile));
}
function setFields(item, cn, en, meta) {
  item.cnChoiceDistractors = cn;
  item.enChoiceDistractors = en;
  item.questionPrepMeta = meta;
  item.choiceDistractorSources = meta.choiceDistractorSources;
  const wordNode = getWordNode(item);
  if (wordNode && typeof wordNode === 'object') {
    wordNode.cnChoiceDistractors = cn;
    wordNode.enChoiceDistractors = en;
    wordNode.questionPrepMeta = meta;
    wordNode.choiceDistractorSources = meta.choiceDistractorSources;
  }
  const content = getContent(item);
  if (content && typeof content === 'object') {
    content.cnChoiceDistractors = cn;
    content.enChoiceDistractors = en;
    content.questionPrepMeta = meta;
    content.choiceDistractorSources = meta.choiceDistractorSources;
  }
}

const books = [];
const all = [];
for (const fileName of requiredBooks) {
  const filePath = path.join(bookDir, fileName);
  const items = readJson(filePath);
  const book = path.basename(fileName, '.json');
  books.push({ book, fileName, filePath, items });
  for (const [index, item] of items.entries()) {
    const word = displayWord(item);
    const override = meaningOverrides.get(normWord(word));
    const trans = getContent(item).trans ?? [];
    if (override && trans.length && !trans.some((entry) => /[\p{Script=Han}]/u.test(clean(entry?.tranCn)))) {
      trans[0].tranCn = override;
    }
    const meaning = primaryMeaning(item);
    const conflictMeaning = allMeaningText(item);
    const pos = primaryPos(item);
    if (!word || !meaning) continue;
    all.push({
      item,
      book,
      index,
      word,
      sourceId: sourceId(item, word),
      pos,
      broadPos: broadPos(pos),
      meaning,
      conflictMeaning,
      meaningProfile: { text: conflictMeaning, units: units(conflictMeaning) },
      rank: Number(item?.wordRank ?? index + 1),
      supplemental: false,
    });
  }
}
for (const [index, candidate] of supplementalDistractors.entries()) {
  const pos = normalizePos(candidate.pos);
  all.push({
    item: null,
    book: 'SUPPLEMENTAL',
    index,
    word: candidate.word,
    sourceId: `supplemental:${candidate.pos}:${candidate.word}`,
    pos,
    broadPos: broadPos(pos),
    meaning: candidate.meaning,
    conflictMeaning: candidate.meaning,
    meaningProfile: { text: candidate.meaning, units: units(candidate.meaning) },
    rank: 100000 + index,
    supplemental: true,
  });
}
const byBook = new Map();
for (const c of all) {
  if (!byBook.has(c.book)) byBook.set(c.book, []);
  byBook.get(c.book).push(c);
}
const report = { generatedAt: new Date().toISOString(), books: {}, repaired: [], incomplete: [] };

function tiersFor(target) {
  const sameBook = byBook.get(target.book) ?? [];
  return [
    ['same-book-same-pos', sameBook.filter((c) => c.pos === target.pos)],
    ['cross-book-same-pos', all.filter((c) => c.pos === target.pos)],
  ];
}
function orderedPool(target, pool) {
  return pool
    .filter((c) => c !== target && c.sourceId !== target.sourceId && normWord(c.word) !== normWord(target.word))
    .sort((a, b) => Math.abs(a.rank - target.rank) - Math.abs(b.rank - target.rank) || a.word.localeCompare(b.word));
}

for (const target of all.filter((candidate) => !candidate.supplemental)) {
  const choices = [];
  const choiceProfiles = [];
  const seenCn = new Set();
  const seenEn = new Set();
  const sources = new Set();
  const addCandidate = (candidate, tier) => {
    if (candidate.pos !== target.pos) return;
    if (candidateConflicts(target, candidate)) return;
    const cnText = candidate.meaning;
    const cnKey = meaningKey(cnText);
    const enText = candidate.word;
    const enKey = normWord(enText);
    if (choices.length >= maxDistractors || !cnKey || !enKey || seenCn.has(cnKey) || seenEn.has(enKey)) return;
    if (relationConflicts(choiceProfiles, candidate.meaningProfile)) return;
    choices.push(candidate);
    choiceProfiles.push(candidate.meaningProfile);
    seenCn.add(cnKey);
    seenEn.add(enKey);
    sources.add(tier);
  };
  for (const [tier, pool] of tiersFor(target)) {
    for (const candidate of orderedPool(target, pool)) {
      addCandidate(candidate, tier);
      if (choices.length >= maxDistractors) break;
    }
    if (choices.length >= maxDistractors) break;
  }
  const cn = choices.map((candidate) => candidate.meaning);
  const en = choices.map((candidate) => candidate.word);
  const meta = {
    ...(target.item.questionPrepMeta ?? {}),
    version: 4,
    maxDistractors,
    displayWord: target.word,
    primaryPos: target.pos || 'unknown',
    broadPos: target.broadPos,
    strictSamePos: true,
    sources: [...sources],
    choiceDistractorSources: choices.map((candidate) => ({
      word: candidate.word,
      pos: candidate.pos || 'unknown',
      broadPos: candidate.broadPos,
      meaning: candidate.meaning,
      book: candidate.book,
    })),
  };
  setFields(target.item, cn, en, meta);
  if (choices.length < maxDistractors) {
    report.incomplete.push({ book: target.book, word: target.word, cn: cn.length, en: en.length });
  }
}

for (const book of books) {
  const entries = byBook.get(book.book) ?? [];
  report.books[book.book] = {
    entries: book.items.length,
    candidates: entries.length,
    incomplete: report.incomplete.filter((item) => item.book === book.book).length,
  };
  writeJson(book.filePath, book.items);
}
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(JSON.stringify(report.books, null, 2));
console.log(`incomplete=${report.incomplete.length} report=${reportPath}`);
if (report.incomplete.length > 0) process.exitCode = 1;
