import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const distDir = path.join(__dirname, '..', 'dist');
const expectedHost = 'pmevdtgogsudnfgxjgiz.supabase.co';
const expectedBase = `https://${expectedHost}/functions/v1`;

function readJsFiles(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...readJsFiles(fullPath));
    } else if (entry.isFile() && entry.name.endsWith('.js')) {
      files.push(fullPath);
    }
  }
  return files;
}

if (!fs.existsSync(distDir)) {
  throw new Error('dist directory does not exist. Run npm.cmd run build:weapp:http first.');
}

const content = readJsFiles(distDir)
  .map((file) => fs.readFileSync(file, 'utf8'))
  .join('\n');

if (!content.includes(expectedBase)) {
  throw new Error(`HTTP build output must include ${expectedBase}`);
}

if (content.includes('Mini Program SDK running in explicit mock mode')) {
  throw new Error('HTTP build output still contains mock SDK mode marker.');
}

if (content.includes('pmevdtgogsudnfgxjgz')) {
  throw new Error('HTTP build output still contains the misspelled Supabase host.');
}

console.log(
  JSON.stringify(
    {
      mode: 'http',
      apiBaseUrl: expectedBase,
      checkedFiles: readJsFiles(distDir).length,
    },
    null,
    2,
  ),
);
