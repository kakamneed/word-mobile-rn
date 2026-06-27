declare const require: (id: string) => any;
declare const __dirname: string;

const fs = require('fs');
const path = require('path');

const dist = path.resolve(__dirname, '..', 'dist');
const maxMainPackageBytes = 2 * 1024 * 1024;

function walk(dir: string): string[] {
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry: any) => {
    const next = path.join(dir, entry.name);
    return entry.isDirectory() ? walk(next) : [next];
  });
}

const files = walk(dist);
const oversized = files.filter((file) => fs.statSync(file).size > maxMainPackageBytes);
if (oversized.length) {
  throw new Error(`Files exceed 2MB: ${oversized.join(', ')}`);
}

const totalBytes = files.reduce((sum, file) => sum + fs.statSync(file).size, 0);
if (totalBytes > maxMainPackageBytes) {
  throw new Error(`dist source size ${totalBytes} exceeds 2MB true-device debug limit`);
}

console.log(JSON.stringify({ files: files.length, totalBytes }, null, 2));
