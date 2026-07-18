import fs from 'node:fs';
import path from 'node:path';

const bookDir = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const reportPath = 'docs/seed-vocab-choice-conflicts-report.json';
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
function clean(text) {
  return String(text ?? '')
    .replace(/[<>\uFFFD]/gu, '')
    .replace(/^[a-z.]+\s+/iu, '')
    .replace(/\s+/gu, ' ')
    .trim();
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
        for (const token of group) {
          if (chunk.includes(token) || token.includes(chunk)) {
            for (const related of group) out.add(related);
          }
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
function overlap(left, right) {
  const a = units(left);
  const b = units(right);
  const shared = [...a].filter((unit) => b.has(unit));
  return shared;
}
function getContent(item) { return item?.content?.word?.content ?? {}; }
function displayWord(item) { return String(item?.displayWord ?? item?.headWord ?? item?.content?.word?.wordHead ?? '').trim(); }
function meanings(item) {
  const values = (getContent(item).trans ?? []).map((entry) => clean(entry?.tranCn)).filter((value) => /[\p{Script=Han}]/u.test(value));
  if (values.length) return values;
  const override = meaningOverrides.get(normWord(displayWord(item)));
  return override ? [override] : [];
}
function primaryMeaning(item) { return meanings(item)[0] ?? ''; }
function normWord(word) { return String(word ?? '').toLowerCase(); }
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
function primaryPos(item) {
  const entry = (getContent(item).trans ?? []).find((candidate) => clean(candidate?.tranCn));
  const meaning = clean(entry?.tranCn);
  const embedded = meaning.match(/^(n|v|vt|vi|adj|adv|prep|conj|pron|int)\./iu)?.[1];
  return normalizePos(embedded ?? entry?.pos);
}
function sourceList(item) {
  const content = getContent(item);
  const sources = item?.choiceDistractorSources ?? item?.questionPrepMeta?.choiceDistractorSources ?? item?.content?.word?.choiceDistractorSources ?? content?.choiceDistractorSources ?? content?.questionPrepMeta?.choiceDistractorSources ?? [];
  return Array.isArray(sources) ? sources : [];
}

const report = { generatedAt: new Date().toISOString(), books: {}, conflicts: [] };
for (const fileName of requiredBooks) {
  const filePath = path.join(bookDir, fileName);
  const entries = readJson(filePath);
  const byWord = new Map(entries.map((item) => [normWord(displayWord(item)), item]));
  const stats = {
    entries: entries.length,
    targetCnConflicts: 0,
    targetEnConflicts: 0,
    optionInternalConflicts: 0,
    sourceMissing: 0,
    sourceShapeConflicts: 0,
    posConflicts: 0,
  };
  for (const item of entries) {
    const word = displayWord(item);
    const targetPos = primaryPos(item);
    const targetMeanings = meanings(item);
    const targetText = targetMeanings.join('；');
    const cn = item.cnChoiceDistractors ?? getContent(item).cnChoiceDistractors ?? [];
    const en = item.enChoiceDistractors ?? getContent(item).enChoiceDistractors ?? [];
    const sources = sourceList(item);
    if (cn.length !== 7 || en.length !== 7 || sources.length !== 7) {
      stats.sourceMissing += 1;
      report.conflicts.push({ book: fileName, word, type: 'source-missing', cn: cn.length, en: en.length, sources: sources.length });
    }
    for (let i = 0; i < Math.min(cn.length, en.length, sources.length); i += 1) {
      const source = sources[i] ?? {};
      const sourcePos = normalizePos(source.pos);
      if (source.word !== en[i] || source.meaning !== cn[i]) {
        stats.sourceShapeConflicts += 1;
        report.conflicts.push({ book: fileName, word, type: 'source-shape', index: i, cn: cn[i], en: en[i], source });
      }
      if (targetPos && sourcePos !== targetPos) {
        stats.posConflicts += 1;
        report.conflicts.push({ book: fileName, word, type: 'pos-mismatch', index: i, targetPos, sourcePos, choice: en[i], meaning: cn[i] });
      }
    }
    for (const choice of cn) {
      const shared = overlap(targetText, choice);
      if (shared.length) {
        stats.targetCnConflicts += 1;
        report.conflicts.push({ book: fileName, word, type: 'target-cn-choice', choice, shared: shared.slice(0, 8), targetMeaning: targetText });
      }
    }
    for (const choiceWord of en) {
      const target = byWord.get(normWord(choiceWord));
      if (!target) continue;
      const choiceMeaning = meanings(target).join('；');
      const shared = overlap(targetText, choiceMeaning);
      if (shared.length) {
        stats.targetEnConflicts += 1;
        report.conflicts.push({ book: fileName, word, type: 'target-en-choice', choice: choiceWord, choiceMeaning, shared: shared.slice(0, 8), targetMeaning: targetText });
      }
    }
    for (let i = 0; i < cn.length; i += 1) {
      for (let j = i + 1; j < cn.length; j += 1) {
        const shared = overlap(cn[i], cn[j]);
        if (shared.length) {
          stats.optionInternalConflicts += 1;
          report.conflicts.push({ book: fileName, word, type: 'cn-choice-internal', left: cn[i], right: cn[j], shared: shared.slice(0, 8) });
        }
      }
    }
  }
  report.books[fileName] = stats;
}
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(JSON.stringify(report.books, null, 2));
console.log(`conflicts=${report.conflicts.length} report=${reportPath}`);
if (report.conflicts.length > 0) process.exitCode = 1;
