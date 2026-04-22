import React, {useEffect, useMemo, useState} from 'react';
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
  fetchWrongWordDetail,
  fetchWrongWords,
  type WrongWordDetail,
  type WrongWordEntry,
  type WrongWordFilter,
} from '../../lib/wrong-word-client';

interface WrongWordListScreenProps {
  onBack?: () => void;
  onSelectWord?: (entryId: number) => void;
  onStartReview?: () => void;
}

const FILTERS: Array<{key: WrongWordFilter; label: string}> = [
  {key: 'all', label: '全部'},
  {key: 'highPriority', label: '高优先级'},
  {key: 'recent', label: '最近错词'},
  {key: 'frequent', label: '高频出错'},
];

export function WrongWordListScreen({
  onBack,
  onSelectWord,
  onStartReview,
}: WrongWordListScreenProps): React.JSX.Element {
  const [words, setWords] = useState<WrongWordEntry[]>([]);
  const [filter, setFilter] = useState<WrongWordFilter>('all');
  const [refreshing, setRefreshing] = useState(false);
  const [expandedEntryId, setExpandedEntryId] = useState<number | null>(null);
  const [expandedDetail, setExpandedDetail] = useState<WrongWordDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);

  const loadAll = async () => {
    try {
      const wrongWords = await fetchWrongWords(filter);
      setWords(wrongWords);
      if (
        expandedEntryId !== null &&
        !wrongWords.some(word => word.entryId === expandedEntryId)
      ) {
        setExpandedEntryId(null);
        setExpandedDetail(null);
      }
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void loadAll();
  }, [filter]);

  const totalErrors = useMemo(
    () => words.reduce((sum, item) => sum + item.errorCount, 0),
    [words],
  );
  const avgPriority = useMemo(() => {
    if (words.length === 0) {
      return '0.0';
    }
    return (
      words.reduce((sum, item) => sum + item.priorityScore, 0) / words.length
    ).toFixed(1);
  }, [words]);

  const handleToggleDetail = async (entryId: number) => {
    if (onSelectWord) {
      onSelectWord(entryId);
      return;
    }
    if (expandedEntryId === entryId) {
      setExpandedEntryId(null);
      setExpandedDetail(null);
      return;
    }
    setExpandedEntryId(entryId);
    setLoadingDetail(true);
    try {
      const detail = await fetchWrongWordDetail(entryId);
      setExpandedDetail(detail);
    } finally {
      setLoadingDetail(false);
    }
  };

  const expandedWord =
    expandedEntryId === null
      ? null
      : words.find(word => word.entryId === expandedEntryId) ?? null;

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
        <Text style={styles.headerTitle}>错词本</Text>
        <View style={styles.headerSpacer} />
      </View>

      <View style={styles.topPanel}>
        <View style={styles.statsCard}>
          <StatBlock label="错词数" value={words.length.toString()} />
          <StatBlock label="累计错误" value={totalErrors.toString()} />
          <StatBlock label="平均优先级" value={avgPriority} />
        </View>

        <View style={styles.filterBar}>
          {FILTERS.map(item => (
            <TouchableOpacity
              key={item.key}
              style={[
                styles.filterButton,
                filter === item.key && styles.filterButtonActive,
              ]}
              onPress={() => setFilter(item.key)}>
              <Text
                style={[
                  styles.filterText,
                  filter === item.key && styles.filterTextActive,
                ]}>
                {item.label}
              </Text>
            </TouchableOpacity>
          ))}
        </View>

        {words.length > 0 ? (
          <TouchableOpacity
            style={styles.reviewButton}
            onPress={onStartReview ?? (() => undefined)}>
            <Text style={styles.reviewButtonText}>
              开始错词强化（最多 {Math.min(words.length, 20)} 个）
            </Text>
          </TouchableOpacity>
        ) : null}
      </View>

      {words.length === 0 ? (
        <View style={styles.emptyState}>
          <Text style={styles.emptyTitle}>还没有错词</Text>
          <Text style={styles.emptyText}>
            完成学习后，答错或跳过的词会出现在这里。
          </Text>
        </View>
      ) : (
        <View style={styles.body}>
          <View style={[styles.listPane, expandedEntryId !== null && styles.listPaneCollapsed]}>
            <ScrollView
              refreshControl={
                <RefreshControl
                  refreshing={refreshing}
                  onRefresh={() => {
                    setRefreshing(true);
                    void loadAll();
                  }}
                />
              }
              contentContainerStyle={styles.listContent}>
              {words.map(word => (
                <WordRow
                  key={word.entryId}
                  word={word}
                  expanded={expandedEntryId === word.entryId}
                  onPress={() => {
                    void handleToggleDetail(word.entryId);
                  }}
                />
              ))}
            </ScrollView>
          </View>

          {expandedEntryId !== null ? (
            <View style={styles.detailPane}>
              <Text style={styles.detailPaneTitle}>
                {expandedWord?.word ?? '详情'}
              </Text>
              <ScrollView
                style={styles.detailScroll}
                showsVerticalScrollIndicator
                nestedScrollEnabled>
                <InlineDetailCard detail={expandedDetail} loading={loadingDetail} />
              </ScrollView>
            </View>
          ) : null}
        </View>
      )}
    </SafeAreaView>
  );
}

