import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { execFileSync } from 'node:child_process';
import { gzipSync } from 'node:zlib';

const manifestPath = resolve(process.argv[2] ?? 'artifacts/domain-wasm/manifest.json');
const root = resolve(dirname(new URL(import.meta.url).pathname.replace(/^\/(.:)/, '$1')), '../..');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const sha256 = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const normalizedTextSha256 = (path) => createHash('sha256')
  .update(readFileSync(path, 'utf8').replace(/\r\n/g, '\n'))
  .digest('hex');
const valueSha256 = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');
const required = (value, name) => { if (value === undefined || value === null || value === '') throw new Error(`Manifest field is required: ${name}`); };
const sameJson = (left, right) => JSON.stringify(left) === JSON.stringify(right);

function fixtureInventory(fixtureManifest) {
  const seenIds = new Set();
  const seenPaths = new Set();
  const inventory = ['fixtures', 'lifecycleFixtures'].flatMap((collection) => {
    if (!Array.isArray(fixtureManifest[collection])) throw new Error(`Fixture manifest ${collection} must be an array.`);
    return fixtureManifest[collection].map((fixture) => {
      required(fixture.id, `${collection}.id`);
      required(fixture.request, `${fixture.id}.request`);
      required(fixture.expected, `${fixture.id}.expected`);
      if (seenIds.has(fixture.id)) throw new Error(`Duplicate fixture id: ${fixture.id}`);
      seenIds.add(fixture.id);
      for (const path of [fixture.request, fixture.expected]) {
        if (seenPaths.has(path)) throw new Error(`Duplicate fixture path: ${path}`);
        seenPaths.add(path);
      }
      return {
        id: fixture.id,
        collection,
        requestPath: fixture.request,
        requestSha256: sha256(resolve(root, 'fixtures/domain/v1', fixture.request)),
        expectedPath: fixture.expected,
        expectedSha256: sha256(resolve(root, 'fixtures/domain/v1', fixture.expected)),
      };
    });
  });
  return inventory.sort((left, right) => left.id.localeCompare(right.id));
}

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
  fixtureManifestSha256: manifest.fixtures?.fixtureManifestSha256,
  fixtureInventorySha256: manifest.fixtures?.fixtureInventorySha256,
  phase6FixtureInventorySha256: manifest.fixtures?.phase6FixtureInventorySha256,
})) required(value, name);

if (manifest.source.dirty !== false || manifest.releaseReady !== true) throw new Error('Pin-ready manifest must record dirty=false and releaseReady=true.');
if (manifest.reproducibility?.checked !== true || manifest.reproducibility?.builds !== 2) throw new Error('Two-build reproducibility evidence is required.');
const head = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
try {
  execFileSync(
    'git',
    ['merge-base', '--is-ancestor', manifest.source.commit, head],
    { cwd: root, stdio: 'ignore' },
  );
} catch {
  throw new Error(
    `Manifest commit ${manifest.source.commit} is not an ancestor of HEAD ${head}.`,
  );
}
const sourceLock = JSON.parse(readFileSync(resolve(root, 'fixtures/domain/v1/source-lock.json'), 'utf8'));
if (manifest.source.sourceLockSha256 !== sourceLock.aggregateSha256) throw new Error('Manifest source-lock digest is stale.');
if (manifest.source.cargoLockSha256 !== normalizedTextSha256(resolve(root, 'Cargo.lock'))) throw new Error('Manifest Cargo.lock hash is stale.');

const artifactNames = ['javascript', 'javascriptTypes', 'packageJson', 'wasm', 'wasmTypes'];
const actualArtifactNames = Object.keys(manifest.artifacts).sort();
if (!sameJson(actualArtifactNames, artifactNames)) throw new Error('Manifest must contain exactly five tracked package outputs.');
for (const name of artifactNames) {
  const artifact = manifest.artifacts[name];
  required(artifact.path, `${name}.path`);
  required(artifact.sha256, `${name}.sha256`);
  required(artifact.rawBytes, `${name}.rawBytes`);
  const path = resolve(dirname(manifestPath), artifact.path);
  if (sha256(path) !== artifact.sha256) throw new Error(`${name} SHA-256 mismatch.`);
  if (readFileSync(path).byteLength !== artifact.rawBytes) throw new Error(`${name} raw size mismatch.`);
}
const wasmBytes = readFileSync(resolve(dirname(manifestPath), manifest.artifacts.wasm.path));
if (gzipSync(wasmBytes).byteLength !== manifest.artifacts.wasm.gzipBytes) throw new Error('wasm gzip size mismatch.');
if (manifest.artifacts.wasm.rawBytes > manifest.budgets.wasmRawBytes || manifest.artifacts.wasm.gzipBytes > manifest.budgets.wasmGzipBytes) throw new Error('WASM size exceeds its locked budget.');
if (manifest.artifacts.javascript.rawBytes > manifest.budgets.javascriptRawBytes) throw new Error('JavaScript glue exceeds its locked budget.');

const fixtureManifestPath = resolve(root, manifest.fixtures.manifestPath);
if (manifest.fixtures.manifestPath !== 'fixtures/domain/v1/manifest.json') throw new Error('Fixture manifest path is not canonical.');
if (manifest.fixtures.fixtureManifestSha256 !== normalizedTextSha256(fixtureManifestPath)) throw new Error('Fixture manifest hash is stale.');
const fixtureManifest = JSON.parse(readFileSync(fixtureManifestPath, 'utf8'));
const inventory = fixtureInventory(fixtureManifest);
if (!sameJson(manifest.fixtures.fixtureInventory, inventory)) throw new Error('Fixture inventory is missing, extra, duplicated, stale, or reordered.');
if (manifest.fixtures.fixtureCount !== inventory.length) throw new Error('Fixture count is stale.');
if (manifest.fixtures.fixtureInventorySha256 !== valueSha256(inventory)) throw new Error('Fixture inventory digest is stale.');
const phase6Inventory = inventory.filter((entry) => entry.id.startsWith('phase6-'));
const phase6Ids = phase6Inventory.map((entry) => entry.id);
if (!sameJson(manifest.fixtures.phase6FixtureIds, phase6Ids)) throw new Error('Phase 6 fixture IDs are stale.');
if (manifest.fixtures.phase6FixtureCount !== phase6Inventory.length) throw new Error('Phase 6 fixture count is stale.');
if (manifest.fixtures.phase6FixtureInventorySha256 !== valueSha256(phase6Inventory)) throw new Error('Phase 6 fixture inventory digest is stale.');
console.log(`Manifest valid for ${manifest.source.commit}; WASM ${manifest.artifacts.wasm.sha256}.`);
