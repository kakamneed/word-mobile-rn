import React, {useCallback, useEffect, useRef, useState} from 'react';
import {
  ActivityIndicator,
  Alert,
  SafeAreaView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import {
  cancelStudySession,
  completeStudySession,
  startStudySession,
  submitStudyAnswer,
  type SessionMode,
  type SessionSummary,
  type StartSessionRequest,
  type StudyQuestion,
  type StudyResult,
  type StudySession,
} from '../../lib/study-client';
import {QuestionCard} from './question-card';
import {StudySummaryScreen} from './study-summary-screen';

type StudyState =
  | {type: 'loading'}
  | {
      type: 'question';
      session: StudySession;
      question: StudyQuestion;
      progress: {current: number; total: number};
    }
  | {
      type: 'feedback';
      session: StudySession;
      currentQuestion: StudyQuestion;
      result: StudyResult;
      nextQuestion: StudyQuestion | null;
      summary: SessionSummary | null;
      nextAction: string | null;
      progress: {current: number; total: number};
    }
  | {
      type: 'complete';
      summary: SessionSummary;
      nextAction: string;
      mode: SessionMode;
    }
  | {type: 'error'; message: string};

interface StudySessionScreenProps {
  mode: SessionMode;
  wordbookId?: number | null;
  entrySourceIds?: string[];
  onComplete: () => void;
  onCancel: () => void;
}

export function StudySessionScreen({
  mode,
  wordbookId,
  entrySourceIds,
  onComplete,
  onCancel,
}: StudySessionScreenProps): React.JSX.Element {
  const [studyState, setStudyState] = useState<StudyState>({type: 'loading'});
  const latestStudyStateRef = useRef<StudyState>({type: 'loading'});

  useEffect(() => {
    latestStudyStateRef.current = studyState;
  }, [studyState]);

  const initializeSession = useCallback(
    async (modeOverride: SessionMode) => {
      try {
        setStudyState({type: 'loading'});
        const request: StartSessionRequest = {
          mode: modeOverride,
          wordbookId: wordbookId ?? null,
          entrySourceIds: entrySourceIds ?? [],
        };
        const response = await startStudySession(request);
        setStudyState({
          type: 'question',
          session: response.session,
          question: response.currentQuestion,
          progress: response.progress,
        });
      } catch (error) {
        setStudyState({
          type: 'error',
          message: error instanceof Error ? error.message : '无法启动学习会话。',
        });
      }
    },
    [entrySourceIds, wordbookId],
  );

  useEffect(() => {
    void initializeSession(mode);
  }, [initializeSession, mode]);

  const handleSubmitAnswer = useCallback(
    async (response: string, responseTimeMs: number) => {
      if (studyState.type !== 'question') {
        return;
      }

      try {
        const result = await submitStudyAnswer({
          questionId: studyState.question.questionId,
          response,
          responseTimeMs,
        });

        setStudyState({
          type: 'feedback',
          session: studyState.session,
          currentQuestion: studyState.question,
          result: result.result,
          nextQuestion: result.currentQuestion,
          summary: result.summary,
          nextAction: result.nextAction,
          progress: result.progress,
        });
      } catch {
        Alert.alert('提交失败', '当前答案提交失败，请重试。');
      }
    },
    [studyState],
  );

  const handleNext = useCallback(() => {
    if (studyState.type !== 'feedback') {
      return;
    }

    if (studyState.summary) {
      setStudyState({
        type: 'complete',
        summary: studyState.summary,
        nextAction: studyState.nextAction ?? '返回今日页',
        mode: studyState.session.mode,
      });
      return;
    }

    if (!studyState.nextQuestion) {
      return;
    }

    setStudyState({
      type: 'question',
      session: studyState.session,
      question: studyState.nextQuestion,
      progress: studyState.progress,
    });
  }, [studyState]);

  const handleComplete = useCallback(async () => {
    if (studyState.type !== 'complete') {
      return;
    }

    try {
      await completeStudySession(studyState.summary.sessionId);
      onComplete();
    } catch {
      Alert.alert('保存失败', '学习结果保存失败，请稍后重试。');
    }
  }, [onComplete, studyState]);

  const handleContinueNextRound = useCallback(async () => {
    if (studyState.type !== 'complete') {
      return;
    }

    const nextMode = nextModeFrom(studyState.mode);
    if (!nextMode) {
      await handleComplete();
      return;
    }

    try {
      await completeStudySession(studyState.summary.sessionId);
      await initializeSession(nextMode);
    } catch {
      Alert.alert('切换失败', '下一轮学习准备失败，请稍后重试。');
    }
  }, [handleComplete, initializeSession, studyState]);

  const handleExitToToday = useCallback(async () => {
    onCancel();
  }, [onCancel]);

  const handleAbandonSession = useCallback(async () => {
    const current = latestStudyStateRef.current;
    if (current.type === 'question' || current.type === 'feedback') {
      await cancelStudySession(current.session.sessionId);
    }
    onCancel();
  }, [onCancel]);

  const handleCancel = useCallback(() => {
    Alert.alert(
      '退出本项学习？',
      '当前进度会被保留，并同步到首页。返回首页后可继续其他学习。',
      [
        {text: '继续学习', style: 'cancel'},
        {
          text: '返回今日页',
          onPress: () => {
            void handleExitToToday();
          },
        },
        {
          text: '结束本轮',
          style: 'destructive',
          onPress: () => {
            void handleAbandonSession();
          },
        },
      ],
    );
  }, [handleAbandonSession, handleExitToToday]);

  const nextModeFrom = (currentMode: SessionMode): SessionMode | null => {
    switch (currentMode) {
      case 'newWord':
        return 'review';
      case 'review':
        return 'mixedTest';
      case 'mixedTest':
        return 'wrongWordReinforcement';
      case 'wrongWordReinforcement':
        return 'rootAffix';
      case 'rootAffix':
        return null;
      default:
        return null;
    }
  };

  switch (studyState.type) {
    case 'loading':
      return (
        <SafeAreaView style={styles.container}>
          <View style={styles.center}>
            <ActivityIndicator size="large" color="#007AFF" />
            <Text style={styles.loadingText}>正在准备学习内容…</Text>
          </View>
        </SafeAreaView>
      );
    case 'error':
      return (
        <SafeAreaView style={styles.container}>
          <View style={styles.center}>
            <Text style={styles.errorText}>{studyState.message}</Text>
          </View>
        </SafeAreaView>
      );
    case 'question':
      return (
        <SafeAreaView style={styles.container}>
          <QuestionCard
            question={studyState.question}
            progress={studyState.progress}
            onSubmit={handleSubmitAnswer}
            onCancel={handleCancel}
          />
        </SafeAreaView>
      );
    case 'feedback':
      return (
        <SafeAreaView style={styles.container}>
          <QuestionCard
            question={studyState.currentQuestion}
            progress={studyState.progress}
            result={studyState.result}
            onNext={handleNext}
            showFeedback
            nextButtonLabel={studyState.summary ? '查看结果' : '下一题'}
          />
        </SafeAreaView>
      );
    case 'complete':
      return (
        <StudySummaryScreen
          summary={studyState.summary}
          nextAction={studyState.nextAction}
          onReturnToToday={handleComplete}
          onContinueNextRound={() => {
            void handleContinueNextRound();
          }}
        />
      );
  }
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#f5f5f5'},
  center: {flex: 1, justifyContent: 'center', alignItems: 'center'},
  loadingText: {marginTop: 16, fontSize: 16, color: '#666'},
  errorText: {fontSize: 16, color: '#D32F2F', textAlign: 'center', padding: 24},
});
