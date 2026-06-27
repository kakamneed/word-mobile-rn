import fs from 'node:fs';
import path from 'node:path';

const bookDir = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const requiredBooks = ['CET4_3.json', 'CET6_3.json', 'KaoYan_3.json', 'MEDICAL_RESP.json'];
const requiredDistractors = 7;

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}

function collectStrings(value, out = []) {
  if (typeof value === 'string') out.push(value);
  else if (Array.isArray(value)) for (const item of value) collectStrings(item, out);
  else if (value && typeof value === 'object') for (const item of Object.values(value)) collectStrings(item, out);
  return out;
}

function hasDuplicate(values) {
  const seen = new Set();
  for (const value of values) {
    const key = String(value ?? '').trim().toLowerCase();
    if (!key) return true;
    if (seen.has(key)) return true;
    seen.add(key);
  }
  return false;
}

let failures = 0;
for (const fileName of requiredBooks) {
  const filePath = path.join(bookDir, fileName);
  if (!fs.existsSync(filePath)) {
    console.error(`missing book: ${fileName}`);
    failures += 1;
    continue;
  }
  const entries = readJson(filePath);
  let missingPrep = 0;
  let shortPrep = 0;
  let duplicatePrep = 0;
  let noisy = 0;
  for (const item of entries) {
    const content = item?.content?.word?.content ?? {};
    const cn = item?.cnChoiceDistractors ?? content?.cnChoiceDistractors ?? [];
    const en = item?.enChoiceDistractors ?? content?.enChoiceDistractors ?? [];
    if (!Array.isArray(cn) || !Array.isArray(en)) {
      missingPrep += 1;
    } else {
      if (cn.length < requiredDistractors || en.length < requiredDistractors) shortPrep += 1;
      if (hasDuplicate(cn) || hasDuplicate(en)) duplicatePrep += 1;
    }
    const meaningStrings = [
      ...(Array.isArray(content?.trans) ? content.trans.map((entry) => entry?.tranCn) : []),
      ...cn,
    ].filter((value) => typeof value === 'string');
    if (meaningStrings.some((text) => /[<>]/u.test(text) || /(?:^|[,，;；、\s])[ABCD]$/u.test(text.trim()))) {
      noisy += 1;
    }
  }
  const allStrings = collectStrings(entries);
  const replacementChars = allStrings.filter((text) => text.includes('\uFFFD')).length;
  console.log(
    `${fileName}: entries=${entries.length} missingPrep=${missingPrep} shortPrep=${shortPrep} duplicatePrep=${duplicatePrep} noisy=${noisy} replacementChars=${replacementChars}`,
  );
  if (missingPrep > 0 || shortPrep > 0 || duplicatePrep > 0 || noisy > 0 || replacementChars > 0) failures += 1;
}

if (failures > 0) process.exitCode = 1;
