import { errorCodes } from '../errors.js';
import { decodeAccessToken } from '../auth/tokens.js';
import { sessionAudience } from '../auth/contracts.js';

export function authContextFromHeaders(headers) {
  const auth = headers.authorization ?? headers.Authorization;
  if (!auth || !auth.startsWith('Bearer ')) {
    return undefined;
  }
  const payload = decodeAccessToken(auth.slice('Bearer '.length));
  if (!payload || payload.aud !== sessionAudience) {
    return undefined;
  }
  return {
    internalUserId: payload.sub,
    sessionId: payload.sid,
    tokenPayload: payload,
  };
}

export function requireAuth(headers) {
  const context = authContextFromHeaders(headers);
  if (!context) {
    const error = new Error('Authentication is required');
    error.code = errorCodes.AUTH_REQUIRED;
    error.statusCode = 401;
    throw error;
  }
  return context;
}
