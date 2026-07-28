import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { execFileSync } from 'node:child_process';

const manifestPath = resolve(process.argv[2] ?? 'artifacts/domain-wasm/manifest.json');
const root = resolve(dirname(new URL(import.meta.url).pathname.replace(/^\/(.:)/, '$1')), '../..');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const sha256 = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const normalizedTextSha256 = (path) => createHash('sha256')
  .update(readFileSync(path, 'utf8').replace(/\r\n/g, '\n'))
  .digest('hex');
const required = (value, name) => { if (value === undefined || value === null || value === '') throw new Error(`Manifest field is required: ${name}`); };

for (const [name, value] of Object.entries({
  schemaVersion: manifest.schemaVersion,
  protocolVersion: manifest.protocolVersion,
  commit: manifest.source?.commit,
  sourceLockSha256: manifest.source?.sourceLockSha256,
  cargoLockSha256: manifest.source?.cargoLockSha256,
  rustc: manifest.build?.rustc,
  cargo: manifest.build?.cargo,
  wasmPack: manifest.build?.wasmPack,
  target: manifest.build?.target,
  profile: manifest.build?.profile,
  command: manifest.build?.command,
  wasmSha256: manifest.artifacts?.wasm?.sha256,
  javascriptSha256: manifest.artifacts?.javascript?.sha256,
})) required(value, name);

if (manifest.source.dirty !== false || manifest.releaseReady !== true) throw new Error('Pin-ready manifest must record dirty=false and releaseReady=true.');
if (manifest.reproducibility?.checked !== true || manifest.reproducibility?.builds !== 2) throw new Error('Two-build reproducibility evidence is required.');
const head = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
if (manifest.source.commit !== head) throw new Error(`Manifest commit ${manifest.source.commit} does not match HEAD ${head}.`);
const sourceLock = JSON.parse(readFileSync(resolve(root, 'fixtures/domain/v1/source-lock.json'), 'utf8'));
if (manifest.source.sourceLockSha256 !== sourceLock.aggregateSha256) throw new Error('Manifest source-lock digest is stale.');
if (manifest.source.cargoLockSha256 !== normalizedTextSha256(resolve(root, 'Cargo.lock'))) throw new Error('Manifest Cargo.lock hash is stale.');

for (const [name, artifact] of Object.entries(manifest.artifacts)) {
  const path = resolve(dirname(manifestPath), artifact.path);
  if (sha256(path) !== artifact.sha256) throw new Error(`${name} SHA-256 mismatch.`);
  if (readFileSync(path).byteLength !== artifact.rawBytes) throw new Error(`${name} raw size mismatch.`);
}
if (manifest.artifacts.wasm.rawBytes > manifest.budgets.wasmRawBytes || manifest.artifacts.wasm.gzipBytes > manifest.budgets.wasmGzipBytes) throw new Error('WASM size exceeds its locked budget.');
if (manifest.artifacts.javascript.rawBytes > manifest.budgets.javascriptRawBytes) throw new Error('JavaScript glue exceeds its locked budget.');
console.log(`Manifest valid for ${manifest.source.commit}; WASM ${manifest.artifacts.wasm.sha256}.`);
