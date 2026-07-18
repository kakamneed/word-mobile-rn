import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { applyParagraphTranslationOverrides } from './import-exam-papers.mjs';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const assetDir = path.resolve(
  process.argv[2] ?? 'apps/flutter_mobile/assets/exam-papers',
);
const overrides = JSON.parse(
  fs.readFileSync(path.join(scriptDir, 'exam-paper-paragraph-translations.json'), 'utf8'),
);

let updatedSections = 0;
for (const name of fs.readdirSync(assetDir).filter((value) => value.endsWith('.json'))) {
  if (name === 'manifest.json') continue;
  const filePath = path.join(assetDir, name);
  const payload = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  const before = JSON.stringify(payload);
  applyParagraphTranslationOverrides(payload.papers ?? [], overrides);
  if (JSON.stringify(payload) === before) continue;
  updatedSections += (payload.papers ?? []).reduce(
    (paperSum, paper) =>
      paperSum +
      (paper.sections ?? []).filter((section) => overrides[section.id] != null).length,
    0,
  );
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

console.log(`paragraphTranslations updatedSections=${updatedSections}`);
