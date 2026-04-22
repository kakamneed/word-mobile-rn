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
import {applyPlanToToday, fetchActivePlan, savePlan} from '../../lib/plan-client';
import {fetchWordbooks, toggleWordbook} from '../../lib/vocabulary-client';

interface OnboardingFlowProps {
  onComplete: () => void;
}

type OnboardingStep = 'welcome' | 'vocabulary' | 'plan' | 'complete';

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
        name: currentPlan?.name ?? 'Starter Plan',
        newWordsPerDay: dailyTarget,
        reviewWordsPerDay: Math.max(dailyTarget * 2, 10),
        mixedTestPerDay: Math.max(Math.round(dailyTarget * 0.4), 4),
        wrongWordTestPerDay: Math.max(Math.round(dailyTarget * 0.2), 2),
        rootAffixPerDay: 2,
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
        {(['welcome', 'vocabulary', 'plan', 'complete'] as OnboardingStep[]).map(
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
          <PlanStep target={dailyTarget} onChange={setDailyTarget} />
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
  return ['welcome', 'vocabulary', 'plan', 'complete'].indexOf(step);
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
}: {
  target: number;
  onChange: (n: number) => void;
}): React.JSX.Element {
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
});
