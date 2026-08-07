import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const validator = resolve(root, 'scripts/domain-export/validate-manifest.mjs');
const fixtureRoot = resolve(root, 'fixtures/domain/v1');
const artifactRoot = resolve(root, 'artifacts/domain-wasm');
const sha256Bytes = (bytes) => createHash('sha256').update(bytes).digest('hex');
const sha256File = (path) => sha256Bytes(readFileSync(path));
const normalizedTextSha256 = (path) => sha256Bytes(readFileSync(path, 'utf8').replace(/\r\n/g, '\n'));
const inventoryDigest = (entries) => sha256Bytes(JSON.stringify(entries));

function fixtureInventory() {
  const manifest = JSON.parse(readFileSync(resolve(fixtureRoot, 'manifest.json'), 'utf8'));
  return ['fixtures', 'lifecycleFixtures']
    .flatMap((collection) => manifest[collection].map((item) => ({
      id: item.id,
      collection,
      requestPath: item.request,
      requestSha256: sha256File(resolve(fixtureRoot, item.request)),
      expectedPath: item.expected,
      expectedSha256: sha256File(resolve(fixtureRoot, item.expected)),
    })))
    .sort((left, right) => left.id.localeCompare(right.id));
}

function artifactEntry(path, includeGzip = false) {
  const bytes = readFileSync(path);
  return {
    path: `package/${basename(path)}`,
    sha256: sha256Bytes(bytes),
    rawBytes: bytes.byteLength,
    ...(includeGzip ? { gzipBytes: gzipSync(bytes).byteLength } : {}),
  };
}

function createManifestFixture() {
  const directory = mkdtempSync(resolve(tmpdir(), 'word-domain-manifest-'));
  const packageDirectory = resolve(directory, 'package');
  cpSync(resolve(artifactRoot, 'package'), packageDirectory, { recursive: true });
  const inventory = fixtureInventory();
  const phase6 = inventory.filter((entry) => entry.id.startsWith('phase6-'));
  const sourceLock = JSON.parse(readFileSync(resolve(fixtureRoot, 'source-lock.json'), 'utf8'));
  const commit = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).stdout.trim();
  const manifest = {
    schemaVersion: 1,
    protocolVersion: 1,
    source: {
      commit,
      dirty: false,
      sourceLockSha256: sourceLock.aggregateSha256,
      cargoLockSha256: normalizedTextSha256(resolve(root, 'Cargo.lock')),
    },
    build: { target: 'wasm32-unknown-unknown', profile: 'release', command: 'test fixture', rustc: 'test', cargo: 'test', wasmPack: 'test' },
    artifacts: {
      packageJson: artifactEntry(resolve(packageDirectory, 'package.json')),
      javascript: artifactEntry(resolve(packageDirectory, 'word_domain_wasm.js')),
      javascriptTypes: artifactEntry(resolve(packageDirectory, 'word_domain_wasm.d.ts')),
      wasm: artifactEntry(resolve(packageDirectory, 'word_domain_wasm_bg.wasm'), true),
      wasmTypes: artifactEntry(resolve(packageDirectory, 'word_domain_wasm_bg.wasm.d.ts')),
    },
    fixtures: {
      manifestPath: 'fixtures/domain/v1/manifest.json',
      fixtureManifestSha256: normalizedTextSha256(resolve(fixtureRoot, 'manifest.json')),
      fixtureCount: inventory.length,
      fixtureInventorySha256: inventoryDigest(inventory),
      fixtureInventory: inventory,
      phase6FixtureIds: phase6.map((entry) => entry.id),
      phase6FixtureCount: phase6.length,
      phase6FixtureInventorySha256: inventoryDigest(phase6),
    },
    budgets: { wasmRawBytes: 1048576, wasmGzipBytes: 524288, javascriptRawBytes: 65536 },
    releaseReady: true,
    reproducibility: { checked: true, builds: 2 },
  };
  const manifestPath = resolve(directory, 'manifest.json');
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  return { directory, manifest, manifestPath };
}

function validate(manifestPath) {
  return spawnSync(process.execPath, [validator, manifestPath], { cwd: root, encoding: 'utf8' });
}

test('accepts a complete five-output and fixture-bound manifest', () => {
  const fixture = createManifestFixture();
  try {
    const result = validate(fixture.manifestPath);
    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /Manifest valid for/);
  } finally {
    rmSync(fixture.directory, { recursive: true, force: true });
  }
});

for (const mutation of [
  ['missing artifact', (manifest) => { delete manifest.artifacts.packageJson; }],
  ['extra artifact', (manifest) => { manifest.artifacts.extra = manifest.artifacts.javascript; }],
  ['duplicate fixture id', (manifest) => { manifest.fixtures.fixtureInventory[1].id = manifest.fixtures.fixtureInventory[0].id; }],
  ['missing fixture', (manifest) => { manifest.fixtures.fixtureInventory.pop(); }],
  ['stale inventory digest', (manifest) => { manifest.fixtures.fixtureInventorySha256 = '0'.repeat(64); }],
  ['reordered inventory', (manifest) => { manifest.fixtures.fixtureInventory.reverse(); }],
  ['stale Phase 6 ids', (manifest) => { manifest.fixtures.phase6FixtureIds.pop(); }],
]) {
  test(`rejects ${mutation[0]}`, () => {
    const fixture = createManifestFixture();
    try {
      mutation[1](fixture.manifest);
      writeFileSync(fixture.manifestPath, `${JSON.stringify(fixture.manifest, null, 2)}\n`);
      const result = validate(fixture.manifestPath);
      assert.notEqual(result.status, 0, `mutation unexpectedly passed: ${mutation[0]}`);
    } finally {
      rmSync(fixture.directory, { recursive: true, force: true });
    }
  });
}

test('rejects changed artifact bytes', () => {
  const fixture = createManifestFixture();
  try {
    writeFileSync(resolve(fixture.directory, 'package/package.json'), '{"mutated":true}\n');
    const result = validate(fixture.manifestPath);
    assert.notEqual(result.status, 0);
  } finally {
    rmSync(fixture.directory, { recursive: true, force: true });
  }
});
