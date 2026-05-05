package com.wordmobile;

import android.content.Context;
import android.content.res.AssetManager;
import android.util.Log;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import org.json.JSONException;
import org.json.JSONObject;

public final class RustBridge {
  private static final String TAG = "RustBridge";
  private static final String LIB_NAME = "word_platform_mobile";
  private static final boolean LIBRARY_LOADED;

  private static boolean initialized = false;

  static {
    boolean loaded = false;
    try {
      System.loadLibrary(LIB_NAME);
      loaded = true;
    } catch (UnsatisfiedLinkError error) {
      Log.w(TAG, "Rust library is not available yet; Flutter bridge will continue to fail safely.", error);
    }
    LIBRARY_LOADED = loaded;
  }

  private RustBridge() {}

  public static synchronized String initialize(Context context) {
    if (!LIBRARY_LOADED) {
      return "library_not_loaded";
    }
    if (initialized) {
      return "";
    }

    File filesDir = context.getFilesDir();
    File cacheDir = context.getCacheDir();
    File noBackupDir = context.getNoBackupFilesDir();
    File bundledResourcesDir = new File(filesDir, "bundled_resources");
    copyBundledAssetDirectory(context.getAssets(), "seed-vocab", new File(bundledResourcesDir, "seed-vocab"));
    copyBundledAssetDirectory(context.getAssets(), "seed-medical", new File(bundledResourcesDir, "seed-medical"));
    String result =
        nativeInit(
            filesDir.getAbsolutePath(),
            noBackupDir.getAbsolutePath(),
            cacheDir.getAbsolutePath(),
            bundledResourcesDir.getAbsolutePath());

    if (result == null || result.isEmpty()) {
      initialized = true;
      return "";
    }
    return result;
  }

  private static void copyBundledAssetDirectory(AssetManager assets, String assetPath, File targetDir) {
    try {
      String[] children = assets.list(assetPath);
      if (children == null || children.length == 0) {
        copyBundledAssetFile(assets, assetPath, targetDir);
        return;
      }
      if (!targetDir.exists() && !targetDir.mkdirs()) {
        Log.w(TAG, "Failed to create asset target dir: " + targetDir.getAbsolutePath());
        return;
      }
      for (String child : children) {
        copyBundledAssetDirectory(assets, assetPath + "/" + child, new File(targetDir, child));
      }
    } catch (IOException error) {
      Log.w(TAG, "Failed to copy asset directory: " + assetPath, error);
    }
  }

  private static void copyBundledAssetFile(AssetManager assets, String assetPath, File targetFile) {
    File parent = targetFile.getParentFile();
    if (parent != null && !parent.exists() && !parent.mkdirs()) {
      Log.w(TAG, "Failed to create asset file parent: " + parent.getAbsolutePath());
      return;
    }
    try (InputStream input = assets.open(assetPath); FileOutputStream output = new FileOutputStream(targetFile)) {
      byte[] buffer = new byte[8192];
      int read;
      while ((read = input.read(buffer)) != -1) {
        output.write(buffer, 0, read);
      }
    } catch (IOException error) {
      Log.w(TAG, "Failed to copy asset file: " + assetPath, error);
    }
  }

  public static JSONObject getBridgeStatus() {
    JSONObject status = new JSONObject();
    try {
      status.put("rustLibraryLoaded", LIBRARY_LOADED);
      status.put("rustInitialized", initialized);
      if (!LIBRARY_LOADED) {
        return status;
      }

      String payload = nativeGetBridgeStatus();
      if (payload != null && !payload.isEmpty()) {
        JSONObject rustPayload = new JSONObject(payload);
        rustPayload.put("rustLibraryLoaded", true);
        rustPayload.put("rustInitialized", initialized);
        return rustPayload;
      }
    } catch (JSONException error) {
      Log.w(TAG, "Failed to decode Rust bridge status.", error);
    } catch (RuntimeException error) {
      Log.w(TAG, "Failed to query Rust bridge status.", error);
    }
    return status;
  }

  public static String getBootstrapState() {
    return requirePayload(nativeGetBootstrapState(), "getBootstrapState");
  }

  public static void markOnboardingCompleted() {
    String result = nativeMarkOnboardingCompleted();
    if (result != null && !result.isEmpty()) {
      throw new IllegalStateException("Rust markOnboardingCompleted failed: " + result);
    }
  }

  public static String getTodayHomeState() {
    return requirePayload(nativeGetTodayHomeState(), "getTodayHomeState");
  }

  public static String getTodayRewardState() {
    return requirePayload(nativeGetTodayRewardState(), "getTodayRewardState");
  }

  public static String drawTodayReward(String requestJson) {
    return requirePayload(nativeDrawTodayReward(requestJson), "drawTodayReward");
  }

  public static String getActivePlan() {
    return requirePayload(nativeGetActivePlan(), "getActivePlan");
  }

  public static String savePlan(String requestJson) {
    return requirePayload(nativeSavePlan(requestJson), "savePlan");
  }

  public static String applySavedPlanToToday() {
    return requirePayload(nativeApplySavedPlanToToday(), "applySavedPlanToToday");
  }

  public static String getWordbooks() {
    return requirePayload(nativeGetWordbooks(), "getWordbooks");
  }

  public static String getReportsOverview() {
    return requirePayload(nativeGetReportsOverview(), "getReportsOverview");
  }

  public static String getResumeSessionHint() {
    return requirePayload(nativeGetResumeSessionHint(), "getResumeSessionHint");
  }

  public static String getSyncStatus() {
    return requirePayload(nativeGetSyncStatus(), "getSyncStatus");
  }

  public static String recordSyncResult(String requestJson) {
    return requirePayload(nativeRecordSyncResult(requestJson), "recordSyncResult");
  }

