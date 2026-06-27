import fs from 'node:fs';
import path from 'node:path';

const distRoot = path.resolve('dist');
const entryFiles = [
  'comp.js',
  'pages/today/index.js',
  'pages/study/index.js',
  'subpkg/account/index.js',
  'subpkg/croc-bti/index.js',
  'subpkg/leaderboard/index.js',
  'subpkg/onboarding/index.js',
  'subpkg/plan/index.js',
  'subpkg/profile/index.js',
  'subpkg/reports/index.js',
  'subpkg/settings/index.js',
  'subpkg/wrong-words/index.js',
];

const failures = [];

for (const relativeFile of entryFiles) {
  const filePath = path.join(distRoot, relativeFile);
  if (!fs.existsSync(filePath)) continue;
  const source = fs.readFileSync(filePath, 'utf8');
  for (const chunk of ['common', 'vendors', 'taro', 'runtime']) {
    if (!source.includes(`/${chunk}")`) && !source.includes(`"./${chunk}")`)) {
      failures.push(`${relativeFile} missing ${chunk} require`);
    }
  }
}

if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}

console.log(JSON.stringify({ checkedEntries: entryFiles.length }, null, 2));