function StatBlock({
  label,
  value,
}: {
  label: string;
  value: string;
}): React.JSX.Element {
  return (
    <View style={styles.statBlock}>
      <Text style={styles.statValue}>{value}</Text>
      <Text style={styles.statLabel}>{label}</Text>
    </View>
  );
}

function WordRow({
  word,
  expanded,
  onPress,
}: {
  word: WrongWordEntry;
  expanded: boolean;
  onPress: () => void;
}): React.JSX.Element {
  const priorityColor =
    word.priorityScore >= 8
      ? '#FF3B30'
      : word.priorityScore >= 5
      ? '#FF9500'
      : '#34C759';

  return (
    <TouchableOpacity
      style={[styles.wordRow, expanded && styles.wordRowExpanded]}
      onPress={onPress}>
      <View style={styles.wordMain}>
        <View style={styles.wordHeader}>
          <Text style={styles.wordText}>{word.word}</Text>
          <View
            style={[
              styles.priorityBadge,
              {backgroundColor: `${priorityColor}22`},
            ]}>
            <Text style={[styles.priorityText, {color: priorityColor}]}>
              {word.priorityScore.toFixed(1)}
            </Text>
          </View>
        </View>
        {word.phoneticUs ? (
          <Text style={styles.phonetic}>{word.phoneticUs}</Text>
        ) : null}
        <Text style={styles.meanings}>{word.meanings.join(' / ')}</Text>
      </View>
      <View style={styles.wordSide}>
        <Text style={styles.errorCount}>错 {word.errorCount}</Text>
        <Text style={styles.arrow}>{expanded ? '∨' : '>'}</Text>
      </View>
    </TouchableOpacity>
  );
}

function InlineDetailCard({
  detail,
  loading,
}: {
  detail: WrongWordDetail | null;
  loading: boolean;
}): React.JSX.Element {
  if (loading || !detail) {
    return <Text style={styles.detailMuted}>正在加载详情...</Text>;
  }

  return (
    <View style={styles.detailCard}>
      <Text style={styles.detailTitle}>{detail.word}</Text>
      <Text style={styles.detailSection}>词性：{detail.partOfSpeech || '-'}</Text>
      <Text style={styles.detailSection}>
        释义：{detail.meanings.map(item => item.meaningCn).join(' / ')}
      </Text>

      <Text style={styles.detailSectionTitle}>题型情况</Text>
      {detail.riskBreakdown.length > 0 ? (
        detail.riskBreakdown.map(item => (
          <Text key={item.questionType} style={styles.detailLine}>
            {questionTypeLabel(item.questionType)}：{item.incorrect + item.skipped}/
            {item.attempts} 失误
          </Text>
        ))
      ) : (
        <Text style={styles.detailMuted}>暂无分题型数据</Text>
      )}

      <Text style={styles.detailSectionTitle}>最近错误</Text>
      {detail.errorHistory.length > 0 ? (
        detail.errorHistory.map((item, index) => (
          <Text key={`${item.date}-${index}`} style={styles.detailLine}>
            {formatHistoryDate(item.date)}  {humanizeContext(item.context)}
          </Text>
        ))
      ) : (
        <Text style={styles.detailMuted}>暂无错误历史</Text>
      )}
    </View>
  );
}

