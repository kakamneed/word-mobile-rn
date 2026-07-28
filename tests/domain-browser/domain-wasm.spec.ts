import { expect, test } from '@playwright/test';
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
  const fixtures = fixtureManifest.fixtures.map((fixture: { id: string; request: string; expected: string }) => ({
    id: fixture.id,
    request: readFileSync(resolve(repositoryRoot, 'fixtures/domain/v1', fixture.request), 'utf8'),
    expected: JSON.parse(readFileSync(resolve(repositoryRoot, 'fixtures/domain/v1', fixture.expected), 'utf8')),
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

    const repeatStart = performance.now();
    for (let index = 0; index < 25; index += 1) domain.execute_v1(cases[index % cases.length].request);
    const repeatCommandMs = (performance.now() - repeatStart) / 25;
    const error = JSON.parse(domain.execute_v1('{"protocolVersion":1,"command":"missing"}'));
    return { startupMs, firstCommandMs, repeatCommandMs, outputs, error };
  }, { moduleUrl: `${origin}${modulePath}`, cases: fixtures });

  for (const fixture of fixtures) {
    expect(measurements.outputs.find((output) => output.id === fixture.id)?.value).toEqual(fixture.expected);
  }
  expect(measurements.error.error.code).toBeTruthy();
  expect(measurements.error.result).toBeUndefined();
  expect(measurements.startupMs).toBeLessThanOrEqual(manifest.budgets.startupMs);
  expect(measurements.firstCommandMs).toBeLessThanOrEqual(manifest.budgets.firstCommandMs);
  expect(measurements.repeatCommandMs).toBeLessThanOrEqual(manifest.budgets.repeatCommandMs);

  writeFileSync(
    resolve(repositoryRoot, 'tests/domain-browser/test-results', `measurements-${testInfo.project.name}.json`),
    `${JSON.stringify({ browser: testInfo.project.name, ...measurements, outputs: undefined }, null, 2)}\n`,
  );
});
