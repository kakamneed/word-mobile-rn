import React from 'react';
import {View, Text, StyleSheet, TouchableOpacity, ActivityIndicator} from 'react-native';
import type {VocabularyStatus} from '../../lib/vocabulary-client';

interface UpdateButtonProps {
  status: VocabularyStatus | null;
  updating: boolean;
  onUpdate: () => void;
  onCheck: () => void;
}

export function UpdateButton({
  status,
  updating,
  onUpdate,
  onCheck,
}: UpdateButtonProps): React.JSX.Element {
  if (updating) {
    return (
      <View style={styles.container}>
        <ActivityIndicator color="#007AFF" />
        <Text style={styles.statusText}>
          {status?.status === 'updating' ? '正在更新词书...' : '正在检查更新...'}
        </Text>
      </View>
    );
  }

  const hasUpdate = status?.availableUpdate ?? false;

  if (hasUpdate) {
    return (
      <TouchableOpacity style={styles.updateButton} onPress={onUpdate}>
        <Text style={styles.updateButtonIcon}>更</Text>
        <View style={styles.updateButtonText}>
          <Text style={styles.updateButtonTitle}>发现可用更新</Text>
          <Text style={styles.updateButtonSubtitle}>有新的词书数据可以下载</Text>
        </View>
      </TouchableOpacity>
    );
  }

  return (
    <TouchableOpacity style={styles.checkButton} onPress={onCheck}>
      <Text style={styles.checkButtonText}>检查更新</Text>
    </TouchableOpacity>
  );
}

const styles = StyleSheet.create({
  container: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 12,
    backgroundColor: '#f0f0f0',
    borderRadius: 8,
  },
  statusText: {marginLeft: 8, fontSize: 14, color: '#666'},
  updateButton: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: '#007AFF',
    borderRadius: 8,
    padding: 12,
  },
  updateButtonIcon: {fontSize: 18, color: '#fff', marginRight: 12, fontWeight: '700'},
  updateButtonText: {flex: 1},
  updateButtonTitle: {color: '#fff', fontSize: 16, fontWeight: '600'},
  updateButtonSubtitle: {color: 'rgba(255,255,255,0.8)', fontSize: 13, marginTop: 2},
  checkButton: {backgroundColor: '#f0f0f0', borderRadius: 8, paddingVertical: 12, alignItems: 'center'},
  checkButtonText: {color: '#007AFF', fontSize: 16, fontWeight: '500'},
});
