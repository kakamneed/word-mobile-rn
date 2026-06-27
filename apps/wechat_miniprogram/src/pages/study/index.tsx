import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Button, Input, Swiper, SwiperItem, Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import { PrimaryButton } from '@/components/PrimaryButton';
import { sdk } from '@/sdk';
import {
  choiceDisplay,
  choiceState,
  isInputQuestionType,
  resolvedCorrectChoiceTextToken,
  sanitizeChoices,
  validateChoicePayload,
} from '@/sdk/studyChoice';
import { handleStudyChoiceTap } from '@/sdk/studyInteraction';
import { buildStudyFeedItems } from '@/sdk/studyFeed';
import {
  CompleteSessionResponse,
  StartSessionResponse,
  StudyMode,
  StudyQuestion,
  StudyResult,
  SubmitAnswerResponse,
  WordbookSummary,
} from '@/sdk/types';

import './index.scss';

const questionTypeLabels: Record<string, string> = {
  enToCnChoice: '根据英文选择中文释义',
  cnToEnChoice: '根据中文选择英文单词',
  exampleToCnChoice: '根据例句选择中文释义',
  exampleToCnChoiceNoTranslation: '根据英文例句选择中文释义',
  enToCnInput: '根据英文填写中文释义',
  wordSkeletonInput: '根据英文补全缺失字母',
  glossToRootInput: '根据含义填写词根/词缀',
  rootToGlossInput: '根据词根/词缀填写含义',
};

function weightsForMode(
  weightsByMode: Record<string, Record<string, number>> | undefined,
  mode: StudyMode,
) {
  const weights = weightsByMode?.[mode];
  if (!weights) return null;
  return Object.entries(weights).map(([questionType, weight]) => ({ questionType, weight }));
}

function activeWordbookId(wordbooks: WordbookSummary[]) {
  return wordbooks.find((wordbook) => wordbook.isActive)?.id;
}

function progressLabelForItem(
  item: ReturnType<typeof buildStudyFeedItems>[number] | undefined,
  session: StartSessionResponse,
) {
  if (!item || item.kind === 'completion') return `${session.progress.current}/${session.progress.total}`;
  return `${item.question.questionIndex}/${item.question.totalQuestions}`;
}

