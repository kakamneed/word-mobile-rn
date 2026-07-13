import fs from 'node:fs';
import path from 'node:path';

const bookDir = 'apps/mobile/android/app/src/main/assets/seed-vocab/book';
const requiredBooks = ['CET4_3.json', 'CET6_3.json', 'KaoYan_3.json', 'MEDICAL_RESP.json'];
const relationFields = ['rootAffixes', 'rootFamilyWords', 'similarFormWords', 'meaningOverlapWords'];

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/, ''));
}
function normalizedWord(text) {
  return String(text ?? '').toLowerCase().replace(/[^a-z0-9]/gu, '');
}

let failures = 0;
for (const fileName of requiredBooks) {
  const filePath = path.join(bookDir, fileName);
  const entries = readJson(filePath);
  let missing = 0;
  let malformed = 0;
  let duplicateTargets = 0;
  let selfTargets = 0;
  const relationCounts = Object.fromEntries(relationFields.map((field) => [field, 0]));

  for (const item of entries) {
    const relations = item?.wordGraphRelations ?? item?.content?.word?.content?.wordGraphRelations;
    const word = item?.displayWord ?? item?.headWord ?? item?.content?.word?.wordHead;
    if (!relations || typeof relations !== 'object') {
      missing += 1;
      continue;
    }
    for (const field of relationFields) {
      if (!Array.isArray(relations[field])) malformed += 1;
      else if (relations[field].length > 0) relationCounts[field] += 1;
    }
    for (const field of ['rootFamilyWords', 'similarFormWords', 'meaningOverlapWords']) {
      const seen = new Set();
      for (const target of relations[field] ?? []) {
        const key = normalizedWord(target?.word) || target?.sourceId;
        if (!key) malformed += 1;
        if (key && seen.has(key)) duplicateTargets += 1;
        seen.add(key);
        if (key && key === normalizedWord(word)) selfTargets += 1;
      }
    }
  }
  console.log(
    `${fileName}: entries=${entries.length} missing=${missing} malformed=${malformed} duplicateTargets=${duplicateTargets} selfTargets=${selfTargets} ` +
      relationFields.map((field) => `${field}=${relationCounts[field]}`).join(' '),
  );
  if (missing || malformed || duplicateTargets || selfTargets) failures += 1;
}
if (failures > 0) process.exitCode = 1;
