/**
 * Wordbook Selector Section
 *
 * Vocabulary-aware wordbook selection with readiness context.
 */

import React, {useEffect, useState} from 'react';
import {View, Text, StyleSheet, Switch} from 'react-native';
import {fetchWordbooks, type Wordbook} from '../../lib/vocabulary-client';

interface WordbookSelectorSectionProps {
  selectedIds: number[];
  onChange: (ids: number[]) => void;
}

export function WordbookSelectorSection({
  selectedIds,
  onChange,
}: WordbookSelectorSectionProps): React.JSX.Element {
  const [wordbooks, setWordbooks] = useState<Wordbook[]>([]);

  useEffect(() => {
    void loadWordbooks();
  }, []);

  const loadWordbooks = async () => {
    const data = await fetchWordbooks();
    setWordbooks(data);
  };

  const toggleWordbook = (id: number) => {
    if (selectedIds.includes(id)) {
      onChange(selectedIds.filter(i => i !== id));
    } else {
      onChange([...selectedIds, id]);
    }
  };

  const selectedCount = selectedIds.length;
  const totalWords = wordbooks
    .filter(wb => selectedIds.includes(wb.id))
    .reduce((sum, wb) => sum + wb.totalEntries, 0);

  const isLimitedCorpus = selectedCount === 0 || totalWords < 1000;

  return (
    <View style={styles.container}>
      <Text style={styles.sectionTitle}>Vocabulary Sources</Text>

      {isLimitedCorpus && (
        <View style={styles.warningBox}>
          <Text style={styles.warningIcon}>WARN</Text>
          <Text style={styles.warningText}>
            {selectedCount === 0
              ? 'Select at least one wordbook to start learning.'
              : 'Limited vocabulary selected. Consider adding more wordbooks for better variety.'}
          </Text>
        </View>
      )}

      <View style={styles.list}>
        {wordbooks.map(wordbook => (
          <WordbookItem
            key={wordbook.id}
            wordbook={wordbook}
            selected={selectedIds.includes(wordbook.id)}
            onToggle={() => toggleWordbook(wordbook.id)}
          />
        ))}
      </View>

      <View style={styles.summary}>
        <Text style={styles.summaryText}>
          {selectedCount} wordbook{selectedCount !== 1 ? 's' : ''} selected
        </Text>
        <Text style={styles.summarySubtext}>
          {totalWords.toLocaleString()} total words available for study
        </Text>
      </View>
    </View>
  );
}

interface WordbookItemProps {
  wordbook: Wordbook;
  selected: boolean;
  onToggle: () => void;
}

function WordbookItem({
  wordbook,
  selected,
  onToggle,
}: WordbookItemProps): React.JSX.Element {
  return (
    <View style={[styles.item, selected && styles.itemSelected]}>
      <View style={styles.itemInfo}>
        <Text style={styles.itemName}>{wordbook.name}</Text>
        <Text style={styles.itemMeta}>
          {wordbook.totalEntries.toLocaleString()} words - {wordbook.category}
        </Text>
      </View>
      <Switch
        value={selected}
        onValueChange={onToggle}
        trackColor={{false: '#e0e0e0', true: '#007AFF'}}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 16,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '600',
    marginBottom: 16,
    color: '#333',
  },
  warningBox: {
    flexDirection: 'row',
    backgroundColor: '#FFF3F3',
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
    borderWidth: 1,
    borderColor: '#FFC7C7',
  },
  warningIcon: {
    fontSize: 12,
    fontWeight: '700',
    marginRight: 8,
    color: '#D32F2F',
  },
  warningText: {
    flex: 1,
    fontSize: 14,
    color: '#D32F2F',
    lineHeight: 20,
  },
  list: {
    gap: 8,
  },
  item: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 12,
    borderRadius: 8,
    backgroundColor: '#f8f8f8',
  },
  itemSelected: {
    backgroundColor: '#f0f8ff',
  },
  itemInfo: {
    flex: 1,
  },
  itemName: {
    fontSize: 15,
    fontWeight: '500',
    color: '#333',
    marginBottom: 2,
  },
  itemMeta: {
    fontSize: 13,
    color: '#666',
  },
  summary: {
    marginTop: 16,
    paddingTop: 16,
    borderTopWidth: 1,
    borderTopColor: '#f0f0f0',
  },
  summaryText: {
    fontSize: 16,
    fontWeight: '600',
    color: '#333',
  },
  summarySubtext: {
    fontSize: 14,
    color: '#666',
    marginTop: 2,
  },
});
