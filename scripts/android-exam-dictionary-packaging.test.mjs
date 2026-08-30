import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const bridgeSource = fs.readFileSync(
  path.join(
    root,
    'apps/flutter_mobile/android/app/src/main/java/com/wordmobile/RustBridge.java',
  ),
  'utf8',
);
const pubspec = fs.readFileSync(
  path.join(root, 'apps/flutter_mobile/pubspec.yaml'),
  'utf8',
);
const dictionary = JSON.parse(
  fs.readFileSync(
    path.join(
      root,
      'apps/flutter_mobile/assets/exam-dictionary/exam-corpus-dictionary.json',
    ),
    'utf8',
  ),
);

test('Android copies the bundled exam dictionary into the Rust resource directory', () => {
  assert.match(pubspec, /^\s+- assets\/exam-dictionary\/\s*$/m);
  const knownPhrase = dictionary.entries.find(
    (entry) => entry.term === 'in case that',
  );
  assert.ok(knownPhrase, 'generated dictionary must contain a known exam phrase');
  assert.ok(knownPhrase.meaning.trim(), 'known exam phrase meaning must not be empty');
  assert.match(
    bridgeSource,
    /copyBundledAssetDirectory\(context\.getAssets\(\),\s*"flutter_assets\/assets\/exam-dictionary",\s*new File\(bundledResourcesDir,\s*"exam-dictionary"\)\)/,
  );
});
