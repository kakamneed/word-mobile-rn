import { expect, test } from '@playwright/test';
import { createHash } from 'node:crypto';
import { createServer, type Server } from 'node:http';
import { readFileSync, writeFileSync } from 'node:fs';
import { extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const artifactRoot = process.env.DOMAIN_WASM_ARTIFACT_ROOT ?? 'artifacts/domain-wasm';
const artifactAbsoluteRoot = resolve(repositoryRoot, artifactRoot);
const manifest = JSON.parse(readFileSync(resolve(artifactAbsoluteRoot, 'manifest.json'), 'utf8'));
const fixtureManifest = JSON.parse(
  readFileSync(resolve(repositoryRoot, 'fixtures/domain/v1/manifest.json'), 'utf8'),
);
const sha256 = (path: string) => createHash('sha256').update(readFileSync(path)).digest('hex');
const normalizedJsonSha256 = (path: string) => createHash('sha256')
  .update(readFileSync(path, 'utf8').replace(/^\uFEFF/, '').replace(/\r\n?/g, '\n'))
  .digest('hex');
const inventoryDigest = (value: unknown) => createHash('sha256').update(JSON.stringify(value)).digest('hex');
const normalizedTextSha256 = (path: string) => createHash('sha256')
  .update(readFileSync(path, 'utf8').replace(/\r\n/g, '\n'))
  .digest('hex');

const liveInventory = ['fixtures', 'lifecycleFixtures']
  .flatMap((collection) => (fixtureManifest[collection] ?? []).map((fixture: { id: string; request: string; expected: string }) => ({
    id: fixture.id,
    collection,
    requestPath: fixture.request,
    requestSha256: normalizedJsonSha256(resolve(repositoryRoot, 'fixtures/domain/v1', fixture.request)),
    expectedPath: fixture.expected,
    expectedSha256: normalizedJsonSha256(resolve(repositoryRoot, 'fixtures/domain/v1', fixture.expected)),
  })))
  .sort((left, right) => left.id.localeCompare(right.id));

let server: Server;
let origin: string;

test.beforeAll(async () => {
  server = createServer((request, response) => {
    if (request.url === '/') {
      response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
      response.end('<!doctype html><title>Word domain WASM gate</title>');
      return;
    }

    const pathname = decodeURIComponent(new URL(request.url ?? '/', 'http://localhost').pathname);
    const filePath = resolve(repositoryRoot, `.${pathname}`);
    if (filePath !== repositoryRoot && !filePath.startsWith(`${repositoryRoot}${sep}`)) {
      response.writeHead(403).end();
      return;
    }
    let contents: Buffer;
    try {
      contents = readFileSync(filePath);
    } catch {
      response.writeHead(404).end();
      return;
    }
    const mime = extname(filePath) === '.wasm'
      ? 'application/wasm'
      : extname(filePath) === '.js'
        ? 'text/javascript; charset=utf-8'
        : 'application/octet-stream';
    response.on('error', () => {});
    response.writeHead(200, { 'content-type': mime });
    response.end(contents);
  });
  await new Promise<void>((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));
  const address = server.address();
  if (!address || typeof address === 'string') throw new Error('Browser gate server did not bind.');
  origin = `http://127.0.0.1:${address.port}`;
});

test.afterAll(async () => {
  await new Promise<void>((resolveClose, reject) =>
    server.close((error) => (error ? reject(error) : resolveClose())),
  );
});

test('generated package executes canonical results and structured errors', async ({ page }, testInfo) => {
  await page.goto(origin);
  const modulePath = `/${artifactRoot.replaceAll('\\', '/')}/${manifest.artifacts.javascript.path.replaceAll('\\', '/')}`;
  expect(manifest.fixtures.fixtureManifestSha256).toBe(
    normalizedTextSha256(resolve(repositoryRoot, 'fixtures/domain/v1/manifest.json')),
  );
  expect(manifest.fixtures.hashEncoding).toBe('utf8-lf-normalized-json-v1');
  expect(manifest.fixtures.fixtureInventory).toEqual(liveInventory);
  expect(manifest.fixtures.fixtureCount).toBe(liveInventory.length);
  expect(manifest.fixtures.fixtureInventorySha256).toBe(inventoryDigest(liveInventory));
  const phase6Ids = liveInventory.filter((entry) => entry.id.startsWith('phase6-')).map((entry) => entry.id);
  expect(manifest.fixtures.phase6FixtureIds).toEqual(phase6Ids);
  expect(manifest.fixtures.phase6FixtureCount).toBe(phase6Ids.length);
  expect(manifest.fixtures.phase6FixtureInventorySha256).toBe(
    inventoryDigest(liveInventory.filter((entry) => entry.id.startsWith('phase6-'))),
  );
  const fixtures = manifest.fixtures.fixtureInventory.map((fixture: { id: string; requestPath: string; expectedPath: string }) => ({
    id: fixture.id,
    request: readFileSync(resolve(repositoryRoot, 'fixtures/domain/v1', fixture.requestPath), 'utf8'),
    expected: JSON.parse(readFileSync(resolve(repositoryRoot, 'fixtures/domain/v1', fixture.expectedPath), 'utf8')),
  }));

  const measurements = await page.evaluate(async ({ moduleUrl, cases }) => {
    const startupStart = performance.now();
    const domain = await import(moduleUrl);
    await domain.default();
    const startupMs = performance.now() - startupStart;

    const firstStart = performance.now();
    const first = JSON.parse(domain.execute_v1(cases[0].request));
    const firstCommandMs = performance.now() - firstStart;

    const outputs = [{ id: cases[0].id, value: first.result }];
    for (const fixture of cases.slice(1)) {
      outputs.push({ id: fixture.id, value: JSON.parse(domain.execute_v1(fixture.request)).result });
    }

    const transitionFixture = cases.find((fixture) => fixture.id === 'lifecycle-transition');
    if (!transitionFixture) throw new Error('The canonical lifecycle transition fixture is missing.');
    const transitionModes = ['newWord', 'review', 'mixedTest', 'wrongWordReinforcement', 'highFrequency', 'rootAffix'].map((mode) => {
      const request = JSON.parse(transitionFixture.request);
      request.requestId = `lifecycle-transition-${mode}`;
      request.payload.snapshot.mode = mode;
      request.payload.snapshot.acceptedAnswers[0].mode = mode;
      return { mode, response: JSON.parse(domain.execute_v1(JSON.stringify(request))) };
    });
    const unknownModeRequest = JSON.parse(transitionFixture.request);
    unknownModeRequest.requestId = 'lifecycle-transition-unknown-mode';
    unknownModeRequest.payload.snapshot.mode = 'futureMode';
    unknownModeRequest.payload.snapshot.acceptedAnswers[0].mode = 'futureMode';
    const unknownMode = JSON.parse(domain.execute_v1(JSON.stringify(unknownModeRequest)));

    const repeatStart = performance.now();
    for (let index = 0; index < 25; index += 1) domain.execute_v1(cases[index % cases.length].request);
    const repeatCommandMs = (performance.now() - repeatStart) / 25;
    const error = JSON.parse(domain.execute_v1('{"protocolVersion":1,"command":"missing"}'));
    return { startupMs, firstCommandMs, repeatCommandMs, outputs, transitionModes, unknownMode, error };
  }, { moduleUrl: `${origin}${modulePath}`, cases: fixtures });

  for (const fixture of fixtures) {
    expect(measurements.outputs.find((output) => output.id === fixture.id)?.value).toEqual(fixture.expected);
  }
  for (const transition of measurements.transitionModes) {
    expect(transition.response.error, transition.mode).toBeUndefined();
    expect(transition.response.result.mode, transition.mode).toBe(transition.mode);
    expect(transition.response.result.state, transition.mode).toBe('completed');
  }
  expect(measurements.unknownMode.error.code).toBe('invalid_data');
  expect(measurements.unknownMode.result).toBeUndefined();
  expect(measurements.error.error.code).toBeTruthy();
  expect(measurements.error.result).toBeUndefined();
  expect(measurements.startupMs).toBeLessThanOrEqual(manifest.budgets.startupMs);
  expect(measurements.firstCommandMs).toBeLessThanOrEqual(manifest.budgets.firstCommandMs);
  expect(measurements.repeatCommandMs).toBeLessThanOrEqual(manifest.budgets.repeatCommandMs);

  writeFileSync(
    resolve(repositoryRoot, 'tests/domain-browser/test-results', `measurements-${testInfo.project.name}.json`),
    `${JSON.stringify({
      browser: testInfo.project.name,
      fixtureIds: fixtures.map((fixture: { id: string }) => fixture.id),
      fixtureCount: fixtures.length,
      fixtureInventorySha256: manifest.fixtures.fixtureInventorySha256,
      fixtureHashEncoding: manifest.fixtures.hashEncoding,
      ...measurements,
      outputs: undefined,
    }, null, 2)}\n`,
  );
});
