/**
 * Vocabulary Client
 */

import {
  getWordbooks as bridgeGetWordbooks,
  toggleWordbook as bridgeToggleWordbook,
  type WordbookSummary,
} from './mobile-bridge';

export interface Wordbook extends WordbookSummary {
  sourceVersionId: number;
}

export interface VocabularyStatus {
  status: 'idle' | 'checking' | 'updating' | 'failed' | 'uptodate';
  lastCheckedAt: string | null;
  lastUpdatedAt: string | null;
  availableUpdate: boolean;
  errorMessage: string | null;
  fallbackAvailable: boolean;
}

export async function fetchVocabularyStatus(): Promise<VocabularyStatus> {
  return {
    status: 'uptodate',
    lastCheckedAt: new Date().toISOString(),
    lastUpdatedAt: new Date().toISOString(),
    availableUpdate: false,
    errorMessage: null,
    fallbackAvailable: true,
  };
}

export async function fetchWordbooks(): Promise<Wordbook[]> {
  const wordbooks = await bridgeGetWordbooks();
  return wordbooks.map(wordbook => ({
    ...wordbook,
    sourceVersionId: 1,
  }));
}

export async function toggleWordbook(
  wordbookId: number,
  isActive: boolean,
): Promise<void> {
  return bridgeToggleWordbook(wordbookId, isActive);
}

export async function startVocabularyUpdate(): Promise<void> {
  return;
}

export async function checkForUpdates(): Promise<boolean> {
  return false;
}
