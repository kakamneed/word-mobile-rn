import { createServer } from 'node:http';

import { AuthService } from './auth/service.js';
import { createRouter } from './http/router.js';
import { MemorySupabaseAdapter } from './supabase/memory-adapter.js';
import { RealSupabaseAdapter } from './supabase/real-adapter.js';
import { MemoryStudyDomain } from './study-domain/memory-domain.js';
import { RustRunnerStudyDomain } from './study-domain/rust-runner-domain.js';
import { createFakeWechatProvider } from './wechat/fake-provider.js';

export function createMiniprogramApiServer({
  adapter,
  studyDomain,
  wechatProvider,
  clock,
} = {}) {
  const resolvedAdapter = adapter ?? createDefaultAdapter();
  const authService = new AuthService({
    adapter: resolvedAdapter,
    wechatProvider: wechatProvider ?? createFakeWechatProvider(),
    clock,
  });
  const fallbackStudyDomain = new MemoryStudyDomain({ adapter: resolvedAdapter });
  const resolvedStudyDomain =
    studyDomain ??
    (process.env.STUDY_DOMAIN_RUNNER === 'rust'
      ? new RustRunnerStudyDomain({
          adapter: resolvedAdapter,
          fallbackDomain: fallbackStudyDomain,
        })
      : fallbackStudyDomain);
  return createServer(
    createRouter({
      authService,
      adapter: resolvedAdapter,
      studyDomain: resolvedStudyDomain,
    }),
  );
}

function createDefaultAdapter() {
  if (process.env.SUPABASE_URL && process.env.SUPABASE_SERVICE_ROLE_KEY) {
    return RealSupabaseAdapter.fromEnv(process.env);
  }
  return new MemorySupabaseAdapter();
}
