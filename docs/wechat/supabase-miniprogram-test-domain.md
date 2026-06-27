# Supabase Mini Program Test Endpoint

## Current Test Base URL

The WeChat mini program HTTP mode defaults to the Supabase Edge Functions root:

```text
https://pmevdtgogsudnfgxjgiz.supabase.co/functions/v1
```

This is the value embedded by `apps/wechat_miniprogram/config/index.ts` when:

```text
TARO_APP_SDK_MODE=http
TARO_APP_API_BASE_URL is not set
```

Do not use the Supabase REST table endpoint as the mini program backend base:

```text
https://pmevdtgogsudnfgxjgiz.supabase.co/rest/v1
```

The mini program expects app backend routes such as:

```text
/v1/auth/wechat-mp/login
/v1/today
/v1/plan/active
```

So the Supabase side should expose these routes from Edge Functions or another API gateway.

## WeChat Request Domain

For direct Supabase testing, add this request legal domain in the WeChat Mini Program console:

```text
https://pmevdtgogsudnfgxjgiz.supabase.co
```

The legal domain must be an origin only. Do not add `/functions/v1` or `/rest/v1`.

## DNSPod Notes

For the current test setup, DNSPod is not required.

If a custom domain is later needed, prefer a dedicated API subdomain:

```text
api.wordnova.fun
```

The DNS CNAME value must be a hostname only:

```text
pmevdtgogsudnfgxjgiz.supabase.co
```

Do not enter protocol or path in DNS:

```text
https://pmevdtgogsudnfgxjgiz.supabase.co/rest/v1/
```

Custom HTTPS for `api.wordnova.fun` requires the target platform to issue a certificate for that host. Supabase custom domains may require Supabase's custom domain support; plain DNS CNAME alone is not enough to guarantee HTTPS.

## Build Commands

Mock mode:

```powershell
$env:TARO_APP_SDK_MODE="mock"
npm.cmd run build:weapp
```

HTTP mode against the default Supabase Functions root:

```powershell
$env:TARO_APP_SDK_MODE="http"
Remove-Item Env:TARO_APP_API_BASE_URL -ErrorAction SilentlyContinue
npm.cmd run build:weapp
```

HTTP mode with an explicit backend:

```powershell
$env:TARO_APP_SDK_MODE="http"
$env:TARO_APP_API_BASE_URL="https://pmevdtgogsudnfgxjgiz.supabase.co/functions/v1"
npm.cmd run build:weapp
```

## Deploy Edge Function Gateway

The mini program currently calls routes such as:

```text
https://pmevdtgogsudnfgxjgiz.supabase.co/functions/v1/v1/auth/wechat-mp/login
```

In Supabase, the first `v1` after `/functions/v1/` is the Edge Function name. Deploy the gateway function:

```powershell
supabase functions deploy v1 --project-ref pmevdtgogsudnfgxjgiz
```

For real WeChat identity exchange, set these function secrets:

```powershell
supabase secrets set WECHAT_MP_APPID=wx6cb366eba492c2ca --project-ref pmevdtgogsudnfgxjgiz
supabase secrets set WECHAT_MP_SECRET=your_wechat_app_secret --project-ref pmevdtgogsudnfgxjgiz
```

Without those secrets, the gateway uses a dev openid derived from the WeChat login code so the mini program can verify the HTTP login flow without showing the local `demo-user`.
