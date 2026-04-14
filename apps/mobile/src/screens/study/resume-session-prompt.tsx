/**
 * Resume Session Prompt
 *
 * Shown when the app detects an interrupted study session.
 * Offers explicit choice: continue or abandon.
 */

import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  Modal,
} from 'react-native';

interface ResumeSessionPromptProps {
  visible: boolean;
  sessionProgress?: { current: number; total: number };
  onResume: () => void;
  onAbandon: () => void;
}

export function ResumeSessionPrompt({
  visible,
  sessionProgress,
  onResume,
  onAbandon,
}: ResumeSessionPromptProps): React.JSX.Element {
  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      onRequestClose={() => {}}
    >
      <View style={styles.overlay}>
        <View style={styles.container}>
          <Text style={styles.icon}>📚</Text>
          <Text style={styles.title}>Resume Study Session?</Text>

          <Text style={styles.description}>
            You have an unfinished study session.
            {sessionProgress && (
              <Text>
                {' '}\n\nProgress: {sessionProgress.current} of {sessionProgress.total} questions answered
              </Text>
            )}
          </Text>

          <View style={styles.buttons}>
            <TouchableOpacity
              style={styles.resumeButton}
              onPress={onResume}
            >
              <Text style={styles.resumeButtonText}>Continue Session</Text>
            </TouchableOpacity>

            <TouchableOpacity
              style={styles.abandonButton}
              onPress={onAbandon}
            >
              <Text style={styles.abandonButtonText}>
                Abandon (Progress Lost)
              </Text>
            </TouchableOpacity>
          </View>
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  overlay: {
    flex: 1,
    backgroundColor: 'rgba(0,0,0,0.5)',
    justifyContent: 'center',
    alignItems: 'center',
    padding: 20,
  },
  container: {
    backgroundColor: '#fff',
    borderRadius: 16,
    padding: 24,
    width: '100%',
    maxWidth: 340,
    alignItems: 'center',
  },
  icon: {
    fontSize: 48,
    marginBottom: 16,
  },
  title: {
    fontSize: 22,
    fontWeight: 'bold',
    marginBottom: 12,
    color: '#333',
  },
  description: {
    fontSize: 15,
    color: '#666',
    textAlign: 'center',
    lineHeight: 22,
    marginBottom: 24,
  },
  buttons: {
    width: '100%',
    gap: 12,
  },
  resumeButton: {
    backgroundColor: '#007AFF',
    borderRadius: 12,
    paddingVertical: 14,
    alignItems: 'center',
  },
  resumeButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  abandonButton: {
    paddingVertical: 14,
    alignItems: 'center',
  },
  abandonButtonText: {
    color: '#FF3B30',
    fontSize: 16,
  },
});
