import { StudyQuestion, StudyResult } from './types';

const fallbackChoiceLabels = ['A', 'B', 'C', 'D'];

type ChoiceLike = NonNullable<StudyQuestion['choices']>[number] & Record<string, unknown>;

export interface ChoiceDisplay {
  value: string;
  label: string;
  text: string;
}

function firstNonEmptyChoiceField(choice: Record<string, unknown>, keys: string[]) {
  for (const key of keys) {
    const value = choice[key];
    if (value == null) continue;
    const text = String(value).trim();
    if (text) return text;
  }
  return undefined;
}

export function choiceDisplay(choice: ChoiceLike, index: number): ChoiceDisplay {
  const fallbackLabel =
    index < fallbackChoiceLabels.length ? fallbackChoiceLabels[index] : String(index + 1);
  const label =
    firstNonEmptyChoiceField(choice, ['label', 'Label', 'value', 'Value']) ?? fallbackLabel;
  const text =
    firstNonEmptyChoiceField(choice, [
      'text',
      'Text',
      'meaning',
      'Meaning',
      'word',
      'Word',
      'value',
      'Value',
    ]) ?? label;
  return { value: label, label, text };
}

export function normalizeChoiceToken(value?: string | null) {
  return (value ?? '').trim().toLowerCase().replace(/\s+/g, ' ');
}

export function choiceUserAnswerTokens(result?: StudyResult, responseOverride?: string) {
  const raw = responseOverride?.trim() || result?.userResponse || '';
  if (!raw.trim()) return new Set<string>();
  const tokens = [normalizeChoiceToken(raw)];
  raw
    .split(/[,，;；、|]+/)
    .map(normalizeChoiceToken)
    .filter(Boolean)
    .forEach((token) => tokens.push(token));
  return new Set(tokens);
}

export function sanitizeChoices(choices?: StudyQuestion['choices']) {
  if (!choices?.length) return [];
  const seen = new Set<string>();
  const cleaned: NonNullable<StudyQuestion['choices']> = [];
  choices.forEach((choice, index) => {
    const display = choiceDisplay(choice, index);
    const key = normalizeChoiceToken(display.text);
    if (!key || seen.has(key)) return;
    seen.add(key);
    cleaned.push({
      label: display.label,
      value: choice.value ?? display.text,
      text: display.text,
    });
  });
  return cleaned;
}

export function validateChoicePayload(question: StudyQuestion) {
  if (!isChoiceQuestionType(question.questionType)) return { valid: true, choices: [] };
  const choices = sanitizeChoices(question.choices);
  return {
    valid: choices.length === 4,
    choices,
    error: choices.length === 4 ? undefined : 'Invalid choice payload',
  };
}

export function isChoiceQuestionType(questionType: StudyQuestion['questionType'] | string) {
  return (
    questionType === 'enToCnChoice' ||
    questionType === 'cnToEnChoice' ||
    questionType === 'exampleToCnChoice' ||
    questionType === 'exampleToCnChoiceNoTranslation'
  );
}

export function isInputQuestionType(questionType: StudyQuestion['questionType'] | string) {
  return !isChoiceQuestionType(questionType);
}

export function resolvedCorrectChoiceTextToken(question: StudyQuestion, result?: StudyResult) {
  const choices = sanitizeChoices(question.choices);
  const correctAnswerToken = normalizeChoiceToken(result?.correctAnswer);
  if (correctAnswerToken) {
    for (const [index, choice] of choices.entries()) {
      const display = choiceDisplay(choice, index);
      const textToken = normalizeChoiceToken(display.text);
      if (textToken === correctAnswerToken) return textToken;
    }
  }

  const correctLabelToken = normalizeChoiceToken(question.correctChoiceLabel);
  if (correctLabelToken) {
    for (const [index, choice] of choices.entries()) {
      const display = choiceDisplay(choice, index);
      if (
        normalizeChoiceToken(display.label) === correctLabelToken ||
        normalizeChoiceToken(display.value) === correctLabelToken
      ) {
        return normalizeChoiceToken(display.text);
      }
    }
    return '';
  }

  if (question.questionType === 'cnToEnChoice') {
    const wordToken = normalizeChoiceToken(question.word);
    for (const [index, choice] of choices.entries()) {
      const display = choiceDisplay(choice, index);
      const textToken = normalizeChoiceToken(display.text);
      if (textToken === wordToken) return textToken;
    }
  }

  return '';
}

function isCorrectChoice(
  question: StudyQuestion,
  displayLabelToken: string,
  displayTextToken: string,
  correctTextToken: string,
) {
  const correctLabelToken = normalizeChoiceToken(question.correctChoiceLabel);
  if (correctTextToken && displayTextToken === correctTextToken) return true;
  if (correctLabelToken) return displayLabelToken === correctLabelToken;

  if (question.questionType === 'cnToEnChoice') {
    const wordToken = normalizeChoiceToken(question.word);
    return Boolean(wordToken) && displayTextToken === wordToken;
  }

  return false;
}

export function choiceState(
  question: StudyQuestion,
  result: StudyResult | undefined,
  display: ChoiceDisplay,
  correctTextToken: string,
  responseOverride?: string,
) {
  const displayLabelToken = normalizeChoiceToken(display.label);
  const displayValueToken = normalizeChoiceToken(display.value);
  const displayTextToken = normalizeChoiceToken(display.text);
  const userTokens = choiceUserAnswerTokens(result, responseOverride);
  const isCorrect =
    Boolean(result) &&
    isCorrectChoice(question, displayLabelToken, displayTextToken, correctTextToken);
  return {
    isCorrect,
    isUserWrong:
      Boolean(result) &&
      (userTokens.has(displayLabelToken) ||
        userTokens.has(displayValueToken) ||
        userTokens.has(displayTextToken)) &&
      !isCorrect &&
      result?.outcome !== 'skipped',
  };
}

export function correctChoiceText(question: StudyQuestion) {
  const choices = sanitizeChoices(question.choices);
  const correctLabelToken = normalizeChoiceToken(question.correctChoiceLabel);
  if (correctLabelToken) {
    const correctIndex = choices.findIndex(
      (choice, index) =>
        normalizeChoiceToken(choiceDisplay(choice, index).label) === correctLabelToken,
    );
    if (correctIndex >= 0) return choiceDisplay(choices[correctIndex], correctIndex).text;
  }
  return question.acceptedMeanings[0] ?? '';
}

export function correctResponseForQuestion(question: StudyQuestion) {
  const choices = sanitizeChoices(question.choices);
  if (choices.length) {
    const correctLabelToken = normalizeChoiceToken(question.correctChoiceLabel);
    const correctIndex = choices.findIndex(
      (choice, index) =>
        normalizeChoiceToken(choiceDisplay(choice, index).label) === correctLabelToken,
    );
    if (correctIndex >= 0) return choiceDisplay(choices[correctIndex], correctIndex).value;
  }
  return question.acceptedMeanings[0] ?? '';
}
