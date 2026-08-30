import fs from 'node:fs';
import path from 'node:path';

import {
  mergePlaceholderQuestionStems,
  repairPlaceholderQuestionStems,
} from './import-exam-papers.mjs';

const assetDir = process.argv[2] ?? 'apps/flutter_mobile/assets/exam-papers';
const structuredDataDir = process.argv[3];
const manifestPath = path.join(assetDir, 'manifest.json');
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
let updatedQuestions = 0;
const structuredPapers = [];

if (structuredDataDir && fs.existsSync(structuredDataDir)) {
  for (const fileName of fs.readdirSync(structuredDataDir).filter((name) => /^\d{4}\.json$/u.test(name))) {
    const raw = JSON.parse(fs.readFileSync(path.join(structuredDataDir, fileName), 'utf8'));
    structuredPapers.push({
      id: `kaoyan-english-1-${path.basename(fileName, '.json')}`,
      sections: Object.values(raw.sections ?? {}),
    });
  }
}

for (const relativePath of manifest.files ?? []) {
  const filePath = path.join(assetDir, path.basename(relativePath));
  const document = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  const merged = mergePlaceholderQuestionStems([
    ...(document.papers ?? []),
    ...structuredPapers,
  ]);
  const result = repairPlaceholderQuestionStems(document.papers);
  const fileUpdates = merged.updatedQuestions + result.updatedQuestions;
  if (fileUpdates === 0) continue;
  fs.writeFileSync(filePath, `${JSON.stringify(document, null, 2)}\n`, 'utf8');
  updatedQuestions += fileUpdates;
}

console.log(`updatedQuestions=${updatedQuestions}`);
