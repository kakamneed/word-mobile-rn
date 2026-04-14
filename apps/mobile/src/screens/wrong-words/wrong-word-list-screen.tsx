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
  fetchWrongWords,
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

  const loadAll = async () => {
    try {
      const wrongWords = await fetchWrongWords(filter);
      setWords(wrongWords);
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void loadAll();
  }, [filter]);

  const totalErrors = words.reduce((sum, item) => sum + item.errorCount, 0);
  const avgPriority =
    words.length > 0
      ? (
          words.reduce((sum, item) => sum + item.priorityScore, 0) / words.length
        ).toFixed(1)
      : '0.0';

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

      <ScrollView
        contentContainerStyle={styles.scroll}
        refreshControl={
          <RefreshControl
            refreshing={refreshing}
            onRefresh={() => {
              setRefreshing(true);
              void loadAll();
            }}
          />
        }>
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

        <View style={styles.infoCard}>
          <Text style={styles.infoTitle}>AI 短文入口已迁移</Text>
          <Text style={styles.infoText}>
            AI 短文现在只会在完成今日任务后，从 Today 页触发生成。这里保留错词查看和错词强化，不再直接生成 AI 短文。
          </Text>
        </View>

        {words.length === 0 ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyTitle}>还没有错词</Text>
            <Text style={styles.emptyText}>
              完成学习后，答错或跳过的词会出现在这里。
            </Text>
          </View>
        ) : (
          words.map(word => (
            <WordRow
              key={word.entryId}
              word={word}
              onPress={() => onSelectWord?.(word.entryId)}
            />
          ))
        )}
      </ScrollView>
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
  onPress,
}: {
  word: WrongWordEntry;
  onPress: () => void;
}): React.JSX.Element {
  const priorityColor =
    word.priorityScore >= 8
      ? '#FF3B30'
      : word.priorityScore >= 5
      ? '#FF9500'
      : '#34C759';

  return (
    <TouchableOpacity style={styles.wordRow} onPress={onPress}>
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
        <Text style={styles.arrow}>{'>'}</Text>
      </View>
    </TouchableOpacity>
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
    marginBottom: 12,
  },
  reviewButtonText: {color: '#fff', fontSize: 16, fontWeight: '600'},
  infoCard: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
  },
  infoTitle: {fontSize: 16, fontWeight: '600', color: '#333', marginBottom: 6},
  infoText: {fontSize: 14, color: '#666', lineHeight: 22},
  emptyState: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 24,
    alignItems: 'center',
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
});