export default function StudyPage() {
  const [session, setSession] = useState<StartSessionResponse>();
  const [selected, setSelected] = useState('');
  const [inputValue, setInputValue] = useState('');
  const [lastSubmittedQuestion, setLastSubmittedQuestion] = useState<StudyQuestion>();
  const [lastSubmittedResult, setLastSubmittedResult] = useState<StudyResult>();
  const [lastSubmittedResponse, setLastSubmittedResponse] = useState('');
  const [completion, setCompletion] = useState<CompleteSessionResponse>();
  const [visiblePageIndex, setVisiblePageIndex] = useState(0);
  const [submitting, setSubmitting] = useState(false);
  const startedAtRef = useRef(Date.now());
  const pendingFeedbackQuestionIdRef = useRef<string>();
  const lastChoiceTapRef = useRef<
    { questionId: string; value: string; at: number; submitted?: boolean } | undefined
  >();

  const startWithBestAvailableSeed = useCallback(async () => {
    const mode = (Taro.getCurrentInstance().router?.params.mode ?? 'review') as StudyMode;
    const resume = Taro.getCurrentInstance().router?.params.resume === '1';
    const rawEntrySourceIds = Taro.getCurrentInstance().router?.params.entrySourceIds;
    const [plan, wordbooks] = await Promise.all([
      sdk.plan.getActivePlan(),
      sdk.plan.getWordbooks(),
    ]);
    const hint = await sdk.study.getResumeSessionHint();
    const resumeHint = resume || hint.mode === mode ? hint : undefined;
    const questionTypeWeights = resumeHint?.hasResume
      ? undefined
      : weightsForMode(plan.questionTypeWeightsByMode, mode);
    const nextSession = await sdk.study.startSession({
      mode: resumeHint?.mode ?? mode,
      entrySourceIds: rawEntrySourceIds ? rawEntrySourceIds.split(',').filter(Boolean) : [],
      wordbookId: activeWordbookId(wordbooks),
      questionTypeWeights,
    });
    startedAtRef.current = Date.now();
    lastChoiceTapRef.current = undefined;
    pendingFeedbackQuestionIdRef.current = undefined;
    setSession(nextSession);
    setVisiblePageIndex(0);
    setCompletion(undefined);
    setLastSubmittedQuestion(undefined);
    setLastSubmittedResult(undefined);
    setLastSubmittedResponse('');
  }, []);

  useEffect(() => {
    startWithBestAvailableSeed().catch((error) =>
      Taro.showToast({ title: String(error), icon: 'none' }),
    );
  }, [startWithBestAvailableSeed]);

  const feedItems = useMemo(
    () =>
      buildStudyFeedItems({
        session,
        lastSubmittedQuestion,
        lastSubmittedResult,
        lastSubmittedResponse,
        completion,
      }),
    [completion, lastSubmittedQuestion, lastSubmittedResponse, lastSubmittedResult, session],
  );

  useEffect(() => {
    if (!feedItems.length) return;
    const feedbackQuestionId = pendingFeedbackQuestionIdRef.current;
    if (feedbackQuestionId) {
      const feedbackIndex = feedItems.findIndex(
        (item) => item.kind === 'answered' && item.question.questionId === feedbackQuestionId,
      );
      if (feedbackIndex >= 0) {
        setVisiblePageIndex(feedbackIndex);
        pendingFeedbackQuestionIdRef.current = undefined;
        return;
      }
    }
    if (completion) setVisiblePageIndex(feedItems.length - 1);
  }, [completion, feedItems]);

  const selectedChoiceValue = (question: StudyQuestion) => {
    const choices = sanitizeChoices(question.choices);
    const selectedIndex = choices.findIndex((choice, index) => {
      const display = choiceDisplay(choice, index);
      return display.label === selected || display.value === selected;
    });
    if (selectedIndex < 0) return selected;
    return choiceDisplay(choices[selectedIndex], selectedIndex).value;
  };

  const applySubmitResponse = (
    question: StudyQuestion,
    response: string,
    answer: SubmitAnswerResponse,
  ) => {
    setLastSubmittedQuestion(question);
    pendingFeedbackQuestionIdRef.current = question.questionId;
    setLastSubmittedResult(answer.result);
    setLastSubmittedResponse(response);
    setSelected('');
    setInputValue('');
    lastChoiceTapRef.current = undefined;
    if (session) {
      setSession({
        session: session.session,
        currentQuestion: answer.currentQuestion ?? question,
        progress: answer.progress,
        answeredQuestions: answer.answeredQuestions,
      });
    }
    if (answer.isComplete && answer.summary) {
      setCompletion({ summary: answer.summary, nextAction: answer.nextAction ?? '返回今日' });
    } else {
      startedAtRef.current = Date.now();
    }
  };

  const submitQuestion = async (
    question: StudyQuestion,
    explicitResponse?: string,
    allowEmpty = false,
  ) => {
    const chosen = explicitResponse ?? selectedChoiceValue(question) ?? inputValue;
    const response = chosen.trim();
    if (!response && !allowEmpty) return;
    setSubmitting(true);
    try {
      const responseTimeMs = Date.now() - startedAtRef.current;
      const answer = await sdk.study.submitAnswer(question.questionId, response, responseTimeMs);
      applySubmitResponse(question, response, answer);
    } finally {
      setSubmitting(false);
    }
  };

  const markEntryMastered = async (question: StudyQuestion) => {
    if (submitting) return;
    setSubmitting(true);
    try {
      const response = await sdk.study.markEntryMastered(question.entrySourceId);
      if (session) {
        setSession({
          ...session,
          currentQuestion: response.currentQuestion ?? session.currentQuestion,
          progress: response.progress,
          answeredQuestions: response.answeredQuestions,
        });
      }
      lastChoiceTapRef.current = undefined;
      if (response.isComplete && response.summary) {
        setCompletion({ summary: response.summary, nextAction: response.nextAction ?? '返回今日' });
      }
    } finally {
      setSubmitting(false);
    }
  };

  const acceptDisputedMeaning = async (question: StudyQuestion) => {
    const result = lastSubmittedResult;
    const submitted = lastSubmittedResponse || result?.userResponse || '';
    if (
      !result ||
      result.outcome !== 'incorrect' ||
      !isInputQuestionType(question.questionType) ||
      lastSubmittedQuestion?.questionId !== question.questionId ||
      !submitted.trim()
    ) {
      Taro.showToast({ title: '只有输入题答错后才能申诉。', icon: 'none' });
      return;
    }
    const response = await sdk.study.acceptDisputedMeaning(question.questionId, submitted);
    setLastSubmittedResult(response.result);
    Taro.showToast({ title: '已接受申诉', icon: 'none' });
  };

  const returnToTodayKeepingProgress = async () => {
    if (session && !completion) {
      await sdk.study.preserveProgress();
    }
    Taro.redirectTo({ url: '/pages/today/index?refresh=1' });
  };

  const showExitOptions = () => {
    Taro.showModal({
      title: '退出学习',
      content: '保留当前进度，还是放弃本轮学习？',
      confirmText: '保留进度',
      cancelText: '放弃本轮',
      success: async (result) => {
        if (result.confirm) {
          await returnToTodayKeepingProgress();
          return;
        }
        if (session?.session.sessionId) await sdk.study.cancelSession(session.session.sessionId);
        Taro.redirectTo({ url: '/pages/today/index?refresh=1' });
      },
    });
  };

  if (!session || !feedItems.length) {
    return (
      <View className="study-feed">
        <Text className="study-loading">正在加载学习会话...</Text>
      </View>
    );
  }

  const currentFeedItem = feedItems[visiblePageIndex];

  return (
    <View className="study-feed">
      <View className="study-progress">
        <Button className="study-nav study-nav--home" onClick={returnToTodayKeepingProgress}>
          <StudyIcon name="home" />
        </Button>
        <Text className="study-progress__label">{progressLabelForItem(currentFeedItem, session)}</Text>
        <Button className="study-nav study-nav--close" onClick={showExitOptions}>
          <StudyIcon name="close" />
        </Button>
      </View>

      <Swiper
        className="study-swiper"
        vertical
        current={visiblePageIndex}
        onChange={(event) => {
          setVisiblePageIndex(event.detail.current);
          setSelected('');
          setInputValue('');
          lastChoiceTapRef.current = undefined;
          startedAtRef.current = Date.now();
        }}
      >
        {feedItems.map((item, index) => (
          <SwiperItem key={`${item.kind}-${index}`}>
            {item.kind === 'completion' ? (
              <View className="study-page study-page--completion">
                <Text className="completion-title">本组完成</Text>
                <Text className="completion-score">{item.completion.summary.accuracyPercent}%</Text>
                <Text className="completion-caption">
                  共 {item.completion.summary.totalQuestions} 题，答对 {item.completion.summary.correctCount} 题
                </Text>
                <View className="result__actions">
                  <PrimaryButton onClick={() => Taro.redirectTo({ url: '/pages/today/index?refresh=1' })}>返回今日</PrimaryButton>
                  <PrimaryButton variant="secondary" onClick={() => Taro.navigateTo({ url: '/subpkg/reports/index' })}>查看报告</PrimaryButton>
                </View>
              </View>
            ) : (
              <QuestionPage
                question={item.question}
                result={item.kind === 'answered' ? item.result : undefined}
                responseOverride={item.kind === 'answered' ? item.responseOverride : undefined}
                selected={selected}
                inputValue={inputValue}
                submitting={submitting}
                onInput={setInputValue}
                onChoiceTap={(choice) => {
                  const now = Date.now();
                  const action = handleStudyChoiceTap({
                    questionId: item.question.questionId,
                    choice,
                    selected,
                    now,
                    previous: lastChoiceTapRef.current,
                    onSelect: setSelected,
                    onSubmit: (value) => submitQuestion(item.question, value),
                  });
                  lastChoiceTapRef.current = {
                    questionId: item.question.questionId,
                    value: choice,
                    at: now,
                    submitted: action.shouldSubmit,
                  };
                }}
                onSubmitInput={() => submitQuestion(item.question)}
                onReveal={() => submitQuestion(item.question, '', true)}
                onMastered={() => markEntryMastered(item.question)}
                onHint={() => Taro.showToast({ title: item.question.userHint || '暂无提示', icon: 'none' })}
                onComment={() => Taro.showToast({ title: '评论入口待接入', icon: 'none' })}
                onDispute={() => acceptDisputedMeaning(item.question)}
              />
            )}
          </SwiperItem>
        ))}
      </Swiper>
    </View>
  );
}

