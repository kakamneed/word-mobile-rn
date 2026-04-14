import React, {useEffect, useState} from 'react';
import {
  ActivityIndicator,
  Alert,
  RefreshControl,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import type {SessionMode} from '../../lib/mobile-bridge';
import {
  fetchLatestPassageForDate,
  fetchTodayPassageContext,
  generateTodayPassage,
  type AIPassage,
  type TodayAiPassageContext,
} from '../../lib/ai-passage-client';
import {
  fetchToday,
  getPrimaryAction,
  type TodayHomeState,
} from '../../lib/today-client';
import {CarryoverSection} from './carryover-section';
import {NextActionCard} from './next-action-card';
import {TaskBreakdownSection} from './task-breakdown-section';

interface TodayScreenProps {
  onNavigateToPlan?: () => void;
  onNavigateToAi?: () => void;
  onStartStudy?: (mode: SessionMode) => void;
}

export function TodayScreen({
  onNavigateToPlan,
  onNavigateToAi,
  onStartStudy,
}: TodayScreenProps): React.JSX.Element {
  const [state, setState] = useState<TodayHomeState | null>(null);
  const [aiContext, setAiContext] = useState<TodayAiPassageContext | null>(null);
  const [todayPassage, setTodayPassage] = useState<AIPassage | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [generatingAi, setGeneratingAi] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadToday = async () => {
    try {
      setError(null);
      const todayData = await fetchToday();
      const nextAiContext = await fetchTodayPassageContext();
      const latestTodayPassage = await fetchLatestPassageForDate(todayData.todayDate);
      setState(todayData);
      setAiContext(nextAiContext);
      setTodayPassage(latestTodayPassage);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载失败');
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void loadToday();
  }, []);

  const handleRefresh = () => {
    setRefreshing(true);
    void loadToday();
  };

  const handleGenerateAi = async () => {
    setGeneratingAi(true);
    try {
      const passage = await generateTodayPassage('intermediate');
      setTodayPassage(passage);
      onNavigateToAi?.();
    } catch (err) {
      Alert.alert(
        '生成失败',
        err instanceof Error ? err.message : 'AI 短文生成失败',
      );
    } finally {
      setGeneratingAi(false);
    }
  };

  if (loading) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.center}>
          <ActivityIndicator size="large" color="#007AFF" />
        </View>
      </SafeAreaView>
    );
  }

  if (error) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.center}>
          <Text style={styles.errorText}>{error}</Text>
          <TouchableOpacity style={styles.retryButton} onPress={() => void loadToday()}>
            <Text style={styles.retryText}>重试</Text>
          </TouchableOpacity>
        </View>
      </SafeAreaView>
    );
  }

  if (!state) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.center}>
          <Text>暂无数据</Text>
        </View>
      </SafeAreaView>
    );
  }

  const nextAction = getPrimaryAction(state.todaySnapshot);

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.date}>{state.todayDate}</Text>
        <TouchableOpacity onPress={onNavigateToPlan}>
          <Text style={styles.planLink}>计划 {'>'}</Text>
        </TouchableOpacity>
      </View>

      <ScrollView
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={handleRefresh} />}
        contentContainerStyle={styles.scroll}>
        <NextActionCard
          action={nextAction}
          onStart={() => nextAction.type !== 'done' && onStartStudy?.(nextAction.type)}
        />

        {state.todaySnapshot ? (
          <TaskBreakdownSection snapshot={state.todaySnapshot} onStartStudy={onStartStudy} />
        ) : null}

        {aiContext?.tasksComplete ? (
          <View style={styles.aiCard}>
            <View style={styles.aiCardHeader}>
              <Text style={styles.aiTitle}>AI 短文</Text>
              <Text style={styles.aiSubtext}>
                {todayPassage
                  ? '今天已经生成过短文，这里直接展示最新一篇。'
                  : '今日任务已完成，可基于今天的错词自动生成一篇短文。'}
              </Text>
            </View>
            <Text style={styles.aiMeta}>今日错词 {aiContext.wrongWords.length} 个</Text>

            {todayPassage ? (
              <TouchableOpacity style={styles.aiPreviewCard} onPress={onNavigateToAi}>
                <Text style={styles.aiPreviewTitle}>{todayPassage.title}</Text>
                <Text style={styles.aiPreviewText}>{todayPassage.preview}</Text>
                <Text style={styles.aiPreviewLink}>进入 AI 页查看全文</Text>
              </TouchableOpacity>
            ) : (
              <TouchableOpacity
                style={[styles.aiButton, generatingAi && styles.aiButtonDisabled]}
                onPress={() => void handleGenerateAi()}
                disabled={generatingAi}>
                <Text style={styles.aiButtonText}>
                  {generatingAi ? '生成中...' : '自动生成 AI 短文'}
                </Text>
              </TouchableOpacity>
            )}
          </View>
        ) : null}

        <CarryoverSection
          activePlan={state.activePlan}
          wordbooks={state.wordbooks}
          onNavigateToPlan={onNavigateToPlan}
        />
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#f5f5f5'},
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: 20,
    paddingVertical: 16,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  date: {fontSize: 18, fontWeight: '600', color: '#333'},
  planLink: {fontSize: 16, color: '#007AFF'},
  scroll: {padding: 16, gap: 16},
  center: {flex: 1, justifyContent: 'center', alignItems: 'center'},
  errorText: {color: '#D32F2F', marginBottom: 16},
  retryButton: {
    backgroundColor: '#007AFF',
    paddingHorizontal: 24,
    paddingVertical: 12,
    borderRadius: 8,
  },
  retryText: {color: '#fff', fontWeight: '600'},
  aiCard: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 16,
  },
  aiCardHeader: {marginBottom: 10},
  aiTitle: {fontSize: 18, fontWeight: '700', color: '#333', marginBottom: 4},
  aiSubtext: {fontSize: 14, color: '#666', lineHeight: 20},
  aiMeta: {fontSize: 13, color: '#666', marginBottom: 12},
  aiButton: {
    backgroundColor: '#007AFF',
    borderRadius: 12,
    paddingVertical: 14,
    alignItems: 'center',
  },
  aiButtonDisabled: {opacity: 0.6},
  aiButtonText: {fontSize: 16, fontWeight: '600', color: '#fff'},
  aiPreviewCard: {
    backgroundColor: '#F4F9FF',
    borderRadius: 12,
    padding: 14,
  },
  aiPreviewTitle: {fontSize: 16, fontWeight: '600', color: '#333', marginBottom: 8},
  aiPreviewText: {fontSize: 14, color: '#555', lineHeight: 22, marginBottom: 8},
  aiPreviewLink: {fontSize: 13, color: '#007AFF', fontWeight: '600'},
});
