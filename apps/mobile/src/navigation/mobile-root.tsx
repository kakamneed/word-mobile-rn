import React from 'react';
import {StyleSheet, Text, TouchableOpacity, View} from 'react-native';
import {AiPassageScreen} from '../screens/ai/ai-passage-screen';
import {PlanScreen} from '../screens/plan/plan-screen';
import {ReportsOverviewScreen} from '../screens/reports/reports-overview-screen';
import {StudySessionScreen} from '../screens/study/study-session-screen';
import {TodayScreen} from '../screens/today/today-screen';
import {WrongWordListScreen} from '../screens/wrong-words/wrong-word-list-screen';
import type {SessionMode} from '../lib/mobile-bridge';

export type Screen =
  | 'today'
  | 'plan'
  | 'wrongWords'
  | 'study'
  | 'reports'
  | 'ai';

interface MobileRootProps {
  currentScreen: Screen;
  currentStudyMode: SessionMode;
  onNavigate: (screen: Screen, studyMode?: SessionMode) => void;
}

export function MobileRoot({
  currentScreen,
  currentStudyMode,
  onNavigate,
}: MobileRootProps): React.JSX.Element {
  return (
    <View style={styles.container}>
      <View style={styles.content}>
        {currentScreen === 'today' && (
          <TodayScreen
            onNavigateToPlan={() => onNavigate('plan')}
            onStartStudy={mode => onNavigate('study', mode)}
            onNavigateToAi={() => onNavigate('ai')}
          />
        )}
        {currentScreen === 'plan' && (
          <PlanScreen onBack={() => onNavigate('today')} />
        )}
        {currentScreen === 'wrongWords' && (
          <WrongWordListScreen onBack={() => onNavigate('today')} />
        )}
        {currentScreen === 'reports' && (
          <ReportsOverviewScreen onBack={() => onNavigate('today')} />
        )}
        {currentScreen === 'ai' && (
          <AiPassageScreen onBack={() => onNavigate('today')} />
        )}
        {currentScreen === 'study' && (
          <StudySessionScreen
            mode={currentStudyMode}
            wordbookId={null}
            entrySourceIds={[]}
            onComplete={() => onNavigate('today')}
            onCancel={() => onNavigate('today')}
          />
        )}
      </View>

      <View style={styles.tabBar}>
        <TabButton
          label="今日"
          icon="今"
          isActive={currentScreen === 'today'}
          onPress={() => onNavigate('today')}
        />
        <TabButton
          label="计划"
          icon="计"
          isActive={currentScreen === 'plan'}
          onPress={() => onNavigate('plan')}
        />
        <TabButton
          label="错词"
          icon="错"
          isActive={currentScreen === 'wrongWords'}
          onPress={() => onNavigate('wrongWords')}
        />
        <TabButton
          label="报告"
          icon="报"
          isActive={currentScreen === 'reports'}
          onPress={() => onNavigate('reports')}
        />
        <TabButton
          label="AI"
          icon="智"
          isActive={currentScreen === 'ai'}
          onPress={() => onNavigate('ai')}
        />
      </View>
    </View>
  );
}

function TabButton({
  label,
  icon,
  isActive,
  onPress,
}: {
  label: string;
  icon: string;
  isActive: boolean;
  onPress: () => void;
}): React.JSX.Element {
  return (
    <TouchableOpacity style={styles.tabButton} onPress={onPress}>
      <Text style={[styles.tabIcon, isActive && styles.tabLabelActive]}>{icon}</Text>
      <Text style={[styles.tabLabel, isActive && styles.tabLabelActive]}>
        {label}
      </Text>
    </TouchableOpacity>
  );
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#fff'},
  content: {flex: 1},
  tabBar: {
    flexDirection: 'row',
    borderTopWidth: 1,
    borderTopColor: '#e0e0e0',
    backgroundColor: '#fff',
    paddingBottom: 8,
    paddingTop: 8,
  },
  tabButton: {flex: 1, alignItems: 'center', paddingVertical: 8},
  tabIcon: {fontSize: 14, fontWeight: '700', marginBottom: 4, color: '#666'},
  tabLabel: {fontSize: 12, color: '#999'},
  tabLabelActive: {color: '#007AFF', fontWeight: '600'},
});