function questionTypeLabel(questionType: string): string {
  switch (questionType) {
    case 'exampleToCnChoice':
      return '例句选义';
    case 'enToCnChoice':
      return '英选中';
    case 'cnToEnChoice':
      return '中选英';
    case 'enToCnInput':
      return '英输中';
    default:
      return questionType;
  }
}

function formatHistoryDate(value: string): string {
  if (!value) {
    return '-';
  }
  const cleaned = value.replace('T', ' ').replace('Z', '');
  return cleaned.slice(0, 19);
}

function humanizeContext(context: string): string {
  if (context === 'mobile review') {
    return '学习中答错';
  }
  return context || '未知场景';
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
  topPanel: {
    backgroundColor: '#f5f5f5',
    paddingHorizontal: 16,
    paddingTop: 12,
    paddingBottom: 12,
  },
  statsCard: {
    flexDirection: 'row',
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
  },
  statBlock: {flex: 1, alignItems: 'center'},
  statValue: {fontSize: 24, fontWeight: '700', color: '#333'},
  statLabel: {fontSize: 13, color: '#666', marginTop: 4},
  filterBar: {flexDirection: 'row', flexWrap: 'wrap', gap: 8, marginBottom: 12},
  filterButton: {
    paddingHorizontal: 14,
    paddingVertical: 8,
    borderRadius: 16,
    backgroundColor: '#e0e0e0',
  },
  filterButtonActive: {backgroundColor: '#007AFF'},
  filterText: {fontSize: 14, color: '#666'},
  filterTextActive: {color: '#fff', fontWeight: '600'},
  reviewButton: {
    backgroundColor: '#FF3B30',
    borderRadius: 12,
    paddingVertical: 14,
    alignItems: 'center',
  },
  reviewButtonText: {color: '#fff', fontSize: 16, fontWeight: '600'},
  body: {
    flex: 1,
    paddingHorizontal: 16,
    paddingBottom: 16,
    gap: 12,
  },
  listPane: {
    flex: 1,
  },
  listPaneCollapsed: {
    flex: 0.48,
  },
  listContent: {
    paddingBottom: 12,
  },
  detailPane: {
    flex: 0.52,
    backgroundColor: '#fff',
    borderRadius: 16,
    paddingTop: 12,
    paddingHorizontal: 16,
    paddingBottom: 12,
  },
  detailPaneTitle: {
    fontSize: 12,
    fontWeight: '700',
    color: '#999',
    marginBottom: 8,
    textTransform: 'uppercase',
  },
  detailScroll: {
    flex: 1,
  },
  emptyState: {
    flex: 1,
    margin: 16,
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 24,
    alignItems: 'center',
    justifyContent: 'center',
  },
  emptyTitle: {fontSize: 20, fontWeight: '600', color: '#333', marginBottom: 8},
  emptyText: {fontSize: 14, color: '#666', textAlign: 'center'},
  wordRow: {
    flexDirection: 'row',
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
  },
  wordRowExpanded: {
    borderWidth: 1,
    borderColor: '#007AFF',
    backgroundColor: '#F4F9FF',
  },
  wordMain: {flex: 1},
  wordHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 4,
  },
  wordText: {fontSize: 20, fontWeight: '600', color: '#333'},
  priorityBadge: {paddingHorizontal: 8, paddingVertical: 4, borderRadius: 12},
  priorityText: {fontSize: 12, fontWeight: '600'},
  phonetic: {fontSize: 14, color: '#666', marginBottom: 4, fontStyle: 'italic'},
  meanings: {fontSize: 14, color: '#666'},
  wordSide: {justifyContent: 'center', alignItems: 'center', marginLeft: 12},
  errorCount: {fontSize: 16, fontWeight: '600', color: '#FF3B30'},
  arrow: {fontSize: 22, color: '#999', marginTop: 4},
  detailCard: {
    paddingBottom: 12,
  },
  detailTitle: {fontSize: 22, fontWeight: '700', color: '#222', marginBottom: 10},
  detailSectionTitle: {
    fontSize: 15,
    fontWeight: '700',
    color: '#333',
    marginTop: 14,
    marginBottom: 8,
  },
  detailSection: {fontSize: 15, color: '#444', lineHeight: 24},
  detailLine: {fontSize: 14, color: '#666', lineHeight: 22, marginBottom: 4},
  detailMuted: {fontSize: 14, color: '#999', lineHeight: 22},
});