function StudyIcon({ name }: { name: string }) {
  return <Text className={`study-icon study-icon--${name}`} />;
}

function QuestionPage({
  question,
  result,
  responseOverride,
  selected,
  inputValue,
  submitting,
  onInput,
  onChoiceTap,
  onSubmitInput,
  onReveal,
  onMastered,
  onHint,
  onComment,
  onDispute,
}: {
  question: StudyQuestion;
  result?: StudyResult;
  responseOverride?: string;
  selected: string;
  inputValue: string;
  submitting: boolean;
  onInput: (value: string) => void;
  onChoiceTap: (choice: string) => void;
  onSubmitInput: () => void;
  onReveal: () => void;
  onMastered: () => void;
  onHint: () => void;
  onComment: () => void;
  onDispute: () => void;
}) {
  const answered = Boolean(result);
  const isCorrect = result?.outcome === 'correct' || result?.outcome === 'fuzzyCorrect';
  const requiresInput = isInputQuestionType(question.questionType);
  const choicePayload = validateChoicePayload(question);
  const choices = choicePayload.choices;
  const correctTextToken = resolvedCorrectChoiceTextToken(question, result);

  return (
    <View className="study-page">
      <View className="study-main">
        <Text className="study-card__type">{questionTypeLabels[question.questionType] ?? question.questionType}</Text>
        <Text className="study-card__word">{question.word}</Text>
        {question.partOfSpeech && <Text className="study-card__meta">{question.partOfSpeech}</Text>}
        {(question.phoneticUk || question.phoneticUs) && <Text className="study-card__phonetic">{question.phoneticUk ?? question.phoneticUs}</Text>}
        <Text className="study-card__prompt">{question.prompt}</Text>
        {question.exampleSentence && (
          <View className="study-card__example">
            <Text className="study-card__sentence">{question.exampleSentence}</Text>
            {question.exampleTranslation && <Text className="study-card__translation">{question.exampleTranslation}</Text>}
          </View>
        )}
      </View>

      <View className="side-actions">
        <Button onClick={onHint}><StudyIcon name="lightbulb" /></Button>
        <Button onClick={onComment}><StudyIcon name="comment" /></Button>
        <Button disabled={answered || submitting} onClick={onReveal}><StudyIcon name="visibility" /></Button>
        <Button disabled={!result || result.outcome !== 'incorrect' || !requiresInput} onClick={onDispute}><StudyIcon name="gavel" /></Button>
        <Button disabled={answered || submitting} onClick={onMastered}><StudyIcon name="delete" /></Button>
      </View>

      <View className="answer-panel">
        {requiresInput && answered ? (
          <View className={`result result--${isCorrect ? 'correct' : 'incorrect'}`}>
            <Text className="result__title">{isCorrect ? '已答对' : result?.outcome === 'skipped' ? '已查看答案' : '需要复习'}</Text>
            <Text className="result__caption">正确答案：{result?.correctAnswer}</Text>
            <Text className="result__caption">你的答案：{responseOverride || result?.userResponse || '未作答'}</Text>
          </View>
        ) : requiresInput ? (
          <View className="input-answer">
            <Input value={inputValue} onInput={(event) => onInput(event.detail.value)} placeholder="输入答案" />
            <PrimaryButton disabled={!inputValue || submitting} onClick={onSubmitInput}>提交</PrimaryButton>
          </View>
        ) : !choicePayload.valid ? (
          <View className="contract-error"><Text>选项数据无效</Text></View>
        ) : (
          <View className="choice-list">
            {choices.map((choice, index) => {
              const display = choiceDisplay(choice, index);
              const state = choiceState(question, result, display, correctTextToken, responseOverride);
              const isSelected = selected === display.value || selected === display.label;
              return (
                <Button
                  key={`${display.label}-${display.text}`}
                  className={`choice ${isSelected ? 'choice--selected' : ''} ${state.isCorrect ? 'choice--correct' : ''} ${state.isUserWrong ? 'choice--wrong' : ''}`}
                  disabled={answered || submitting}
                  onClick={() => onChoiceTap(display.value)}
                >
                  <Text className="choice__label">{display.label}</Text>
                  <Text className="choice__text">{display.text}</Text>
                  {state.isCorrect && <Text className="choice__marker choice__marker--correct" />}
                  {state.isUserWrong && <Text className="choice__marker choice__marker--wrong" />}
                </Button>
              );
            })}
          </View>
        )}
      </View>
    </View>
  );
}
