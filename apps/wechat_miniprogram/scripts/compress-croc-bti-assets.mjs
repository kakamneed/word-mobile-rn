import fs from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const sharp = require('C:/Users/clf20/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/sharp');

const root = path.resolve(import.meta.dirname, '..');
const sourceDir = path.join(root, 'src', 'assets', 'croc_bti');
const outputDir = path.join(root, 'src', 'assets', 'croc_bti_compressed');

const assets = [
  'alligator_jade.png',
  'armor_guard_croc.png',
  'bard_croc.jpg',
  'battle_mage_pencil_croc.png',
  'book_guest_bu_e_ke.png',
  'cavalry_croc.png',
  'chanter_croc.png',
  'classic_croc.png',
  'correction_officer_croc.jpg',
  'forgemaster_croc.png',
  'hermit_croc.png',
  'ranger_croc.jpg',
  'scroll_master_croc.png',
  'stargazer_croc.jpg',
  'stele_croc.png',
  'swordsman_croc.png',
];

await fs.mkdir(outputDir, { recursive: true });

let totalBytes = 0;
for (const asset of assets) {
  const input = path.join(sourceDir, asset);
  const basename = asset.replace(/\.(png|jpe?g)$/i, '.jpg');
  const output = path.join(outputDir, basename);
  const image = sharp(input, { animated: false }).rotate();
  await image
    .resize({ width: 360, height: 360, fit: 'inside', withoutEnlargement: true })
    .jpeg({ quality: 72, mozjpeg: true })
    .toFile(output);
  const stat = await fs.stat(output);
  totalBytes += stat.size;
}

console.log(JSON.stringify({ files: assets.length, totalBytes }, null, 2));
