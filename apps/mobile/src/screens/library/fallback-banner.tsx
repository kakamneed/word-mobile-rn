import React from 'react';
import {View, Text, StyleSheet, TouchableOpacity} from 'react-native';

interface FallbackBannerProps {
  errorMessage: string | null;
  onRetry: () => void;
}

export function FallbackBanner({
  errorMessage,
  onRetry,
}: FallbackBannerProps): React.JSX.Element {
  return (
    <View style={styles.container}>
      <View style={styles.iconRow}>
        <Text style={styles.icon}>警</Text>
        <Text style={styles.title}>更新失败</Text>
      </View>

      <Text style={styles.description}>
        最新词书更新没有成功。
        {errorMessage ? <Text style={styles.errorDetail}>{`\n\n${errorMessage}`}</Text> : null}
      </Text>

      <View style={styles.fallbackBox}>
        <Text style={styles.fallbackIcon}>可</Text>
        <Text style={styles.fallbackText}>
          <Text style={styles.fallbackBold}>仍可继续学习：</Text>
          当前本地词书仍然可用，不会影响你继续学习。
        </Text>
      </View>

      <TouchableOpacity style={styles.retryButton} onPress={onRetry}>
        <Text style={styles.retryButtonText}>重新更新</Text>
      </TouchableOpacity>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    backgroundColor: '#FFF9E6',
    borderRadius: 12,
    padding: 16,
    marginBottom: 16,
    borderWidth: 1,
    borderColor: '#FFE4B3',
  },
  iconRow: {flexDirection: 'row', alignItems: 'center', marginBottom: 12},
  icon: {fontSize: 18, marginRight: 8, fontWeight: '700', color: '#B35900'},
  title: {fontSize: 18, fontWeight: '600', color: '#B35900'},
  description: {fontSize: 15, color: '#666', lineHeight: 22, marginBottom: 16},
  errorDetail: {color: '#999', fontSize: 13},
  fallbackBox: {
    flexDirection: 'row',
    backgroundColor: '#E8F5E9',
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
    borderWidth: 1,
    borderColor: '#C8E6C9',
  },
  fallbackIcon: {fontSize: 16, color: '#2E7D32', marginRight: 8, fontWeight: '700'},
  fallbackText: {flex: 1, fontSize: 14, color: '#2E7D32', lineHeight: 20},
  fallbackBold: {fontWeight: '600'},
  retryButton: {backgroundColor: '#007AFF', borderRadius: 8, paddingVertical: 12, alignItems: 'center'},
  retryButtonText: {color: '#fff', fontSize: 16, fontWeight: '600'},
});
