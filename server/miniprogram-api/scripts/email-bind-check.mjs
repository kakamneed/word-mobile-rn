import { AuthService } from '../src/auth/service.js';
import { MemorySupabaseAdapter } from '../src/supabase/memory-adapter.js';
import { createFakeWechatProvider } from '../src/wechat/fake-provider.js';

const adapter = new MemorySupabaseAdapter();
const service = new AuthService({
  adapter,
  wechatProvider: createFakeWechatProvider(),
});

const login = await service.loginWithWechatCode({
  code: 'bind-user',
  appVersion: '1.0.0',
  client: { platform: 'wechat_mp' },
});

const start = await service.startEmailBind(login.user.internalUserId, {
  email: 'New.User@Example.com',
});
const bound = await service.verifyEmailBind(login.user.internalUserId, {
  challengeId: start.challengeId,
  token: '123456',
});
if (bound.bindingState !== 'bound' || !bound.user.hasEmailBinding) {
  throw new Error('Expected non-conflicting email bind to succeed');
}

adapter.seedEmailAccount('old.user@example.com');
const conflictLogin = await service.loginWithWechatCode({
  code: 'conflict-user',
  appVersion: '1.0.0',
  client: { platform: 'wechat_mp' },
});
const conflictStart = await service.startEmailBind(conflictLogin.user.internalUserId, {
  email: 'old.user@example.com',
});
const conflict = await service.verifyEmailBind(conflictLogin.user.internalUserId, {
  challengeId: conflictStart.challengeId,
  token: '123456',
});
if (conflict.bindingState !== 'merge_decision_required') {
  throw new Error('Expected existing email owner to require merge decision');
}

const preview = await adapter.getMergePreview({
  mergeDecisionId: conflict.mergeDecisionId,
});
if (!preview.mergePreview.emailUser.hasStudyEvents) {
  throw new Error('Expected merge preview to include existing email user data');
}

console.log(
  JSON.stringify(
    {
      bindState: bound.bindingState,
      conflictState: conflict.bindingState,
      mergeDecisionId: conflict.mergeDecisionId,
    },
    null,
    2,
  ),
);
