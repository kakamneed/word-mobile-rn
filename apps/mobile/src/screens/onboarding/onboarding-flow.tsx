import React, {useEffect, useState} from 'react';
import {
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import {markOnboardingCompleted} from '../../lib/mobile-bridge';
import {
  CROC_BTI_QUESTIONS,
  evaluateCrocBti,
  type CrocBtiAnswer,
  type CrocBtiAnswerValue,
  type CrocBtiResult,
} from '../../lib/croc-bti';
import {applyPlanToToday, fetchActivePlan, savePlan} from '../../lib/plan-client';
import {fetchWordbooks, toggleWordbook} from '../../lib/vocabulary-client';

interface OnboardingFlowProps {
  onComplete: () => void;
}

type OnboardingStep = 'welcome' | 'vocabulary' | 'crocBti' | 'plan' | 'complete';

type WordbookOption = {
  id: string;
  name: string;
  count: number;
};

export function OnboardingFlow({
  onComplete,
}: OnboardingFlowProps): React.JSX.Element {
  const [currentStep, setCurrentStep] = useState<OnboardingStep>('welcome');
  const [selectedWordbooks, setSelectedWordbooks] = useState<string[]>([]);
  const [dailyTarget, setDailyTarget] = useState(20);
  const [crocBtiResult, setCrocBtiResult] = useState<CrocBtiResult | null>(null);
  const [wordbooks, setWordbooks] = useState<WordbookOption[]>([]);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    void (async () => {
      const loaded = await fetchWordbooks();
      const options = loaded.map(book => ({
        id: String(book.id),
        name: book.name,
        count: book.totalEntries,
      }));
      setWordbooks(options);
      const active = loaded.filter(book => book.isActive).map(book => String(book.id));
      setSelectedWordbooks(active.length > 0 ? active : options.slice(0, 1).map(book => book.id));
    })();
  }, []);

  const handleNext = async () => {
    switch (currentStep) {
      case 'welcome':
        setCurrentStep('vocabulary');
        break;
      case 'vocabulary':
        setCurrentStep('crocBti');
        break;
      case 'crocBti':
        setCurrentStep('plan');
        break;
      case 'plan':
        setCurrentStep('complete');
        break;
      case 'complete':
        await finishOnboarding();
        break;
    }
  };

  const handleBack = () => {
    switch (currentStep) {
      case 'vocabulary':
        setCurrentStep('welcome');
        break;
      case 'plan':
        setCurrentStep('crocBti');
        break;
      case 'crocBti':
        setCurrentStep('vocabulary');
        break;
      case 'complete':
        setCurrentStep('plan');
        break;
    }
  };

  const finishOnboarding = async () => {
    setSaving(true);
    try {
      const currentPlan = await fetchActivePlan();
      const planId = currentPlan?.id ?? 1;
      await savePlan(planId, {
        name: crocBtiResult
          ? `${currentPlan?.name ?? 'Starter Plan'} - ${crocBtiResult.title}`
          : currentPlan?.name ?? 'Starter Plan',
        ...(crocBtiResult
          ? crocBtiResult.planInput
          : {
              newWordsPerDay: dailyTarget,
              reviewWordsPerDay: Math.max(dailyTarget * 2, 10),
              mixedTestPerDay: Math.max(Math.round(dailyTarget * 0.4), 4),
              wrongWordTestPerDay: Math.max(Math.round(dailyTarget * 0.2), 2),
              rootAffixPerDay: 2,
            }),
      });
      for (const wordbook of wordbooks) {
        await toggleWordbook(
          parseInt(wordbook.id, 10),
          selectedWordbooks.includes(wordbook.id),
        );
      }
      await applyPlanToToday();
      await markOnboardingCompleted();
      onComplete();
    } finally {
      setSaving(false);
    }
  };

  const canProceed = currentStep !== 'vocabulary' || selectedWordbooks.length > 0;

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.progressBar}>
        {(['welcome', 'vocabulary', 'crocBti', 'plan', 'complete'] as OnboardingStep[]).map(
          (step, index) => (
            <View
              key={step}
              style={[
                styles.progressDot,
                getStepIndex(currentStep) >= index && styles.progressDotActive,
              ]}
            />
          ),
        )}
      </View>

      <ScrollView contentContainerStyle={styles.scroll}>
        {currentStep === 'welcome' ? <WelcomeStep /> : null}
        {currentStep === 'vocabulary' ? (
          <VocabularyStep
            wordbooks={wordbooks}
            selected={selectedWordbooks}
            onToggle={id => {
              setSelectedWordbooks(prev =>
                prev.includes(id) ? prev.filter(item => item !== id) : [...prev, id],
              );
            }}
          />
        ) : null}
        {currentStep === 'plan' ? (
          <PlanStep
            target={dailyTarget}
            onChange={setDailyTarget}
            crocBtiResult={crocBtiResult}
          />
        ) : null}
        {currentStep === 'crocBti' ? (
          <OnboardingCrocBtiStep
            result={crocBtiResult}
            onComplete={result => {
              setCrocBtiResult(result);
              setCurrentStep('plan');
            }}
            onSkip={() => {
              setCrocBtiResult(null);
              setCurrentStep('plan');
            }}
          />
        ) : null}
        {currentStep === 'complete' ? (
          <CompleteStep onEnter={() => void handleNext()} saving={saving} />
        ) : null}
      </ScrollView>

      {currentStep !== 'welcome' && currentStep !== 'complete' ? (
        <View style={styles.navButtons}>
          <TouchableOpacity style={styles.backButton} onPress={handleBack}>
            <Text style={styles.backButtonText}>Back</Text>
          </TouchableOpacity>
          <TouchableOpacity
            style={[styles.nextButton, !canProceed && styles.nextButtonDisabled]}
            onPress={() => void handleNext()}
            disabled={!canProceed}>
            <Text style={styles.nextButtonText}>
              {currentStep === 'plan' ? 'Finish Setup' : 'Next'}
            </Text>
          </TouchableOpacity>
        </View>
      ) : null}

      {currentStep === 'welcome' ? (
        <View style={styles.singleButton}>
          <TouchableOpacity style={styles.nextButton} onPress={() => void handleNext()}>
            <Text style={styles.nextButtonText}>Start Setup</Text>
          </TouchableOpacity>
        </View>
      ) : null}
    </SafeAreaView>
  );
}

