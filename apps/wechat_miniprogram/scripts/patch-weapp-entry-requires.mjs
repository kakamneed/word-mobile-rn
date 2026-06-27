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

function requirePrefixFor(relativeFile) {
  const depth = relativeFile.split('/').length - 1;
  const up = depth === 0 ? '.' : Array.from({ length: depth }, () => '..').join('/');
  return [
    `require("${up}/common");`,
    `require("${up}/vendors");`,
    `require("${up}/taro");`,
    `require("${up}/runtime");`,
  ].join('');
}

for (const relativeFile of entryFiles) {
  const filePath = path.join(distRoot, relativeFile);
  if (!fs.existsSync(filePath)) continue;

  const original = fs.readFileSync(filePath, 'utf8');
  const alreadyHasRuntimeRoots = ['common', 'vendors', 'taro', 'runtime'].every(
    (chunk) => original.includes(`/${chunk}")`) || original.includes(`"./${chunk}")`),
  );
  if (alreadyHasRuntimeRoots) continue;

  const prefix = requirePrefixFor(relativeFile);
  const patched = original.startsWith('"use strict";')
    ? original.replace('"use strict";', `"use strict";${prefix}`)
    : `${prefix}${original}`;

  fs.writeFileSync(filePath, patched, 'utf8');
}
