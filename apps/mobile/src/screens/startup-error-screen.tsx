/**
 * Startup Error Screen
 *
 * Displays blocking startup failures with explicit recovery actions.
 * This screen appears when bootstrap fails in a way that prevents
 * the app from proceeding safely.
 */

import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  SafeAreaView,
  ScrollView,
} from 'react-native';

interface StartupErrorScreenProps {
  reason: string;
  recoverable: boolean;
  onRetry: () => void;
}

export function StartupErrorScreen({
  reason,
  recoverable,
  onRetry,
}: StartupErrorScreenProps): React.JSX.Element {
  return (
    <SafeAreaView style={styles.container}>
      <ScrollView contentContainerStyle={styles.scroll}>
        <View style={styles.iconContainer}>
          <Text style={styles.icon}>⚠️</Text>
        </View>

        <Text style={styles.title}>Unable to Start</Text>

        <Text style={styles.description}>
          The app encountered a problem while initializing that prevents it from
          starting safely.
        </Text>

        <View style={styles.detailsBox}>
          <Text style={styles.detailsLabel}>Error Details</Text>
          <Text style={styles.detailsText}>{reason}</Text>
        </View>

        {recoverable ? (
          <View style={styles.actions}>
            <TouchableOpacity style={styles.primaryButton} onPress={onRetry}>
              <Text style={styles.primaryButtonText}>Try Again</Text>
            </TouchableOpacity>

            <Text style={styles.hint}>
              If the problem persists, please check that your device has
              sufficient storage space and try restarting the app.
            </Text>
          </View>
        ) : (
          <View style={styles.actions}>
            <View style={styles.fatalBox}>
              <Text style={styles.fatalTitle}>Critical Error</Text>
              <Text style={styles.fatalText}>
                This error requires app reinstallation. Please uninstall and
                reinstall the app from the app store.
              </Text>
            </View>
          </View>
        )}
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#fff',
  },
  scroll: {
    flexGrow: 1,
    padding: 24,
    justifyContent: 'center',
  },
  iconContainer: {
    alignItems: 'center',
    marginBottom: 24,
  },
  icon: {
    fontSize: 64,
  },
  title: {
    fontSize: 24,
    fontWeight: 'bold',
    textAlign: 'center',
    marginBottom: 16,
    color: '#1a1a1a',
  },
  description: {
    fontSize: 16,
    textAlign: 'center',
    color: '#666',
    marginBottom: 24,
    lineHeight: 22,
  },
  detailsBox: {
    backgroundColor: '#f5f5f5',
    borderRadius: 8,
    padding: 16,
    marginBottom: 24,
  },
  detailsLabel: {
    fontSize: 12,
    fontWeight: '600',
    color: '#999',
    textTransform: 'uppercase',
    marginBottom: 8,
  },
  detailsText: {
    fontSize: 14,
    color: '#333',
    lineHeight: 20,
  },
  actions: {
    gap: 16,
  },
  primaryButton: {
    backgroundColor: '#007AFF',
    borderRadius: 8,
    paddingVertical: 14,
    paddingHorizontal: 24,
    alignItems: 'center',
  },
  primaryButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  hint: {
    fontSize: 14,
    color: '#999',
    textAlign: 'center',
    lineHeight: 20,
  },
  fatalBox: {
    backgroundColor: '#FFF3F3',
    borderRadius: 8,
    padding: 16,
    borderWidth: 1,
    borderColor: '#FFC7C7',
  },
  fatalTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: '#D32F2F',
    marginBottom: 8,
  },
  fatalText: {
    fontSize: 14,
    color: '#666',
    lineHeight: 20,
  },
});
