import React, {useEffect, useState} from 'react';
import {ActivityIndicator, StyleSheet, Text, TouchableOpacity, View} from 'react-native';
import {AiPassageScreen} from '../screens/ai/ai-passage-screen';
import {CrocBtiScreen} from '../screens/plan/croc-bti-screen';
import {PlanScreen} from '../screens/plan/plan-screen';
import {ReportsOverviewScreen} from '../screens/reports/reports-overview-screen';
import {StudySessionScreen} from '../screens/study/study-session-screen';
import {TodayScreen} from '../screens/today/today-screen';
import {WrongWordListScreen} from '../screens/wrong-words/wrong-word-list-screen';
import type {SessionMode} from '../lib/mobile-bridge';
import {fetchActivePlan, type PlanSummary} from '../lib/plan-client';

export type Screen =
  | 'today'
  | 'plan'
  | 'wrongWords'
  | 'study'
  | 'reports'
  | 'ai'
  | 'crocBti';

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
  const [sideMenuOpen, setSideMenuOpen] = useState(false);

  const navigateFromMenu = (screen: Screen) => {
    setSideMenuOpen(false);
    onNavigate(screen);
  };

  return (
    <View style={styles.container}>
      <View style={styles.content}>
        <TouchableOpacity
          style={styles.menuButton}
          onPress={() => setSideMenuOpen(true)}>
          <Text style={styles.menuButtonText}>菜单</Text>
        </TouchableOpacity>

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
        {currentScreen === 'crocBti' && (
          <CrocBtiEntryScreen
            onBack={() => onNavigate('today')}
            onApplied={() => onNavigate('today')}
          />
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

      {sideMenuOpen ? (
        <View style={styles.sideMenuOverlay}>
          <TouchableOpacity
            style={styles.sideMenuScrim}
            onPress={() => setSideMenuOpen(false)}
          />
          <View style={styles.sideMenu}>
            <Text style={styles.sideMenuTitle}>学习工具</Text>
            <SideMenuItem label="今日学习" onPress={() => navigateFromMenu('today')} />
            <SideMenuItem label="学习计划" onPress={() => navigateFromMenu('plan')} />
            <SideMenuItem label="鳄bti" onPress={() => navigateFromMenu('crocBti')} />
            <SideMenuItem label="错词本" onPress={() => navigateFromMenu('wrongWords')} />
            <SideMenuItem label="学习报告" onPress={() => navigateFromMenu('reports')} />
          </View>
        </View>
      ) : null}

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

function CrocBtiEntryScreen({
  onBack,
  onApplied,
}: {
  onBack: () => void;
  onApplied: () => void;
}): React.JSX.Element {
  const [plan, setPlan] = useState<PlanSummary | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    void (async () => {
      try {
        setPlan(await fetchActivePlan());
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  if (loading) {
    return (
      <View style={styles.center}>
        <ActivityIndicator size="large" color="#007AFF" />
      </View>
    );
  }

  if (!plan) {
    return (
      <View style={styles.center}>
        <Text style={styles.emptyText}>还没有可应用的学习计划</Text>
        <TouchableOpacity style={styles.emptyButton} onPress={onBack}>
          <Text style={styles.emptyButtonText}>返回</Text>
        </TouchableOpacity>
      </View>
    );
  }

  return <CrocBtiScreen plan={plan} onBack={onBack} onApplied={onApplied} />;
}

function SideMenuItem({
  label,
  onPress,
}: {
  label: string;
  onPress: () => void;
}): React.JSX.Element {
  return (
    <TouchableOpacity style={styles.sideMenuItem} onPress={onPress}>
      <Text style={styles.sideMenuItemText}>{label}</Text>
    </TouchableOpacity>
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
  center: {flex: 1, justifyContent: 'center', alignItems: 'center', padding: 24},
  menuButton: {
    position: 'absolute',
    top: 12,
    left: 12,
    zIndex: 5,
    backgroundColor: '#fff',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: '#e0e0e0',
    paddingHorizontal: 10,
    paddingVertical: 6,
  },
  menuButtonText: {fontSize: 13, color: '#333', fontWeight: '700'},
  sideMenuOverlay: {
    ...StyleSheet.absoluteFillObject,
    zIndex: 20,
    flexDirection: 'row',
  },
  sideMenuScrim: {flex: 1, backgroundColor: 'rgba(0,0,0,0.22)'},
  sideMenu: {
    position: 'absolute',
    left: 0,
    top: 0,
    bottom: 0,
    width: 240,
    backgroundColor: '#fff',
    paddingTop: 52,
    paddingHorizontal: 16,
  },
  sideMenuTitle: {fontSize: 20, fontWeight: '800', color: '#222', marginBottom: 18},
  sideMenuItem: {
    paddingVertical: 14,
    borderBottomWidth: 1,
    borderBottomColor: '#f0f0f0',
  },
  sideMenuItemText: {fontSize: 16, color: '#333', fontWeight: '600'},
  emptyText: {fontSize: 16, color: '#333', marginBottom: 16},
  emptyButton: {
    backgroundColor: '#007AFF',
    borderRadius: 8,
    paddingHorizontal: 20,
    paddingVertical: 10,
  },
  emptyButtonText: {color: '#fff', fontWeight: '700'},
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
