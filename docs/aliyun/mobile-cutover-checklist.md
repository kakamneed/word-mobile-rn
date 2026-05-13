# Mobile Cutover Checklist

Use this after the Aliyun backend is reachable over HTTPS.

## Build Inputs

- API base URL: `https://api.example.com`
- Backend health must return `storeMode: postgres`.
- Use the latest mobile bridge code that prevents cloud aggregate expansion into many local study results.

## Before Building

Run:

```powershell
cd D:\projects\word-mobile-rn
cargo check -p word-platform-mobile
```

If the app build script has a configurable backend URL, set it to the Aliyun HTTPS URL before packaging.

## Phone Validation

For a clean validation:

1. Clear app storage or reinstall the APK.
2. Log in to `2143067337@qq.com` or another test account.
3. Wait for restore to finish.
4. Check wrong words:
   - no word should show dozens or hundreds of errors
   - priority values should stay bounded
5. Check reports:
   - total questions should be close to restored study points, not thousands
   - report snapshots may be rebuilt locally after restore
6. Perform a small new study session.
7. Confirm the new study data uploads to PostgreSQL.

## Rollback

If restore looks wrong:

1. Stop testing immediately.
2. Do not keep repeatedly logging in with the same dirty client.
3. Restore the latest PostgreSQL dump.
4. Rebuild the APK from the latest bridge code.
5. Test with a fresh app install.
