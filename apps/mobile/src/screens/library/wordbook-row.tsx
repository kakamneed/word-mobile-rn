/**
 * Wordbook Row
 */

import React from 'react';
import {View, Text, StyleSheet, Switch} from 'react-native';
import type {Wordbook} from '../../lib/vocabulary-client';

interface WordbookRowProps {
  wordbook: Wordbook;
  onToggle: (wordbookId: number, isActive: boolean) => void;
}

export function WordbookRow({
  wordbook,
  onToggle,
}: WordbookRowProps): React.JSX.Element {
  return (
    <View style={[styles.container, wordbook.isActive && styles.containerActive]}>
      <View style={styles.info}>
        <Text style={styles.name}>{wordbook.name}</Text>
        <Text style={styles.meta}>
          {wordbook.totalEntries.toLocaleString()} 词 · {wordbook.category}
        </Text>
      </View>

      <Switch
        value={wordbook.isActive}
        onValueChange={value => onToggle(wordbook.id, value)}
        trackColor={{false: '#e0e0e0', true: '#007AFF'}}
        thumbColor="#fff"
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 12,
    paddingHorizontal: 12,
    borderRadius: 8,
    marginBottom: 8,
    backgroundColor: '#f8f8f8',
  },
  containerActive: {
    backgroundColor: '#f0f8ff',
  },
  info: {
    flex: 1,
  },
  name: {
    fontSize: 16,
    fontWeight: '500',
    color: '#333',
    marginBottom: 2,
  },
  meta: {
    fontSize: 13,
    color: '#666',
  },
});
