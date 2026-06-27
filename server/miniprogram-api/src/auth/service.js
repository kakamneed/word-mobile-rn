import { errorCodes } from '../errors.js';
import {
  createAccessToken,
  createOpaqueToken,
  hashToken,
} from './tokens.js';

const accessTokenExpiryMs = 15 * 60 * 1000;
const refreshTokenExpiryMs = 30 * 24 * 60 * 60 * 1000;

export class AuthService {
  constructor({ adapter, wechatProvider, clock = () => new Date() }) {
    this.adapter = adapter;
    this.wechatProvider = wechatProvider;
    this.clock = clock;
  }

  async loginWithWechatCode(input) {
    validateWechatLoginInput(input);
    const wechatIdentity = await this.wechatProvider.exchangeCode(input.code);
    validateWechatIdentity(wechatIdentity);

    const account = await this.adapter.findOrCreateWechatIdentity({
      openid: wechatIdentity.openid,
      unionid: wechatIdentity.unionid,
      client: input.client,
    });

    return this.issueSessionResponse(account, input.client);
  }

  async refreshSession(refreshToken) {
    if (!refreshToken) {
      throw serviceError(errorCodes.AUTH_REQUIRED, 'Refresh token is required', 401);
    }

    const account = await this.adapter.refreshMiniprogramSession({
      refreshTokenHash: hashToken(refreshToken),
      now: this.clock(),
    });

    return this.issueSessionResponse(account, { platform: 'wechat_mp' });
  }

  async logout(sessionId) {
    if (!sessionId) {
      throw serviceError(errorCodes.AUTH_REQUIRED, 'Session id is required', 401);
    }
    await this.adapter.revokeMiniprogramSession({ sessionId, revokedAt: this.clock() });
    return { ok: true };
  }

  async getMe(internalUserId) {
    if (!internalUserId) {
      throw serviceError(errorCodes.AUTH_REQUIRED, 'Authentication is required', 401);
    }
    const account = await this.adapter.getAccountState({ internalUserId });
    return toAccountState(account);
  }

  async startEmailBind(internalUserId, input) {
    if (!internalUserId) {
      throw serviceError(errorCodes.AUTH_REQUIRED, 'Authentication is required', 401);
    }
    const normalizedEmail = normalizeEmail(input?.email);
    if (!normalizedEmail) {
      throw serviceError(errorCodes.VALIDATION_FAILED, 'Valid email is required');
    }
    return this.adapter.startEmailBind({
      internalUserId,
      normalizedEmail,
      now: this.clock(),
    });
  }

  async verifyEmailBind(internalUserId, input) {
    if (!internalUserId) {
      throw serviceError(errorCodes.AUTH_REQUIRED, 'Authentication is required', 401);
    }
    if (!input?.challengeId || !input?.token) {
      throw serviceError(
        errorCodes.EMAIL_BIND_CHALLENGE_INVALID,
        'Email binding challenge and token are required',
      );
    }
    return this.adapter.verifyEmailBind({
      internalUserId,
      challengeId: input.challengeId,
      token: input.token,
      now: this.clock(),
    });
  }

  async getMergePreview(internalUserId, mergeDecisionId) {
    if (!internalUserId) {
      throw serviceError(errorCodes.AUTH_REQUIRED, 'Authentication is required', 401);
    }
    return this.adapter.getMergePreview({ internalUserId, mergeDecisionId });
  }

  async confirmMerge(internalUserId, input) {
    if (!internalUserId) {
      throw serviceError(errorCodes.AUTH_REQUIRED, 'Authentication is required', 401);
    }
    if (!input?.mergeDecisionId) {
      throw serviceError(errorCodes.VALIDATION_FAILED, 'Merge decision id is required');
    }
    return this.adapter.confirmMerge({
      internalUserId,
      mergeDecisionId: input.mergeDecisionId,
    });
  }

  async issueSessionResponse(account, client) {
    const now = this.clock();
    const refreshToken = createOpaqueToken('wmp_refresh');
    const session = await this.adapter.issueMiniprogramSession({
      internalUserId: account.internalUserId,
      refreshTokenHash: hashToken(refreshToken),
      expiresAt: new Date(now.getTime() + refreshTokenExpiryMs),
      client,
    });
    const accessToken = createAccessToken(account.internalUserId, session.sessionId, now);

    return {
      ...toAccountState(account),
      session: {
        accessToken,
        refreshToken,
        expiresAt: new Date(now.getTime() + accessTokenExpiryMs).toISOString(),
      },
    };
  }
}

function validateWechatLoginInput(input) {
  if (!input || typeof input.code !== 'string' || input.code.trim().length < 3) {
    throw serviceError(errorCodes.VALIDATION_FAILED, 'Valid WeChat login code is required');
  }
}

function validateWechatIdentity(identity) {
  if (!identity || typeof identity.openid !== 'string' || identity.openid.length === 0) {
    throw serviceError(errorCodes.WECHAT_RESPONSE_INVALID, 'WeChat response did not include openid', 502);
  }
}

function normalizeEmail(email) {
  if (typeof email !== 'string' || !email.includes('@')) {
    return undefined;
  }
  return email.trim().toLowerCase();
}

function toAccountState(account) {
  return {
    user: {
      internalUserId: account.internalUserId,
      primaryIdentity: account.primaryIdentity,
      hasEmailBinding: account.hasEmailBinding,
      hasWechatBinding: account.hasWechatBinding,
    },
    accountState: {
      phase: account.phase,
      needsBindDecision: account.needsBindDecision,
      cloudDataState: account.cloudDataState,
    },
  };
}

function serviceError(code, message, statusCode = 400) {
  const error = new Error(message);
  error.code = code;
  error.statusCode = statusCode;
  return error;
}