function getStepIndex(step: OnboardingStep): number {
  return ['welcome', 'vocabulary', 'crocBti', 'plan', 'complete'].indexOf(step);
}

function WelcomeStep(): React.JSX.Element {
  return (
    <View style={styles.stepContainer}>
      <Text style={styles.icon}>Word</Text>
      <Text style={styles.title}>Welcome to Word Mobile</Text>
      <Text style={styles.description}>
        Choose your wordbooks and daily target first. When setup is done,
        Today will start from your real study state right away.
      </Text>
    </View>
  );
}

function VocabularyStep({
  wordbooks,
  selected,
  onToggle,
}: {
  wordbooks: WordbookOption[];
  selected: string[];
  onToggle: (id: string) => void;
}): React.JSX.Element {
  return (
    <View style={styles.stepContainer}>
      <Text style={styles.stepTitle}>Choose Wordbooks</Text>
      <Text style={styles.stepDescription}>You can select multiple wordbooks and adjust them later from Plan.</Text>

      {wordbooks.map(book => (
        <TouchableOpacity
          key={book.id}
          style={[
            styles.wordbookCard,
            selected.includes(book.id) && styles.wordbookCardSelected,
          ]}
          onPress={() => onToggle(book.id)}>
          <View style={styles.wordbookInfo}>
            <Text style={styles.wordbookName}>{book.name}</Text>
            <Text style={styles.wordbookCount}>{book.count} words</Text>
          </View>
          <Text style={styles.checkmark}>{selected.includes(book.id) ? 'Selected' : ''}</Text>
        </TouchableOpacity>
      ))}

      <View style={styles.tipBox}>
        <Text style={styles.tipText}>
          The selected wordbooks and starter plan will be written into real app state, not a demo-only flow.
        </Text>
      </View>
    </View>
  );
}

function PlanStep({
  target,
  onChange,
  crocBtiResult,
}: {
  target: number;
  onChange: (n: number) => void;
  crocBtiResult: CrocBtiResult | null;
}): React.JSX.Element {
  if (crocBtiResult) {
    return (
      <View style={styles.stepContainer}>
        <Text style={styles.stepTitle}>鳄bti 已接管初始计划</Text>
        <Text style={styles.stepDescription}>
          你的类型是 {crocBtiResult.title}。初始计划会按测试结果分配新词、复习、混测和错题权重。
        </Text>

        <View style={styles.btiResultBox}>
          <Text style={styles.btiResultCode}>{crocBtiResult.code}</Text>
          <Text style={styles.btiResultTitle}>{crocBtiResult.title}</Text>
          <Text style={styles.btiResultText}>{crocBtiResult.advice}</Text>
        </View>
      </View>
    );
  }

  return (
    <View style={styles.stepContainer}>
      <Text style={styles.stepTitle}>Set Daily Target</Text>
      <Text style={styles.stepDescription}>How many new words do you want to learn each day?</Text>

      <View style={styles.targetSelector}>
        <TouchableOpacity
          style={styles.targetButton}
          onPress={() => onChange(Math.max(5, target - 5))}>
          <Text style={styles.targetButtonText}>-</Text>
        </TouchableOpacity>

        <View style={styles.targetDisplay}>
          <Text style={styles.targetNumber}>{target}</Text>
          <Text style={styles.targetLabel}>words / day</Text>
        </View>

        <TouchableOpacity
          style={styles.targetButton}
          onPress={() => onChange(Math.min(100, target + 5))}>
          <Text style={styles.targetButtonText}>+</Text>
        </TouchableOpacity>
      </View>

      <Text style={styles.targetHint}>
        The app will derive review, mixed-test, wrong-word, and root-affix starter targets from this baseline.
      </Text>
    </View>
  );
}

