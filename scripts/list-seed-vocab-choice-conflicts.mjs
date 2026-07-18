import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const BOOK_DIR = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const BOOK_FILES = ['CET4_3.json', 'CET6_3.json', 'KaoYan_3.json', 'MEDICAL_RESP.json'];
const SPLIT_RE = /[;,，；、。:：/?？\s]+/u;
const GENERIC_PHRASES = new Set([
  '上的', '以下', '以上', '之中', '其中', '当中', '之间', '一种', '一个', '结果', '过程',
  '进行', '表示', '用于', '相关', '发生', '产生', '造成', '引起', '导致', '具有', '可以',
  '可能', '以及', '工作', '方法', '的人', '令人', '发出', '认为', '全体', '用力', '自然',
  '能力', '优势',
]);

const SEMANTIC_GROUPS = [
  '控制 约束 抑制 克制 限制 管制 压制 镇压 制服 支配 统治 束缚 牵制',
  '阻止 制止 劝阻 禁止 不准',
  '移置 转移 移动 调动 迁移 移居 置换 替换 取代 替代 代替 更换 调换 互换 交换',
  '插入 嵌入 插进 注入 塞入 放入 夹入 纳入 镶嵌 嵌进 安插',
  '炉 炉子 火炉 熔炉 电炉 烤箱 炉灶 灶 壁炉',
  '取消 撤销 删除 删去 抵消 废除 作废 解除 终止 停止',
  '吸收 消化 同化 并入 合并',
  '爆炸 爆发 引爆 爆破 猛烈爆发',
  '治疗 疗法 医治 诊治 医疗 治愈',
  '基础 根基 地基 根本 基本 基准',
  '帮助 协助 援助 支持 辅助',
  '减少 降低 下降 削减 缩小 减轻',
  '增加 增长 提高 扩大 增强 加强 膨胀 扩张',
  '重要 重大 关键 要紧',
  '明显 显著 清楚 明白 突出',
  '困难 艰难 难题 困境',
  '快速 迅速 立即 马上',
  '破坏 损坏 毁坏 毁灭 摧毁',
  '连接 联合 结合 联系 关联',
  '分离 分开 隔离 脱离',
  '选择 挑选 选举 选拔',
  '证明 证实 说明 表明 显示',
  '改变 变化 改造 转变 转化 变换',
  '拒绝 拒纳 退回 驳回 排斥 抵制',
  '恐惧 害怕 惊吓 惊恐 畏惧',
  '攻击 袭击 打击 进攻',
  '保护 维持 保存 保藏 保留',
].map((line) => line.split(/\s+/u));

function clean(text) {
  return String(text ?? '')
    .replace(/[<>\uFFFD]/gu, '')
    .replace(/\.{2,}|…+/gu, '')
    .replace(/\s+/gu, '')
    .trim();
}

function components(text) {
  return String(text ?? '')
    .replace(/[()（）\[\]【】]/gu, '；')
    .split(SPLIT_RE)
    .map(clean)
    .map((part) => part.length > 2 ? part.replace(/^(使|被|把|给|将|对|与|和|以|为|向|从)/u, '') : part)
    .filter(Boolean);
}

function sharedSemanticGroup(left, right) {
  for (const group of SEMANTIC_GROUPS) {
    const matches = (part, token) => token.length === 1 ? part === token : part.includes(token);
    const leftHits = group.filter((token) => left.some((part) => matches(part, token)));
    const rightHits = group.filter((token) => right.some((part) => matches(part, token)));
    if (leftHits.length && rightHits.length) {
      return [...new Set([...leftHits, ...rightHits])].join('/');
    }
  }
  return '';
}

function longestSharedHan(left, right) {
  let best = '';
  for (const a of left) {
    for (const b of right) {
      const aChars = Array.from(a);
      for (let start = 0; start < aChars.length; start += 1) {
        for (let end = start + 2; end <= aChars.length; end += 1) {
          const candidate = aChars.slice(start, end).join('');
          if (
            /^[\p{Script=Han}]+$/u.test(candidate) &&
            !GENERIC_PHRASES.has(candidate) &&
            !candidate.endsWith('的') &&
            b.includes(candidate) &&
            candidate.length > best.length
          ) {
            best = candidate;
          }
        }
      }
    }
  }
  return best;
}

export function classifyMeaningConflict(correctMeaning, distractorMeaning) {
  const correct = components(correctMeaning);
  const distractor = components(distractorMeaning);
  if (!correct.length || !distractor.length) return null;

  for (const left of correct) {
    for (const right of distractor) {
      if (left === right) return { type: 'exact', evidence: left };
    }
  }

  for (const left of correct) {
    for (const right of distractor) {
      const shorter = left.length <= right.length ? left : right;
      if (shorter.length >= 2 && (left.includes(right) || right.includes(left))) {
        return { type: 'contains', evidence: shorter };
      }
    }
  }

  const phrase = longestSharedHan(correct, distractor);
  if (phrase) return { type: 'shared-phrase', evidence: phrase };

  const semanticEvidence = sharedSemanticGroup(correct, distractor);
  if (semanticEvidence) return { type: 'semantic-group', evidence: semanticEvidence };
  return null;
}

