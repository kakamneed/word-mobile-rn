import React from 'react';
import {View, Text, StyleSheet} from 'react-native';
import type {Wordbook} from '../../lib/vocabulary-client';
import {WordbookRow} from './wordbook-row';

interface WordbookListProps {
  wordbooks: Wordbook[];
  onToggle: (wordbookId: number, isActive: boolean) => void;
}

export function WordbookList({wordbooks, onToggle}: WordbookListProps): React.JSX.Element {
  const activeWordbooks = wordbooks.filter(wb => wb.isActive);
  const inactiveWordbooks = wordbooks.filter(wb => !wb.isActive);

  return (
    <View style={styles.container}>
      <Text style={styles.sectionTitle}>词书配置</Text>

      {activeWordbooks.length > 0 ? (
        <View style={styles.section}>
          <Text style={styles.subsectionTitle}>已启用（{activeWordbooks.length}）</Text>
          {activeWordbooks.map(wordbook => (
            <WordbookRow key={wordbook.id} wordbook={wordbook} onToggle={onToggle} />
          ))}
        </View>
      ) : null}

      {inactiveWordbooks.length > 0 ? (
        <View style={styles.section}>
          <Text style={styles.subsectionTitle}>可选（{inactiveWordbooks.length}）</Text>
          {inactiveWordbooks.map(wordbook => (
            <WordbookRow key={wordbook.id} wordbook={wordbook} onToggle={onToggle} />
          ))}
        </View>
      ) : null}

      {wordbooks.length === 0 ? (
        <View style={styles.emptyState}>
          <Text style={styles.emptyText}>当前没有可用词书</Text>
          <Text style={styles.emptySubtext}>请检查词书数据状态</Text>
        </View>
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {backgroundColor: '#fff', borderRadius: 12, padding: 16},
  sectionTitle: {fontSize: 18, fontWeight: '600', marginBottom: 16, color: '#333'},
  section: {marginBottom: 16},
  subsectionTitle: {fontSize: 14, color: '#999', marginBottom: 8},
  emptyState: {alignItems: 'center', paddingVertical: 32},
  emptyText: {fontSize: 16, color: '#666', marginBottom: 4},
  emptySubtext: {fontSize: 14, color: '#999'},
});
