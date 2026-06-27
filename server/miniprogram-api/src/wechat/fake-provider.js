export function createFakeWechatProvider(mapping = {}) {
  return {
    async exchangeCode(code) {
      if (mapping[code]) {
        return mapping[code];
      }
      return {
        openid: `openid_${code}`,
        unionid: `unionid_${code}`,
      };
    },
  };
}