function getContent(item) {
  return item?.content?.word?.content ?? {};
}

function normalizePos(pos) {
  const value = String(pos ?? '').trim().replace(/\.+$/u, '').toLowerCase();
  if (value.startsWith('v')) return 'v';
  if (value.startsWith('n')) return 'n';
  if (value.startsWith('adj') || value === 'a') return 'adj';
  if (value.startsWith('adv')) return 'adv';
  if (value.startsWith('prep')) return 'prep';
  if (value.startsWith('conj')) return 'conj';
  if (value.startsWith('pron')) return 'pron';
  return value;
}

function displayWord(item) {
  return String(
    item?.displayWord ?? item?.content?.word?.displayWord ?? item?.headWord ?? item?.content?.word?.wordHead ?? '',
  ).trim();
}

function correctMeaning(item) {
  return (getContent(item).trans ?? [])
    .map((entry) => clean(entry?.tranCn))
    .filter((value) => /\p{Script=Han}/u.test(value))
    .join('；');
}

function primaryEntry(item, index) {
  const translation = (getContent(item).trans ?? []).find((entry) => clean(entry?.tranCn));
  return {
    id: index,
    word: displayWord(item),
    pos: normalizePos(translation?.pos),
    meaning: clean(translation?.tranCn),
    rank: Number(item?.wordRank ?? index),
  };
}

function candidateEntries(item, index) {
  return (getContent(item).trans ?? [])
    .map((translation, meaningIndex) => ({
      id: index,
      meaningId: `${index}:${meaningIndex}`,
      word: displayWord(item),
      pos: normalizePos(translation?.pos),
      meaning: clean(translation?.tranCn),
      rank: Number(item?.wordRank ?? index),
    }))
    .filter((entry) => entry.word && entry.pos && entry.meaning);
}

function normalizeOverlapText(value) {
  return clean(value).replace(/[,.，。;；:：/\s]/gu, '');
}

function runtimeOverlapScore(leftValue, rightValue) {
  const left = normalizeOverlapText(leftValue);
  const right = normalizeOverlapText(rightValue);
  if (!left || !right) return 0;
  if (left.includes(right)) return 100 + Array.from(right).length;
  const leftChars = new Set(Array.from(left));
  return Array.from(right).filter((character) => leftChars.has(character)).length;
}

export function rankRuntimeDistractors(target, candidates, limit = 7) {
  const ranked = candidates
    .filter((candidate) => candidate.id !== target.id)
    .filter((candidate) => target.pos && candidate.pos === target.pos)
    .filter((candidate) => normalizeOverlapText(candidate.meaning) !== normalizeOverlapText(target.meaning))
    .map((candidate) => ({
      candidate,
      score: runtimeOverlapScore(target.meaning, candidate.meaning),
      distance: Math.abs(candidate.rank - target.rank),
    }))
    .sort((left, right) =>
      right.score - left.score ||
      left.distance - right.distance ||
      left.candidate.id - right.candidate.id,
    );
  const selected = [];
  const seenMeanings = new Set();
  for (const item of ranked) {
    const key = normalizeOverlapText(item.candidate.meaning);
    if (!key || seenMeanings.has(key)) continue;
    seenMeanings.add(key);
    selected.push(item.candidate);
    if (selected.length >= limit) break;
  }
  return selected;
}

function withRuntimeDistractors(items) {
  const targets = items.map(primaryEntry).filter((entry) => entry.word && entry.pos && entry.meaning);
  const candidates = items.flatMap(candidateEntries);
  const byId = new Map(targets.map((entry) => [entry.id, entry]));
  return items.map((item, index) => {
    const target = byId.get(index);
    if (!target) return item;
    const choices = rankRuntimeDistractors(target, candidates);
    return {
      ...item,
      cnChoiceDistractors: choices.map((choice) => choice.meaning),
      enChoiceDistractors: choices.map((choice) => choice.word),
    };
  });
}

export function applyEffectiveRuntimeQuestionPreps(items) {
  const generated = withRuntimeDistractors(items);
  return items.map((item, index) => {
    if (cnChoices(item).length >= 3 && enChoices(item).length >= 3) return item;
    return generated[index];
  });
}

function cnChoices(item) {
  const content = getContent(item);
  const values = item?.cnChoiceDistractors ?? content?.cnChoiceDistractors ?? [];
  return Array.isArray(values) ? values : [];
}

function enChoices(item) {
  const content = getContent(item);
  const values = item?.enChoiceDistractors ?? content?.enChoiceDistractors ?? [];
  return Array.isArray(values) ? values : [];
}

