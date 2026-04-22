import React, {useEffect, useMemo, useRef, useState} from 'react';
import {
  KeyboardAvoidingView,
  Platform,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import type {
  ChoiceOption,
  SessionProgress,
  StudyQuestion,
  StudyResult,
} from '../../lib/study-client';

interface QuestionCardProps {
  question: StudyQuestion;
  progress: SessionProgress;
  result?: StudyResult;
  onSubmit?: (response: string, responseTimeMs: number) => void;
  onNext?: () => void;
  onCancel?: () => void;
  showFeedback?: boolean;
  nextButtonLabel?: string;
}

export function QuestionCard({
  question,
  progress,
  result,
  onSubmit,
  onNext,
  onCancel,
  showFeedback,
  nextButtonLabel,
}: QuestionCardProps): React.JSX.Element {
  const [answer, setAnswer] = useState('');
  const inputRef = useRef<TextInput | null>(null);
  const startTime = useRef(Date.now());

  const isChoice = Array.isArray(question.choices) && question.choices.length > 0;
  const isCnToEnChoice = question.questionType === 'cnToEnChoice';
  const isRootAffix =
    question.questionType === 'glossToRootInput' ||
    question.questionType === 'rootToGlossInput';
  const isExampleChoice =
    question.questionType === 'exampleToCnChoice' &&
    Boolean(question.exampleSentence);
  const isWordPromptQuestion =
    question.questionType === 'enToCnChoice' ||
    question.questionType === 'enToCnInput';
  const isInputQuestion = !isChoice;
  const compactCard = isRootAffix || isExampleChoice || showFeedback;
  const compactInputMode = isRootAffix && !showFeedback;
  const canSubmit = isInputQuestion || answer.trim().length > 0;
  const hideInlinePrompt =
    isRootAffix || isExampleChoice || isCnToEnChoice || isWordPromptQuestion;
  const displayWord = isCnToEnChoice ? question.prompt : question.word;
  const displayPhonetic = isCnToEnChoice
    ? null
    : question.phoneticUs || question.phoneticUk || null;
  const displayPartOfSpeech = isCnToEnChoice
    ? null
    : question.partOfSpeech?.trim() || null;
  const rootFragment = normalizeRootFragment(
    question.questionType === 'glossToRootInput'
      ? (question.acceptedMeanings[0] ?? '')
      : question.word,
  );

  const rootExampleRows = useMemo(() => {
    if (!isRootAffix || !question.exampleSentence) {
      return [];
    }

    const examples = question.exampleSentence
      .split(',')
      .map(item => item.trim())
      .filter(Boolean)
      .slice(0, 3);
    const glosses = (question.exampleTranslation ?? '')
      .split(',')
      .map(item => item.trim())
      .filter(Boolean)
      .slice(0, 3);

    return examples.map((word, index) => ({
      word,
      gloss: glosses[index] ?? '',
    }));
  }, [isRootAffix, question.exampleSentence, question.exampleTranslation]);

  useEffect(() => {
    setAnswer('');
    startTime.current = Date.now();
  }, [question.questionId]);

  useEffect(() => {
    if (!showFeedback && isInputQuestion) {
      const timer = setTimeout(() => inputRef.current?.focus(), 150);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [showFeedback, isInputQuestion, question.questionId]);

  const handleSubmit = () => {
    if (!canSubmit || !onSubmit) {
      return;
    }
    onSubmit(answer.trim(), Date.now() - startTime.current);
  };

  const handleSkip = () => {
    if (!onSubmit) {
      return;
    }
    onSubmit('', Date.now() - startTime.current);
  };

  return (
    <KeyboardAvoidingView
      style={styles.keyboardContainer}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
      keyboardVerticalOffset={Platform.OS === 'ios' ? 20 : 0}>
      <View style={styles.container}>
        <View style={styles.progressBar}>
          <View style={styles.progressFill}>
            <View
              style={[
                styles.progressIndicator,
                {width: `${(progress.current / progress.total) * 100}%`},
              ]}
            />
          </View>
          <Text style={styles.progressText}>
            {progress.current} / {progress.total}
          </Text>
        </View>

        <ScrollView
          contentContainerStyle={[
            styles.scroll,
            compactCard && styles.scrollCompact,
          ]}
          keyboardShouldPersistTaps="handled">
          <View style={[styles.header, compactCard && styles.headerCompact]}>
            <Text style={[styles.word, compactCard && styles.wordCompact]}>
              {displayWord}
            </Text>
            {displayPhonetic || displayPartOfSpeech ? (
              <View style={styles.metaRow}>
                {displayPhonetic ? (
                  <Text style={styles.phonetic}>{displayPhonetic}</Text>
                ) : null}
                {displayPartOfSpeech ? (
                  <Text style={styles.partOfSpeech}>{displayPartOfSpeech}</Text>
                ) : null}
              </View>
            ) : null}
          </View>

          <View
            style={[
              styles.promptSection,
              compactCard && styles.promptSectionCompact,
            ]}>
            <Text style={styles.promptLabel}>
              {getQuestionLabel(question.questionType)}
            </Text>
            {!hideInlinePrompt ? (
              <Text style={styles.prompt}>{question.prompt}</Text>
            ) : null}
          </View>

          {isExampleChoice && question.exampleSentence ? (
            <View style={[styles.exampleBox, styles.exampleBoxCompact]}>
              {renderHighlightedSentence(
                question.exampleSentence,
                question.word,
                styles.exampleEn,
                styles.exampleHighlight,
              )}
              {question.exampleTranslation ? (
                <Text style={styles.exampleCn}>{question.exampleTranslation}</Text>
              ) : null}
            </View>
          ) : null}

          {isRootAffix && rootExampleRows.length > 0 ? (
            <View style={[styles.exampleBox, styles.exampleBoxCompact]}>
              {rootExampleRows.map(item => (
                <View
                  key={`${item.word}-${item.gloss}`}
                  style={styles.rootExampleRow}>
                  {renderHighlightedWord(item.word, rootFragment, Boolean(compactCard))}
                  {item.gloss ? (
                    <Text style={[styles.rootGloss, styles.rootGlossCompact]}>
                      {item.gloss}
                    </Text>
                  ) : null}
                </View>
              ))}
            </View>
          ) : null}

          {!showFeedback ? (
            <>
              {isChoice && question.choices ? (
                <View style={styles.choicesContainer}>
                  {question.choices.map(choice => (
                    <ChoiceButton
                      key={choice.label}
                      choice={choice}
                      selected={answer === choice.label}
                      onPress={() => setAnswer(choice.label)}
                    />
                  ))}
                </View>
              ) : (
                <>
                  <TextInput
                    ref={inputRef}
                    style={[
                      styles.textInput,
                      compactInputMode && styles.textInputCompact,
                    ]}
                    value={answer}
                    onChangeText={setAnswer}
                    placeholder="请输入答案"
                    placeholderTextColor="#999"
                    autoCapitalize="none"
                    autoCorrect={false}
                    multiline={false}
                    blurOnSubmit={false}
                    returnKeyType="send"
                    onSubmitEditing={handleSubmit}
                  />

                  {!compactInputMode ? (
                    <View style={styles.inlineActions}>
                      <TouchableOpacity
                        style={styles.inlinePrimaryButton}
                        onPress={handleSubmit}
                        >
                        <Text style={styles.inlinePrimaryButtonText}>提交答案</Text>
                      </TouchableOpacity>

                      <TouchableOpacity
                        style={styles.inlineSkipButton}
                        onPress={handleSkip}>
                        <Text style={styles.inlineSkipOverlay}>Skip</Text>
                        <Text style={styles.inlineSkipText}>璺宠繃</Text>
                      </TouchableOpacity>

                      {onCancel ? (
                        <TouchableOpacity
                          style={styles.inlineCancelButton}
                          onPress={onCancel}>
                          <Text style={styles.inlineCancelText}>结束本轮</Text>
                        </TouchableOpacity>
                      ) : null}
                    </View>
                  ) : null}

                  {compactInputMode ? (
                    <View style={styles.compactActions}>
                      <TouchableOpacity
                        style={styles.compactActionPrimary}
                        onPress={handleSubmit}>
                        <Text style={styles.compactActionPrimaryText}>Submit</Text>
                      </TouchableOpacity>
                      <TouchableOpacity
                        style={styles.compactActionGhost}
                        onPress={handleSkip}>
                        <Text style={styles.compactActionGhostText}>Skip</Text>
                      </TouchableOpacity>
                      {onCancel ? (
                        <TouchableOpacity
                          style={styles.compactActionGhost}
                          onPress={onCancel}>
                          <Text style={styles.compactActionGhostText}>End</Text>
                        </TouchableOpacity>
                      ) : null}
                    </View>
                  ) : null}
                </>
              )}
            </>
          ) : (
            <View style={styles.feedbackSection}>
              <View
                style={[
                  styles.feedbackBadge,
                  result?.outcome === 'correct' && styles.feedbackCorrect,
                  result?.outcome === 'fuzzyCorrect' && styles.feedbackFuzzy,
                  result?.outcome === 'incorrect' && styles.feedbackIncorrect,
                  result?.outcome === 'skipped' && styles.feedbackSkipped,
                ]}>
                <Text style={styles.feedbackText}>
                  {getOutcomeLabel(result?.outcome)}
                </Text>
              </View>

              {result?.correctAnswer ? (
                <View style={styles.correctAnswerBox}>
                  <Text style={styles.correctAnswerLabel}>正确答案</Text>
                  <Text style={styles.correctAnswerText}>{result.correctAnswer}</Text>
                </View>
              ) : null}
            </View>
          )}
        </ScrollView>

        {!showFeedback && isChoice ? (
          <View style={styles.bottomBar}>
            <TouchableOpacity
              style={[
                styles.primaryButton,
                !canSubmit && styles.primaryButtonDisabled,
              ]}
              onPress={handleSubmit}
              disabled={!canSubmit}>
              <Text style={styles.primaryButtonText}>提交答案</Text>
            </TouchableOpacity>

            {onCancel ? (
              <TouchableOpacity style={styles.secondaryButton} onPress={onCancel}>
                <Text style={styles.secondaryButtonText}>结束本轮</Text>
              </TouchableOpacity>
            ) : null}
          </View>
        ) : showFeedback ? (
          <View style={styles.bottomBar}>
            <TouchableOpacity style={styles.primaryButton} onPress={onNext}>
              <Text style={styles.primaryButtonText}>
                {nextButtonLabel ?? '下一题'}
              </Text>
            </TouchableOpacity>
          </View>
        ) : null}
      </View>
    </KeyboardAvoidingView>
  );
}

function ChoiceButton({
  choice,
  selected,
  onPress,
}: {
  choice: ChoiceOption;
  selected: boolean;
  onPress: () => void;
}): React.JSX.Element {
  return (
    <TouchableOpacity
      style={[styles.choiceButton, selected && styles.choiceButtonSelected]}
      onPress={onPress}>
      <Text style={[styles.choiceLabel, selected && styles.choiceLabelSelected]}>
        {choice.label}
      </Text>
      <Text style={[styles.choiceText, selected && styles.choiceTextSelected]}>
        {choice.text}
      </Text>
    </TouchableOpacity>
  );
}

function getQuestionLabel(type: string): string {
  switch (type) {
    case 'enToCnChoice':
      return '根据英文选择中文释义';
    case 'exampleToCnChoice':
      return '根据例句选择中文释义';
    case 'cnToEnChoice':
      return '根据中文选择英文单词';
    case 'enToCnInput':
      return '根据英文填写中文释义';
    case 'glossToRootInput':
      return '根据例词填写词根/词缀含义';
    case 'rootToGlossInput':
      return '根据词根/词缀填写中文含义';
    default:
      return '回答问题';
  }
}

function getOutcomeLabel(outcome: string | undefined): string {
  switch (outcome) {
    case 'correct':
      return '回答正确';
    case 'fuzzyCorrect':
      return '基本正确';
    case 'incorrect':
      return '回答错误';
    case 'skipped':
      return '已跳过';
    default:
      return '';
  }
}

function normalizeRootFragment(word: string): string {
  return word.replace(/[-\s]/g, '').trim().toLowerCase();
}

function renderHighlightedWord(
  word: string,
  fragment: string,
  compact: boolean,
): React.JSX.Element {
  const lower = word.toLowerCase();
  const index = fragment ? lower.indexOf(fragment) : -1;

  if (index < 0 || !fragment) {
    return (
      <Text style={[styles.rootExampleWord, compact && styles.rootExampleWordCompact]}>
        {word}
      </Text>
    );
  }

  const before = word.slice(0, index);
  const hit = word.slice(index, index + fragment.length);
  const after = word.slice(index + fragment.length);

  return (
    <Text style={[styles.rootExampleWord, compact && styles.rootExampleWordCompact]}>
      {before}
      <Text style={styles.rootHighlight}>{hit}</Text>
      {after}
    </Text>
  );
}

function renderHighlightedSentence(
  sentence: string,
  target: string,
  textStyle: object,
  highlightStyle: object,
): React.JSX.Element {
  const normalizedTarget = target.trim().toLowerCase();
  if (!normalizedTarget) {
    return <Text style={textStyle}>{sentence}</Text>;
  }

  const lowerSentence = sentence.toLowerCase();
  const segments: React.ReactNode[] = [];
  let cursor = 0;
  let matchIndex = lowerSentence.indexOf(normalizedTarget, cursor);

  while (matchIndex >= 0) {
    if (matchIndex > cursor) {
      segments.push(sentence.slice(cursor, matchIndex));
    }
    segments.push(
      <Text
        key={`${matchIndex}-${sentence.slice(matchIndex, matchIndex + normalizedTarget.length)}`}
        style={highlightStyle}>
        {sentence.slice(matchIndex, matchIndex + normalizedTarget.length)}
      </Text>,
    );
    cursor = matchIndex + normalizedTarget.length;
    matchIndex = lowerSentence.indexOf(normalizedTarget, cursor);
  }

  if (segments.length === 0) {
    return <Text style={textStyle}>{sentence}</Text>;
  }

  if (cursor < sentence.length) {
    segments.push(sentence.slice(cursor));
  }

  return <Text style={textStyle}>{segments}</Text>;
}

const styles = StyleSheet.create({
  keyboardContainer: {flex: 1},
  container: {flex: 1, backgroundColor: '#fff'},
  progressBar: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 16,
    backgroundColor: '#f5f5f5',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  progressFill: {
    flex: 1,
    height: 6,
    backgroundColor: '#e0e0e0',
    borderRadius: 3,
    marginRight: 12,
  },
  progressIndicator: {
    height: '100%',
    backgroundColor: '#007AFF',
    borderRadius: 3,
  },
  progressText: {fontSize: 14, color: '#666', minWidth: 50, textAlign: 'right'},
  scroll: {padding: 20, paddingBottom: 12},
  scrollCompact: {paddingTop: 12, paddingBottom: 4},
  header: {
    alignItems: 'center',
    marginBottom: 16,
    paddingVertical: 10,
    backgroundColor: '#f8f8f8',
    borderRadius: 12,
    minHeight: 104,
    justifyContent: 'center',
  },
  headerCompact: {
    minHeight: 72,
    marginBottom: 10,
    paddingVertical: 4,
  },
  metaRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
    marginTop: 2,
  },
  word: {fontSize: 30, fontWeight: 'bold', color: '#333', marginBottom: 6},
  wordCompact: {fontSize: 26, marginBottom: 0},
  phonetic: {fontSize: 16, color: '#666', fontStyle: 'italic'},
  partOfSpeech: {
    fontSize: 13,
    color: '#3B6BFF',
    fontWeight: '700',
    textTransform: 'uppercase',
    backgroundColor: '#EEF4FF',
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 999,
  },
  promptSection: {marginBottom: 10},
  promptSectionCompact: {marginBottom: 6},
  promptLabel: {fontSize: 14, color: '#999', marginBottom: 6},
  prompt: {fontSize: 17, color: '#333', lineHeight: 24},
  exampleBox: {
    backgroundColor: '#f0f8ff',
    borderRadius: 8,
    padding: 16,
    marginBottom: 16,
  },
  exampleBoxCompact: {
    paddingHorizontal: 12,
    paddingVertical: 10,
    marginBottom: 10,
  },
  exampleEn: {fontSize: 15, color: '#333', marginBottom: 6, fontStyle: 'italic'},
  exampleHighlight: {
    color: '#007AFF',
    fontWeight: '700',
    textDecorationLine: 'underline',
  },
  exampleCn: {fontSize: 13, color: '#666', lineHeight: 18},
  rootExampleRow: {marginBottom: 10},
  rootExampleWord: {
    fontSize: 18,
    color: '#333',
    fontStyle: 'italic',
    marginBottom: 4,
  },
  rootExampleWordCompact: {
    fontSize: 15,
    marginBottom: 2,
  },
  rootGloss: {fontSize: 15, color: '#666', lineHeight: 20},
  rootGlossCompact: {fontSize: 13, lineHeight: 17},
  rootHighlight: {
    color: '#007AFF',
    fontWeight: '700',
    textDecorationLine: 'underline',
  },
  choicesContainer: {gap: 12, marginBottom: 24},
  choiceButton: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 16,
    borderRadius: 12,
    borderWidth: 2,
    borderColor: '#e0e0e0',
    backgroundColor: '#fff',
  },
  choiceButtonSelected: {borderColor: '#007AFF', backgroundColor: '#f0f8ff'},
  choiceLabel: {
    fontSize: 18,
    fontWeight: 'bold',
    color: '#666',
    marginRight: 12,
    minWidth: 28,
  },
  choiceLabelSelected: {color: '#007AFF'},
  choiceText: {flex: 1, fontSize: 16, color: '#333'},
  choiceTextSelected: {color: '#007AFF'},
  textInput: {
    borderWidth: 2,
    borderColor: '#e0e0e0',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 14,
    fontSize: 18,
    marginBottom: 10,
    backgroundColor: '#f8f8f8',
  },
  textInputCompact: {
    paddingVertical: 10,
    fontSize: 16,
    marginBottom: 4,
  },
  inlineActions: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 12,
    marginBottom: 8,
  },
  inlinePrimaryButton: {
    flex: 1,
    minHeight: 44,
    borderRadius: 12,
    backgroundColor: '#007AFF',
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 12,
  },
  inlinePrimaryButtonText: {fontSize: 16, fontWeight: '600', color: '#fff'},
  inlineSkipButton: {
    minHeight: 44,
    borderRadius: 12,
    backgroundColor: '#f0f4ff',
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 14,
  },
  inlineSkipText: {fontSize: 1, color: 'transparent', position: 'absolute'},
  inlineSkipOverlay: {fontSize: 15, color: '#3B6BFF', fontWeight: '600'},
  compactActions: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 10,
    marginTop: 6,
    marginBottom: 6,
  },
  compactActionPrimary: {
    flex: 1,
    minHeight: 42,
    borderRadius: 12,
    backgroundColor: '#007AFF',
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 12,
  },
  compactActionPrimaryText: {fontSize: 15, fontWeight: '600', color: '#fff'},
  compactActionGhost: {
    minHeight: 42,
    borderRadius: 12,
    backgroundColor: '#f0f4ff',
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 14,
  },
  compactActionGhostText: {fontSize: 14, color: '#3B6BFF', fontWeight: '600'},
  inlineCancelButton: {
    minHeight: 44,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 8,
  },
  inlineCancelText: {fontSize: 15, color: '#999'},
  feedbackSection: {alignItems: 'center', paddingVertical: 4},
  feedbackBadge: {
    paddingHorizontal: 24,
    paddingVertical: 12,
    borderRadius: 24,
    marginBottom: 14,
  },
  feedbackCorrect: {backgroundColor: '#34C759'},
  feedbackFuzzy: {backgroundColor: '#FF9500'},
  feedbackIncorrect: {backgroundColor: '#FF3B30'},
  feedbackSkipped: {backgroundColor: '#999'},
  feedbackText: {color: '#fff', fontSize: 18, fontWeight: '600'},
  correctAnswerBox: {
    backgroundColor: '#f0f8ff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    width: '100%',
  },
  correctAnswerLabel: {fontSize: 14, color: '#666', marginBottom: 8},
  correctAnswerText: {fontSize: 18, fontWeight: '600', color: '#007AFF'},
  bottomBar: {
    paddingHorizontal: 20,
    paddingTop: 12,
    paddingBottom: 20,
    borderTopWidth: 1,
    borderTopColor: '#f0f0f0',
    backgroundColor: '#fff',
    gap: 12,
  },
  primaryButton: {
    backgroundColor: '#007AFF',
    borderRadius: 12,
    paddingVertical: 16,
    alignItems: 'center',
  },
  primaryButtonDisabled: {backgroundColor: '#ccc'},
  primaryButtonText: {color: '#fff', fontSize: 18, fontWeight: '600'},
  secondaryButton: {alignItems: 'center', paddingVertical: 8},
  secondaryButtonText: {color: '#999', fontSize: 16},
});