function OnboardingCrocBtiStep({
  result,
  onComplete,
  onSkip,
}: {
  result: CrocBtiResult | null;
  onComplete: (result: CrocBtiResult) => void;
  onSkip: () => void;
}): React.JSX.Element {
  const [answerMap, setAnswerMap] = useState<Record<string, CrocBtiAnswerValue>>({});
  const answers: CrocBtiAnswer[] = Object.entries(answerMap).map(
    ([questionId, value]) => ({
      questionId,
      value,
    }),
  );
  const currentResult = evaluateCrocBti(answers);
  const answered = Object.keys(answerMap).length;
  const allAnswered = answered === CROC_BTI_QUESTIONS.length;

  return (
    <View style={styles.btiStep}>
      <Text style={styles.stepTitle}>鳄bti 学习人格</Text>
      <Text style={styles.stepDescription}>
        可跳过。完成后会用你的学习人格生成更合适的初始计划。
      </Text>
      {result ? (
        <View style={styles.btiResultBox}>
          <Text style={styles.btiResultCode}>{result.code}</Text>
          <Text style={styles.btiResultTitle}>{result.title}</Text>
          <Text style={styles.btiResultText}>{result.summary}</Text>
        </View>
      ) : null}

      {CROC_BTI_QUESTIONS.map((question, index) => (
        <View key={question.id} style={styles.btiQuestion}>
          <Text style={styles.btiQuestionText}>
            {index + 1}. {question.text}
          </Text>
          <View style={styles.btiOptions}>
            {([-2, -1, 0, 1, 2] as CrocBtiAnswerValue[]).map(value => {
              const selected = answerMap[question.id] === value;
              return (
                <TouchableOpacity
                  key={value}
                  style={[styles.btiOption, selected && styles.btiOptionSelected]}
                  onPress={() =>
                    setAnswerMap(current => ({...current, [question.id]: value}))
                  }>
                  <Text
                    style={[
                      styles.btiOptionText,
                      selected && styles.btiOptionTextSelected,
                    ]}>
                    {value === -2
                      ? '很不同意'
                      : value === -1
                        ? '不同意'
                        : value === 0
                          ? '不确定'
                          : value === 1
                            ? '同意'
                            : '很同意'}
                  </Text>
                </TouchableOpacity>
              );
            })}
          </View>
        </View>
      ))}

      <View style={styles.btiActions}>
        <TouchableOpacity style={styles.btiSkipButton} onPress={onSkip}>
          <Text style={styles.btiSkipText}>跳过</Text>
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.btiApplyButton, !allAnswered && styles.nextButtonDisabled]}
          onPress={() => onComplete(currentResult)}
          disabled={!allAnswered}>
          <Text style={styles.btiApplyText}>
            {allAnswered ? '使用鳄bti结果' : `${answered}/${CROC_BTI_QUESTIONS.length}`}
          </Text>
        </TouchableOpacity>
      </View>
    </View>
  );
}

