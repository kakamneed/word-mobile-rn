/**
 * Word Mobile - Main App Entry
 */

import React, {useEffect, useState} from 'react';
import {View, Text, StyleSheet, ActivityIndicator, SafeAreaView} from 'react-native';
import {getBootstrapState, type BootstrapState, type SessionMode} from './lib/mobile-bridge';
import {StartupErrorScreen} from './screens/startup-error-screen';
import {OnboardingFlow} from './screens/onboarding/onboarding-flow';
import {MobileRoot, type Screen} from './navigation/mobile-root';

type AppState =
  | {type: 'loading'}
  | {type: 'error'; reason: string; recoverable: boolean}
  | {type: 'onboarding'}
  | {type: 'app'; currentScreen: Screen; currentStudyMode: SessionMode};

export function App(): React.JSX.Element {
  const [appState, setAppState] = useState<AppState>({type: 'loading'});

  useEffect(() => {
    void performBootstrap();
  }, []);

  const performBootstrap = async () => {
    try {
      const state: BootstrapState = await getBootstrapState();

      if (!state.appReady) {
        setAppState({
          type: 'error',
          reason: state.blockingReason ?? '应用尚未准备好',
          recoverable: state.databaseStatus !== 'init_failed',
        });
        return;
      }

      if (state.firstRunRequired) {
        setAppState({type: 'onboarding'});
      } else {
        setAppState({type: 'app', currentScreen: 'today', currentStudyMode: 'newWord'});
      }
    } catch (error) {
      setAppState({
        type: 'error',
        reason: error instanceof Error ? error.message : '应用初始化失败',
        recoverable: true,
      });
    }
  };

  const handleRetry = () => {
    setAppState({type: 'loading'});
    void performBootstrap();
  };

  const handleOnboardingComplete = () => {
    setAppState({type: 'app', currentScreen: 'today', currentStudyMode: 'newWord'});
  };

  const handleNavigate = (screen: Screen, studyMode?: SessionMode) => {
    if (appState.type === 'app') {
      setAppState({
        ...appState,
        currentScreen: screen,
        currentStudyMode: studyMode ?? appState.currentStudyMode,
      });
    }
  };

  switch (appState.type) {
    case 'loading':
      return (
        <SafeAreaView style={styles.container}>
          <View style={styles.center}>
            <ActivityIndicator size="large" color="#007AFF" />
            <Text style={styles.loadingText}>正在初始化...</Text>
          </View>
        </SafeAreaView>
      );
    case 'error':
      return (
        <StartupErrorScreen
          reason={appState.reason}
          recoverable={appState.recoverable}
          onRetry={handleRetry}
        />
      );
    case 'onboarding':
      return <OnboardingFlow onComplete={handleOnboardingComplete} />;
    case 'app':
      return (
        <MobileRoot
          currentScreen={appState.currentScreen}
          currentStudyMode={appState.currentStudyMode}
          onNavigate={handleNavigate}
        />
      );
  }
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#fff'},
  center: {flex: 1, justifyContent: 'center', alignItems: 'center', padding: 20},
  loadingText: {marginTop: 16, fontSize: 16, color: '#666'},
});
