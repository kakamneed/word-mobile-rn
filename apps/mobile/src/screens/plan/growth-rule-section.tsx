/**
 * Growth Rule Section
 *
 * Auto-growth rule configuration for the plan editor.
 */

import React from 'react';
import { View, Text, StyleSheet, TouchableOpacity } from 'react-native';

interface GrowthRuleSectionProps {
  intervalDays: number;
  increment: number;
  onChangeInterval: (days: number) => void;
  onChangeIncrement: (inc: number) => void;
}

const INTERVAL_OPTIONS = [3, 5, 7, 14, 30];
const INCREMENT_OPTIONS = [3, 5, 10, 15, 20];

export function GrowthRuleSection({
  intervalDays,
  increment,
  onChangeInterval,
  onChangeIncrement,
}: GrowthRuleSectionProps): React.JSX.Element {
  return (
    <View style={styles.container}>
      <Text style={styles.sectionTitle}>Auto Growth Rules</Text>
      <Text style={styles.description}>
        Automatically increase your daily targets as you progress
      </Text>

      {/* Interval Selection */}
      <View style={styles.optionGroup}>
        <Text style={styles.optionLabel}>Increase every</Text>
        <View style={styles.optionsRow}>
          {INTERVAL_OPTIONS.map((days) => (
            <TouchableOpacity
              key={days}
              style={[
                styles.optionButton,
                intervalDays === days && styles.optionButtonActive,
              ]}
              onPress={() => onChangeInterval(days)}
            >
              <Text
                style={[
                  styles.optionText,
                  intervalDays === days && styles.optionTextActive,
                ]}
              >
                {days}d
              </Text>
            </TouchableOpacity>
          ))}
        </View>
      </View>

      {/* Increment Selection */}
      <View style={styles.optionGroup}>
        <Text style={styles.optionLabel}>Increase by</Text>
        <View style={styles.optionsRow}>
          {INCREMENT_OPTIONS.map((inc) => (
            <TouchableOpacity
              key={inc}
              style={[
                styles.optionButton,
                increment === inc && styles.optionButtonActive,
              ]}
              onPress={() => onChangeIncrement(inc)}
            >
              <Text
                style={[
                  styles.optionText,
                  increment === inc && styles.optionTextActive,
                ]}
              >
                +{inc}
              </Text>
            </TouchableOpacity>
          ))}
        </View>
      </View>

      {/* Summary */}
      <View style={styles.summaryBox}>
        <Text style={styles.summaryText}>
          Your daily targets will increase by{' '}
          <Text style={styles.summaryHighlight}>{increment}</Text> words every{' '}
          <Text style={styles.summaryHighlight}>{intervalDays}</Text> days of consistent study.
        </Text>
      </View>
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
    marginBottom: 8,
    color: '#333',
  },
  description: {
    fontSize: 14,
    color: '#666',
    marginBottom: 20,
  },
  optionGroup: {
    marginBottom: 20,
  },
  optionLabel: {
    fontSize: 14,
    fontWeight: '500',
    color: '#333',
    marginBottom: 10,
  },
  optionsRow: {
    flexDirection: 'row',
    gap: 8,
  },
  optionButton: {
    flex: 1,
    paddingVertical: 10,
    paddingHorizontal: 8,
    borderRadius: 8,
    backgroundColor: '#f0f0f0',
    alignItems: 'center',
  },
  optionButtonActive: {
    backgroundColor: '#007AFF',
  },
  optionText: {
    fontSize: 14,
    fontWeight: '500',
    color: '#666',
  },
  optionTextActive: {
    color: '#fff',
  },
  summaryBox: {
    backgroundColor: '#f8f8f8',
    borderRadius: 8,
    padding: 12,
  },
  summaryText: {
    fontSize: 14,
    color: '#666',
    lineHeight: 20,
  },
  summaryHighlight: {
    color: '#007AFF',
    fontWeight: '600',
  },
});
