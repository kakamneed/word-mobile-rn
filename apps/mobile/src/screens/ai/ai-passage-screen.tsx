import React, {useEffect, useState} from 'react';
import {
  RefreshControl,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import {
  fetchPassage,
  fetchPassageHistory,
  fetchLatestPassageForDate,
  fetchTodayPassageContext,
  type AIPassage,
  type AIPassageHistoryItem,
  type PassageBlock,
} from '../../lib/ai-passage-client';

interface AiPassageScreenProps {
  onBack?: () => void;
}

export function AiPassageScreen({
  onBack,
}: AiPassageScreenProps): React.JSX.Element {
  const [passage, setPassage] = useState<AIPassage | null>(null);
  const [history, setHistory] = useState<AIPassageHistoryItem[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [todayReady, setTodayReady] = useState(false);

  const load = async () => {
    try {
      const [nextHistory, context] = await Promise.all([
        fetchPassageHistory(),
        fetchTodayPassageContext(),
      ]);
      setHistory(nextHistory);
      setTodayReady(context.tasksComplete);
      const latestToday = await fetchLatestPassageForDate(context.date);
      if (latestToday) {
        setPassage(latestToday);
      } else if (nextHistory.length > 0) {
        const latest = await fetchPassage(nextHistory[0].passageId);
        setPassage(latest);
      } else {
        setPassage(null);
      }
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        {onBack ? (
          <TouchableOpacity onPress={onBack}>
            <Text style={styles.backButton}>返回</Text>
          </TouchableOpacity>
        ) : (
          <View style={styles.headerSpacer} />
        )}
        <Text style={styles.headerTitle}>AI 短文</Text>
        <View style={styles.headerSpacer} />
      </View>

      <ScrollView
        contentContainerStyle={styles.scroll}
        refreshControl={
          <RefreshControl
            refreshing={refreshing}
            onRefresh={() => {
              setRefreshing(true);
              void load();
            }}
          />
        }>
        <View style={styles.heroCard}>
          <Text style={styles.heroTitle}>AI 历史与阅读</Text>
          <Text style={styles.heroText}>
            AI 短文会在今日任务完成后，从今日错词自动生成。这里用于查看最近一篇短文和历史记录。
          </Text>
          {!todayReady ? (
            <Text style={styles.heroHint}>今日任务尚未完成，Today 页完成后会出现生成入口。</Text>
          ) : null}
        </View>

        {passage ? (
          <View style={styles.passageCard}>
            <Text style={styles.passageTitle}>{passage.title}</Text>
            <Text style={styles.passageMeta}>
              {passage.targetLevel} / {passage.wordCount} 词 /{' '}
              {passage.generatedAt.slice(0, 10)}
            </Text>

            {passage.failureReason ? (
              <Text style={styles.failureText}>{passage.failureReason}</Text>
            ) : null}

            <View style={styles.sourceSection}>
              <Text style={styles.sectionTitle}>输入错词</Text>
              <View style={styles.chipWrap}>
                {passage.wrongWords.map(item => (
                  <View key={`${item.entryId}-${item.word}`} style={styles.sourceChip}>
                    <Text style={styles.sourceWord}>{item.word}</Text>
                    <Text style={styles.sourceGloss}>{item.primaryGloss}</Text>
                  </View>
                ))}
              </View>
            </View>

            <View style={styles.sourceSection}>
              <Text style={styles.sectionTitle}>结构化内容</Text>
              {passage.blocks.map((block, index) => (
                <PassageBlockView key={`${block.blockType}-${index}`} block={block} />
              ))}
            </View>
          </View>
        ) : (
          <View style={styles.emptyCard}>
            <Text style={styles.emptyTitle}>还没有 AI 短文</Text>
            <Text style={styles.emptyText}>
              请先在 Today 页完成今日任务并生成短文，再来这里查看结果和历史。
            </Text>
          </View>
        )}

        <View style={styles.historyCard}>
          <Text style={styles.historyTitle}>历史记录</Text>
          {history.length === 0 ? (
            <Text style={styles.emptyText}>暂无历史记录。</Text>
          ) : (
            history.map(item => (
              <TouchableOpacity
                key={item.passageId}
                style={styles.historyRow}
                onPress={async () => {
                  const next = await fetchPassage(item.passageId);
                  setPassage(next);
                }}>
                <View style={styles.historyContent}>
                  <Text style={styles.historyItemTitle}>{item.title}</Text>
                  <Text style={styles.historyPreview}>{item.preview}</Text>
                </View>
                <View style={styles.historyMeta}>
                  <Text style={styles.historyDate}>
                    {item.generatedAt.slice(0, 10)}
                  </Text>
                  <Text style={styles.historyStatus}>{item.validationStatus}</Text>
                </View>
              </TouchableOpacity>
            ))
          )}
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

function PassageBlockView({block}: {block: PassageBlock}): React.JSX.Element {
  return (
    <View style={styles.block}>
      <Text style={styles.blockText}>
        {block.segments.map((segment: PassageBlock['segments'][number], index: number) =>
          segment.type === 'word' ? (
            <Text key={`${segment.text}-${index}`} style={styles.wordSegment}>
              {segment.text}
              {segment.glossZh ? (
                <Text style={styles.wordGloss}>（{segment.glossZh}）</Text>
              ) : null}
            </Text>
          ) : (
            <Text key={`${segment.text}-${index}`}>{segment.text}</Text>
          ),
        )}
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#f5f5f5'},
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    backgroundColor: '#fff',
    paddingHorizontal: 16,
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: '#e5e5e5',
  },
  backButton: {fontSize: 16, color: '#007AFF', minWidth: 40},
  headerTitle: {fontSize: 18, fontWeight: '600', color: '#333'},
  headerSpacer: {minWidth: 40},
  scroll: {padding: 16},
  heroCard: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 12},
  heroTitle: {fontSize: 20, fontWeight: '700', color: '#333', marginBottom: 8},
  heroText: {fontSize: 14, color: '#666', lineHeight: 22, marginBottom: 8},
  heroHint: {fontSize: 13, color: '#999'},
  passageCard: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 12},
  passageTitle: {fontSize: 18, fontWeight: '600', color: '#333', marginBottom: 6},
  passageMeta: {fontSize: 13, color: '#666', marginBottom: 12},
  failureText: {fontSize: 13, color: '#FF3B30', marginBottom: 12},
  sourceSection: {marginTop: 12},
  sectionTitle: {fontSize: 15, fontWeight: '600', color: '#333', marginBottom: 8},
  chipWrap: {flexDirection: 'row', flexWrap: 'wrap', gap: 8},
  sourceChip: {
    backgroundColor: '#EEF5FF',
    borderRadius: 999,
    paddingHorizontal: 12,
    paddingVertical: 8,
  },
  sourceWord: {fontSize: 13, fontWeight: '600', color: '#007AFF'},
  sourceGloss: {fontSize: 12, color: '#666', marginTop: 2},
  block: {
    backgroundColor: '#F8F8F8',
    borderRadius: 10,
    padding: 12,
    marginBottom: 10,
  },
  blockText: {fontSize: 15, color: '#444', lineHeight: 24},
  wordSegment: {fontWeight: '700', color: '#007AFF'},
  wordGloss: {fontWeight: '400', color: '#666'},
  emptyCard: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 24,
    alignItems: 'center',
    marginBottom: 12,
  },
  emptyTitle: {fontSize: 18, fontWeight: '600', color: '#333', marginBottom: 8},
  emptyText: {fontSize: 14, color: '#666', textAlign: 'center'},
  historyCard: {backgroundColor: '#fff', borderRadius: 12, padding: 16},
  historyTitle: {fontSize: 18, fontWeight: '600', color: '#333', marginBottom: 8},
  historyRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    gap: 12,
    paddingVertical: 12,
    borderTopWidth: 1,
    borderTopColor: '#f0f0f0',
  },
  historyContent: {flex: 1},
  historyItemTitle: {fontSize: 14, fontWeight: '600', color: '#333'},
  historyPreview: {fontSize: 13, color: '#666', marginTop: 2},
  historyMeta: {alignItems: 'flex-end'},
  historyDate: {fontSize: 12, color: '#999'},
  historyStatus: {fontSize: 12, color: '#666', marginTop: 4},
});
