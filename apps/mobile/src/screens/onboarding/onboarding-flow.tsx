import React, {useState} from 'react';
import {View, Text, StyleSheet, TouchableOpacity, SafeAreaView, ScrollView} from 'react-native';

interface OnboardingFlowProps {
  onComplete: () => void;
}

type OnboardingStep = 'welcome' | 'vocabulary' | 'plan' | 'complete';

export function OnboardingFlow({onComplete}: OnboardingFlowProps): React.JSX.Element {
  const [currentStep, setCurrentStep] = useState<OnboardingStep>('welcome');
  const [selectedWordbooks, setSelectedWordbooks] = useState<string[]>(['cet4']);
  const [dailyTarget, setDailyTarget] = useState(20);

  const handleNext = () => {
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
        onComplete();
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

  const canProceed = currentStep !== 'vocabulary' || selectedWordbooks.length > 0;

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.progressBar}>
        {(['welcome', 'vocabulary', 'plan', 'complete'] as OnboardingStep[]).map((step, index) => (
          <View
            key={step}
            style={[styles.progressDot, getStepIndex(currentStep) >= index && styles.progressDotActive]}
          />
        ))}
      </View>

      <ScrollView contentContainerStyle={styles.scroll}>
        {currentStep === 'welcome' ? <WelcomeStep /> : null}
        {currentStep === 'vocabulary' ? (
          <VocabularyStep
            selected={selectedWordbooks}
            onToggle={id => {
              setSelectedWordbooks(prev =>
                prev.includes(id) ? prev.filter(w => w !== id) : [...prev, id],
              );
            }}
          />
        ) : null}
        {currentStep === 'plan' ? (
          <PlanStep target={dailyTarget} onChange={setDailyTarget} />
        ) : null}
        {currentStep === 'complete' ? <CompleteStep onEnter={handleNext} /> : null}
      </ScrollView>

      {currentStep !== 'welcome' && currentStep !== 'complete' ? (
        <View style={styles.navButtons}>
          <TouchableOpacity style={styles.backButton} onPress={handleBack}>
            <Text style={styles.backButtonText}>上一步</Text>
          </TouchableOpacity>
          <TouchableOpacity
            style={[styles.nextButton, !canProceed && styles.nextButtonDisabled]}
            onPress={handleNext}
            disabled={!canProceed}>
            <Text style={styles.nextButtonText}>{currentStep === 'plan' ? '完成设置' : '下一步'}</Text>
          </TouchableOpacity>
        </View>
      ) : null}

      {currentStep === 'welcome' ? (
        <View style={styles.singleButton}>
          <TouchableOpacity style={styles.nextButton} onPress={handleNext}>
            <Text style={styles.nextButtonText}>开始设置</Text>
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
      <Text style={styles.icon}>词</Text>
      <Text style={styles.title}>欢迎使用 Word Mobile</Text>
      <Text style={styles.description}>
        通过每日学习计划、复习、测试和错词回顾，逐步建立稳定的词汇学习节奏。
      </Text>
    </View>
  );
}

function VocabularyStep({
  selected,
  onToggle,
}: {
  selected: string[];
  onToggle: (id: string) => void;
}): React.JSX.Element {
  const wordbooks = [
    {id: 'cet4', name: 'CET-4 核心词汇', count: 4500},
    {id: 'cet6', name: 'CET-6 核心词汇', count: 3000},
    {id: 'kaoyan', name: '考研核心词汇', count: 5500},
    {id: 'medical_resp', name: '医学英语（呼吸专题）', count: 1800},
  ];

  return (
    <View style={styles.stepContainer}>
      <Text style={styles.stepTitle}>选择词书</Text>
      <Text style={styles.stepDescription}>可多选，后续也可以在词书页继续调整。</Text>

      {wordbooks.map(book => (
        <TouchableOpacity
          key={book.id}
          style={[styles.wordbookCard, selected.includes(book.id) && styles.wordbookCardSelected]}
          onPress={() => onToggle(book.id)}>
          <View style={styles.wordbookInfo}>
            <Text style={styles.wordbookName}>{book.name}</Text>
            <Text style={styles.wordbookCount}>{book.count} 词</Text>
          </View>
          <Text style={styles.checkmark}>{selected.includes(book.id) ? '已选' : ''}</Text>
        </TouchableOpacity>
      ))}

      <View style={styles.tipBox}>
        <Text style={styles.tipText}>词根词缀模式会在学习页中作为独立学习模式显示。</Text>
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
      <Text style={styles.stepTitle}>设置每日目标</Text>
      <Text style={styles.stepDescription}>你希望每天学习多少个新词？</Text>

      <View style={styles.targetSelector}>
        <TouchableOpacity style={styles.targetButton} onPress={() => onChange(Math.max(5, target - 5))}>
          <Text style={styles.targetButtonText}>-</Text>
        </TouchableOpacity>

        <View style={styles.targetDisplay}>
          <Text style={styles.targetNumber}>{target}</Text>
          <Text style={styles.targetLabel}>个 / 天</Text>
        </View>

        <TouchableOpacity style={styles.targetButton} onPress={() => onChange(Math.min(100, target + 5))}>
          <Text style={styles.targetButtonText}>+</Text>
        </TouchableOpacity>
      </View>

      <Text style={styles.targetHint}>后续可以在计划页中继续调整更完整的学习参数。</Text>
    </View>
  );
}

function CompleteStep({onEnter}: {onEnter: () => void}): React.JSX.Element {
  return (
    <View style={styles.stepContainer}>
      <Text style={styles.icon}>好</Text>
      <Text style={styles.title}>设置完成</Text>
      <Text style={styles.description}>你的学习计划已经准备好，现在可以开始今天的学习。</Text>

      <TouchableOpacity style={styles.enterButton} onPress={onEnter}>
        <Text style={styles.enterButtonText}>进入学习</Text>
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
