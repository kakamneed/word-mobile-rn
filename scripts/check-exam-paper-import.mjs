import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

function fail(message) {
  throw new Error(message);
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8').replace(/^\uFEFF/u, ''));
}

function incrementKind(byKind, kind, hasAnswer, autoGradable) {
  const current = byKind[kind] ?? { questions: 0, answerBearing: 0, autoGradable: 0 };
  current.questions += 1;
  if (hasAnswer) current.answerBearing += 1;
  if (autoGradable) current.autoGradable += 1;
  byKind[kind] = current;
}

export function auditExamPaperAssets({
  assetDir = 'apps/flutter_mobile/assets/exam-papers',
} = {}) {
  const manifestPath = path.join(assetDir, 'manifest.json');
  const manifest = readJson(manifestPath);
  if (manifest.schemaVersion !== 1) fail('manifest schemaVersion must be 1');
  if (!Array.isArray(manifest.files) || manifest.files.length === 0) {
    fail('manifest files must not be empty');
  }

  let totalPapers = 0;
  let totalQuestions = 0;
  let answerBearingQuestions = 0;
  const latestByExam = {};
  const byKind = {};
  const capabilities = {
    browsable: 0,
    answerable: 0,
    autoGradable: 0,
    causalAnalyzable: 0,
  };

  for (const relativeFile of manifest.files) {
    const filePath = path.isAbsolute(relativeFile)
      ? relativeFile
      : path.resolve(relativeFile);
    const payload = readJson(filePath);
    if (payload.schemaVersion !== 1) fail(`${relativeFile}: schemaVersion must be 1`);
    if (!payload.exam) fail(`${relativeFile}: missing exam`);
    if (!Array.isArray(payload.papers) || payload.papers.length === 0) {
      fail(`${relativeFile}: papers missing`);
    }
    for (const paper of payload.papers) {
      totalPapers += 1;
      if (!paper.id || !paper.exam || !paper.title || !paper.year) {
        fail(`${relativeFile}: incomplete paper identity`);
      }
      if (!paper.source?.repo || !paper.source?.path) {
        fail(`${paper.id}: source metadata missing`);
      }
      if (!Array.isArray(paper.sections) || paper.sections.length === 0) {
        fail(`${paper.id}: sections missing`);
      }
      latestByExam[paper.exam] = Math.max(latestByExam[paper.exam] ?? 0, paper.year);
      for (const section of paper.sections) {
        if (!section.id || !section.title) fail(`${paper.id}: incomplete section`);
        if (!Array.isArray(section.questions)) fail(`${section.id}: questions must be an array`);
        const translations = section.paragraphTranslations ?? [];
        if (!Array.isArray(translations)) {
          fail(`${paper.id}/${section.id}: paragraphTranslations must be an array`);
        }
        const paragraphCount = String(section.passage ?? '')
          .split(/\n+/u)
          .filter((paragraph) => paragraph.trim()).length;
        if (translations.length > 0 && translations.length !== paragraphCount) {
          fail(
            `${paper.id}/${section.id}: ${translations.length} translations for ${paragraphCount} paragraphs`,
          );
        }
        for (const question of section.questions) {
          totalQuestions += 1;
          if (!question.id || !Number.isFinite(question.number)) {
            fail(`${section.id}: incomplete question`);
          }
          if (!Array.isArray(question.choices)) fail(`${question.id}: choices must be an array`);
          for (const choice of question.choices) {
            if (!/^[A-Z]$/u.test(choice.label) || !choice.text) {
              fail(`${question.id}: invalid choice`);
            }
          }

          const kind = String(question.kind || 'subjective');
          const hasAnswer = Boolean(question.answer);
          const choiceLabels = new Set(question.choices.map((choice) => choice.label));
          const declaredAutoGradable = question.capabilities?.autoGradable === true;
          if (
            declaredAutoGradable &&
            !choiceLabels.has(String(question.answer).trim().toUpperCase())
          ) {
            fail(`${question.id}: answer ${question.answer} is not present in choices`);
          }
          const autoGradable =
            kind === 'objective' &&
            question.choices.length > 0 &&
            hasAnswer &&
            choiceLabels.has(String(question.answer).trim().toUpperCase());
          const answerable =
            question.choices.length > 0 ||
            kind === 'translation' ||
            kind === 'writing' ||
            Boolean(String(question.stem ?? '').trim());
          const causalAnalyzable =
            autoGradable &&
            Boolean(String(section.passage ?? question.stem ?? '').trim());

          capabilities.browsable += 1;
          if (answerable) capabilities.answerable += 1;
          if (autoGradable) capabilities.autoGradable += 1;
          if (causalAnalyzable) capabilities.causalAnalyzable += 1;
          if (hasAnswer) answerBearingQuestions += 1;
          incrementKind(byKind, kind, hasAnswer, autoGradable);
        }
      }
    }
  }

  if (totalPapers !== manifest.totalPapers) {
    fail(`manifest totalPapers=${manifest.totalPapers} does not match ${totalPapers}`);
  }

  return {
    totalPapers,
    totalQuestions,
    answerBearingQuestions,
    latestByExam,
    capabilities,
    byKind,
  };
}

function parseArgs(values) {
  const args = new Map();
  for (const arg of values) {
    const [key, value] = arg.split('=', 2);
    if (key?.startsWith('--')) args.set(key.slice(2), value ?? '');
  }
  return args;
}

if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  const args = parseArgs(process.argv.slice(2));
  const result = auditExamPaperAssets({
    assetDir: args.get('assetDir') ?? 'apps/flutter_mobile/assets/exam-papers',
  });
  console.log(
    `examPaperImport ok papers=${result.totalPapers} questions=${result.totalQuestions} ` +
      `answers=${result.answerBearingQuestions} capabilities=${JSON.stringify(result.capabilities)} ` +
      `byKind=${JSON.stringify(result.byKind)} latest=${JSON.stringify(result.latestByExam)}`,
  );
}
