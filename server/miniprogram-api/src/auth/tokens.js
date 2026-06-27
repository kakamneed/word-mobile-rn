import { createHash, randomBytes } from 'node:crypto';

import { sessionAudience } from './contracts.js';

export function createOpaqueToken(prefix) {
  return `${prefix}_${randomBytes(24).toString('base64url')}`;
}

export function hashToken(token) {
  return createHash('sha256').update(token).digest('hex');
}

export function createAccessToken(internalUserId, sessionId, now = new Date()) {
  const issuedAt = Math.floor(now.getTime() / 1000);
  const expiresAt = issuedAt + 15 * 60;
  const payload = {
    sub: internalUserId,
    sid: sessionId,
    aud: sessionAudience,
    iat: issuedAt,
    exp: expiresAt,
    identity_provider: 'wechat_mp',
  };

  return Buffer.from(JSON.stringify(payload), 'utf8').toString('base64url');
}

export function decodeAccessToken(token) {
  try {
    return JSON.parse(Buffer.from(token, 'base64url').toString('utf8'));
  } catch (_error) {
    return undefined;
  }
}
