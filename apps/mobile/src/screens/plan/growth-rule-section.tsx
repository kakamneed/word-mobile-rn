import React, {useEffect, useMemo, useRef, useState} from 'react';
import {
  LayoutChangeEvent,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import type {GrowthModeKey, GrowthRule} from '../../lib/mobile-bridge';

interface GrowthRuleSectionProps {
  mode: 'shared' | 'perMode';
  sharedRule: GrowthRule;
  perModeRules: Record<GrowthModeKey, GrowthRule>;
  onChangeMode: (mode: 'shared' | 'perMode') => void;
  onChangeSharedRule: (rule: GrowthRule) => void;
  onChangePerModeRule: (mode: GrowthModeKey, rule: GrowthRule) => void;
}

const INTERVAL_OPTIONS = Array.from({length: 30}, (_, index) => index + 1);
const INCREMENT_OPTIONS = Array.from({length: 21}, (_, index) => index);
const ITEM_WIDTH = 72;
const MODE_LABELS: Record<GrowthModeKey, string> = {
  newWord: '新词',
  review: '复习',
  mixedTest: '混合',
  wrongWordReinforcement: '错词',
  rootAffix: '词根词缀',
};

export function GrowthRuleSection({
  mode,
  sharedRule,
  perModeRules,
  onChangeMode,
  onChangeSharedRule,
  onChangePerModeRule,
}: GrowthRuleSectionProps): React.JSX.Element {
  const modeKeys = useMemo(() => Object.keys(MODE_LABELS) as GrowthModeKey[], []);

  return (
    <View style={styles.container}>
      <Text style={styles.title}>增长规则</Text>
      <Text style={styles.description}>
        用连续滑动的方式设置“每 N 天 + M”，也可以切到每个模式单独控制。
      </Text>

      <View style={styles.modeSwitch}>
        <ModeToggle
          active={mode === 'shared'}
          label="全部共用"
          onPress={() => onChangeMode('shared')}
        />
        <ModeToggle
          active={mode === 'perMode'}
          label="分别设置"
          onPress={() => onChangeMode('perMode')}
        />
      </View>

      {mode === 'shared' ? (
        <RuleCard
          title="统一增长规则"
          subtitle="所有模式共用这套规则"
          rule={sharedRule}
          onChange={onChangeSharedRule}
        />
      ) : (
        <View style={styles.modeRuleList}>
          {modeKeys.map(modeKey => (
            <RuleCard
              key={modeKey}
              title={MODE_LABELS[modeKey]}
              subtitle="该模式单独增长"
              rule={perModeRules[modeKey]}
              onChange={rule => onChangePerModeRule(modeKey, rule)}
            />
          ))}
        </View>
      )}
    </View>
  );
}

function ModeToggle({
  active,
  label,
  onPress,
}: {
  active: boolean;
  label: string;
  onPress: () => void;
}): React.JSX.Element {
  return (
    <TouchableOpacity
      style={[styles.modeButton, active && styles.modeButtonActive]}
      onPress={onPress}>
      <Text style={[styles.modeButtonText, active && styles.modeButtonTextActive]}>
        {label}
      </Text>
    </TouchableOpacity>
  );
}

function RuleCard({
  title,
  subtitle,
  rule,
  onChange,
}: {
  title: string;
  subtitle: string;
  rule: GrowthRule;
  onChange: (rule: GrowthRule) => void;
}): React.JSX.Element {
  return (
    <View style={styles.ruleCard}>
      <Text style={styles.ruleTitle}>{title}</Text>
      <Text style={styles.ruleSubtitle}>{subtitle}</Text>

      <WheelStrip
        label="每几天"
        options={INTERVAL_OPTIONS}
        suffix="天"
        value={rule.intervalDays}
        onChange={value => onChange({...rule, intervalDays: value})}
      />

      <WheelStrip
        label="增加多少"
        options={INCREMENT_OPTIONS}
        suffix="个"
        value={rule.increment}
        onChange={value => onChange({...rule, increment: value})}
      />

      <Text style={styles.ruleSummary}>
        每 <Text style={styles.ruleSummaryStrong}>{rule.intervalDays}</Text> 天增加{' '}
        <Text style={styles.ruleSummaryStrong}>{rule.increment}</Text> 个计划单位
      </Text>
    </View>
  );
}

function WheelStrip({
  label,
  options,
  suffix,
  value,
  onChange,
}: {
  label: string;
  options: number[];
  suffix: string;
  value: number;
  onChange: (value: number) => void;
}): React.JSX.Element {
  const scrollRef = useRef<ScrollView | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);
  const sidePadding = Math.max((containerWidth - ITEM_WIDTH) / 2, 0);

  useEffect(() => {
    if (!scrollRef.current) {
      return;
    }
    const index = Math.max(options.indexOf(value), 0);
    scrollRef.current.scrollTo({
      x: index * ITEM_WIDTH,
      animated: false,
    });
  }, [options, value, containerWidth]);

  const handleSnap = (offsetX: number) => {
    const rawIndex = Math.round(offsetX / ITEM_WIDTH);
    const index = Math.max(0, Math.min(options.length - 1, rawIndex));
    const nextValue = options[index];
    if (nextValue !== value) {
      onChange(nextValue);
    }
    scrollRef.current?.scrollTo({
      x: index * ITEM_WIDTH,
      animated: true,
    });
  };

  const handleLayout = (event: LayoutChangeEvent) => {
    setContainerWidth(event.nativeEvent.layout.width);
  };

  return (
    <View style={styles.wheelBlock}>
      <Text style={styles.wheelLabel}>{label}</Text>
      <View style={styles.wheelShell} onLayout={handleLayout}>
        <View pointerEvents="none" style={styles.wheelCenterHighlight} />
        <ScrollView
          ref={scrollRef}
          horizontal
          showsHorizontalScrollIndicator={false}
          snapToInterval={ITEM_WIDTH}
          decelerationRate="fast"
          bounces={false}
          contentContainerStyle={[
            styles.wheelContent,
            {paddingHorizontal: sidePadding},
          ]}
          onMomentumScrollEnd={event =>
            handleSnap(event.nativeEvent.contentOffset.x)
          }
          onScrollEndDrag={event =>
            handleSnap(event.nativeEvent.contentOffset.x)
          }>
          {options.map(option => {
            const active = option === value;
            return (
              <View key={`${label}-${option}`} style={styles.wheelItem}>
                <Text style={[styles.wheelValue, active && styles.wheelValueActive]}>
                  {option}
                  {suffix}
                </Text>
              </View>
            );
          })}
        </ScrollView>
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
  title: {fontSize: 18, fontWeight: '600', color: '#222', marginBottom: 8},
  description: {fontSize: 14, color: '#667085', lineHeight: 20, marginBottom: 14},
  modeSwitch: {flexDirection: 'row', gap: 10, marginBottom: 14},
  modeButton: {
    flex: 1,
    borderRadius: 999,
    backgroundColor: '#EDF2FA',
    paddingVertical: 10,
    alignItems: 'center',
  },
  modeButtonActive: {backgroundColor: '#1473E6'},
  modeButtonText: {fontSize: 15, fontWeight: '600', color: '#52637A'},
  modeButtonTextActive: {color: '#fff'},
  modeRuleList: {gap: 12},
  ruleCard: {
    backgroundColor: '#F7FAFF',
    borderRadius: 12,
    padding: 14,
    marginBottom: 12,
  },
  ruleTitle: {fontSize: 16, fontWeight: '600', color: '#22324A'},
  ruleSubtitle: {fontSize: 12, color: '#6B7A90', marginTop: 4, marginBottom: 12},
  wheelBlock: {marginBottom: 14},
  wheelLabel: {fontSize: 14, color: '#44556E', marginBottom: 8},
  wheelShell: {
    position: 'relative',
    backgroundColor: '#EEF3FB',
    borderRadius: 14,
    overflow: 'hidden',
    height: 74,
    justifyContent: 'center',
  },
  wheelCenterHighlight: {
    position: 'absolute',
    top: 8,
    bottom: 8,
    width: ITEM_WIDTH,
    alignSelf: 'center',
    borderRadius: 12,
    backgroundColor: '#FFFFFF',
    shadowColor: '#20304A',
    shadowOpacity: 0.08,
    shadowRadius: 8,
    shadowOffset: {width: 0, height: 2},
    elevation: 2,
  },
  wheelContent: {
    alignItems: 'center',
  },
  wheelItem: {
    width: ITEM_WIDTH,
    alignItems: 'center',
    justifyContent: 'center',
    height: 74,
  },
  wheelValue: {
    fontSize: 22,
    fontWeight: '600',
    color: '#95A2B6',
  },
  wheelValueActive: {
    color: '#111C2D',
    fontSize: 26,
    fontWeight: '700',
  },
  ruleSummary: {fontSize: 14, color: '#55657E'},
  ruleSummaryStrong: {fontWeight: '700', color: '#1473E6'},
});
