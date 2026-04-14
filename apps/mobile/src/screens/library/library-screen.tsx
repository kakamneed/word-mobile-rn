import React, {useEffect, useState} from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  TouchableOpacity,
  RefreshControl,
  SafeAreaView,
} from 'react-native';
import {
  fetchVocabularyStatus,
  fetchWordbooks,
  startVocabularyUpdate,
  checkForUpdates,
  type VocabularyStatus,
  type Wordbook,
} from '../../lib/vocabulary-client';
import {FallbackBanner} from './fallback-banner';
import {WordbookList} from './wordbook-list';
import {UpdateButton} from './update-button';

interface LibraryScreenProps {
  onBack?: () => void;
}

export function LibraryScreen({onBack}: LibraryScreenProps): React.JSX.Element {
  const [status, setStatus] = useState<VocabularyStatus | null>(null);
  const [wordbooks, setWordbooks] = useState<Wordbook[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [updating, setUpdating] = useState(false);

  const loadData = async () => {
    const [statusData, wordbooksData] = await Promise.all([
      fetchVocabularyStatus(),
      fetchWordbooks(),
    ]);
    setStatus(statusData);
    setWordbooks(wordbooksData);
    setRefreshing(false);
  };

  useEffect(() => {
    void loadData();
  }, []);

  const handleRefresh = () => {
    setRefreshing(true);
    void loadData();
  };

  const handleUpdate = async () => {
    setUpdating(true);
    try {
      await startVocabularyUpdate();
      await loadData();
    } finally {
      setUpdating(false);
    }
  };

  const handleCheckUpdates = async () => {
    setUpdating(true);
    try {
      await checkForUpdates();
      await loadData();
    } finally {
      setUpdating(false);
    }
  };

  const handleToggleWordbook = async (wordbookId: number, isActive: boolean) => {
    setWordbooks(prev => prev.map(wb => (wb.id === wordbookId ? {...wb, isActive} : wb)));
  };

  const activeCount = wordbooks.filter(wb => wb.isActive).length;
  const totalEntries = wordbooks
    .filter(wb => wb.isActive)
    .reduce((sum, wb) => sum + wb.totalEntries, 0);

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        {onBack ? (
          <TouchableOpacity onPress={onBack}>
            <Text style={styles.backButton}>返回</Text>
          </TouchableOpacity>
        ) : (
          <View style={styles.placeholder} />
        )}
        <Text style={styles.headerTitle}>词书管理</Text>
        <View style={styles.placeholder} />
      </View>

      <ScrollView
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={handleRefresh} />}
        contentContainerStyle={styles.scroll}>
        <View style={styles.statusCard}>
          <View style={styles.statusHeader}>
            <Text style={styles.statusLabel}>当前状态</Text>
            <StatusBadge status={status?.status ?? 'idle'} />
          </View>

          <View style={styles.statsRow}>
            <View style={styles.stat}>
              <Text style={styles.statValue}>{activeCount}</Text>
              <Text style={styles.statLabel}>启用词书</Text>
            </View>
            <View style={styles.stat}>
              <Text style={styles.statValue}>{totalEntries.toLocaleString()}</Text>
              <Text style={styles.statLabel}>总词量</Text>
            </View>
          </View>

          <UpdateButton
            status={status}
            updating={updating}
            onUpdate={handleUpdate}
            onCheck={handleCheckUpdates}
          />
        </View>

        {status?.status === 'failed' && status.fallbackAvailable ? (
          <FallbackBanner errorMessage={status.errorMessage} onRetry={handleUpdate} />
        ) : null}

        <WordbookList wordbooks={wordbooks} onToggle={handleToggleWordbook} />
      </ScrollView>
    </SafeAreaView>
  );
}

function StatusBadge({status}: {status: VocabularyStatus['status']}): React.JSX.Element {
  const config = {
    idle: {text: '可用', color: '#34C759'},
    checking: {text: '检查中', color: '#FF9500'},
    updating: {text: '更新中', color: '#007AFF'},
    failed: {text: '更新失败', color: '#FF3B30'},
    uptodate: {text: '已最新', color: '#34C759'},
  };

  const {text, color} = config[status];
  return (
    <View style={[styles.badge, {backgroundColor: color + '20'}]}>
      <View style={[styles.badgeDot, {backgroundColor: color}]} />
      <Text style={[styles.badgeText, {color}]}>{text}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#f5f5f5'},
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 12,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  backButton: {fontSize: 16, color: '#007AFF', minWidth: 60},
  headerTitle: {fontSize: 18, fontWeight: '600'},
  placeholder: {minWidth: 60},
  scroll: {padding: 16},
  statusCard: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 16},
  statusHeader: {flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16},
  statusLabel: {fontSize: 12, color: '#999', marginBottom: 0},
  badge: {flexDirection: 'row', alignItems: 'center', paddingHorizontal: 10, paddingVertical: 4, borderRadius: 12},
  badgeDot: {width: 6, height: 6, borderRadius: 3, marginRight: 6},
  badgeText: {fontSize: 12, fontWeight: '600'},
  statsRow: {flexDirection: 'row', marginBottom: 16},
  stat: {flex: 1},
  statValue: {fontSize: 28, fontWeight: 'bold', color: '#333'},
  statLabel: {fontSize: 13, color: '#666', marginTop: 2},
});
