declare const require: any;
declare const process: {
  cwd(): string;
  exit(code?: number): never;
};

const fs = require('fs');
const path = require('path');

const root = process.cwd();

const filesToScan = [
  'src/pages/study/index.tsx',
  'src/subpkg/reports/index.tsx',
  'src/subpkg/plan/index.tsx',
  'src/subpkg/wrong-words/index.tsx',
  'src/subpkg/croc-bti/index.tsx',
  'src/pages/today/index.tsx',
  'src/components/Screen.tsx',
  'src/components/Screen.scss',
  'src/pages/study/index.scss',
  'src/sdk/mockData.ts',
  'src/sdk/crocBti.ts',
  '../../docs/wechat/miniprogram_flutter_gap_analysis.md',
  '../../docs/wechat/miniprogram_flutter_change_feedback_template.md',
];

const forbiddenPatterns: Array<{ label: string; pattern: RegExp }> = [
  { label: 'mojibake latin-1 marker', pattern: /[ÃÂ]/ },
  { label: 'mojibake cp936 marker', pattern: /[鏍鎶璁閿瀛澶绛姣]/ },
  { label: 'mojibake replacement marker', pattern: /[鈥€�]/ },
  { label: 'broken mojibake toast JSX', pattern: /Taro\.showToast\(\{[^}]*[鏍鎶璁閿瀛澶绛姣鈥€�][^}]*icon:\s*'none'\s*}\)/ },
  { label: 'literal home icon content', pattern: /content:\s*"home"/ },
  { label: 'literal close icon content', pattern: /content:\s*"close"/ },
  { label: 'literal today icon content', pattern: /content:\s*"today"/ },
  { label: 'literal tune icon content', pattern: /content:\s*"tune"/ },
  { label: 'literal menu_book icon content', pattern: /content:\s*"menu_book"/ },
  { label: 'literal query_stats icon content', pattern: /content:\s*"query_stats"/ },
  { label: 'literal lightbulb icon content', pattern: /content:\s*"lightbulb"/ },
  { label: 'literal comment icon content', pattern: /content:\s*"comment"/ },
  { label: 'literal visibility icon content', pattern: /content:\s*"visibility"/ },
  { label: 'literal gavel icon content', pattern: /content:\s*"gavel"/ },
  { label: 'literal delete icon content', pattern: /content:\s*"delete"/ },
  { label: 'literal icon name render', pattern: />\{name\}<\/Text>/ },
];

const failures: string[] = [];

for (const relativePath of filesToScan) {
  const absolutePath = path.join(root, relativePath);
  if (!fs.existsSync(absolutePath)) {
    failures.push(`${relativePath}: file is missing`);
    continue;
  }
  const source = fs.readFileSync(absolutePath, 'utf8');
  for (const { label, pattern } of forbiddenPatterns) {
    if (pattern.test(source)) failures.push(`${relativePath}: ${label} (${pattern})`);
  }
}

const gapAnalysis = fs.readFileSync(
  path.resolve(root, '../../docs/wechat/miniprogram_flutter_gap_analysis.md'),
  'utf8',
);
for (const required of [
  'Study Session And Answering',
  'Root/Affix Source Chain',
  'Today State, Progress, And Refresh',
  'Reports Chart',
  'Plan And Croc BTI',
  'Flutter/Rust source',
  'Mini-program source',
]) {
  if (!gapAnalysis.includes(required)) {
    failures.push(`docs/wechat/miniprogram_flutter_gap_analysis.md: missing ${required}`);
  }
}

if (failures.length) {
  console.error('Source integrity check failed:');
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log('Source integrity checks passed.');
