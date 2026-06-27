import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectConfigPath = path.resolve(__dirname, '..', 'dist', 'project.config.json');

if (fs.existsSync(projectConfigPath)) {
  fs.rmSync(projectConfigPath);
}