function readBook(fileName, source, apkPath) {
  const relativePath = path.posix.join(BOOK_DIR.replaceAll('\\', '/'), fileName);
  let raw;
  if (source === 'head') {
    raw = execFileSync('git', ['show', `HEAD:${relativePath}`], { encoding: 'utf8', maxBuffer: 128 * 1024 * 1024 });
  } else if (source === 'apk-runtime') {
    raw = execFileSync('tar', ['-xOf', apkPath, `assets/seed-vocab/book/${fileName}`], { encoding: 'utf8', maxBuffer: 128 * 1024 * 1024 });
  } else {
    raw = fs.readFileSync(path.join(BOOK_DIR, fileName), 'utf8');
  }
  return JSON.parse(raw.replace(/^\uFEFF/u, ''));
}

export function auditBooks(source = 'head', apkPath = '') {
  const report = {
    generatedAt: new Date().toISOString(),
    source,
    apkPath: source === 'apk-runtime' ? apkPath : undefined,
    books: {},
    totalEntries: 0,
    totalProblemWords: 0,
    totalConflictingChoices: 0,
    findings: [],
  };

  for (const fileName of BOOK_FILES) {
    const rawEntries = readBook(fileName, source, apkPath);
    const entries = source === 'apk-runtime'
      ? withRuntimeDistractors(rawEntries)
      : source === 'worktree-runtime'
        ? applyEffectiveRuntimeQuestionPreps(rawEntries)
        : rawEntries;
    let problemWords = 0;
    let conflictingChoices = 0;
    report.totalEntries += entries.length;
    for (const item of entries) {
      const correct = correctMeaning(item);
      const chineseChoices = cnChoices(item);
      const englishChoices = enChoices(item);
      const conflicts = [];
      for (let index = 0; index < chineseChoices.length; index += 1) {
        const match = classifyMeaningConflict(correct, chineseChoices[index]);
        if (!match) continue;
        conflicts.push({
          index,
          sourceWord: englishChoices[index] ?? '',
          distractorMeaning: clean(chineseChoices[index]),
          ...match,
        });
      }
      if (!conflicts.length) continue;
      problemWords += 1;
      conflictingChoices += conflicts.length;
      report.findings.push({ book: fileName, word: displayWord(item), correctMeaning: correct, conflicts });
    }
    report.books[fileName] = { entries: entries.length, problemWords, conflictingChoices };
    report.totalProblemWords += problemWords;
    report.totalConflictingChoices += conflictingChoices;
  }
  return report;
}

function escapeCell(text) {
  return String(text ?? '').replaceAll('|', '\\|').replaceAll('\n', ' ');
}

export function renderMarkdown(report) {
  const lines = [
    '# Seed Vocabulary Choice Conflict Inventory',
    '',
    `- Source: \`${report.source}\``,
    ...(report.apkPath ? [`- APK: \`${report.apkPath}\``] : []),
    `- Generated: \`${report.generatedAt}\``,
    `- Books: ${BOOK_FILES.length}`,
    `- Entries scanned: ${report.totalEntries}`,
    `- Problem words: ${report.totalProblemWords}`,
    `- Conflicting choices: ${report.totalConflictingChoices}`,
    '',
    'Detection is conservative and explainable: exact component, containment, shared Chinese phrase, or a curated semantic group.',
    '',
  ];
  for (const fileName of BOOK_FILES) {
    const stats = report.books[fileName];
    lines.push(`## ${fileName}`, '', `Entries: ${stats.entries}; problem words: ${stats.problemWords}; conflicting choices: ${stats.conflictingChoices}.`, '');
    const findings = report.findings.filter((finding) => finding.book === fileName);
    if (!findings.length) {
      lines.push('No conflicts found.', '');
      continue;
    }
    lines.push('| Word | Correct meaning | Conflicting distractors |', '| --- | --- | --- |');
    for (const finding of findings) {
      const conflicts = finding.conflicts
        .map((conflict) => `${conflict.sourceWord || '#'}: ${conflict.distractorMeaning} [${conflict.type}: ${conflict.evidence}]`)
        .join('<br>');
      lines.push(`| ${escapeCell(finding.word)} | ${escapeCell(finding.correctMeaning)} | ${escapeCell(conflicts)} |`);
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function optionValue(name, fallback) {
  const prefix = `--${name}=`;
  return process.argv.find((arg) => arg.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function main() {
  const source = optionValue('source', 'head');
  if (!['head', 'worktree', 'worktree-runtime', 'apk-runtime'].includes(source)) {
    throw new Error(`Unsupported source: ${source}`);
  }
  const apkPath = optionValue('apk', 'releases/word-mobile-1.0.2+3.apk');
  const suffix = source === 'head' ? 'baseline' : source === 'worktree' ? 'current' : source;
  const jsonPath = optionValue('json', `docs/seed-vocab-choice-conflict-inventory-${suffix}.json`);
  const markdownPath = optionValue('markdown', `docs/seed-vocab-choice-conflict-inventory-${suffix}.md`);
  const report = auditBooks(source, apkPath);
  fs.writeFileSync(jsonPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.writeFileSync(markdownPath, renderMarkdown(report), 'utf8');
  console.log(JSON.stringify({ source, books: report.books, totalEntries: report.totalEntries, totalProblemWords: report.totalProblemWords, totalConflictingChoices: report.totalConflictingChoices, jsonPath, markdownPath }, null, 2));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
