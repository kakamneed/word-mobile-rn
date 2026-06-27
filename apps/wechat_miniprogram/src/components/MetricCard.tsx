import { Text, View } from '@tarojs/components';

import './MetricCard.scss';

interface MetricCardProps {
  label: string;
  value: string | number;
  tone?: 'default' | 'good' | 'warn';
}

export function MetricCard({ label, value, tone = 'default' }: MetricCardProps) {
  return (
    <View className={`metric metric--${tone}`}>
      <Text className="metric__value">{value}</Text>
      <Text className="metric__label">{label}</Text>
    </View>
  );
}
