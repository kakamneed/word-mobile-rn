import { MiniProgramSdk } from '../src/sdk/client';
import { questionBank, rootAffixFallbackPayloads } from '../src/sdk/mockData';
import { StudyQuestion } from '../src/sdk/types';
import {
  choiceDisplay,
  choiceState,
  resolvedCorrectChoiceTextToken,
  sanitizeChoices,
  validateChoicePayload,
} from '../src/sdk/studyChoice';
import { buildStudyFeedItems } from '../src/sdk/studyFeed';
import { handleStudyChoiceTap } from '../src/sdk/studyInteraction';

async function main() {
  const sdk = new MiniProgramSdk();
  const uniqueNewWords = new Set(questionBank.newWord.slice(0, 20).map((question) => question.word));
  if (uniqueNewWords.size !== 20) {
    throw new Error('Expected new-word bank to provide 20 distinct first-session words');
  }
  if (questionBank.newWord.slice(0, 20).some((question) => sanitizeChoices(question.choices).length !== 4)) {
    throw new Error('Expected every new-word question to keep four sanitized choices');
  }

  const beforeReports = await sdk.reports.getOverview();
  const beforeWrongWords = await sdk.wrongWords.list();
  const beforeRank = await sdk.leaderboard.getSummary('weekly');
  const beforeToday = await sdk.today.getTodayHomeState();
  const plan = await sdk.plan.getActivePlan();
  const questionTypeWeights = Object.entries(plan.questionTypeWeightsByMode?.review ?? {}).map(
    ([questionType, weight]) => ({ questionType, weight }),
  );
  const session = await sdk.study.startSession({ mode: 'newWord', questionTypeWeights });
  if (session.currentQuestion.word !== questionBank.newWord[0].word) {
    throw new Error('Expected session to start from the first generated new-word question');
  }

  let selected = '';
  const tapProbe: { submitCount: number } = { submitCount: 0 };
  const firstChoice = sanitizeChoices(session.currentQuestion.choices)[0];
  const firstChoiceDisplay = choiceDisplay(firstChoice, 0);
  handleStudyChoiceTap({
    questionId: session.currentQuestion.questionId,
    choice: firstChoiceDisplay.value,
    selected,
    now: 1000,
    previous: { questionId: session.currentQuestion.questionId, value: '', at: 0 },
    onSelect: (value) => {
      selected = value;
    },
    onSubmit: () => {
      tapProbe.submitCount += 1;
    },
  });

  if (!selected || tapProbe.submitCount !== 0) {
    throw new Error('Expected first choice tap to select without submitting');
  }

  handleStudyChoiceTap({
    questionId: session.currentQuestion.questionId,
    choice: selected,
    selected,
    now: 1800,
    previous: { questionId: session.currentQuestion.questionId, value: selected, at: 1000 },
    onSelect: (value) => {
      selected = value;
    },
    onSubmit: () => {
      tapProbe.submitCount += 1;
    },
  });

  if (tapProbe.submitCount !== 0) {
    throw new Error('Expected a slow second tap not to submit');
  }

  handleStudyChoiceTap({
    questionId: session.currentQuestion.questionId,
    choice: selected,
    selected,
    now: 1920,
    previous: { questionId: session.currentQuestion.questionId, value: selected, at: 1800 },
    onSelect: (value) => {
      selected = value;
    },
    onSubmit: () => {
      tapProbe.submitCount += 1;
    },
  });

  const submittedCount = Number(tapProbe.submitCount);
  if (submittedCount !== 1) {
    throw new Error('Expected a quick second tap on the selected choice to submit once');
  }

  const duplicatePayload = validateChoicePayload({
    ...session.currentQuestion,
    choices: [
      { label: 'A', text: 'same', value: 'same' },
      { label: 'B', text: 'same', value: 'same' },
      { label: 'C', text: 'other', value: 'other' },
      { label: 'D', text: 'third', value: 'third' },
    ],
  });
  if (duplicatePayload.valid) {
    throw new Error('Expected duplicate-choice payload to be rejected');
  }

  const currentChoices = sanitizeChoices(session.currentQuestion.choices);
  const wrongChoiceIndex = currentChoices.findIndex(
    (choice, index) => choiceDisplay(choice, index).text !== session.currentQuestion.acceptedMeanings[0],
  );
  if (wrongChoiceIndex < 0) throw new Error('Expected a wrong choice fixture');
  const wrongResponse = choiceDisplay(currentChoices[wrongChoiceIndex], wrongChoiceIndex).value;
  const answer = await sdk.study.submitAnswer(
    session.currentQuestion.questionId,
    wrongResponse,
    1800,
  );
  try {
    await sdk.study.submitAnswer(session.currentQuestion.questionId, wrongResponse, 1800);
    throw new Error('Expected stale question submit to fail');
  } catch (error) {
    if (!(error instanceof Error) || !error.message.includes('current question')) throw error;
  }
  const feedItems = buildStudyFeedItems({
    session: {
      ...session,
      currentQuestion: answer.currentQuestion ?? session.currentQuestion,
      progress: answer.progress,
      answeredQuestions: answer.answeredQuestions,
    },
    lastSubmittedQuestion: session.currentQuestion,
    lastSubmittedResult: answer.result,
    lastSubmittedResponse: wrongResponse,
    completion: answer.summary ? { summary: answer.summary, nextAction: answer.nextAction ?? 'Return to today' } : undefined,
  });

  const afterReports = await sdk.reports.getOverview();
  const afterWrongWords = await sdk.wrongWords.list();
  const afterRank = await sdk.leaderboard.getSummary('weekly');
  const afterOneToday = await sdk.today.getTodayHomeState();

  if (afterReports.totalQuestionsAnswered !== beforeReports.totalQuestionsAnswered + 1) {
    throw new Error('Expected report question total to increase by one');
  }

  if (afterWrongWords.length < beforeWrongWords.length) {
    throw new Error('Expected wrong-word list to retain or add entries after an incorrect answer');
  }

  if (!afterRank.currentUser) {
    throw new Error('Expected leaderboard to include the current user');
  }

  if (
    beforeRank.currentUser &&
    afterRank.currentUser.score !== beforeRank.currentUser.score + 1
  ) {
    throw new Error('Expected weekly leaderboard score to follow report daily totals');
  }

  if (!feedItems.some((item) => item.kind === 'answered')) {
    throw new Error('Expected study feed to include answered item');
  }

  if (answer.isComplete) {
    throw new Error('Expected a 20-question plan not to complete after the first answer');
  }

  if (afterOneToday.dailyProgress.completedTasks !== beforeToday.dailyProgress.completedTasks) {
    throw new Error('Expected Today completed task count not to change after one answer');
  }

  if (
    !afterOneToday.resumeHint?.hasResume ||
    afterOneToday.resumeHint.sessionId !== session.session.sessionId ||
    afterOneToday.resumeHint.current !== 2 ||
    afterOneToday.resumeHint.total !== 20
  ) {
    throw new Error('Expected Today to expose a resume hint after an unfinished session');
  }
  const preserved = await sdk.study.preserveProgress();
  if (
    !preserved.resumeHint.hasResume ||
    preserved.resumeHint.sessionId !== session.session.sessionId ||
    preserved.today.modeProgress?.newWord?.completed !== 1
  ) {
    throw new Error('Expected explicit preserveProgress to keep unfinished mode progress');
  }

  const resumed = await sdk.study.startSession({ mode: afterOneToday.resumeHint.mode ?? 'newWord' });
  if (
    resumed.session.sessionId !== session.session.sessionId ||
    resumed.currentQuestion.questionId !== answer.currentQuestion?.questionId ||
    resumed.answeredQuestions.length !== 1
  ) {
    throw new Error('Expected resume to continue the active session instead of restarting');
  }
  const sameModeReentry = await sdk.study.startSession({ mode: 'newWord' });
  if (
    sameModeReentry.session.sessionId !== session.session.sessionId ||
    sameModeReentry.currentQuestion.questionId !== answer.currentQuestion?.questionId ||
    sameModeReentry.answeredQuestions.length !== 1
  ) {
    throw new Error('Expected same-mode reentry to continue the active session instead of restarting');
  }

  if (answer.progress.current !== 2 || answer.progress.total !== 20) {
    throw new Error('Expected first answer to advance the session from 1/20 to 2/20');
  }

  if (!answer.currentQuestion || answer.currentQuestion.word === session.currentQuestion.word) {
    throw new Error('Expected first answer to advance to a different word');
  }

  const rootSdk = new MiniProgramSdk();
  const rootSession = await rootSdk.study.startSession({ mode: 'rootAffix' });
  if (rootAffixFallbackPayloads.length !== 4) {
    throw new Error('Expected mini root-affix fixture to mirror the Flutter fallback payload count used by the current plan');
  }
  if (rootSession.currentQuestion.questionType !== 'rootToGlossInput') {
    throw new Error('Expected root-affix mode to start from a root/gloss input question');
  }
  if (rootSession.currentQuestion.word !== rootAffixFallbackPayloads[0].form) {
    throw new Error('Expected root-affix questions to be generated from rootAffixFallbackPayloads');
  }
  if (rootSession.currentQuestion.word === 'trans-' || rootSession.currentQuestion.word === 're-') {
    throw new Error('Expected root-affix fixture not to use the previous hand-written roots as active session source');
  }
  const rootAnswer = await rootSdk.study.submitAnswer(
    rootSession.currentQuestion.questionId,
    rootSession.currentQuestion.acceptedMeanings[0],
    700,
  );
  if (rootAnswer.isComplete || rootAnswer.progress.total !== 4) {
    throw new Error('Expected root-affix mode to use its four-question task chain');
  }

  const correctStateProbe = currentChoices.map((choice, index) => {
    const display = choiceDisplay(choice, index);
    return choiceState(
      session.currentQuestion,
      answer.result,
      display,
      resolvedCorrectChoiceTextToken(session.currentQuestion, answer.result),
      wrongResponse,
    );
  });
  if (correctStateProbe.filter((state) => state.isCorrect).length !== 1) {
    throw new Error('Expected exactly one choice to be marked correct after answering');
  }
  if (correctStateProbe.filter((state) => state.isUserWrong).length !== 1) {
    throw new Error('Expected exactly one choice to be marked as the user wrong answer');
  }

  if (!feedItems.some((item) => item.kind === 'completion')) {
    if (!feedItems.some((item) => item.kind === 'unanswered')) {
      throw new Error('Expected study feed to include the next unanswered item');
    }
  }

  let current = answer;
  let question: StudyQuestion | undefined = answer.currentQuestion;
  while (!current.isComplete && question) {
    current = await sdk.study.submitAnswer(question.questionId, question.acceptedMeanings[0], 900);
    question = current.currentQuestion;
  }

  if (!current.isComplete || !current.summary) {
    throw new Error('Expected session to complete after all planned questions are answered');
  }

  if (current.summary.totalQuestions !== 20) {
    throw new Error('Expected completion summary to count all 20 planned questions');
  }

  const afterCompleteToday = await sdk.today.getTodayHomeState();
  if (afterCompleteToday.dailyProgress.completedTasks !== beforeToday.dailyProgress.completedTasks + 1) {
    throw new Error('Expected Today completed task count to change once after session completion');
  }

  const summaryModes = [
    { mode: 'newWord', total: 20 },
    { mode: 'review', total: 28 },
    { mode: 'mixedTest', total: 34 },
    { mode: 'wrongWordReinforcement', total: 26 },
    { mode: 'rootAffix', total: 4 },
  ] as const;
  const summarySdk = new MiniProgramSdk();
  for (const { mode, total } of summaryModes) {
    let modeSession = await summarySdk.study.startSession({ mode });
    let modeCurrent: Awaited<ReturnType<typeof summarySdk.study.submitAnswer>> | undefined;
    let nextQuestion: StudyQuestion | undefined = modeSession.currentQuestion;
    while (nextQuestion) {
      modeCurrent = await summarySdk.study.submitAnswer(
        nextQuestion.questionId,
        nextQuestion.acceptedMeanings[0],
        500,
      );
      nextQuestion = modeCurrent.currentQuestion;
    }
    if (!modeCurrent?.isComplete || !modeCurrent.summary) {
      throw new Error(`Expected ${mode} to produce a completion summary`);
    }
    if (
      modeCurrent.summary.totalQuestions !== total ||
      modeCurrent.summary.correctCount !== total ||
      modeCurrent.summary.incorrectCount !== 0 ||
      modeCurrent.summary.skippedCount !== 0 ||
      modeCurrent.summary.accuracyPercent !== 100
    ) {
      throw new Error(`Unexpected ${mode} summary: ${JSON.stringify(modeCurrent.summary)}`);
    }
    const todayAfterMode = await summarySdk.today.getTodayHomeState();
    if (todayAfterMode.modeProgress?.[mode]?.completed !== total || !todayAfterMode.modeProgress?.[mode]?.isComplete) {
      throw new Error(`Expected Today mode progress to mark ${mode} complete`);
    }
    modeSession = await summarySdk.study.startSession({ mode });
    if (modeSession.answeredQuestions.length) {
      throw new Error(`Expected starting ${mode} after completion to create a fresh session`);
    }
  }

  console.log(
    JSON.stringify(
      {
        totalQuestionsAnswered: afterReports.totalQuestionsAnswered,
        wrongWords: afterWrongWords.length,
        weeklyRank: afterRank.currentUser.rank,
        weeklyScore: afterRank.currentUser.score,
        feedItems: feedItems.map((item) => item.kind),
        finalSummary: current.summary,
        questionTypeWeights: questionTypeWeights.length,
      },
      null,
      2,
    ),
  );
}

main().catch((error) => {
  console.error(error);
  throw error;
});
