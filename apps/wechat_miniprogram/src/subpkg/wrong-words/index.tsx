import { useEffect, useMemo, useState } from 'react';
import { Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import { Screen } from '@/components/Screen';
import {
  getCachedWrongWords,
  saveCachedWrongWords,
} from '@/sdk/cloudCacheStore';
import { sdk } from '@/sdk';
import { WrongWordDetail, WrongWordEntry } from '@/sdk/types';

import './index.scss';

type FilterKey = 'all' | 'priority' | 'recent' | 'frequent';

const filters: Array<{ key: FilterKey; label: string }> = [
  { key: 'all', label: '全部' },
  { key: 'priority', label: '高优先级' },
  { key: 'recent', label: '最近错词' },
  { key: 'frequent', label: '高频出错' },
];

function primaryMeaning(word: WrongWordEntry) {
  const first = word.meanings[0];
  if (!first) return '暂无释义';
  if (typeof first === 'string') return first;
  if (typeof first === 'object' && first !== null) {
    const record = first as Record<string, unknown>;
    return String(record.meaningCn ?? record.meaning ?? record.text ?? '暂无释义');
  }
  return String(first);
}

function shortDate(value?: string) {
  if (!value) return '无记录';
  return value.replace('T', ' ').slice(0, 16);
}

function filterWords(words: WrongWordEntry[], filter: FilterKey) {
  if (filter === 'priority') return words.filter((word) => word.priorityScore >= 8);
  if (filter === 'frequent') return words.filter((word) => word.errorCount >= 3);
  if (filter === 'recent') {
    return [...words].sort((left, right) =>
      String(right.lastWrongAt ?? '').localeCompare(String(left.lastWrongAt ?? '')),
    );
  }
  return words;
}

export default function WrongWordsPage() {
  const cached = getCachedWrongWords();
  const [words, setWords] = useState<WrongWordEntry[]>(cached ?? []);
  const [loading, setLoading] = useState(!cached);
  const [filter, setFilter] = useState<FilterKey>('all');
  const [details, setDetails] = useState<Record<number, WrongWordDetail>>({});
  const [expandedEntryId, setExpandedEntryId] = useState<number | undefined>();

  useEffect(() => {
    let disposed = false;
    setLoading(!getCachedWrongWords());
    sdk.wrongWords
      .list()
      .then((nextWords) => {
        if (disposed) return;
        saveCachedWrongWords(nextWords);
        setWords(nextWords);
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
    };
  }, []);

  const filteredWords = useMemo(() => filterWords(words, filter), [filter, words]);
  const summary = useMemo(() => {
    const mistakes = words.reduce((total, word) => total + word.errorCount, 0);
    const priority = words.length
      ? words.reduce((total, word) => total + word.priorityScore, 0) / words.length
      : 0;
    return { mistakes, priority };
  }, [words]);

  function toggleDetail(word: WrongWordEntry) {
    if (expandedEntryId === word.entryId) {
      setExpandedEntryId(undefined);
      return;
    }
    setExpandedEntryId(word.entryId);
    if (!details[word.entryId]) {
      sdk.wrongWords.getDetail(word.entryId)
        .then((detail) => setDetails((current) => ({ ...current, [word.entryId]: detail })))
        .catch(() => undefined);
    }
  }

  function startReinforcement() {
    const entrySourceIds = filteredWords.slice(0, 20).map((word) => String(word.entryId));
    Taro.navigateTo({
      url: `/pages/study/index?mode=wrongWordReinforcement&entrySourceIds=${encodeURIComponent(entrySourceIds.join(','))}`,
    });
  }

  return (
    <Screen title="错词本" activeTab="wrong">
      {loading && <Text className="page-loading">正在同步云端错词...</Text>}

      <View className="wrong-summary">
        <Text className="wrong-summary__title">错词本</Text>
        <Text className="wrong-summary__caption">这里会聚合历史和今天暴露出来的薄弱点，并给出强化优先级。</Text>
        <View className="wrong-summary__metrics">
          <View className="wrong-metric">
            <Text>{words.length}</Text>
            <Text>错词数</Text>
          </View>
          <View className="wrong-metric">
            <Text>{summary.mistakes}</Text>
            <Text>累计错误</Text>
          </View>
          <View className="wrong-metric">
            <Text>{summary.priority.toFixed(1)}</Text>
            <Text>平均优先级</Text>
          </View>
        </View>
      </View>

      <View className="wrong-card">
        <Text className="wrong-card__title">筛选</Text>
        <Text className="wrong-card__caption">先缩小范围，再打开词条详情看风险拆解和错误记录。</Text>
        <View className="wrong-filter-row">
          {filters.map((item) => (
            <View
              key={item.key}
              className={`wrong-filter ${filter === item.key ? 'wrong-filter--active' : ''}`}
              onClick={() => setFilter(item.key)}
            >
              {item.label}
            </View>
          ))}
        </View>
      </View>

      <View className="wrong-card">
        <Text className="wrong-card__title">强化入口</Text>
        <Text className="wrong-card__caption">按当前筛选结果进入错词强化，优先回收高风险词条。</Text>
        <View className="reinforce-button" onClick={startReinforcement}>
          开始错词强化（最多 20 个）
        </View>
      </View>

      <View className="wrong-card">
        <Text className="wrong-card__title">错词列表</Text>
        <Text className="wrong-card__caption">点击某个词条展开详情，再次点击可收起。</Text>
        <View className="wrong-list">
          {filteredWords.map((word) => {
            const expanded = expandedEntryId === word.entryId;
            return (
              <View key={word.entryId} className="wrong-list__item">
                <View
                  className={`wrong-word-tile ${expanded ? 'wrong-word-tile--selected' : ''}`}
                  onClick={() => toggleDetail(word)}
                >
                  <View className="wrong-word-tile__copy">
                    <Text className="wrong-word-tile__word">{word.word}</Text>
                    <Text className="wrong-word-tile__meaning">{primaryMeaning(word)}</Text>
                    <Text className="wrong-word-tile__meta">
                      错误 {word.errorCount} 次 · 优先级 {word.priorityScore.toFixed(1)}
                    </Text>
                    {word.userHint && <Text className="wrong-word-tile__hint">提示：{word.userHint}</Text>}
                  </View>
                  <Text className="wrong-word-tile__score">{word.priorityScore.toFixed(1)}</Text>
                </View>
                {expanded && details[word.entryId] && <WrongDetailCard detail={details[word.entryId]} />}
              </View>
            );
          })}
        </View>
      </View>
    </Screen>
  );
}

function WrongDetailCard({ detail }: { detail: WrongWordDetail }) {
  return (
    <View className="wrong-detail-card">
      <Text className="wrong-detail-card__title">词条详情</Text>
      <Text className="wrong-detail-card__subtitle">{detail.word}</Text>

      {detail.userHint && (
        <View className="wrong-detail-section wrong-detail-section--hint">
          <Text className="wrong-detail-section__title">提示</Text>
          <Text className="wrong-detail-section__body">{detail.userHint}</Text>
        </View>
      )}

      <View className="wrong-detail-section">
        <Text className="wrong-detail-section__title">最近错误</Text>
        {detail.errorHistory.length ? (
          detail.errorHistory.map((item) => (
            <View key={`${item.outcome}-${item.answeredAt}`} className="wrong-detail-row">
              <Text>{item.outcome}</Text>
              <Text>{shortDate(item.answeredAt)}</Text>
            </View>
          ))
        ) : (
          <Text className="wrong-detail-section__body">暂无错误记录</Text>
        )}
      </View>

      <View className="wrong-detail-section">
        <Text className="wrong-detail-section__title">例句</Text>
        {detail.examples.map((example) => (
          <View key={example.sentence} className="wrong-example">
            <Text className="wrong-example__sentence">{example.sentence}</Text>
            {example.translation && <Text className="wrong-example__translation">{example.translation}</Text>}
          </View>
        ))}
      </View>

      {!!detail.relatedWords.length && (
        <View className="wrong-detail-section">
          <Text className="wrong-detail-section__title">关联词</Text>
          <View className="wrong-chip-row">
            {detail.relatedWords.map((word) => (
              <Text key={word} className="wrong-chip">
                {word}
              </Text>
            ))}
          </View>
        </View>
      )}

      {!!detail.rootsAffixes?.length && (
        <View className="wrong-detail-section">
          <Text className="wrong-detail-section__title">词根词缀</Text>
          <View className="wrong-chip-row">
            {detail.rootsAffixes.map((item) => (
              <Text key={item} className="wrong-chip wrong-chip--root">
                {item}
              </Text>
            ))}
          </View>
        </View>
      )}
    </View>
  );
}