function CompleteStep({
  onEnter,
  saving,
}: {
  onEnter: () => void;
  saving: boolean;
}): React.JSX.Element {
  return (
    <View style={styles.stepContainer}>
      <Text style={styles.icon}>OK</Text>
      <Text style={styles.title}>Setup Complete</Text>
      <Text style={styles.description}>
        Your wordbooks, plan, and Today state will be saved together, so you can start studying immediately.
      </Text>

      <TouchableOpacity
        style={[styles.enterButton, saving && styles.nextButtonDisabled]}
        onPress={onEnter}
        disabled={saving}>
        <Text style={styles.enterButtonText}>{saving ? 'Saving...' : 'Enter App'}</Text>
      </TouchableOpacity>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#fff'},
  progressBar: {flexDirection: 'row', justifyContent: 'center', paddingVertical: 16, gap: 8},
  progressDot: {width: 8, height: 8, borderRadius: 4, backgroundColor: '#e0e0e0'},
  progressDotActive: {backgroundColor: '#007AFF'},
  scroll: {flexGrow: 1},
  stepContainer: {flex: 1, padding: 24, alignItems: 'center', justifyContent: 'center'},
  icon: {fontSize: 48, marginBottom: 24, fontWeight: '700', color: '#007AFF'},
  title: {fontSize: 28, fontWeight: 'bold', textAlign: 'center', marginBottom: 16},
  description: {fontSize: 16, color: '#666', textAlign: 'center', lineHeight: 24},
  stepTitle: {fontSize: 24, fontWeight: 'bold', marginBottom: 8, alignSelf: 'flex-start'},
  stepDescription: {fontSize: 16, color: '#666', marginBottom: 24, alignSelf: 'flex-start'},
  wordbookCard: {
    flexDirection: 'row',
    alignItems: 'center',
    width: '100%',
    padding: 16,
    borderRadius: 12,
    borderWidth: 2,
    borderColor: '#e0e0e0',
    marginBottom: 12,
  },
  wordbookCardSelected: {borderColor: '#007AFF', backgroundColor: '#f0f8ff'},
  wordbookInfo: {flex: 1},
  wordbookName: {fontSize: 16, fontWeight: '600', marginBottom: 4},
  wordbookCount: {fontSize: 14, color: '#999'},
  checkmark: {fontSize: 16, color: '#007AFF', fontWeight: '700', width: 36, textAlign: 'right'},
  tipBox: {marginTop: 8, padding: 12, borderRadius: 8, backgroundColor: '#f8f8f8', width: '100%'},
  tipText: {fontSize: 13, color: '#666', lineHeight: 18},
  targetSelector: {flexDirection: 'row', alignItems: 'center', marginVertical: 32},
  targetButton: {
    width: 48,
    height: 48,
    borderRadius: 24,
    backgroundColor: '#f0f0f0',
    justifyContent: 'center',
    alignItems: 'center',
  },
  targetButtonText: {fontSize: 24, fontWeight: 'bold', color: '#007AFF'},
  targetDisplay: {alignItems: 'center', marginHorizontal: 32},
  targetNumber: {fontSize: 48, fontWeight: 'bold', color: '#007AFF'},
  targetLabel: {fontSize: 14, color: '#666', marginTop: 4},
  targetHint: {fontSize: 14, color: '#999', textAlign: 'center'},
  navButtons: {flexDirection: 'row', padding: 24, gap: 12},
  backButton: {flex: 1, paddingVertical: 14, alignItems: 'center'},
  backButtonText: {fontSize: 16, color: '#666'},
  nextButton: {flex: 2, backgroundColor: '#007AFF', borderRadius: 12, paddingVertical: 14, alignItems: 'center'},
  nextButtonDisabled: {backgroundColor: '#ccc'},
  nextButtonText: {color: '#fff', fontSize: 16, fontWeight: '600'},
  singleButton: {padding: 24},
  enterButton: {backgroundColor: '#007AFF', borderRadius: 12, paddingVertical: 16, paddingHorizontal: 32, marginTop: 24},
  enterButtonText: {color: '#fff', fontSize: 18, fontWeight: '600'},
  btiStep: {padding: 24},
  btiQuestion: {
    width: '100%',
    borderWidth: 1,
    borderColor: '#e5e5e5',
    borderRadius: 12,
    padding: 14,
    marginBottom: 12,
  },
  btiQuestionText: {fontSize: 15, color: '#333', lineHeight: 22, marginBottom: 10},
  btiOptions: {gap: 8},
  btiOption: {
    borderWidth: 1,
    borderColor: '#e0e0e0',
    borderRadius: 8,
    paddingVertical: 9,
    alignItems: 'center',
  },
  btiOptionSelected: {borderColor: '#8DAA3D', backgroundColor: '#F2F7E8'},
  btiOptionText: {fontSize: 13, color: '#555'},
  btiOptionTextSelected: {fontWeight: '700', color: '#50651F'},
  btiActions: {flexDirection: 'row', gap: 12, marginTop: 8},
  btiSkipButton: {flex: 1, paddingVertical: 14, alignItems: 'center'},
  btiSkipText: {fontSize: 16, color: '#666'},
  btiApplyButton: {
    flex: 2,
    backgroundColor: '#6B7F32',
    borderRadius: 12,
    paddingVertical: 14,
    alignItems: 'center',
  },
  btiApplyText: {fontSize: 16, color: '#fff', fontWeight: '700'},
  btiResultBox: {
    width: '100%',
    backgroundColor: '#F2F7E8',
    borderRadius: 12,
    padding: 16,
    marginVertical: 16,
  },
  btiResultCode: {fontSize: 13, color: '#6B7F32', fontWeight: '800'},
  btiResultTitle: {fontSize: 24, color: '#222', fontWeight: '800', marginTop: 4},
  btiResultText: {fontSize: 14, color: '#555', lineHeight: 22, marginTop: 8},
});