  public static String recordCloudRestoreAttempt(String requestJson) {
    return requirePayload(nativeRecordCloudRestoreAttempt(requestJson), "recordCloudRestoreAttempt");
  }

  public static String enqueueCloudBackfill(String requestJson) {
    return requirePayload(nativeEnqueueCloudBackfill(requestJson), "enqueueCloudBackfill");
  }

  public static String preserveGuestLocalData() {
    return requirePayload(nativePreserveGuestLocalData(), "preserveGuestLocalData");
  }

  public static String reconcileLocalDataOwner(String requestJson) {
    return requirePayload(nativeReconcileLocalDataOwner(requestJson), "reconcileLocalDataOwner");
  }

  public static String restoreCloudDataSnapshot(String requestJson) {
    return requirePayload(nativeRestoreCloudDataSnapshot(requestJson), "restoreCloudDataSnapshot");
  }

  public static String getWrongWords(String filter) {
    return requirePayload(nativeGetWrongWords(filter), "getWrongWords");
  }

  public static String getWrongWordDetail(int entryId) {
    return requirePayload(nativeGetWrongWordDetail(entryId), "getWrongWordDetail");
  }

  public static String saveWordHint(String requestJson) {
    return requirePayload(nativeSaveWordHint(requestJson), "saveWordHint");
  }

  public static String getWordHintSuggestions(int entryId) {
    return requirePayload(nativeGetWordHintSuggestions(entryId), "getWordHintSuggestions");
  }

  public static String getTodayAiPassageContext() {
    return requirePayload(nativeGetTodayAiPassageContext(), "getTodayAiPassageContext");
  }

  public static String getAiPassageHistory() {
    return requirePayload(nativeGetAiPassageHistory(), "getAiPassageHistory");
  }

  public static String getAiPassage(String passageId) {
    return requirePayload(nativeGetAiPassage(passageId), "getAiPassage");
  }

  public static String generateAiPassage(String requestJson) {
    return requirePayload(nativeGenerateAiPassage(requestJson), "generateAiPassage");
  }

  public static String analyzeWrongWordImport(String requestJson) {
    return requirePayload(nativeAnalyzeWrongWordImport(requestJson), "analyzeWrongWordImport");
  }

  public static String commitWrongWordImport(String requestJson) {
    return requirePayload(nativeCommitWrongWordImport(requestJson), "commitWrongWordImport");
  }

  public static void toggleWordbook(int wordbookId, boolean isActive) {
    String result = nativeToggleWordbook(wordbookId, isActive);
    if (result != null && !result.isEmpty()) {
      throw new IllegalStateException("Rust toggleWordbook failed: " + result);
    }
  }

  public static String startStudySession(String requestJson) {
    return requirePayload(nativeStartStudySession(requestJson), "startStudySession");
  }

  public static String submitStudyAnswer(String requestJson) {
    return requirePayload(nativeSubmitStudyAnswer(requestJson), "submitStudyAnswer");
  }

  public static String completeStudySession(String sessionId) {
    return requirePayload(nativeCompleteStudySession(sessionId), "completeStudySession");
  }

  public static void cancelStudySession(String sessionId) {
    String result = nativeCancelStudySession(sessionId);
    if (result != null && !result.isEmpty()) {
      throw new IllegalStateException("Rust cancelStudySession failed: " + result);
    }
  }

  private static String requirePayload(String payload, String methodName) {
    if (payload == null || payload.isEmpty()) {
      throw new IllegalStateException("Rust " + methodName + " returned empty payload");
    }
    if (payload.startsWith("ERROR:")) {
      throw new IllegalStateException(payload.substring("ERROR:".length()));
    }
    return payload;
  }

  private static native String nativeInit(
      String app_data_dir,
      String app_config_dir,
      String app_cache_dir,
      String bundle_resource_dir);

  private static native String nativeGetBridgeStatus();

  private static native String nativeGetBootstrapState();

  private static native String nativeGetTodayHomeState();

  private static native String nativeGetTodayRewardState();

  private static native String nativeDrawTodayReward(String requestJson);

  private static native String nativeMarkOnboardingCompleted();

  private static native String nativeGetActivePlan();

  private static native String nativeSavePlan(String requestJson);

  private static native String nativeApplySavedPlanToToday();

  private static native String nativeGetWordbooks();

  private static native String nativeToggleWordbook(int wordbookId, boolean isActive);

  private static native String nativeGetReportsOverview();

  private static native String nativeGetResumeSessionHint();

  private static native String nativeGetSyncStatus();

  private static native String nativeRecordSyncResult(String requestJson);

  private static native String nativeRecordCloudRestoreAttempt(String requestJson);

  private static native String nativeEnqueueCloudBackfill(String requestJson);

  private static native String nativePreserveGuestLocalData();

  private static native String nativeReconcileLocalDataOwner(String requestJson);

  private static native String nativeRestoreCloudDataSnapshot(String requestJson);

  private static native String nativeGetWrongWords(String filter);

  private static native String nativeGetWrongWordDetail(int entryId);

  private static native String nativeSaveWordHint(String requestJson);

  private static native String nativeGetWordHintSuggestions(int entryId);

  private static native String nativeGetTodayAiPassageContext();

  private static native String nativeGetAiPassageHistory();

  private static native String nativeGetAiPassage(String passageId);

  private static native String nativeGenerateAiPassage(String requestJson);

  private static native String nativeAnalyzeWrongWordImport(String requestJson);

  private static native String nativeCommitWrongWordImport(String requestJson);

  private static native String nativeStartStudySession(String requestJson);

  private static native String nativeSubmitStudyAnswer(String requestJson);

  private static native String nativeCompleteStudySession(String sessionId);

  private static native String nativeCancelStudySession(String sessionId);
}
