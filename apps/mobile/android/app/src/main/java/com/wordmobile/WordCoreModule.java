package com.wordmobile;

import android.content.res.AssetManager;
import android.content.Context;
import android.content.SharedPreferences;
import com.facebook.react.bridge.Promise;
import com.facebook.react.bridge.ReactApplicationContext;
import com.facebook.react.bridge.ReactContextBaseJavaModule;
import com.facebook.react.bridge.ReactMethod;
import java.io.BufferedReader;
import java.io.File;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Date;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

public class WordCoreModule extends ReactContextBaseJavaModule {
  private static final int QUESTION_ENGINE_VERSION = 2;
  private static final String PREFS = "word_core_state";
  private static final String KEY_SAVED_PLAN = "saved_plan";
  private static final String KEY_TODAY_PLAN = "today_plan";
  private static final String KEY_WORDBOOKS = "wordbooks";
  private static final String KEY_SAVED_WORDBOOKS = "saved_wordbooks";
  private static final String KEY_TODAY_WORDBOOKS = "today_wordbooks";
  private static final String KEY_COMPLETED = "completed";
  private static final String KEY_SESSIONS = "sessions";
  private static final String KEY_WRONG_POOL = "wrong_pool";
  private static final String KEY_LEARNED = "learned";
  private static final String KEY_HISTORY = "history";
  private static final String KEY_AI_HISTORY = "ai_history";
  private static final String KEY_FIRST_ACTIVE_DATE = "first_active_date";
  private static final String[] BOOK_ASSET_FILES = {"CET4_3.json", "CET6_3.json", "KaoYan_3.json", "MEDICAL_RESP.json"};
  private static final int[] BOOK_IDS = {1, 2, 3, 4};
  private static final String MEDICAL_ROOT_AFFIX_ASSET = "seed-medical/medical-root-affix.txt";
  private static final String[] MODES = {"newWord", "review", "mixedTest", "wrongWordReinforcement", "rootAffix"};
  private static final String[] ROOT_IDS = {"root_bio", "root_tele", "root_micro", "root_photo"};
  private static final String[][] ROOTS = {
      {"bio-", "生命", "biology, biography, antibiotic", "biology（生物学）, biography（传记）, antibiotic（抗生素）"},
      {"tele-", "远", "telephone, television, teleport", "telephone（电话）, television（电视）, teleport（远距离传送）"},
      {"micro-", "微小", "microbe, microscope, microchip", "microbe（微生物）, microscope（显微镜）, microchip（微型芯片）"},
      {"photo-", "光", "photo, photon, photograph", "photo（照片）, photon（光子）, photograph（摄影作品）"}
  };
  private static final Card[] CARDS = {
      new Card(1, "adapt", "适应", "/əˈdæpt/", "We must adapt to change.", "我们必须适应变化。"),
      new Card(1, "clarify", "澄清", "/ˈklærəfaɪ/", "Please clarify your answer.", "请澄清你的答案。"),
      new Card(1, "retain", "保留", "/rɪˈteɪn/", "Please retain this note.", "请保留这份笔记。"),
      new Card(1, "modify", "修改", "/ˈmɑːdəfaɪ/", "They need to modify the plan.", "他们需要修改计划。"),
      new Card(1, "approach", "方法", "/əˈprəʊtʃ/", "Try a different approach.", "试试不同的方法。"),
      new Card(1, "structure", "结构", "/ˈstrʌktʃər/", "The structure is stable.", "这个结构很稳定。"),
      new Card(1, "analyze", "分析", "/ˈænəlaɪz/", "We should analyze the feedback first.", "我们应该先分析反馈。"),
      new Card(1, "assess", "评估", "/əˈses/", "Please assess the current risk.", "请评估当前风险。"),
      new Card(1, "revise", "修订", "/rɪˈvaɪz/", "They need to revise the draft.", "他们需要修订草稿。"),
      new Card(1, "organize", "组织", "/ˈɔːrɡənaɪz/", "Organize your notes by topic.", "按主题整理你的笔记。"),
      new Card(1, "concept", "概念", "/ˈkɒnsept/", "The concept is easier than it looks.", "这个概念比看起来更容易。"),
      new Card(1, "pattern", "模式", "/ˈpætərn/", "We noticed a new pattern today.", "我们今天注意到一个新模式。"),
      new Card(2, "coherent", "连贯的", "/kəʊˈhɪərənt/", "Her explanation was coherent.", "她的解释很连贯。"),
      new Card(2, "resilient", "有韧性的", "/rɪˈzɪliənt/", "Resilient teams recover quickly.", "有韧性的团队恢复得更快。"),
      new Card(2, "concise", "简洁的", "/kənˈsaɪs/", "Keep the summary concise.", "让总结保持简洁。"),
      new Card(2, "flexible", "灵活的", "/ˈfleksəbəl/", "A flexible plan can handle change.", "灵活的计划能应对变化。"),
      new Card(2, "accurate", "准确的", "/ˈækjərət/", "We need accurate numbers.", "我们需要准确的数据。"),
      new Card(2, "logical", "合乎逻辑的", "/ˈlɒdʒɪkəl/", "Her argument was logical.", "她的论证很合乎逻辑。"),
      new Card(2, "context", "语境", "/ˈkɒntekst/", "Context helps you understand the sentence.", "语境能帮助你理解句子。"),
      new Card(2, "priority", "优先级", "/praɪˈɒrəti/", "Set the priority before you begin.", "开始前先设定优先级。"),
      new Card(2, "strategy", "策略", "/ˈstrætədʒi/", "This strategy saves time.", "这个策略能节省时间。"),
      new Card(2, "summary", "总结", "/ˈsʌməri/", "Write a short summary after reading.", "阅读后写一段简短总结。"),
      new Card(3, "infer", "推断", "/ɪnˈfɜːr/", "We can infer the result from the data.", "我们可以从数据中推断结果。"),
      new Card(3, "compile", "整理", "/kəmˈpaɪl/", "Compile the report before noon.", "请在中午前整理报告。"),
      new Card(3, "deduce", "推导", "/dɪˈdjuːs/", "It is easier to deduce the cause from the evidence.", "从证据中更容易推导原因。"),
      new Card(3, "interpret", "解释", "/ɪnˈtɜːrprɪt/", "Please interpret the result carefully.", "请仔细解释结果。"),
      new Card(3, "confirm", "证实", "/kənˈfɜːrm/", "We need to confirm the final result.", "我们需要证实最终结果。"),
      new Card(3, "estimate", "估计", "/ˈestɪmeɪt/", "Try to estimate the total cost first.", "先试着估计总成本。"),
      new Card(3, "evidence", "证据", "/ˈevɪdəns/", "The evidence supports the conclusion.", "这些证据支持结论。"),
      new Card(3, "analysis", "分析", "/əˈnæləsɪs/", "The analysis explains the change clearly.", "分析清楚解释了这种变化。"),
      new Card(3, "conclusion", "结论", "/kənˈkluːʒən/", "The conclusion is based on enough data.", "这个结论基于足够的数据。"),
      new Card(3, "dataset", "数据集", "/ˈdeɪtəset/", "This dataset contains useful signals.", "这个数据集包含有用信号。"),
      new Card(4, "clinical", "临床的", "/ˈklɪnɪkəl/", "She works in a clinical team.", "她在临床团队工作。"),
      new Card(4, "therapy", "治疗", "/ˈθerəpi/", "The therapy was effective.", "这种治疗很有效。"),
      new Card(4, "diagnosis", "诊断", "/ˌdaɪəɡˈnəʊsɪs/", "The diagnosis was confirmed yesterday.", "诊断结果昨天确认了。"),
      new Card(4, "symptom", "症状", "/ˈsɪmptəm/", "Fever is a common symptom.", "发热是常见症状。"),
      new Card(4, "recovery", "康复", "/rɪˈkʌvəri/", "Her recovery was faster than expected.", "她的康复比预期更快。"),
      new Card(4, "treatment", "治疗方案", "/ˈtriːtmənt/", "The treatment plan needs adjustment.", "治疗方案需要调整。"),
      new Card(4, "prescribe", "开处方", "/prɪˈskraɪb/", "Doctors prescribe medicine carefully.", "医生会谨慎开处方。"),
      new Card(4, "patient", "病人", "/ˈpeɪʃənt/", "The patient waited in the hall.", "病人在大厅里等候。"),
      new Card(4, "chronic", "慢性的", "/ˈkrɒnɪk/", "Chronic pain needs long-term care.", "慢性疼痛需要长期护理。"),
      new Card(4, "acute", "急性的", "/əˈkjuːt/", "The acute illness required quick action.", "急性疾病需要快速处理。"),
      new Card(4, "infection", "感染", "/ɪnˈfekʃən/", "The infection spread quickly.", "感染扩散得很快。"),
      new Card(4, "vaccine", "疫苗", "/ˈvæksiːn/", "The vaccine was given yesterday.", "疫苗昨天已经接种。")
  };
  private List<Card> vocabCardsCache = null;
  private List<RootAffixCard> rootAffixCardsCache = null;

  public WordCoreModule(ReactApplicationContext context) {
    super(context);
    ensureDefaults();
    new Thread(() -> {
      try {
        cards();
        rootAffixCards();
      } catch (Exception ignored) {}
    }).start();
  }

  @Override
  public String getName() {
    return "WordCoreModule";
  }

  @ReactMethod
  public void getBootstrapState(Promise promise) {
    try {
      ensureDefaults();
      if (RustBridge.isLibraryLoaded()) {
        if (prefs().getBoolean("onboarding_completed", false) || hasMeaningfulLocalProgress()) {
          prefs().edit().putBoolean("onboarding_completed", true).apply();
          RustBridge.markOnboardingCompleted();
        }
        promise.resolve(RustBridge.getBootstrapState());
        return;
      }
      JSONObject o = new JSONObject();
      o.put("appReady", true);
      o.put("firstRunRequired", !prefs().getBoolean("onboarding_completed", false));
      o.put("databaseStatus", "ready");
      o.put("snapshotStatus", "ready");
      o.put("connectivityStatus", "offline");
      o.put("aiConfigStatus", "missing");
      o.put("settingsEntryAvailable", true);
      o.put("blockingReason", JSONObject.NULL);
      o.put("rustBridgeStatus", RustBridge.getBridgeStatus());
      promise.resolve(o.toString());
    } catch (Exception e) {
      promise.reject("BOOTSTRAP_ERROR", e);
    }
  }

  @ReactMethod
  public void markOnboardingCompleted(Promise promise) {
    try {
      prefs().edit().putBoolean("onboarding_completed", true).apply();
      if (RustBridge.isLibraryLoaded()) {
        RustBridge.markOnboardingCompleted();
      }
      promise.resolve(null);
    } catch (Exception e) {
      promise.reject("ONBOARDING_COMPLETE_ERROR", e);
    }
  }

  @ReactMethod
  public void getTodayHomeState(Promise promise) {
    try {
      ensureDefaults();
      String today = today();
      pruneSessions(today);
      if (RustBridge.isLibraryLoaded()) {
        JSONObject request = new JSONObject();
        request.put("todayDate", today);
        request.put("activePlan", plan(true));
        request.put("wordbooks", wordbooks(true));
        request.put("targets", targetSeed(today));
        request.put("completions", new JSONObject()
            .put("newWordsCompleted", done(today, "newWord"))
            .put("reviewWordsCompleted", done(today, "review"))
            .put("mixedTestCompleted", done(today, "mixedTest"))
            .put("wrongWordTestCompleted", done(today, "wrongWordReinforcement"))
            .put("rootAffixCompleted", done(today, "rootAffix")));
        promise.resolve(RustBridge.buildTodayHomeState(request.toString()));
        return;
      }
      JSONObject o = new JSONObject();
      o.put("todayDate", today);
      o.put("activePlan", plan(true));
      o.put("todaySnapshot", snapshot(today));
      o.put("wordbooks", wordbooks(true));
      o.put("dailyProgress", progress(today));
      promise.resolve(o.toString());
    } catch (Exception e) {
      promise.reject("TODAY_ERROR", e);
    }
  }

  @ReactMethod
  public void getSettings(Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.getSettings());
        return;
      }
      JSONObject o = new JSONObject();
      o.put("aiConfigured", false);
      o.put("syncConfigured", false);
      o.put("appVersion", BuildConfig.VERSION_NAME);
      o.put("schemaVersion", 1);
      o.put("databasePath", new File(getReactApplicationContext().getFilesDir(), "word.db").getAbsolutePath());
      o.put("rustBridgeStatus", RustBridge.getBridgeStatus());
      promise.resolve(o.toString());
    } catch (Exception e) {
      promise.reject("SETTINGS_ERROR", e);
    }
  }

  @ReactMethod
  public void getAiProviderConfig(Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.getAiProviderConfig());
        return;
      }
      promise.reject("AI_PROVIDER_CONFIG_UNAVAILABLE", "Rust bridge is not available");
    } catch (Exception e) {
      promise.reject("AI_PROVIDER_CONFIG_ERROR", e);
    }
  }

  @ReactMethod
  public void saveAiProviderConfig(String requestJson, Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.saveAiProviderConfig(requestJson));
        return;
      }
      promise.reject("AI_PROVIDER_CONFIG_UNAVAILABLE", "Rust bridge is not available");
    } catch (Exception e) {
      promise.reject("AI_PROVIDER_CONFIG_SAVE_ERROR", e);
    }
  }

  @ReactMethod
  public void getActivePlan(Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.getActivePlan());
        return;
      }
      promise.resolve(plan(false).toString());
    } catch (Exception e) {
      promise.reject("PLAN_ERROR", e);
    }
  }

  @ReactMethod
  public void savePlan(String requestJson, Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        String payload = RustBridge.savePlan(requestJson);
        write(KEY_SAVED_PLAN, new JSONObject(payload));
        promise.resolve(payload);
        return;
      } else {
        JSONObject input = new JSONObject(requestJson).optJSONObject("input");
        if (input == null) {
          throw new IllegalArgumentException("Missing plan input.");
        }
        JSONObject next = mergePlan(plan(false), input);
        write(KEY_SAVED_PLAN, next);
        promise.resolve(next.toString());
      }
    } catch (Exception e) {
      promise.reject("SAVE_PLAN_ERROR", e);
    }
  }

  @ReactMethod
  public void applySavedPlanToToday(Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        String payload = RustBridge.applySavedPlanToToday();
        JSONObject applied = new JSONObject(payload);
        write(KEY_TODAY_PLAN, applied);
        write(wordbooksKey(true), new JSONObject(read(wordbooksKey(false)).toString()));
        resetTodayStudyState(today());
        promise.resolve(payload);
        return;
      } else {
        JSONObject applied = new JSONObject(plan(false).toString());
        write(KEY_TODAY_PLAN, applied);
        write(wordbooksKey(true), new JSONObject(read(wordbooksKey(false)).toString()));
        resetTodayStudyState(today());
        promise.resolve(applied.toString());
      }
    } catch (Exception e) {
      promise.reject("APPLY_PLAN_ERROR", e);
    }
  }

  @ReactMethod
  public void getWordbooks(Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.getWordbooks());
        return;
      }
      promise.resolve(wordbooks(false).toString());
    } catch (Exception e) {
      promise.reject("WORDBOOK_ERROR", e);
    }
  }

  @ReactMethod
  public void toggleWordbook(int wordbookId, boolean isActive, Promise promise) {
    try {
      JSONObject o = read(wordbooksKey(false));
      o.put(String.valueOf(wordbookId), isActive);
      write(wordbooksKey(false), o);
      if (RustBridge.isLibraryLoaded()) {
        RustBridge.toggleWordbook(wordbookId, isActive);
      }
      promise.resolve(null);
    } catch (Exception e) {
      promise.reject("TOGGLE_WORDBOOK_ERROR", e);
    }
  }

  @ReactMethod
  public void startStudySession(String requestJson, Promise promise) {
    try {
      ensureDefaults();
      String today = today();
      pruneSessions(today);
      JSONObject req = new JSONObject(requestJson);
      String mode = req.optString("mode", "newWord");
      JSONObject sessions = read(KEY_SESSIONS);
      JSONObject existing = sessions.optJSONObject(mode);
      String basisSignature = studyBasisSignature();
      if (shouldUseRustStudy(mode)) {
        JSONArray expectedIds = resolveEntrySourceIds(mode, req.optJSONArray("entrySourceIds"));
        if (!canReuseRustSelection(existing, mode, today, basisSignature, expectedIds)) {
          if (existing != null && existing.optBoolean("rustManaged")) {
            try {
              RustBridge.cancelStudySession(existing.optString("sessionId"));
            } catch (Exception ignored) {}
          }
          sessions.remove(mode);
          existing = null;
        }
        JSONArray selectedIds = existing != null && existing.optJSONArray("selectedEntryIds") != null
            ? existing.optJSONArray("selectedEntryIds")
            : expectedIds;
        JSONObject effectiveReq = new JSONObject(req.toString());
        effectiveReq.put("entrySourceIds", selectedIds);
        effectiveReq.put("entryPayloads", buildEntryPayloads(mode, selectedIds));
        effectiveReq.put("distractorPayloads", buildDistractorPayloads(mode, selectedIds));
        String payload = RustBridge.startStudySession(effectiveReq.toString());
        JSONObject rustResponse = new JSONObject(payload);
        JSONObject rustSession = rustResponse.getJSONObject("session");
        JSONObject mirror = buildRustSessionMirror(
            mode,
            rustSession,
            selectedIds,
            today,
            basisSignature);
        sessions.put(mode, mirror);
        write(KEY_SESSIONS, sessions);
        promise.resolve(payload);
        return;
      }
      if (existing != null
          && !existing.optBoolean("completed")
          && today.equals(existing.optString("targetDate"))
          && basisSignature.equals(existing.optString("basisSignature"))) {
        promise.resolve(startPayload(existing).toString());
        return;
      }
      JSONArray qs = buildQuestions(mode, req.optJSONArray("entrySourceIds"));
      if (qs.length() == 0) {
        throw new IllegalStateException("No study content for " + mode);
      }
      JSONObject s = new JSONObject();
      s.put("sessionId", mode + "-session-" + System.currentTimeMillis());
      s.put("mode", mode);
      s.put("totalWords", distinctCount(qs));
      s.put("startedAt", now());
      s.put("targetDate", today);
      s.put("nextIndex", 0);
      s.put("correctCount", 0);
      s.put("incorrectCount", 0);
      s.put("totalTimeMs", 0);
      s.put("completed", false);
      s.put("basisSignature", basisSignature);
      s.put("wrongIds", new JSONArray());
      s.put("questions", qs);
      sessions.put(mode, s);
      write(KEY_SESSIONS, sessions);
      promise.resolve(startPayload(s).toString());
    } catch (Exception e) {
      promise.reject("START_SESSION_ERROR", e);
    }
  }

  private boolean canReuseRustSelection(JSONObject existing, String mode, String today, String basisSignature, JSONArray expectedIds) {
    if (existing == null || existing.optJSONArray("selectedEntryIds") == null) {
      return false;
    }
    if (existing.optInt("questionEngineVersion", 0) != QUESTION_ENGINE_VERSION) {
      return false;
    }
    if (existing.optBoolean("completed")) {
      return false;
    }
    if (!today.equals(existing.optString("targetDate"))) {
      return false;
    }
    if (!basisSignature.equals(existing.optString("basisSignature"))) {
      return false;
    }
    JSONArray selectedIds = existing.optJSONArray("selectedEntryIds");
    return selectedIds != null && sameIdArray(selectedIds, expectedIds);
  }

  private boolean sameIdArray(JSONArray left, JSONArray right) {
    if (left == null || right == null || left.length() != right.length()) {
      return false;
    }
    for (int i = 0; i < left.length(); i += 1) {
      if (!left.optString(i).equals(right.optString(i))) {
        return false;
      }
    }
    return true;
  }

  @ReactMethod
  public void submitStudyAnswer(String requestJson, Promise promise) {
    try {
      JSONObject req = new JSONObject(requestJson);
      String questionId = req.optString("questionId");
      JSONObject sessions = read(KEY_SESSIONS);
      String mode = findSessionModeForQuestionId(sessions, questionId);
      JSONObject s = mode == null ? null : sessions.optJSONObject(mode);
      if (s == null) {
        throw new IllegalStateException("No active session for question " + questionId);
      }
      if (s.optBoolean("rustManaged")) {
        String payload = RustBridge.submitStudyAnswer(requestJson);
        JSONObject rustPayload = new JSONObject(payload);
        updateRustSessionMirror(mode, s, req, rustPayload);
        sessions.put(mode, s);
        write(KEY_SESSIONS, sessions);
        promise.resolve(payload);
        return;
      }
      JSONArray qs = s.getJSONArray("questions");
      int index = s.optInt("nextIndex");
      if (index >= qs.length()) {
        JSONObject summary = s.optJSONObject("summary");
        JSONObject payload = new JSONObject();
        payload.put("result", JSONObject.NULL);
        payload.put("isComplete", true);
        payload.put("currentQuestion", JSONObject.NULL);
        payload.put("summary", summary == null ? JSONObject.NULL : summary);
        payload.put("nextAction", nextHint(today()));
        payload.put("progress", progressObj(qs.length(), qs.length()));
        promise.resolve(payload.toString());
        return;
      }
      JSONObject q = qs.getJSONObject(index);
      String response = req.optString("response", "");
      String outcome = judgeOutcome(q, normalize(response));
      boolean correct = "correct".equals(outcome) || "fuzzyCorrect".equals(outcome);
      s.put("nextIndex", index + 1);
      s.put("totalTimeMs", s.optInt("totalTimeMs") + req.optInt("responseTimeMs"));
      String counter =
          "correct".equals(outcome)
              ? "correctCount"
              : ("fuzzyCorrect".equals(outcome) ? "fuzzyCorrectCount" : "incorrectCount");
      s.put(counter, s.optInt(counter) + 1);
      if ("incorrect".equals(outcome)) {
        pushUnique(s.getJSONArray("wrongIds"), q.optString("entrySourceId"));
      }

      JSONObject result = new JSONObject();
      result.put("questionId", questionId);
      result.put("entrySourceId", q.optString("entrySourceId"));
      result.put("questionType", q.optString("questionType"));
      result.put("userResponse", response);
      result.put("normalizedResponse", normalize(response));
      result.put("correctAnswer", answerText(q));
      result.put("outcome", outcome);
      result.put("responseTimeMs", req.optInt("responseTimeMs"));
      result.put("answeredAt", now());
      appendSessionResult(s, result);

      JSONObject payload = new JSONObject();
      payload.put("result", result);
      boolean done = s.optInt("nextIndex") >= qs.length();
      if (done) {
        s.put("completed", true);
        JSONObject summary = summary(s);
        s.put("summary", summary);
        addDone(today(), mode, qs.length());
        appendHistory(s, summary);
        if ("newWord".equals(mode)) {
          growLearnedPool(sessionSourceIds(s));
        }
        growWrongPoolFromResults(s.optJSONArray("results"));
        payload.put("isComplete", true);
        payload.put("currentQuestion", JSONObject.NULL);
        payload.put("summary", summary);
        payload.put("nextAction", nextHint(today()));
        payload.put("progress", progressObj(qs.length(), qs.length()));
      } else {
        payload.put("isComplete", false);
        payload.put("currentQuestion", inflate(qs.getJSONObject(s.optInt("nextIndex")), s.getString("sessionId"), s.optInt("nextIndex"), qs.length()));
        payload.put("summary", JSONObject.NULL);
        payload.put("nextAction", JSONObject.NULL);
        payload.put("progress", progressObj(s.optInt("nextIndex") + 1, qs.length()));
      }
      sessions.put(mode, s);
      write(KEY_SESSIONS, sessions);
      promise.resolve(payload.toString());
    } catch (Exception e) {
      promise.reject("SUBMIT_ANSWER_ERROR", e);
    }
  }

  @ReactMethod
  public void completeStudySession(String sessionId, Promise promise) {
    try {
      JSONObject sessions = read(KEY_SESSIONS);
      JSONObject found = null;
      String foundMode = null;
      for (String mode : MODES) {
        JSONObject candidate = sessions.optJSONObject(mode);
        if (candidate != null && sessionId.equals(candidate.optString("sessionId"))) {
          found = candidate;
          foundMode = mode;
          sessions.remove(mode);
          break;
        }
      }
      if (found == null) {
        throw new IllegalStateException("Session not found: " + sessionId);
      }
      write(KEY_SESSIONS, sessions);
      if (found.optBoolean("rustManaged")) {
        promise.resolve(RustBridge.completeStudySession(sessionId));
        return;
      }
      JSONObject payload = new JSONObject();
      payload.put("summary", found.optJSONObject("summary") != null ? found.optJSONObject("summary") : summary(found));
      payload.put("nextAction", nextHint(today()));
      promise.resolve(payload.toString());
    } catch (Exception e) {
      promise.reject("COMPLETE_SESSION_ERROR", e);
    }
  }

  @ReactMethod
  public void cancelStudySession(String sessionId, Promise promise) {
    try {
      JSONObject sessions = read(KEY_SESSIONS);
      String today = today();
      String foundMode = null;
      JSONObject candidate = null;
      for (String mode : MODES) {
        JSONObject current = sessions.optJSONObject(mode);
        if (current != null && sessionId.equals(current.optString("sessionId"))) {
          foundMode = mode;
          candidate = current;
          break;
        }
      }
      if (candidate == null || foundMode == null) {
        promise.resolve(null);
        return;
      }
      if (!candidate.optBoolean("completed")) {
        int answered = Math.max(candidate.optInt("nextIndex"), 0);
        if (answered > 0) {
          addDone(today, foundMode, answered);
        }
      }
      if (candidate.optBoolean("rustManaged")) {
        RustBridge.cancelStudySession(sessionId);
      }
      sessions.remove(foundMode);
      write(KEY_SESSIONS, sessions);
      promise.resolve(null);
    } catch (Exception e) {
      promise.reject("CANCEL_SESSION_ERROR", e);
    }
  }

  @ReactMethod
  public void getReportsOverview(Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        JSONObject request = new JSONObject()
            .put("todayDate", today())
            .put("history", readArray(KEY_HISTORY))
            .put("learnedCount", read(KEY_LEARNED).length());
        promise.resolve(RustBridge.buildReportsOverview(request.toString()));
        return;
      }
      promise.resolve(buildReportsOverview().toString());
    } catch (Exception e) {
      promise.reject("REPORTS_ERROR", e);
    }
  }

  @ReactMethod
  public void getWrongWords(String filter, Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        JSONObject request = new JSONObject()
            .put("filter", filter)
            .put("entries", buildWrongWordSeed());
        promise.resolve(RustBridge.buildWrongWords(request.toString()));
        return;
      }
      promise.resolve(buildWrongWordEntries(filter).toString());
    } catch (Exception e) {
      promise.reject("WRONG_WORDS_ERROR", e);
    }
  }

  @ReactMethod
  public void getWrongWordDetail(int entryId, Promise promise) {
    try {
      JSONObject detail = buildWrongWordDetail(entryId);
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.buildWrongWordDetail(detail.toString()));
        return;
      }
      promise.resolve(detail.toString());
    } catch (Exception e) {
      promise.reject("WRONG_WORD_DETAIL_ERROR", e);
    }
  }

  @ReactMethod
  public void getTodayAiPassageContext(Promise promise) {
    try {
      String currentDate = today();
      JSONObject snapshot = snapshot(currentDate);
      JSONArray wrongWords = collectDayWrongWords(currentDate);
      if (RustBridge.isLibraryLoaded()) {
        JSONObject request = new JSONObject()
            .put("date", currentDate)
            .put("snapshot", snapshot)
            .put("wrongWords", wrongWords);
        promise.resolve(RustBridge.buildTodayAiPassageContext(request.toString()));
        return;
      }
      boolean tasksComplete =
          snapshot.optInt("newWordsCompleted") >= snapshot.optInt("newWordsTarget")
              && snapshot.optInt("reviewWordsCompleted") >= snapshot.optInt("reviewWordsTarget")
              && snapshot.optInt("mixedTestCompleted") >= snapshot.optInt("mixedTestTarget")
              && snapshot.optInt("wrongWordTestCompleted") >= snapshot.optInt("wrongWordTestTarget")
              && snapshot.optInt("rootAffixCompleted") >= snapshot.optInt("rootAffixTarget");
      promise.resolve(
          new JSONObject()
              .put("date", currentDate)
              .put("tasksComplete", tasksComplete)
              .put("wrongWords", wrongWords)
              .toString());
    } catch (Exception e) {
      promise.reject("TODAY_AI_CONTEXT_ERROR", e);
    }
  }

  @ReactMethod
  public void generateAiPassage(String requestJson, Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.generateAiPassage(requestJson));
        return;
      }
      JSONObject request = new JSONObject(requestJson);
      promise.resolve(createAiPassage(request).toString());
    } catch (Exception e) {
      promise.reject("AI_PASSAGE_ERROR", e);
    }
  }

  @ReactMethod
  public void getAiPassageHistory(Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.getAiPassageHistory());
        return;
      }
      JSONArray history = readArray(KEY_AI_HISTORY);
      JSONArray result = new JSONArray();
      for (int i = history.length() - 1; i >= 0; i -= 1) {
        JSONObject item = history.getJSONObject(i);
        result.put(
            new JSONObject()
                .put("passageId", item.optString("passageId"))
                .put("title", item.optString("title"))
                .put("preview", item.optString("preview"))
                .put("wordCount", item.optInt("wordCount"))
                .put("generatedAt", item.optString("generatedAt"))
                .put("validationStatus", item.optString("validationStatus", "pending")));
      }
      promise.resolve(result.toString());
    } catch (Exception e) {
      promise.reject("AI_HISTORY_ERROR", e);
    }
  }

  @ReactMethod
  public void getAiPassage(String passageId, Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        promise.resolve(RustBridge.getAiPassage(passageId));
        return;
      }
      JSONArray history = readArray(KEY_AI_HISTORY);
      for (int i = history.length() - 1; i >= 0; i -= 1) {
        JSONObject item = history.getJSONObject(i);
        if (passageId.equals(item.optString("passageId"))) {
          promise.resolve(item.toString());
          return;
        }
      }
      promise.resolve("null");
    } catch (Exception e) {
      promise.reject("AI_PASSAGE_LOOKUP_ERROR", e);
    }
  }

  @ReactMethod
  public void saveAiPassage(String requestJson, Promise promise) {
    try {
      if (RustBridge.isLibraryLoaded()) {
        RustBridge.saveAiPassage(requestJson);
        promise.resolve(null);
        return;
      }
      JSONArray history = readArray(KEY_AI_HISTORY);
      JSONObject incoming = new JSONObject(requestJson);
      JSONArray next = new JSONArray();
      next.put(incoming);
      for (int i = 0; i < history.length(); i += 1) {
        JSONObject item = history.getJSONObject(i);
        if (!incoming.optString("passageId").equals(item.optString("passageId"))) {
          next.put(item);
        }
      }
      writeArray(KEY_AI_HISTORY, next);
      promise.resolve(null);
    } catch (Exception e) {
      promise.reject("AI_PASSAGE_SAVE_ERROR", e);
    }
  }

  private SharedPreferences prefs() { return getReactApplicationContext().getSharedPreferences(PREFS, Context.MODE_PRIVATE); }
  private JSONObject read(String key) { try { return new JSONObject(prefs().getString(key, "{}")); } catch (Exception e) { return new JSONObject(); } }
  private JSONArray readArray(String key) { try { return new JSONArray(prefs().getString(key, "[]")); } catch (Exception e) { return new JSONArray(); } }
  private void write(String key, JSONObject value) { prefs().edit().putString(key, value.toString()).apply(); }
  private void writeArray(String key, JSONArray value) { prefs().edit().putString(key, value.toString()).apply(); }
  private String today() { return new SimpleDateFormat("yyyy-MM-dd", Locale.US).format(new Date()); }
  private String now() { return new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.US).format(new Date()); }
  private String normalize(String v) { return v == null ? "" : v.trim().replace("，", "").replace(",", "").replace("。", "").replace(".", "").replace(" ", "").toLowerCase(Locale.ROOT); }

  private void ensureDefaults() {
    if (!prefs().contains(KEY_SAVED_PLAN)) { write(KEY_SAVED_PLAN, defaultPlan()); }
    if (!prefs().contains(KEY_TODAY_PLAN)) { write(KEY_TODAY_PLAN, defaultPlan()); }
    JSONObject migratedWordbooks = prefs().contains(KEY_WORDBOOKS) ? read(KEY_WORDBOOKS) : defaultWordbooksSelection();
    if (!prefs().contains(wordbooksKey(false))) { write(wordbooksKey(false), migratedWordbooks); }
    if (!prefs().contains(wordbooksKey(true))) { write(wordbooksKey(true), migratedWordbooks); }
    if (!prefs().contains(KEY_COMPLETED)) { write(KEY_COMPLETED, new JSONObject()); }
    if (!prefs().contains(KEY_SESSIONS)) { write(KEY_SESSIONS, new JSONObject()); }
    if (!prefs().contains(KEY_WRONG_POOL)) { write(KEY_WRONG_POOL, new JSONObject()); }
    if (!prefs().contains(KEY_LEARNED)) { write(KEY_LEARNED, new JSONObject()); }
    if (!prefs().contains(KEY_HISTORY)) { prefs().edit().putString(KEY_HISTORY, "[]").apply(); }
    if (!prefs().contains(KEY_AI_HISTORY)) { prefs().edit().putString(KEY_AI_HISTORY, "[]").apply(); }
    if (!prefs().contains(KEY_FIRST_ACTIVE_DATE)) { prefs().edit().putString(KEY_FIRST_ACTIVE_DATE, today()).apply(); }
  }

  private boolean hasMeaningfulLocalProgress() {
    return read(KEY_COMPLETED).length() > 0
        || read(KEY_LEARNED).length() > 0
        || read(KEY_WRONG_POOL).length() > 0
        || readArray(KEY_HISTORY).length() > 0;
  }

  private JSONObject defaultPlan() {
    JSONObject p = new JSONObject();
    try {
      String today = today();
      JSONObject sharedGrowthRule = new JSONObject()
          .put("intervalDays", 7)
          .put("increment", 5);
      JSONObject growthRuleStartDatesByMode = new JSONObject()
          .put("newWord", today)
          .put("review", today)
          .put("mixedTest", today)
          .put("wrongWordReinforcement", today)
          .put("rootAffix", today);
      JSONObject growthRulesByMode = new JSONObject()
          .put("newWord", new JSONObject(sharedGrowthRule.toString()))
          .put("review", new JSONObject(sharedGrowthRule.toString()))
          .put("mixedTest", new JSONObject(sharedGrowthRule.toString()))
          .put("wrongWordReinforcement", new JSONObject(sharedGrowthRule.toString()))
          .put("rootAffix", new JSONObject(sharedGrowthRule.toString()));
      p.put("id", 1); p.put("name", "Starter Plan"); p.put("newWordsPerDay", 5); p.put("reviewWordsPerDay", 6); p.put("mixedTestPerDay", 4); p.put("wrongWordTestPerDay", 3); p.put("rootAffixPerDay", 2); p.put("growthIntervalDays", 7); p.put("growthIncrement", 5);
      p.put("growthRuleMode", "shared");
      p.put("growthRuleStartDate", today);
      p.put("sharedGrowthRule", sharedGrowthRule);
      p.put("growthRulesByMode", growthRulesByMode);
      p.put("growthRuleStartDatesByMode", growthRuleStartDatesByMode);
    } catch (Exception ignored) {}
    return p;
  }

  private JSONObject defaultWordbooksSelection() {
    JSONObject w = new JSONObject();
    try { w.put("1", true); w.put("2", false); w.put("3", false); w.put("4", false); } catch (Exception ignored) {}
    return w;
  }

  private JSONObject plan(boolean todayPlan) { return read(todayPlan ? KEY_TODAY_PLAN : KEY_SAVED_PLAN); }
  private String wordbooksKey(boolean todayWordbooks) { return todayWordbooks ? KEY_TODAY_WORDBOOKS : KEY_SAVED_WORDBOOKS; }

  private JSONObject mergePlan(JSONObject base, JSONObject input) throws JSONException {
    JSONObject next = new JSONObject(base.toString());
    for (String key : Arrays.asList("name", "newWordsPerDay", "reviewWordsPerDay", "mixedTestPerDay", "wrongWordTestPerDay", "rootAffixPerDay", "growthIntervalDays", "growthIncrement", "growthRuleMode", "sharedGrowthRule", "growthRulesByMode")) {
      if (input.has(key)) { next.put(key, input.get(key)); }
    }
    stampGrowthRuleStartDates(base, input, next);
    return next;
  }

  private void stampGrowthRuleStartDates(JSONObject previous, JSONObject input, JSONObject next) throws JSONException {
    String today = today();
    boolean modeChanged = input.has("growthRuleMode")
        && !input.optString("growthRuleMode", previous.optString("growthRuleMode", "shared"))
            .equals(previous.optString("growthRuleMode", "shared"));
    JSONObject previousShared = previous.optJSONObject("sharedGrowthRule");
    JSONObject nextShared = next.optJSONObject("sharedGrowthRule");
    if (previousShared == null) {
      previousShared = new JSONObject()
          .put("intervalDays", previous.optInt("growthIntervalDays", 7))
          .put("increment", previous.optInt("growthIncrement", 5));
    }
    if (nextShared == null) {
      nextShared = new JSONObject()
          .put("intervalDays", next.optInt("growthIntervalDays", 7))
          .put("increment", next.optInt("growthIncrement", 5));
      next.put("sharedGrowthRule", nextShared);
    }
    if (modeChanged || !sameGrowthRule(previousShared, nextShared)) {
      next.put("growthRuleStartDate", today);
    } else if (!next.has("growthRuleStartDate")) {
      next.put("growthRuleStartDate", previous.optString("growthRuleStartDate", today));
    }

    JSONObject previousByMode = previous.optJSONObject("growthRulesByMode");
    JSONObject nextByMode = next.optJSONObject("growthRulesByMode");
    JSONObject startDates = previous.optJSONObject("growthRuleStartDatesByMode");
    JSONObject nextStartDates = startDates == null ? new JSONObject() : new JSONObject(startDates.toString());
    if (nextByMode == null) {
      nextByMode = new JSONObject();
      next.put("growthRulesByMode", nextByMode);
    }
    for (String mode : MODES) {
      JSONObject previousModeRule = previousByMode == null ? null : previousByMode.optJSONObject(mode);
      JSONObject nextModeRule = nextByMode.optJSONObject(mode);
      if (nextModeRule == null) {
        nextModeRule = new JSONObject(nextShared.toString());
        nextByMode.put(mode, nextModeRule);
      }
      if (modeChanged || previousModeRule == null || !sameGrowthRule(previousModeRule, nextModeRule)) {
        nextStartDates.put(mode, today);
      } else if (!nextStartDates.has(mode)) {
        nextStartDates.put(mode, today);
      }
    }
    next.put("growthRuleStartDatesByMode", nextStartDates);
  }

  private boolean sameGrowthRule(JSONObject left, JSONObject right) {
    if (left == null || right == null) {
      return false;
    }
    return left.optInt("intervalDays", 7) == right.optInt("intervalDays", 7)
        && left.optInt("increment", 5) == right.optInt("increment", 5);
  }

  private JSONArray wordbooks(boolean todayWordbooks) throws JSONException {
    JSONObject active = read(wordbooksKey(todayWordbooks));
    JSONArray arr = new JSONArray();
    arr.put(wordbook(1, "cet4", "CET-4", "exam", 4500, active.optBoolean("1", true)));
    arr.put(wordbook(2, "cet6", "CET-6", "exam", 3000, active.optBoolean("2", false)));
    arr.put(wordbook(3, "kaoyan", "KaoYan", "exam", 5500, active.optBoolean("3", false)));
    arr.put(wordbook(4, "medical", "Medical English", "specialized", 1800, active.optBoolean("4", false)));
    return arr;
  }

  private JSONObject wordbook(int id, String code, String name, String category, int totalEntries, boolean active) throws JSONException {
    JSONObject o = new JSONObject();
    o.put("id", id); o.put("code", code); o.put("name", name); o.put("category", category); o.put("totalEntries", totalEntries); o.put("isActive", active);
    return o;
  }

  private JSONObject snapshot(String today) throws JSONException {
    JSONObject p = plan(true); JSONObject s = new JSONObject();
    s.put("date", today);
    s.put("newWordsTarget", snapshotTargetQuestions(today, p, "newWord")); s.put("newWordsCompleted", done(today, "newWord"));
    s.put("reviewWordsTarget", snapshotTargetQuestions(today, p, "review")); s.put("reviewWordsCompleted", done(today, "review"));
    s.put("mixedTestTarget", snapshotTargetQuestions(today, p, "mixedTest")); s.put("mixedTestCompleted", done(today, "mixedTest"));
    s.put("wrongWordTestTarget", snapshotTargetQuestions(today, p, "wrongWordReinforcement")); s.put("wrongWordTestCompleted", done(today, "wrongWordReinforcement"));
    s.put("rootAffixTarget", snapshotTargetQuestions(today, p, "rootAffix")); s.put("rootAffixCompleted", done(today, "rootAffix"));
    return s;
  }

  private JSONObject progress(String today) throws JSONException {
    JSONObject s = snapshot(today); JSONObject p = new JSONObject();
    int total = s.optInt("newWordsTarget") + s.optInt("reviewWordsTarget") + s.optInt("mixedTestTarget") + s.optInt("wrongWordTestTarget") + s.optInt("rootAffixTarget");
    int complete = s.optInt("newWordsCompleted") + s.optInt("reviewWordsCompleted") + s.optInt("mixedTestCompleted") + s.optInt("wrongWordTestCompleted") + s.optInt("rootAffixCompleted");
    p.put("totalTasks", total); p.put("completedTasks", complete); p.put("nextRecommendedAction", nextHint(today)); return p;
  }

  private void clamp(String today, JSONObject p) {
    JSONObject all = read(KEY_COMPLETED); JSONObject day = all.optJSONObject(today); if (day == null) { day = new JSONObject(); }
    try {
      day.put("newWord", Math.min(day.optInt("newWord"), targetQuestions(today, p, "newWord")));
      day.put("review", Math.min(day.optInt("review"), targetQuestions(today, p, "review")));
      day.put("mixedTest", Math.min(day.optInt("mixedTest"), targetQuestions(today, p, "mixedTest")));
      day.put("wrongWordReinforcement", Math.min(day.optInt("wrongWordReinforcement"), targetQuestions(today, p, "wrongWordReinforcement")));
      day.put("rootAffix", Math.min(day.optInt("rootAffix"), targetQuestions(today, p, "rootAffix")));
      all.put(today, day); write(KEY_COMPLETED, all);
    } catch (Exception ignored) {}
  }

  private int done(String today, String mode) {
    JSONObject all = read(KEY_COMPLETED); JSONObject day = all.optJSONObject(today); int persisted = day == null ? 0 : day.optInt(mode); JSONObject live = read(KEY_SESSIONS).optJSONObject(mode);
    if (live != null && !studyBasisSignature().equals(live.optString("basisSignature"))) { live = null; }
    if (live == null || live.optBoolean("completed")) { return Math.min(targetQuestions(today, plan(true), mode), persisted); }
    int answered = live.optInt("nextIndex");
    return Math.min(targetQuestions(today, plan(true), mode), persisted + answered);
  }

  private void addDone(String today, String mode, int amount) throws JSONException {
    JSONObject all = read(KEY_COMPLETED); JSONObject day = all.optJSONObject(today); if (day == null) { day = new JSONObject(); }
    int max = targetQuestions(today, plan(true), mode);
    day.put(mode, Math.min(day.optInt(mode) + amount, max)); all.put(today, day); write(KEY_COMPLETED, all);
  }

  private int configuredTarget(JSONObject plan, String mode) {
    return configuredTarget(plan, mode, today());
  }

  private int configuredTarget(JSONObject plan, String mode, String date) {
    int base;
    if ("newWord".equals(mode)) { base = plan.optInt("newWordsPerDay"); }
    else if ("review".equals(mode)) { base = plan.optInt("reviewWordsPerDay"); }
    else if ("mixedTest".equals(mode)) { base = plan.optInt("mixedTestPerDay"); }
    else if ("wrongWordReinforcement".equals(mode)) { base = plan.optInt("wrongWordTestPerDay"); }
    else { base = plan.optInt("rootAffixPerDay"); }
    JSONObject growthRule = growthRuleForMode(plan, mode);
    int intervalDays = Math.max(growthRule.optInt("intervalDays", plan.optInt("growthIntervalDays", 7)), 1);
    int increment = Math.max(growthRule.optInt("increment", plan.optInt("growthIncrement", 5)), 0);
    String effectiveStart = effectiveStartDate(read(KEY_COMPLETED), readArray(KEY_HISTORY), read(KEY_SESSIONS));
    String growthStart = growthRuleStartDateForMode(plan, mode, effectiveStart);
    String startDate = daysBetween(effectiveStart, growthStart) > 0 ? growthStart : effectiveStart;
    int elapsedDays = Math.max(daysBetween(startDate, date), 0);
    int growthSteps = intervalDays <= 0 ? 0 : elapsedDays / intervalDays;
    return Math.max(base + (growthSteps * increment), 0);
  }

  private boolean shouldUseRustStudy(String mode) {
    return RustBridge.isLibraryLoaded()
        && ("newWord".equals(mode)
            || "review".equals(mode)
            || "mixedTest".equals(mode)
            || "wrongWordReinforcement".equals(mode)
            || "rootAffix".equals(mode));
  }

  private JSONArray resolveEntrySourceIds(String mode, JSONArray requested) throws JSONException {
    JSONArray resolved = new JSONArray();
    if ("rootAffix".equals(mode)) {
      List<RootAffixCard> pool = rootAffixCardsForActiveWordbooks();
      if (requested != null && requested.length() > 0) {
        for (int i = 0; i < requested.length(); i += 1) {
          String id = requested.optString(i);
          if (!id.isEmpty()) {
            resolved.put(id);
          }
        }
      } else {
        int limit = Math.min(remainingBundleCount(mode), pool.size());
        int start = rootAffixSelectionOffset(pool.size());
        for (int i = 0; i < limit; i += 1) {
          resolved.put(pool.get((start + i) % pool.size()).id);
        }
      }
      return resolved;
    }
    List<Card> cards = requestedCards(requested);
    if (cards.isEmpty()) {
      cards = rustSelectedCards(mode);
    }
    for (Card card : cards) {
      resolved.put(card.id);
    }
    return resolved;
  }

  private List<Card> rustSelectedCards(String mode) {
    if ("rootAffix".equals(mode)) {
      return new ArrayList<>();
    }
    if ("mixedTest".equals(mode) || "wrongWordReinforcement".equals(mode)) {
      List<Card> pool = "wrongWordReinforcement".equals(mode) ? wrongPoolCards() : mixedTestCardsFast();
      if (pool.isEmpty()) {
        return pool;
      }
      List<Card> selected = new ArrayList<>();
      int target = remainingQuestions(mode);
      for (int i = 0; i < target; i += 1) {
        int index = Math.floorMod((mode + ":" + today() + ":" + i).hashCode(), pool.size());
        selected.add(pool.get(index));
      }
      return selected;
    }
    return autoCards(mode);
  }

  private JSONArray buildEntryPayloads(String mode, JSONArray selectedIds) throws JSONException {
    JSONArray payloads = new JSONArray();
    if ("rootAffix".equals(mode)) {
      for (int i = 0; i < selectedIds.length(); i += 1) {
        RootAffixCard card = rootAffixCardById(selectedIds.optString(i));
        if (card == null) {
          continue;
        }
        String exampleWords = limitRootExamples(card.exampleWords, 3);
        String exampleGlosses = limitRootExamples(card.exampleGlosses, 3);
      payloads.put(new JSONObject()
          .put("sourceId", card.id)
          .put("word", card.form)
          .put("partOfSpeech", JSONObject.NULL)
          .put("frequency", 0.0)
          .put("phoneticUs", JSONObject.NULL)
          .put("phoneticUk", JSONObject.NULL)
          .put("meanings", new JSONArray().put(card.meaningCn))
            .put("exampleSentence", exampleWords == null || exampleWords.isEmpty() ? JSONObject.NULL : exampleWords)
            .put("exampleTranslation", exampleGlosses == null || exampleGlosses.isEmpty() ? JSONObject.NULL : exampleGlosses));
      }
      return payloads;
    }

    for (int i = 0; i < selectedIds.length(); i += 1) {
      Card card = cardById(selectedIds.optString(i));
      if (card == null) {
        continue;
      }
      payloads.put(cardPayload(card));
    }
    return payloads;
  }

  private JSONArray buildDistractorPayloads(String mode, JSONArray selectedIds) throws JSONException {
    JSONArray payloads = new JSONArray();
    if ("rootAffix".equals(mode)) {
      return payloads;
    }

    List<Card> selectedCards = requestedCards(selectedIds);
    Set<String> selectedCardIds = new LinkedHashSet<>();
    for (Card card : selectedCards) {
      selectedCardIds.add(card.id);
    }

    List<Card> universe = distractorUniverse(mode);
    Map<String, Card> ordered = new LinkedHashMap<>();
    for (Card target : selectedCards) {
      for (Card candidate : rankedDistractorCards(target, universe)) {
        if (selectedCardIds.contains(candidate.id)) {
          continue;
        }
        ordered.putIfAbsent(candidate.id, candidate);
        if (ordered.size() >= 24) {
          break;
        }
      }
      if (ordered.size() >= 24) {
        break;
      }
    }

    for (Card candidate : stableOrderCards(mode + ":distractor:" + today(), universe)) {
      if (selectedCardIds.contains(candidate.id)) {
        continue;
      }
      ordered.putIfAbsent(candidate.id, candidate);
      if (ordered.size() >= 24) {
        break;
      }
    }

    for (Card candidate : ordered.values()) {
      payloads.put(cardPayload(candidate));
    }
    return payloads;
  }

  private List<Card> distractorUniverse(String mode) {
    List<Card> universe = new ArrayList<>(activeCards());
    appendUniqueCards(universe, reviewCards());
    if ("mixedTest".equals(mode) || "wrongWordReinforcement".equals(mode)) {
      appendUniqueCards(universe, wrongPoolCards());
    }
    if (universe.isEmpty()) {
      universe.addAll(cards());
    }
    return universe;
  }

  private void appendUniqueCards(List<Card> base, List<Card> candidates) {
    Set<String> existing = new LinkedHashSet<>();
    for (Card card : base) {
      existing.add(card.id);
    }
    for (Card card : candidates) {
      if (existing.add(card.id)) {
        base.add(card);
      }
    }
  }

  private JSONObject cardPayload(Card card) throws JSONException {
    return new JSONObject()
        .put("sourceId", card.id)
        .put("word", card.word)
        .put("partOfSpeech", card.partOfSpeech == null || card.partOfSpeech.isEmpty() ? JSONObject.NULL : card.partOfSpeech)
        .put("frequency", card.frequency)
        .put("phoneticUs", card.phonetic == null || card.phonetic.isEmpty() ? JSONObject.NULL : card.phonetic)
        .put("phoneticUk", card.phonetic == null || card.phonetic.isEmpty() ? JSONObject.NULL : card.phonetic)
        .put("meaningDetails", meaningDetailsArray(card.meaningDetails))
        .put("meanings", new JSONArray(card.meanings))
        .put("exampleSentence", card.exampleEn == null || card.exampleEn.isEmpty() ? JSONObject.NULL : card.exampleEn)
        .put("exampleTranslation", card.exampleCn == null || card.exampleCn.isEmpty() ? JSONObject.NULL : card.exampleCn);
  }

  private JSONArray meaningDetailsArray(List<MeaningDetail> meaningDetails) throws JSONException {
    JSONArray arr = new JSONArray();
    for (MeaningDetail detail : meaningDetails) {
      arr.put(new JSONObject()
          .put("pos", detail.pos == null || detail.pos.isEmpty() ? JSONObject.NULL : detail.pos)
          .put("meaningCn", detail.meaningCn)
          .put("meaningEn", detail.meaningEn == null || detail.meaningEn.isEmpty() ? JSONObject.NULL : detail.meaningEn));
    }
    return arr;
  }

  private JSONObject buildRustSessionMirror(
      String mode,
      JSONObject rustSession,
      JSONArray selectedIds,
      String today,
      String basisSignature) throws JSONException {
    return new JSONObject()
        .put("sessionId", rustSession.optString("sessionId"))
        .put("mode", mode)
        .put("totalWords", rustSession.optInt("totalWords"))
        .put("startedAt", rustSession.optString("startedAt", now()))
        .put("targetDate", today)
        .put("nextIndex", 0)
        .put("correctCount", 0)
        .put("fuzzyCorrectCount", 0)
        .put("incorrectCount", 0)
        .put("totalTimeMs", 0)
        .put("completed", false)
        .put("questionEngineVersion", QUESTION_ENGINE_VERSION)
        .put("basisSignature", basisSignature)
        .put("wrongIds", new JSONArray())
        .put("selectedEntryIds", new JSONArray(selectedIds.toString()))
        .put("rustManaged", true);
  }

  private void updateRustSessionMirror(
      String mode,
      JSONObject session,
      JSONObject request,
      JSONObject payload) throws JSONException {
    JSONObject result = payload.optJSONObject("result");
      if (result != null) {
        String outcome = result.optString("outcome");
        if ("correct".equals(outcome)) {
          session.put("correctCount", session.optInt("correctCount") + 1);
      } else if ("fuzzyCorrect".equals(outcome)) {
        session.put("fuzzyCorrectCount", session.optInt("fuzzyCorrectCount") + 1);
        } else if ("incorrect".equals(outcome)) {
          session.put("incorrectCount", session.optInt("incorrectCount") + 1);
          pushUnique(session.getJSONArray("wrongIds"), result.optString("entrySourceId"));
        }
        appendSessionResult(session, result);
      }
    session.put("totalTimeMs", session.optInt("totalTimeMs") + request.optInt("responseTimeMs"));
    JSONObject progress = payload.optJSONObject("progress");
    int total = progress == null ? 0 : progress.optInt("total");
    int current = progress == null ? 0 : progress.optInt("current");
    session.put("nextIndex", Math.max(Math.min(current - 1, total), 0));

    if (payload.optBoolean("isComplete")) {
      session.put("completed", true);
      JSONObject summary = payload.optJSONObject("summary");
      if (summary != null) {
        session.put("summary", summary);
        addDone(today(), mode, summary.optInt("totalQuestions"));
        appendHistory(session, summary);
      }
      if ("newWord".equals(mode)) {
        growLearnedPool(session.optJSONArray("selectedEntryIds"));
      }
      growWrongPoolFromResults(session.optJSONArray("results"));
    }
  }

  private String limitRootExamples(String value, int maxItems) {
    if (value == null || value.isEmpty()) {
      return "";
    }
    String[] parts = value.split(",");
    List<String> limited = new ArrayList<>();
    for (String part : parts) {
      String trimmed = part.trim();
      if (trimmed.isEmpty()) {
        continue;
      }
      limited.add(trimmed);
      if (limited.size() >= maxItems) {
        break;
      }
    }
    return String.join(", ", limited);
  }

  private String findSessionModeForQuestionId(JSONObject sessions, String questionId) {
    for (String mode : MODES) {
      JSONObject session = sessions.optJSONObject(mode);
      if (session == null) {
        continue;
      }
      String sessionId = session.optString("sessionId");
      if (!sessionId.isEmpty() && questionId.startsWith(sessionId + "_")) {
        return mode;
      }
      if (!session.optBoolean("rustManaged")) {
        JSONArray questions = session.optJSONArray("questions");
        if (questions == null) {
          continue;
        }
        for (int i = 0; i < questions.length(); i += 1) {
          String candidateQuestionId = sessionId + "_q" + i;
          if (questionId.equals(candidateQuestionId)) {
            return mode;
          }
        }
      }
    }
    if (questionId.contains("-session-")) {
      return questionId.substring(0, questionId.indexOf("-session-"));
    }
    return null;
  }

  private JSONObject targetSeed(String date) throws JSONException {
    JSONObject plan = plan(true);
    TargetBreakdown newWord = targetBreakdown(date, plan, "newWord");
    TargetBreakdown review = targetBreakdown(date, plan, "review");
    TargetBreakdown mixed = targetBreakdown(date, plan, "mixedTest");
    TargetBreakdown wrong = targetBreakdown(date, plan, "wrongWordReinforcement");
    TargetBreakdown root = targetBreakdown(date, plan, "rootAffix");
    return new JSONObject()
        .put("newWordsTarget", newWord.total)
        .put("newWordsBaseTarget", newWord.base)
        .put("newWordsCarryoverTarget", newWord.carryover)
        .put("reviewWordsTarget", review.total)
        .put("reviewWordsBaseTarget", review.base)
        .put("reviewWordsCarryoverTarget", review.carryover)
        .put("mixedTestTarget", mixed.total)
        .put("mixedTestBaseTarget", mixed.base)
        .put("mixedTestCarryoverTarget", mixed.carryover)
        .put("wrongWordTestTarget", wrong.total)
        .put("wrongWordTestBaseTarget", wrong.base)
        .put("wrongWordTestCarryoverTarget", wrong.carryover)
        .put("rootAffixTarget", root.total)
        .put("rootAffixBaseTarget", root.base)
        .put("rootAffixCarryoverTarget", root.carryover);
  }

  private int snapshotTargetQuestions(String date, JSONObject plan, String mode) {
    return targetBreakdown(date, plan, mode).total;
  }

  private int targetQuestions(String date, JSONObject plan, String mode) {
    return targetBreakdown(date, plan, mode).total;
  }

  private TargetBreakdown targetBreakdown(String date, JSONObject plan, String mode) {
    JSONObject completed = read(KEY_COMPLETED);
    JSONArray history = readArray(KEY_HISTORY);
    JSONObject sessions = read(KEY_SESSIONS);
    String startDate = effectiveStartDate(completed, history, sessions);
    int availableCapacity = availableQuestionCapacity(mode);
    if (daysBetween(startDate, date) < 0) {
      return new TargetBreakdown(0, 0, 0);
    }
    int base = baseTargetQuestions(plan, mode, date);
    int carryover = 0;
    for (int dayOffset = 1; dayOffset <= 5; dayOffset += 1) {
      String previousDate = shiftDate(date, -dayOffset);
      if (daysBetween(startDate, previousDate) < 0) {
        continue;
      }
      int previousBase = baseTargetQuestions(plan, mode, previousDate);
      int previousDone = persistedDone(completed, previousDate, mode);
      int remaining = Math.max(previousBase - previousDone, 0);
      carryover += carryoverShare(remaining, dayOffset);
    }
    int total = Math.min(base + carryover, availableCapacity);
    int multiplier = questionMultiplier(mode);
    if (multiplier > 1 && total > 0) {
      total = Math.min(divCeil(total, multiplier) * multiplier, availableCapacity);
    }
    return new TargetBreakdown(total, Math.min(base, total), Math.max(total - Math.min(base, total), 0));
  }

  private int persistedDone(JSONObject completed, String date, String mode) {
    JSONObject day = completed.optJSONObject(date);
    return day == null ? 0 : day.optInt(mode);
  }

  private String effectiveStartDate(JSONObject completed, JSONArray history, JSONObject sessions) {
    String startDate = prefs().getString(KEY_FIRST_ACTIVE_DATE, today());
    if (!isValidStudyDate(startDate)) {
      startDate = today();
    }

    for (java.util.Iterator<String> it = completed.keys(); it.hasNext(); ) {
      String date = it.next();
      if (isValidStudyDate(date) && daysBetween(date, startDate) > 0) {
        startDate = date;
      }
    }

    for (int i = 0; i < history.length(); i += 1) {
      JSONObject item = history.optJSONObject(i);
      if (item == null) {
        continue;
      }
      String date = item.optString("date");
      if (isValidStudyDate(date) && daysBetween(date, startDate) > 0) {
        startDate = date;
      }
    }

    for (String mode : MODES) {
      JSONObject session = sessions.optJSONObject(mode);
      if (session == null) {
        continue;
      }
      String startedAt = session.optString("startedAt");
      String startedDate = extractStudyDate(startedAt);
      if (isValidStudyDate(startedDate) && daysBetween(startedDate, startDate) > 0) {
        startDate = startedDate;
      }
    }

    prefs().edit().putString(KEY_FIRST_ACTIVE_DATE, startDate).apply();
    return startDate;
  }

  private String extractStudyDate(String timestamp) {
    if (timestamp == null) {
      return "";
    }
    String trimmed = timestamp.trim();
    if (trimmed.length() >= 10) {
      return trimmed.substring(0, 10);
    }
    return trimmed;
  }

  private boolean isValidStudyDate(String value) {
    return value != null && value.matches("\\d{4}-\\d{2}-\\d{2}");
  }

  private int carryoverShare(int remainingQuestions, int dayOffset) {
    if (remainingQuestions <= 0 || dayOffset < 1 || dayOffset > 5) {
      return 0;
    }
    int baseShare = remainingQuestions / 5;
    int remainder = remainingQuestions % 5;
    return baseShare + (dayOffset <= remainder ? 1 : 0);
  }

  private int baseTargetQuestions(JSONObject plan, String mode, String date) {
    return Math.max(configuredTarget(plan, mode, date) * questionMultiplier(mode), 0);
  }

  private JSONObject growthRuleForMode(JSONObject plan, String mode) {
    JSONObject shared = plan.optJSONObject("sharedGrowthRule");
    if (shared == null) {
      shared = new JSONObject();
      try {
        shared.put("intervalDays", plan.optInt("growthIntervalDays", 7));
        shared.put("increment", plan.optInt("growthIncrement", 5));
      } catch (JSONException ignored) {}
    }
    if (!"perMode".equals(plan.optString("growthRuleMode", "shared"))) {
      return shared;
    }
    JSONObject byMode = plan.optJSONObject("growthRulesByMode");
    if (byMode == null) {
      return shared;
    }
    JSONObject modeRule = byMode.optJSONObject(mode);
    return modeRule != null ? modeRule : shared;
  }

  private String growthRuleStartDateForMode(JSONObject plan, String mode, String fallback) {
    if (!"perMode".equals(plan.optString("growthRuleMode", "shared"))) {
      String sharedStart = plan.optString("growthRuleStartDate", fallback);
      return isValidStudyDate(sharedStart) ? sharedStart : fallback;
    }
    JSONObject byMode = plan.optJSONObject("growthRuleStartDatesByMode");
    if (byMode == null) {
      return fallback;
    }
    String start = byMode.optString(mode, fallback);
    return isValidStudyDate(start) ? start : fallback;
  }

  private int remainingQuestions(String mode) { return Math.max(targetQuestions(today(), plan(true), mode) - done(today(), mode), 0); }
  private int remainingBundleCount(String mode) { return Math.max(divCeil(remainingQuestions(mode), questionMultiplier(mode)), 0); }
  private int questionMultiplier(String mode) { return ("newWord".equals(mode) || "review".equals(mode)) ? 4 : 1; }
  private int divCeil(int value, int divisor) { return divisor <= 0 ? 0 : (value + divisor - 1) / divisor; }
  private int availableQuestionCapacity(String mode) {
    if ("newWord".equals(mode)) { return newWordCards().size() * 4; }
    if ("review".equals(mode)) { return reviewCards().size() * 4; }
    if ("mixedTest".equals(mode)) { return mixedTestCards().size() * 4; }
    if ("wrongWordReinforcement".equals(mode)) { return wrongPoolCards().size() * 4; }
    return rootAffixCardsForActiveWordbooks().size();
  }
  private String studyBasisSignature() {
    return stableJsonValue(plan(true)) + "|" + stableJsonValue(read(wordbooksKey(true)));
  }

  private String stableJsonValue(Object value) {
    if (value == null || value == JSONObject.NULL) {
      return "null";
    }
    if (value instanceof JSONObject) {
      JSONObject object = (JSONObject) value;
      List<String> keys = new ArrayList<>();
      for (java.util.Iterator<String> it = object.keys(); it.hasNext(); ) {
        keys.add(it.next());
      }
      Collections.sort(keys);
      StringBuilder builder = new StringBuilder("{");
      for (int i = 0; i < keys.size(); i += 1) {
        String key = keys.get(i);
        if (i > 0) {
          builder.append(',');
        }
        builder.append(JSONObject.quote(key)).append(':');
        builder.append(stableJsonValue(object.opt(key)));
      }
      return builder.append('}').toString();
    }
    if (value instanceof JSONArray) {
      JSONArray array = (JSONArray) value;
      StringBuilder builder = new StringBuilder("[");
      for (int i = 0; i < array.length(); i += 1) {
        if (i > 0) {
          builder.append(',');
        }
        builder.append(stableJsonValue(array.opt(i)));
      }
      return builder.append(']').toString();
    }
    if (value instanceof String) {
      return JSONObject.quote((String) value);
    }
    return String.valueOf(value);
  }

  private void resetTodayStudyState(String today) throws JSONException {
    write(KEY_SESSIONS, new JSONObject());
    JSONObject completed = read(KEY_COMPLETED);
    completed.remove(today);
    write(KEY_COMPLETED, completed);
  }

  private void pruneSessions(String today) {
    JSONObject sessions = read(KEY_SESSIONS); boolean changed = false;
    String basisSignature = studyBasisSignature();
    for (String mode : MODES) {
      JSONObject s = sessions.optJSONObject(mode);
      if (s != null && (!today.equals(s.optString("targetDate")) || !basisSignature.equals(s.optString("basisSignature")))) {
        sessions.remove(mode); changed = true;
      }
    }
    if (changed) { write(KEY_SESSIONS, sessions); }
  }

  private JSONArray buildQuestions(String mode, JSONArray requested) throws JSONException {
    JSONArray arr = new JSONArray();
    if ("rootAffix".equals(mode)) {
      List<RootAffixCard> pool = rootAffixCardsForActiveWordbooks();
      int limit = requested != null && requested.length() > 0 ? requested.length() : Math.min(remainingBundleCount(mode), pool.size());
      for (int i = 0; i < limit; i += 1) {
        RootAffixCard card = requested != null && requested.length() > i ? rootAffixCardById(requested.optString(i)) : pool.get(i);
        if (card == null) { card = pool.get(i); }
        arr.put(rootQuestion(card, "rootToGlossInput"));
      }
      return arr;
    }
    List<Card> selected = requestedCards(requested);
    if (selected.isEmpty()) { selected = autoCards(mode); }
    List<Card> distractors = distractors(selected);
    if ("newWord".equals(mode) || "review".equals(mode)) {
      String[] typeOrder = {"exampleToCnChoice", "enToCnChoice", "cnToEnChoice", "enToCnInput"};
      for (int round = 0; round < typeOrder.length; round += 1) {
        for (Card c : orderedCardsForRound(selected, mode, round)) {
          arr.put(loopQuestion(typeOrder[round], c, distractors));
        }
      }
    } else if ("mixedTest".equals(mode) || "wrongWordReinforcement".equals(mode)) {
      for (JSONObject question : buildSampledQuestions(mode, selected, distractors)) { arr.put(question); }
    }
    return arr;
  }

  private List<Card> requestedCards(JSONArray requested) {
    List<Card> list = new ArrayList<>(); if (requested == null) { return list; }
    for (int i = 0; i < requested.length(); i += 1) { Card c = cardById(requested.optString(i)); if (c != null) { list.add(c); } }
    return list;
  }

  private List<Card> autoCards(String mode) {
    List<Card> pool;
    if ("newWord".equals(mode)) {
      pool = newWordCards();
    } else if ("review".equals(mode)) {
      pool = reviewCards();
    } else if ("wrongWordReinforcement".equals(mode)) {
      pool = wrongPoolCards();
    } else {
      pool = mixedTestCards();
    }
    if (pool.isEmpty() && !"wrongWordReinforcement".equals(mode)) { pool = activeCards(); }
    if (pool.isEmpty()) { pool = new ArrayList<>(cards()); }
    List<Card> list = stableOrderCards(mode + ":" + today(), pool);
    if ("mixedTest".equals(mode) || "wrongWordReinforcement".equals(mode)) {
      return list;
    }
    return new ArrayList<>(list.subList(0, Math.min(remainingBundleCount(mode), list.size())));
  }

  private List<Card> activeCards() {
    JSONObject active = read(wordbooksKey(true)); List<Card> list = new ArrayList<>();
    for (Card c : cards()) { if (active.optBoolean(String.valueOf(c.bookId), false)) { list.add(c); } }
    return list;
  }

  private List<Card> wrongPoolCards() {
    JSONObject pool = read(KEY_WRONG_POOL); List<Card> list = new ArrayList<>();
    for (Card c : cards()) { if (wrongMeta(pool, c.id).optInt("count", 0) > 0) { list.add(c); } }
    return sortCardsByWrongPriority(list, pool, false);
  }

  private List<Card> newWordCards() {
    JSONObject learned = read(KEY_LEARNED);
    List<Card> unseen = new ArrayList<>();
    for (Card c : activeCards()) {
      if (learned.optInt(c.id, 0) == 0) {
        unseen.add(c);
      }
    }
    return sortCardsByFrequency(unseen.isEmpty() ? activeCards() : unseen);
  }

  private List<Card> reviewCards() {
    JSONObject learned = read(KEY_LEARNED);
    JSONObject wrongPool = read(KEY_WRONG_POOL);
    List<Card> reviewed = sortCardsByWrongPriority(activeCards(), wrongPool, true);
    Set<String> included = new LinkedHashSet<>();
    for (Card c : reviewed) {
      included.add(c.id);
    }
    List<Card> learnedFallback = new ArrayList<>();
    for (Card c : activeCards()) {
      if (learned.optInt(c.id, 0) > 0 && !isSameStudyDate(wrongMeta(wrongPool, c.id).optString("lastWrongAt", ""))) {
        learnedFallback.add(c);
      }
    }
    for (Card c : stableOrderCards("review:fallback:" + today(), learnedFallback)) {
      if (included.add(c.id)) {
        reviewed.add(c);
      }
    }
    if (reviewed.isEmpty()) {
      for (Card c : activeCards()) {
        if (!isSameStudyDate(wrongMeta(wrongPool, c.id).optString("lastWrongAt", ""))) {
          reviewed.add(c);
        }
      }
    }
    return sortCardsByFrequency(reviewed);
  }

  private List<Card> mixedTestCards() {
    List<Card> pool = new ArrayList<>(activeCards());
    List<Card> reviewed = reviewCards();
    for (Card c : reviewed) {
      boolean exists = false;
      for (Card existing : pool) {
        if (existing.id.equals(c.id)) {
          exists = true;
          break;
        }
      }
      if (!exists) {
        pool.add(c);
      }
    }
    return sortCardsByFrequency(stableOrderCards("mixed:" + today(), pool));
  }

  private List<Card> mixedTestCardsFast() {
    List<Card> pool = new ArrayList<>(activeCards());
    Set<String> included = new LinkedHashSet<>();
    for (Card card : pool) {
      included.add(card.id);
    }
    for (Card card : reviewCards()) {
      if (included.add(card.id)) {
        pool.add(card);
      }
    }
    return sortCardsByFrequency(pool);
  }

  private List<Card> sortCardsByFrequency(List<Card> items) {
    List<Card> ordered = new ArrayList<>(items);
    ordered.sort((left, right) -> {
      int compare = Double.compare(right.frequency, left.frequency);
      if (compare != 0) {
        return compare;
      }
      return left.id.compareTo(right.id);
    });
    return ordered;
  }

  private List<Card> distractors(List<Card> selected) {
    Set<String> ids = new LinkedHashSet<>(); for (Card c : selected) { ids.add(c.id); }
    List<Card> list = new ArrayList<>(); for (Card c : cards()) { if (!ids.contains(c.id)) { list.add(c); } }
    return list.isEmpty() ? new ArrayList<>(cards()) : list;
  }

  private List<JSONObject> buildSampledQuestions(String mode, List<Card> pool, List<Card> distractors) throws JSONException {
    List<JSONObject> questions = new ArrayList<>();
    if (pool.isEmpty()) { return questions; }

    List<Card> orderedPool = stableOrderCards(mode + ":sample:" + today(), pool);
    List<String> questionTypes = Arrays.asList("exampleToCnChoice", "enToCnChoice", "cnToEnChoice", "enToCnInput");
    int target = Math.min(remainingQuestions(mode), orderedPool.size() * questionTypes.size());
    Set<String> seenPairs = new LinkedHashSet<>();
    int cursor = 0;
    while (questions.size() < target && seenPairs.size() < orderedPool.size() * questionTypes.size()) {
      Card card = orderedPool.get(Math.floorMod((mode + ":card:" + cursor + ":" + today()).hashCode(), orderedPool.size()));
      String type = questionTypes.get(Math.floorMod((mode + ":type:" + cursor + ":" + today()).hashCode(), questionTypes.size()));
      String key = card.id + ":" + type;
      cursor += 1;
      if (!seenPairs.add(key)) {
        continue;
      }
      questions.add(sampledQuestion(type, card, distractors));
    }
    return questions;
  }

  private JSONObject sampledQuestion(String type, Card card, List<Card> distractors) throws JSONException {
    if ("enToCnChoice".equals(type)) {
      return choiceQuestion(type, card, distractors, false);
    }
    if ("exampleToCnChoice".equals(type)) {
      return choiceQuestion(type, card, distractors, true);
    }
    if ("cnToEnChoice".equals(type)) {
      return englishChoice(card, distractors);
    }
    return inputQuestion(card);
  }

  private JSONObject loopQuestion(String type, Card card, List<Card> distractors) throws JSONException {
    if ("exampleToCnChoice".equals(type)) {
      return choiceQuestion(type, card, distractors, true);
    }
    if ("enToCnChoice".equals(type)) {
      return choiceQuestion(type, card, distractors, false);
    }
    if ("cnToEnChoice".equals(type)) {
      return englishChoice(card, distractors);
    }
    return inputQuestion(card);
  }

  private List<Card> orderedCardsForRound(List<Card> cards, String mode, int round) {
    return stableOrderCards(mode + ":round:" + round + ":" + today(), cards);
  }

  private JSONObject choiceQuestion(String type, Card c, List<Card> others, boolean example) throws JSONException {
    JSONObject q = baseQuestion(c.id, c.word, partOfSpeechFor(c), c.phonetic, example ? c.exampleEn : c.word, new JSONArray(cardMeaningArray(c)), example ? c.exampleEn : JSONObject.NULL, example ? c.exampleCn : JSONObject.NULL);
    q.put("questionType", type); q.put("choices", chineseChoices(c, others)); return q;
  }

  private JSONObject englishChoice(Card c, List<Card> others) throws JSONException {
    JSONObject q = baseQuestion(c.id, c.meaning, partOfSpeechFor(c), "", c.meaning, new JSONArray(cardMeaningArray(c)), JSONObject.NULL, JSONObject.NULL);
    q.put("questionType", "cnToEnChoice"); q.put("choices", englishChoices(c, others)); return q;
  }

  private JSONObject inputQuestion(Card c) throws JSONException {
    JSONObject q = baseQuestion(c.id, c.word, partOfSpeechFor(c), c.phonetic, c.word, new JSONArray(cardMeaningArray(c)), JSONObject.NULL, JSONObject.NULL);
    q.put("questionType", "enToCnInput"); q.put("choices", JSONObject.NULL); q.put("correctChoiceLabel", JSONObject.NULL); return q;
  }

  private JSONObject rootQuestion(RootAffixCard root, String type) throws JSONException {
    boolean glossToRoot = "glossToRootInput".equals(type);
    JSONArray acceptedMeanings = new JSONArray();
    if (glossToRoot) {
      pushUnique(acceptedMeanings, root.form);
      String normalizedForm = normalizeRootAffixForm(root.form);
      if (!normalizedForm.isEmpty()) {
        pushUnique(acceptedMeanings, normalizedForm);
      }
    } else {
      pushUnique(acceptedMeanings, root.meaningCn);
    }
    JSONObject q =
        baseQuestion(
            root.id,
            glossToRoot ? root.meaningCn : root.form,
            "",
            "",
            glossToRoot ? root.meaningCn : root.form,
            acceptedMeanings,
            root.exampleSentence(),
            root.exampleTranslation());
    q.put("questionType", type);
    if (glossToRoot) {
      q.put("prompt", "根据例词填写该词根/词缀的中文含义");
    } else {
      q.put("prompt", "根据词根/词缀和例词填写中文含义");
    }
    q.put("choices", JSONObject.NULL);
    q.put("correctChoiceLabel", JSONObject.NULL);
    return q;
  }

  private JSONArray cardMeaningArray(Card card) {
    return new JSONArray(card.meanings);
  }

  private JSONObject baseQuestion(String id, String word, String partOfSpeech, String phonetic, Object prompt, JSONArray meanings, Object exampleSentence, Object exampleTranslation) throws JSONException {
    JSONObject q = new JSONObject();
    q.put("entrySourceId", id); q.put("word", word); q.put("partOfSpeech", partOfSpeech == null || partOfSpeech.isEmpty() ? JSONObject.NULL : partOfSpeech); q.put("phoneticUs", phonetic); q.put("phoneticUk", phonetic); q.put("prompt", prompt); q.put("acceptedMeanings", meanings); q.put("exampleSentence", exampleSentence); q.put("exampleTranslation", exampleTranslation);
    return q;
  }

  private JSONArray chineseChoices(Card c, List<Card> others) throws JSONException {
    List<String> distractorTexts = distractorMeanings(c, others);
    return buildBalancedChoices(
        distractorTexts,
        c.meaning,
        hashChoicePosition(c.id, "cn"));
  }

  private JSONArray englishChoices(Card c, List<Card> others) throws JSONException {
    List<String> distractorTexts = distractorWords(c, others);
    return buildBalancedChoices(
        distractorTexts,
        c.word,
        hashChoicePosition(c.id, "en"));
  }

  private List<String> distractorMeanings(Card target, List<Card> others) {
    List<String> result = new ArrayList<>();
    for (Card candidate : rankedDistractorCards(target, others)) {
      if (!result.contains(candidate.meaning)) {
        result.add(candidate.meaning);
      }
      if (result.size() >= 3) { break; }
    }
    for (Card candidate : rankedDistractorCards(target, cards())) {
      if (!candidate.meaning.equals(target.meaning) && !result.contains(candidate.meaning)) {
        result.add(candidate.meaning);
      }
      if (result.size() >= 3) { break; }
    }
    return result;
  }

  private List<String> distractorWords(Card target, List<Card> others) {
    List<String> result = new ArrayList<>();
    for (Card candidate : rankedDistractorCards(target, others)) {
      if (!result.contains(candidate.word)) {
        result.add(candidate.word);
      }
      if (result.size() >= 3) { break; }
    }
    for (Card candidate : rankedDistractorCards(target, cards())) {
      if (!candidate.word.equals(target.word) && !result.contains(candidate.word)) {
        result.add(candidate.word);
      }
      if (result.size() >= 3) { break; }
    }
    return result;
  }

  private List<Card> rankedDistractorCards(Card target, List<Card> others) {
    List<Card> samePosSameBook = new ArrayList<>();
    List<Card> samePos = new ArrayList<>();
    List<Card> sameBook = new ArrayList<>();
    List<Card> fallback = new ArrayList<>();
    String targetPos = partOfSpeechFor(target);

    for (Card candidate : others) {
      if (candidate.id.equals(target.id)) { continue; }
      String candidatePos = partOfSpeechFor(candidate);
      if (candidate.bookId == target.bookId && targetPos.equals(candidatePos)) {
        samePosSameBook.add(candidate);
      } else if (targetPos.equals(candidatePos)) {
        samePos.add(candidate);
      } else if (candidate.bookId == target.bookId) {
        sameBook.add(candidate);
      } else {
        fallback.add(candidate);
      }
    }

    List<Card> ordered = new ArrayList<>();
    ordered.addAll(rotateDistractors(target.id, samePosSameBook));
    ordered.addAll(rotateDistractors(target.id + ":pos", samePos));
    ordered.addAll(rotateDistractors(target.id + ":book", sameBook));
    ordered.addAll(rotateDistractors(target.id + ":fallback", fallback));
    return ordered;
  }

  private List<Card> rotateDistractors(String seed, List<Card> items) {
    if (items.size() <= 1) { return new ArrayList<>(items); }
    List<Card> rotated = new ArrayList<>(items);
    Collections.rotate(rotated, -(Math.abs(seed.hashCode()) % rotated.size()));
    return rotated;
  }

  private List<Card> stableOrderCards(String seed, List<Card> items) {
    List<Card> ordered = new ArrayList<>(items);
    ordered.sort((left, right) -> {
      int leftHash = Math.abs((seed + ":" + left.id).hashCode());
      int rightHash = Math.abs((seed + ":" + right.id).hashCode());
      if (leftHash == rightHash) { return left.id.compareTo(right.id); }
      return Integer.compare(leftHash, rightHash);
    });
    return ordered;
  }

  private List<Card> sortCardsByWrongPriority(List<Card> items, JSONObject pool, boolean excludeToday) {
    List<Card> ordered = new ArrayList<>();
    for (Card card : items) {
      JSONObject meta = wrongMeta(pool, card.id);
      if (meta.optInt("count", 0) <= 0) {
        continue;
      }
      if (excludeToday && isSameStudyDate(meta.optString("lastWrongAt", ""))) {
        continue;
      }
      ordered.add(card);
    }
    ordered.sort((left, right) -> {
      double rightScore = wrongPriorityScore(wrongMeta(pool, right.id));
      double leftScore = wrongPriorityScore(wrongMeta(pool, left.id));
      int compare = Double.compare(rightScore, leftScore);
      if (compare != 0) {
        return compare;
      }
      compare = Double.compare(right.frequency, left.frequency);
      if (compare != 0) {
        return compare;
      }
      return left.id.compareTo(right.id);
    });
    return ordered;
  }

  private List<JSONObject> stableOrderQuestions(String seed, List<JSONObject> items) {
    List<JSONObject> ordered = new ArrayList<>(items);
    ordered.sort((left, right) -> {
      String leftId = left.optString("entrySourceId") + ":" + left.optString("questionType");
      String rightId = right.optString("entrySourceId") + ":" + right.optString("questionType");
      int leftHash = Math.abs((seed + ":" + leftId).hashCode());
      int rightHash = Math.abs((seed + ":" + rightId).hashCode());
      if (leftHash == rightHash) { return leftId.compareTo(rightId); }
      return Integer.compare(leftHash, rightHash);
    });
    return ordered;
  }

  private boolean isSameStudyDate(String timestamp) {
    return !timestamp.isEmpty() && timestamp.startsWith(today());
  }

  private double wrongPriorityScore(JSONObject meta) {
    int count = meta.optInt("count", 0);
    long ageDays = daysSince(meta.optString("lastWrongAt", ""));
    double recencyBoost = Math.max(0, 21 - Math.min(ageDays, 21));
    double asymmetryBoost = inputPenalty(meta) - choicePenalty(meta);
    return Math.min(10.0, (count * 2.2) + (recencyBoost * 0.2) + Math.max(asymmetryBoost, 0.0));
  }

  private double inputPenalty(JSONObject meta) {
    return failureRate(meta, "enToCnInput") * 2.4;
  }

  private double choicePenalty(JSONObject meta) {
    return failureRate(meta, "enToCnChoice") + failureRate(meta, "exampleToCnChoice");
  }

  private double failureRate(JSONObject meta, String questionType) {
    JSONObject bucket = questionTypeBucket(meta, questionType);
    int attempts = bucket.optInt("attempts", 0);
    if (attempts == 0) {
      return 0.0;
    }
    double failures = bucket.optInt("incorrect", 0) + bucket.optInt("skipped", 0) + (bucket.optInt("fuzzyCorrect", 0) * 0.5);
    return failures / attempts;
  }

  private JSONObject questionTypeBucket(JSONObject meta, String questionType) {
    JSONObject byType = meta.optJSONObject("byQuestionType");
    if (byType == null) {
      return new JSONObject();
    }
    JSONObject bucket = byType.optJSONObject(questionType);
    return bucket == null ? new JSONObject() : bucket;
  }

  private long daysSince(String timestamp) {
    String datePart = timestamp == null ? "" : timestamp.trim();
    if (datePart.length() >= 10) {
      datePart = datePart.substring(0, 10);
    }
    if (datePart.isEmpty()) {
      return 365;
    }
    return Math.max(daysBetween(datePart, today()), 0);
  }

  private String partOfSpeechFor(Card card) {
    if (card.partOfSpeech != null && !card.partOfSpeech.isEmpty()) { return card.partOfSpeech; }
    if (Arrays.asList("adapt", "clarify", "retain", "modify", "infer", "compile").contains(card.id)) { return "verb"; }
    if (Arrays.asList("approach", "structure", "therapy", "diagnosis", "symptom").contains(card.id)) { return "noun"; }
    return "adjective";
  }

  private JSONArray buildBalancedChoices(
      List<String> distractorTexts,
      String correctText,
      int correctIndex) throws JSONException {
    List<String> all = new ArrayList<>(distractorTexts);
    all.add(correctIndex, correctText);
    JSONArray arr = new JSONArray();
    for (int i = 0; i < 4; i += 1) {
      arr.put(option(String.valueOf((char) ('A' + i)), all.get(i), i == correctIndex));
    }
    return arr;
  }

  private int hashChoicePosition(String sourceId, String salt) {
    int hash = Math.abs((sourceId + ":" + salt).hashCode());
    return hash % 4;
  }

  private JSONObject option(String label, String text, boolean correct) throws JSONException { return new JSONObject().put("label", label).put("text", text).put("correct", correct); }
  private int distinctCount(JSONArray qs) { Set<String> ids = new LinkedHashSet<>(); for (int i = 0; i < qs.length(); i += 1) { ids.add(qs.optJSONObject(i).optString("entrySourceId")); } return ids.size(); }

  private JSONObject startPayload(JSONObject s) throws JSONException { JSONArray qs = s.getJSONArray("questions"); return new JSONObject().put("session", sessionJson(s)).put("currentQuestion", inflate(qs.getJSONObject(s.optInt("nextIndex")), s.getString("sessionId"), s.optInt("nextIndex"), qs.length())).put("progress", progressObj(s.optInt("nextIndex") + 1, qs.length())); }
  private JSONObject sessionJson(JSONObject s) throws JSONException { return new JSONObject().put("sessionId", s.getString("sessionId")).put("mode", s.getString("mode")).put("totalWords", s.optInt("totalWords")).put("wordbookId", JSONObject.NULL).put("startedAt", s.optString("startedAt")); }
  private JSONObject progressObj(int current, int total) throws JSONException { return new JSONObject().put("current", current).put("total", total); }

  private JSONObject inflate(JSONObject q, String sessionId, int index, int total) throws JSONException {
    JSONObject out = new JSONObject(q.toString()); out.put("questionId", sessionId + "_q" + index); out.put("questionIndex", index); out.put("totalQuestions", total);
    JSONArray choices = out.optJSONArray("choices");
    if (choices != null) {
      String correct = "A";
      for (int i = 0; i < choices.length(); i += 1) {
        if (choices.getJSONObject(i).optBoolean("correct")) { correct = choices.getJSONObject(i).optString("label"); }
        choices.getJSONObject(i).remove("correct");
      }
      out.put("correctChoiceLabel", correct);
    }
    return out;
  }

  private String judgeOutcome(JSONObject q, String normalizedResponse) {
    JSONArray choices = q.optJSONArray("choices");
    if (choices != null) {
      for (int i = 0; i < choices.length(); i += 1) {
        if (choices.optJSONObject(i).optBoolean("correct")) {
          return normalizedResponse.equals(normalize(choices.optJSONObject(i).optString("label"))) ? "correct" : "incorrect";
        }
      }
    }
    JSONArray meanings = q.optJSONArray("acceptedMeanings");
    for (int i = 0; meanings != null && i < meanings.length(); i += 1) {
      String normalizedMeaning = normalize(meanings.optString(i));
      if (normalizedResponse.equals(normalizedMeaning)) {
        return "correct";
      }
      if (!normalizedResponse.isEmpty() && !normalizedMeaning.isEmpty() && (normalizedMeaning.contains(normalizedResponse) || normalizedResponse.contains(normalizedMeaning))) {
        return "fuzzyCorrect";
      }
      if (!normalizedResponse.isEmpty() && levenshteinDistance(normalizedResponse, normalizedMeaning) <= 1) {
        return "fuzzyCorrect";
      }
    }
    return "incorrect";
  }

  private String answerText(JSONObject q) {
    JSONArray choices = q.optJSONArray("choices");
    if (choices != null) { for (int i = 0; i < choices.length(); i += 1) { if (choices.optJSONObject(i).optBoolean("correct")) { return choices.optJSONObject(i).optString("text"); } } }
    JSONArray meanings = q.optJSONArray("acceptedMeanings"); return meanings != null && meanings.length() > 0 ? meanings.optString(0) : "";
  }

  private JSONObject summary(JSONObject s) throws JSONException {
    int totalQ = s.getJSONArray("questions").length(); int correct = s.optInt("correctCount"); int fuzzy = s.optInt("fuzzyCorrectCount"); int incorrect = s.optInt("incorrectCount");
    return new JSONObject().put("sessionId", s.optString("sessionId")).put("totalQuestions", totalQ).put("correctCount", correct).put("fuzzyCorrectCount", fuzzy).put("incorrectCount", incorrect).put("skippedCount", 0).put("totalWords", s.optInt("totalWords")).put("wrongWordCount", s.getJSONArray("wrongIds").length()).put("accuracyPercent", totalQ == 0 ? 0 : ((correct + (fuzzy * 0.5)) * 100.0 / totalQ)).put("totalTimeMs", s.optInt("totalTimeMs")).put("completedAt", now());
  }

  private int levenshteinDistance(String left, String right) {
    if (left.equals(right)) { return 0; }
    if (left.isEmpty()) { return right.length(); }
    if (right.isEmpty()) { return left.length(); }

    int[] previous = new int[right.length() + 1];
    int[] current = new int[right.length() + 1];
    for (int j = 0; j <= right.length(); j += 1) { previous[j] = j; }
    for (int i = 1; i <= left.length(); i += 1) {
      current[0] = i;
      for (int j = 1; j <= right.length(); j += 1) {
        int cost = left.charAt(i - 1) == right.charAt(j - 1) ? 0 : 1;
        current[j] = Math.min(
            Math.min(current[j - 1] + 1, previous[j] + 1),
            previous[j - 1] + cost);
      }
      int[] swap = previous; previous = current; current = swap;
    }
    return previous[right.length()];
  }

  private void pushUnique(JSONArray arr, String value) { for (int i = 0; i < arr.length(); i += 1) { if (value.equals(arr.optString(i))) { return; } } arr.put(value); }
  private JSONObject wrongMeta(JSONObject pool, String id) {
    Object raw = pool.opt(id);
    if (raw instanceof JSONObject) {
      return (JSONObject) raw;
    }
    JSONObject meta = new JSONObject();
    try {
      meta.put("count", raw instanceof Number ? ((Number) raw).intValue() : 0);
      meta.put("lastWrongAt", today());
      meta.put("byQuestionType", new JSONObject());
    } catch (JSONException ignored) {}
    return meta;
  }
  private void growWrongPoolFromResults(JSONArray results) throws JSONException {
    if (results == null) {
      return;
    }
    JSONObject pool = read(KEY_WRONG_POOL);
    for (int i = 0; i < results.length(); i += 1) {
      JSONObject result = results.optJSONObject(i);
      if (result == null) {
        continue;
      }
      String id = result.optString("entrySourceId");
      if (id.isEmpty()) {
        continue;
      }
      JSONObject meta = wrongMeta(pool, id);
      recordQuestionTypeStat(meta, result);
      pool.put(id, meta);
    }
    write(KEY_WRONG_POOL, pool);
  }
  private void appendSessionResult(JSONObject session, JSONObject result) throws JSONException {
    JSONArray results = session.optJSONArray("results");
    if (results == null) {
      results = new JSONArray();
      session.put("results", results);
    }
    results.put(new JSONObject(result.toString()));
  }
  private void recordQuestionTypeStat(JSONObject meta, JSONObject result) throws JSONException {
    String questionType = result.optString("questionType", "unknown");
    String outcome = result.optString("outcome", "incorrect");
    JSONObject byType = meta.optJSONObject("byQuestionType");
    if (byType == null) {
      byType = new JSONObject();
      meta.put("byQuestionType", byType);
    }
    JSONObject bucket = byType.optJSONObject(questionType);
    if (bucket == null) {
      bucket = new JSONObject();
      bucket.put("attempts", 0);
      bucket.put("incorrect", 0);
      bucket.put("skipped", 0);
      bucket.put("fuzzyCorrect", 0);
      bucket.put("correct", 0);
    }
    bucket.put("attempts", bucket.optInt("attempts") + 1);
    bucket.put(outcome, bucket.optInt(outcome) + 1);
    byType.put(questionType, bucket);
    if ("incorrect".equals(outcome) || "skipped".equals(outcome)) {
      meta.put("count", meta.optInt("count", 0) + 1);
      meta.put("lastWrongAt", result.optString("answeredAt", now()));
    }
  }
  private void growLearnedPool(JSONArray learnedIds) throws JSONException { JSONObject pool = read(KEY_LEARNED); for (int i = 0; i < learnedIds.length(); i += 1) { String id = learnedIds.optString(i); pool.put(id, pool.optInt(id) + 1); } write(KEY_LEARNED, pool); }
  private JSONArray sessionSourceIds(JSONObject session) {
    JSONArray ids = new JSONArray();
    JSONArray questions = session.optJSONArray("questions");
    if (questions == null) { return ids; }
    Set<String> seen = new LinkedHashSet<>();
    for (int i = 0; i < questions.length(); i += 1) {
      String id = questions.optJSONObject(i).optString("entrySourceId");
      if (seen.add(id)) {
        ids.put(id);
      }
    }
    return ids;
  }

  private void appendHistory(JSONObject session, JSONObject summary) throws JSONException {
    JSONArray history = readArray(KEY_HISTORY);
    history.put(
        new JSONObject()
            .put("date", today())
            .put("mode", session.optString("mode"))
            .put("completedAt", summary.optString("completedAt", now()))
            .put("wrongIds", new JSONArray(session.optJSONArray("wrongIds") == null ? "[]" : session.optJSONArray("wrongIds").toString()))
            .put("summary", summary));
    writeArray(KEY_HISTORY, history);
  }

  private JSONObject buildReportsOverview() throws JSONException {
    JSONArray history = readArray(KEY_HISTORY);
    JSONObject learned = read(KEY_LEARNED);
    JSONObject byDate = new JSONObject();
    JSONObject byMode = new JSONObject();
    JSONObject byModeDate = new JSONObject();
    Set<String> studyDays = new LinkedHashSet<>();

    int totalQuestions = 0;
    int totalCorrect = 0;
    for (int i = 0; i < history.length(); i += 1) {
      JSONObject item = history.getJSONObject(i);
      String date = item.optString("date");
      String mode = item.optString("mode");
      JSONObject summary = item.optJSONObject("summary");
      if (summary == null) {
        continue;
      }
      studyDays.add(date);
      totalQuestions += summary.optInt("totalQuestions");
      totalCorrect += summary.optInt("correctCount");

      JSONObject day = byDate.optJSONObject(date);
      if (day == null) {
        day = new JSONObject();
        day.put("date", date);
        day.put("totalQuestions", 0);
        day.put("correctCount", 0);
        day.put("studyTimeMs", 0);
      }
      day.put("totalQuestions", day.optInt("totalQuestions") + summary.optInt("totalQuestions"));
      day.put("correctCount", day.optInt("correctCount") + summary.optInt("correctCount"));
      day.put("studyTimeMs", day.optInt("studyTimeMs") + summary.optInt("totalTimeMs"));
      byDate.put(date, day);

      JSONObject modeAgg = byMode.optJSONObject(mode);
      if (modeAgg == null) {
        modeAgg = new JSONObject();
        modeAgg.put("mode", mode);
        modeAgg.put("totalQuestions", 0);
        modeAgg.put("correctCount", 0);
      }
      modeAgg.put("totalQuestions", modeAgg.optInt("totalQuestions") + summary.optInt("totalQuestions"));
      modeAgg.put("correctCount", modeAgg.optInt("correctCount") + summary.optInt("correctCount"));
      byMode.put(mode, modeAgg);

      JSONObject modeDateAgg = byModeDate.optJSONObject(mode);
      if (modeDateAgg == null) {
        modeDateAgg = new JSONObject();
      }
      JSONObject modeDay = modeDateAgg.optJSONObject(date);
      if (modeDay == null) {
        modeDay = new JSONObject();
        modeDay.put("date", date);
        modeDay.put("totalQuestions", 0);
        modeDay.put("correctCount", 0);
        modeDay.put("studyTimeMs", 0);
      }
      modeDay.put("totalQuestions", modeDay.optInt("totalQuestions") + summary.optInt("totalQuestions"));
      modeDay.put("correctCount", modeDay.optInt("correctCount") + summary.optInt("correctCount"));
      modeDay.put("studyTimeMs", modeDay.optInt("studyTimeMs") + summary.optInt("totalTimeMs"));
      modeDateAgg.put(date, modeDay);
      byModeDate.put(mode, modeDateAgg);
    }

    JSONArray last7Days = buildRecentDays(byDate, 7);
    JSONArray dailySeries = buildAllDaysSeries(byDate);
    JSONArray modeBreakdown = buildModeBreakdown(byMode);
    JSONObject modeSeries = buildAllModeSeries(byModeDate);
    JSONObject streakInfo = buildStreakInfo(studyDays);

    JSONObject result = new JSONObject();
    result.put("totalStudyDays", studyDays.size());
    result.put("totalWordsLearned", learned.length());
    result.put("totalQuestionsAnswered", totalQuestions);
    result.put("overallAccuracy", totalQuestions == 0 ? 0 : (totalCorrect * 100.0 / totalQuestions));
    result.put("streakInfo", streakInfo);
    result.put("modeBreakdown", modeBreakdown);
    result.put("last7Days", last7Days);
    result.put("dailySeries", dailySeries);
    result.put("modeSeries", modeSeries);
    return result;
  }

  private JSONArray buildRecentDays(JSONObject byDate, int days) throws JSONException {
    JSONArray array = new JSONArray();
    for (int offset = days - 1; offset >= 0; offset -= 1) {
      long ts = System.currentTimeMillis() - (offset * 86400000L);
      String date = new SimpleDateFormat("yyyy-MM-dd", Locale.US).format(new Date(ts));
      JSONObject day = byDate.optJSONObject(date);
      int totalQuestions = day == null ? 0 : day.optInt("totalQuestions");
      int correctCount = day == null ? 0 : day.optInt("correctCount");
      int studyTimeMs = day == null ? 0 : day.optInt("studyTimeMs");
      array.put(
          new JSONObject()
              .put("date", date)
              .put("totalQuestions", totalQuestions)
              .put("correctCount", correctCount)
              .put("accuracyPercent", totalQuestions == 0 ? 0 : (correctCount * 100.0 / totalQuestions))
              .put("studyTimeMs", studyTimeMs));
    }
    return array;
  }

  private JSONArray buildAllDaysSeries(JSONObject byDate) throws JSONException {
    List<String> dates = new ArrayList<>();
    for (java.util.Iterator<String> iterator = byDate.keys(); iterator.hasNext(); ) {
      dates.add(iterator.next());
    }
    Collections.sort(dates);
    JSONArray array = new JSONArray();
    for (String date : dates) {
      JSONObject day = byDate.optJSONObject(date);
      if (day == null) { continue; }
      int totalQuestions = day.optInt("totalQuestions");
      int correctCount = day.optInt("correctCount");
      int studyTimeMs = day.optInt("studyTimeMs");
      array.put(
          new JSONObject()
              .put("date", date)
              .put("totalQuestions", totalQuestions)
              .put("correctCount", correctCount)
              .put("accuracyPercent", totalQuestions == 0 ? 0 : (correctCount * 100.0 / totalQuestions))
              .put("studyTimeMs", studyTimeMs));
    }
    return array;
  }

  private JSONArray buildModeBreakdown(JSONObject byMode) throws JSONException {
    JSONArray array = new JSONArray();
    for (String mode : MODES) {
      JSONObject modeAgg = byMode.optJSONObject(mode);
      int totalQuestions = modeAgg == null ? 0 : modeAgg.optInt("totalQuestions");
      int correctCount = modeAgg == null ? 0 : modeAgg.optInt("correctCount");
      array.put(
          new JSONObject()
              .put("mode", mode)
              .put("totalQuestions", totalQuestions)
              .put("correctCount", correctCount)
              .put("accuracyPercent", totalQuestions == 0 ? 0 : (correctCount * 100.0 / totalQuestions)));
    }
    return array;
  }

  private JSONObject buildAllModeSeries(JSONObject byModeDate) throws JSONException {
    JSONObject result = new JSONObject();
    for (String mode : MODES) {
      JSONObject modeByDate = byModeDate.optJSONObject(mode);
      JSONArray series = buildAllDaysSeries(modeByDate == null ? new JSONObject() : modeByDate);
      result.put(mode, series);
    }
    return result;
  }

  private JSONObject buildStreakInfo(Set<String> studyDays) throws JSONException {
    List<String> ordered = new ArrayList<>(studyDays);
    Collections.sort(ordered);
    int longest = 0;
    int current = 0;
    int running = 0;
    String prev = null;
    for (String date : ordered) {
      if (prev == null || daysBetween(prev, date) == 1) {
        running += 1;
      } else {
        running = 1;
      }
      longest = Math.max(longest, running);
      prev = date;
    }
    String cursor = today();
    while (studyDays.contains(cursor)) {
      current += 1;
      cursor = shiftDate(cursor, -1);
    }
    return new JSONObject()
        .put("currentStreak", current)
        .put("longestStreak", longest)
        .put("lastStudyDate", ordered.isEmpty() ? JSONObject.NULL : ordered.get(ordered.size() - 1));
  }

  private int daysBetween(String from, String to) {
    try {
      SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd", Locale.US);
      long diff = format.parse(to).getTime() - format.parse(from).getTime();
      return (int) (diff / 86400000L);
    } catch (Exception e) {
      return 0;
    }
  }

  private String shiftDate(String date, int deltaDays) {
    try {
      SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd", Locale.US);
      Date parsed = format.parse(date);
      return format.format(new Date(parsed.getTime() + (deltaDays * 86400000L)));
    } catch (Exception e) {
      return date;
    }
  }

  private JSONArray collectAiWrongWordInputs(String date) throws JSONException {
    JSONArray dayWrongWords = collectDayWrongWords(date);
    if (dayWrongWords.length() > 0) {
      return dayWrongWords;
    }
    return collectActiveWrongWordNotebookWords();
  }

  private JSONArray collectDayWrongWords(String date) throws JSONException {
    JSONArray history = readArray(KEY_HISTORY);
    JSONArray result = new JSONArray();
    Set<String> seen = new LinkedHashSet<>();
    for (int i = 0; i < history.length(); i += 1) {
      JSONObject item = history.getJSONObject(i);
      if (!date.equals(item.optString("date"))) {
        continue;
      }
      JSONArray wrongIds = item.optJSONArray("wrongIds");
      if (wrongIds == null) {
        continue;
      }
      for (int j = 0; j < wrongIds.length(); j += 1) {
        String sourceId = wrongIds.optString(j);
        if (sourceId.isEmpty() || !seen.add(sourceId)) {
          continue;
        }
        Card card = cardById(sourceId);
        if (card != null) {
          result.put(buildWrongWordInput(card));
        }
      }
    }
    return sortWrongWordInputs(result);
  }

  private JSONArray collectActiveWrongWordNotebookWords() throws JSONException {
    JSONArray entries = buildWrongWordEntries("highPriority");
    JSONArray result = new JSONArray();
    for (int i = 0; i < entries.length() && i < 12; i += 1) {
      Card card = cardByIndex(entries.getJSONObject(i).optInt("entryId"));
      if (card != null) {
        result.put(buildWrongWordInput(card));
      }
    }
    return sortWrongWordInputs(result);
  }

  private JSONObject buildWrongWordInput(Card card) throws JSONException {
    return new JSONObject()
        .put("entryId", cardIndex(card.id))
        .put("word", card.word)
        .put("primaryGloss", card.meaning)
        .put("partOfSpeech", partOfSpeechFor(card));
  }

  private JSONArray sortWrongWordInputs(JSONArray inputs) throws JSONException {
    List<JSONObject> items = new ArrayList<>();
    for (int i = 0; i < inputs.length(); i += 1) {
      items.add(inputs.getJSONObject(i));
    }
    items.sort((left, right) -> Integer.compare(left.optInt("entryId"), right.optInt("entryId")));
    JSONArray result = new JSONArray();
    for (JSONObject item : items) {
      result.put(item);
    }
    return result;
  }

  private JSONArray buildWrongWordEntries(String filter) throws JSONException {
    JSONObject pool = read(KEY_WRONG_POOL);
    List<JSONObject> items = new ArrayList<>();
    for (Card card : cards()) {
      JSONObject meta = wrongMeta(pool, card.id);
      int count = meta.optInt("count", 0);
      if (count <= 0) {
        continue;
      }
      double priority = wrongPriorityScore(meta);
      JSONObject item = new JSONObject();
      item.put("entryId", cardIndex(card.id));
      item.put("word", card.word);
      item.put("phoneticUs", card.phonetic);
      item.put("phoneticUk", card.phonetic);
      item.put("meanings", new JSONArray(card.meanings));
      item.put("errorCount", count);
      item.put("lastWrongAt", meta.optString("lastWrongAt", today()));
      item.put("priorityScore", priority);
      item.put("isActive", true);
      items.add(item);
    }
    items.sort((left, right) -> Double.compare(right.optDouble("priorityScore"), left.optDouble("priorityScore")));
    if ("recent".equals(filter)) {
      items.sort((left, right) -> right.optString("lastWrongAt").compareTo(left.optString("lastWrongAt")));
    } else if ("frequent".equals(filter)) {
      items.sort((left, right) -> Integer.compare(right.optInt("errorCount"), left.optInt("errorCount")));
    } else if ("highPriority".equals(filter)) {
      items.removeIf(item -> item.optDouble("priorityScore") < 8.0);
    }
    JSONArray result = new JSONArray();
    for (JSONObject item : items) {
      result.put(item);
    }
    return result;
  }

  private JSONArray buildWrongWordSeed() throws JSONException {
    JSONObject pool = read(KEY_WRONG_POOL);
    JSONArray result = new JSONArray();
    for (Card card : cards()) {
      JSONObject meta = wrongMeta(pool, card.id);
      int count = meta.optInt("count", 0);
      if (count <= 0) {
        continue;
      }
      result.put(new JSONObject()
          .put("entryId", cardIndex(card.id))
          .put("word", card.word)
          .put("phoneticUs", card.phonetic)
          .put("phoneticUk", card.phonetic)
          .put("meanings", new JSONArray(card.meanings))
          .put("errorCount", count)
          .put("lastWrongAt", meta.optString("lastWrongAt", today()))
          .put("isActive", true));
    }
    return result;
  }

  private JSONObject buildWrongWordDetail(int entryId) throws JSONException {
    Card card = cardByIndex(entryId);
    if (card == null) {
      return new JSONObject();
    }
    JSONObject meta = wrongMeta(read(KEY_WRONG_POOL), card.id);
    JSONArray examples = new JSONArray()
        .put(new JSONObject().put("sentenceEn", card.exampleEn).put("sentenceCn", card.exampleCn));
    JSONArray meanings = new JSONArray();
    for (String meaning : card.meanings) {
      meanings.put(new JSONObject().put("pos", partOfSpeechFor(card)).put("meaningCn", meaning).put("meaningEn", card.word));
    }
    JSONArray history = new JSONArray()
        .put(new JSONObject().put("date", meta.optString("lastWrongAt", today())).put("context", "mobile review"));
    JSONArray riskBreakdown = new JSONArray();
    JSONObject byType = meta.optJSONObject("byQuestionType");
    if (byType != null) {
      for (String type : Arrays.asList("exampleToCnChoice", "enToCnChoice", "cnToEnChoice", "enToCnInput")) {
        JSONObject bucket = byType.optJSONObject(type);
        if (bucket == null || bucket.optInt("attempts", 0) == 0) {
          continue;
        }
        riskBreakdown.put(new JSONObject()
            .put("questionType", type)
            .put("attempts", bucket.optInt("attempts"))
            .put("incorrect", bucket.optInt("incorrect"))
            .put("skipped", bucket.optInt("skipped"))
            .put("fuzzyCorrect", bucket.optInt("fuzzyCorrect"))
            .put("correct", bucket.optInt("correct")));
      }
    }
    JSONArray related = new JSONArray();
    for (Card candidate : cards()) {
      if (!candidate.id.equals(card.id) && related.length() < 4) {
        related.put(candidate.word);
      }
    }
    return new JSONObject()
        .put("entryId", entryId)
        .put("word", card.word)
        .put("lemma", card.word)
        .put("phoneticUs", card.phonetic)
        .put("phoneticUk", card.phonetic)
        .put("partOfSpeech", partOfSpeechFor(card))
        .put("meanings", meanings)
        .put("examples", examples)
        .put("errorHistory", history)
        .put("riskBreakdown", riskBreakdown)
        .put("relatedWords", related);
  }

  private JSONObject createAiPassage(JSONObject request) throws JSONException {
    JSONArray history = readArray(KEY_AI_HISTORY);
    String date = request.optString("date", today());
    JSONArray wrongWords = collectAiWrongWordInputs(date);
    JSONArray targets = request.optJSONArray("targetWords");
    JSONArray selectedWrongWords = new JSONArray();
    List<String> targetList = new ArrayList<>();

    if (targets != null && targets.length() > 0) {
      Set<String> requestedTargets = new LinkedHashSet<>();
      for (int i = 0; i < targets.length(); i += 1) {
        String target = targets.optString(i);
        if (!target.isEmpty()) {
          requestedTargets.add(target);
        }
      }

      for (int i = 0; i < wrongWords.length() && targetList.size() < 6; i += 1) {
        JSONObject wordInput = wrongWords.getJSONObject(i);
        String word = wordInput.optString("word");
        if (requestedTargets.contains(word) && !targetList.contains(word)) {
          selectedWrongWords.put(wordInput);
          targetList.add(word);
        }
      }

      for (String target : requestedTargets) {
        if (targetList.size() >= 6) {
          break;
        }
        if (!target.isEmpty() && !targetList.contains(target)) {
          targetList.add(target);
        }
      }
    }

    if (targetList.isEmpty()) {
      for (int i = 0; i < wrongWords.length() && targetList.size() < 6; i += 1) {
        JSONObject wordInput = wrongWords.getJSONObject(i);
        selectedWrongWords.put(wordInput);
        targetList.add(wordInput.optString("word"));
      }
    }
    if (targetList.isEmpty()) {
      targetList.add("adapt");
      targetList.add("clarify");
    }
    String level = request.optString("level", "intermediate");
    JSONArray inputWords = new JSONArray();
    JSONArray covered = new JSONArray();
    JSONArray sourceWrongWords = selectedWrongWords.length() > 0 ? selectedWrongWords : wrongWords;
    for (int i = 0; i < sourceWrongWords.length(); i += 1) {
      JSONObject wordInput = sourceWrongWords.getJSONObject(i);
      inputWords.put(wordInput);
      covered.put(wordInput.optInt("entryId"));
    }

    JSONArray blocks = new JSONArray();
    JSONArray paragraphOne = new JSONArray();
    paragraphOne.put(new JSONObject().put("type", "text").put("text", "晚上回顾今天的学习时，你把最容易卡住的词放回一个更自然的情境里。"));
    for (int i = 0; i < Math.min(targetList.size(), 3); i += 1) {
      String target = targetList.get(i);
      Card card = cardByWord(target);
      String meaning = card == null ? "重点词" : card.meaning;
      paragraphOne.put(new JSONObject().put("type", "text").put("text", i == 0 ? " 先想到 " : " 然后又想到 "));
      paragraphOne.put(
          new JSONObject()
              .put("type", "word")
              .put("text", target)
              .put("entryId", card == null ? 0 : cardIndex(card.id))
              .put("glossZh", meaning)
              .put("highlighted", true));
      if (card != null) {
        paragraphOne.put(new JSONObject().put("type", "text").put("text", "，就会联想到“" + card.exampleCn + "”。"));
      } else {
        paragraphOne.put(new JSONObject().put("type", "text").put("text", "，提醒自己把它放回真实语境里。"));
      }
    }
    blocks.put(new JSONObject().put("blockType", "paragraph").put("segments", paragraphOne));

    JSONArray paragraphTwo = new JSONArray();
    paragraphTwo.put(new JSONObject().put("type", "text").put("text", "你没有再把这些词当成孤立的卡片，而是想象自己正在整理一份学习备忘。"));
    for (int i = 0; i < targetList.size(); i += 1) {
      String target = targetList.get(i);
      Card card = cardByWord(target);
      String meaning = card == null ? "重点词" : card.meaning;
      paragraphTwo.put(new JSONObject().put("type", "text").put("text", i == 0 ? " 这时 " : " 接着 "));
      paragraphTwo.put(
          new JSONObject()
              .put("type", "word")
              .put("text", target)
              .put("entryId", card == null ? 0 : cardIndex(card.id))
              .put("glossZh", meaning)
              .put("highlighted", true));
      if (card != null) {
        paragraphTwo.put(new JSONObject().put("type", "text").put("text", " 不只是对应“" + meaning + "”，还会带出“" + card.exampleEn + "”这样的完整场景。"));
      } else {
        paragraphTwo.put(new JSONObject().put("type", "text").put("text", " 也被重新放回一句完整的话里。"));
      }
    }
    paragraphTwo.put(new JSONObject().put("type", "text").put("text", "这样复盘时，你记住的不是词条列表，而是一段更像真实使用场景的短文。"));
    blocks.put(new JSONObject().put("blockType", "paragraph").put("segments", paragraphTwo));

    String preview = targetList.isEmpty()
        ? "暂无错词输入。"
        : "今晚复盘围绕 " + String.join("、", targetList) + " 展开。";
    JSONObject passage = new JSONObject()
        .put("passageId", "passage_" + System.currentTimeMillis())
        .put("title", "今日错词情境短文")
        .put("blocks", blocks)
        .put("wrongWords", inputWords)
        .put("coveredWordIds", covered)
        .put("missingWordIds", new JSONArray())
        .put("validationStatus", "passed")
        .put("failureReason", JSONObject.NULL)
        .put("wordCount", Math.max(60, targetList.size() * 24))
        .put("targetLevel", level)
        .put("date", date)
        .put("generatedAt", now())
        .put("preview", preview);
    history.put(passage);
    writeArray(KEY_AI_HISTORY, history);
    return passage;
  }

  private int cardIndex(String id) {
    List<Card> all = cards();
    for (int i = 0; i < all.size(); i += 1) {
      if (all.get(i).id.equals(id)) {
        return i + 1;
      }
    }
    return 0;
  }

  private Card cardByIndex(int entryId) {
    List<Card> all = cards();
    int index = entryId - 1;
    return index >= 0 && index < all.size() ? all.get(index) : null;
  }

  private String nextHint(String today) {
    try {
      JSONObject s = snapshot(today);
      if (s.optInt("newWordsCompleted") < s.optInt("newWordsTarget")) { return "Continue with new words"; }
      if (s.optInt("reviewWordsCompleted") < s.optInt("reviewWordsTarget")) { return "Continue with review"; }
      if (s.optInt("mixedTestCompleted") < s.optInt("mixedTestTarget")) { return "Continue with mixed test"; }
      if (s.optInt("wrongWordTestCompleted") < s.optInt("wrongWordTestTarget")) { return "Continue with wrong-word reinforcement"; }
      if (s.optInt("rootAffixCompleted") < s.optInt("rootAffixTarget")) { return "Continue with root/affix"; }
    } catch (Exception ignored) {}
    return "Return to Today";
  }

  private int indexOf(String[] items, String value) { for (int i = 0; i < items.length; i += 1) { if (items[i].equals(value)) { return i; } } return -1; }
  private Card cardById(String id) { for (Card c : cards()) { if (c.id.equals(id)) { return c; } } return null; }
  private Card cardByWord(String word) { for (Card c : cards()) { if (c.word.equals(word)) { return c; } } return null; }
  private RootAffixCard rootAffixCardById(String id) { for (RootAffixCard c : rootAffixCardsForActiveWordbooks()) { if (c.id.equals(id)) { return c; } } return null; }

  private List<Card> cards() {
    if (vocabCardsCache != null && !vocabCardsCache.isEmpty()) {
      return vocabCardsCache;
    }

    List<Card> loaded = loadCardsFromAssets();
    if (!loaded.isEmpty()) {
      vocabCardsCache = loaded;
      return vocabCardsCache;
    }

    vocabCardsCache = Arrays.asList(CARDS);
    return vocabCardsCache;
  }

  private List<RootAffixCard> rootAffixCardsForActiveWordbooks() {
    List<RootAffixCard> all = rootAffixCards();
    if (all.isEmpty()) { return all; }

    JSONObject active = read(wordbooksKey(true));
    boolean includeMedical = active.optBoolean("4", false);
    boolean includeShared = active.optBoolean("1", false) || active.optBoolean("2", false) || active.optBoolean("3", false);
    List<RootAffixCard> scoped = new ArrayList<>();
    for (RootAffixCard card : all) {
      if (isReliableRootAffixCard(card)
          && (("medical".equals(card.scope) && includeMedical) || ("shared".equals(card.scope) && includeShared))) {
        scoped.add(card);
      }
    }
    List<RootAffixCard> visible = scoped.isEmpty() ? all : scoped;
    visible.sort((left, right) -> left.id.compareTo(right.id));
    return visible;
  }

  private List<RootAffixCard> rootAffixCards() {
    if (rootAffixCardsCache != null && !rootAffixCardsCache.isEmpty()) {
      return rootAffixCardsCache;
    }

    List<RootAffixCard> loaded = new ArrayList<>();
    try {
      AssetManager assets = getReactApplicationContext().getAssets();
      loaded.addAll(loadSharedRootAffixCards(assets));
      loaded.addAll(loadMedicalRootAffixCards(assets));
    } catch (Exception ignored) {}

    if (loaded.isEmpty()) {
      for (int i = 0; i < ROOTS.length; i += 1) {
        loaded.add(new RootAffixCard(
            ROOT_IDS[i],
            ROOTS[i][0],
            ROOTS[i][1],
            ROOTS[i][2],
            ROOTS[i][3],
            "shared"));
      }
    }

    rootAffixCardsCache = loaded;
    return rootAffixCardsCache;
  }

  private List<RootAffixCard> loadSharedRootAffixCards(AssetManager assets) throws Exception {
    Map<String, RootAffixCard> byId = new LinkedHashMap<>();
    for (String assetFile : BOOK_ASSET_FILES) {
      if ("MEDICAL_RESP.json".equals(assetFile)) { continue; }
      try (InputStream stream = assets.open("seed-vocab/book/" + assetFile);
           BufferedReader reader = new BufferedReader(new InputStreamReader(stream, StandardCharsets.UTF_8))) {
        StringBuilder builder = new StringBuilder();
        String line;
        while ((line = reader.readLine()) != null) { builder.append(line); }
        JSONArray arr = new JSONArray(builder.toString());
        for (int i = 0; i < arr.length(); i += 1) {
          JSONObject item = arr.getJSONObject(i);
          JSONObject word = item.optJSONObject("content") == null ? null : item.optJSONObject("content").optJSONObject("word");
          JSONObject content = word == null ? null : word.optJSONObject("content");
          String displayWord = word == null ? item.optString("headWord") : word.optString("wordHead", item.optString("headWord"));
          String gloss = primaryMeaning(content);
          String remMethod = content == null || content.optJSONObject("remMethod") == null ? "" : content.optJSONObject("remMethod").optString("val");
          for (RootAffixCard card : parseSharedRootAffixCardsStable(displayWord, gloss, remMethod)) {
            mergeRootAffixCard(byId, card);
          }
        }
      }
    }
    List<RootAffixCard> cards = new ArrayList<>();
    for (RootAffixCard card : byId.values()) {
      if (isReliableRootAffixCard(card)) {
        cards.add(card);
      }
    }
    return cards;
  }

  private List<RootAffixCard> parseSharedRootAffixCards(String word, String gloss, String remMethod) {
    List<RootAffixCard> cards = new ArrayList<>();
    if (remMethod == null || remMethod.isEmpty() || word == null || word.length() < 3) { return cards; }
    String normalized = remMethod.replace('+', '·');
    String formula = normalized.split("→")[0];
    String[] segments = formula.split("·");
    for (int i = 0; i < segments.length; i += 1) {
      String segment = segments[i].trim();
      int open = segment.indexOf('(');
      int close = segment.indexOf(')');
      if (open < 0 || close <= open) { continue; }
      String form = segment.substring(0, open).trim();
      String meaning = segment.substring(open + 1, close).trim();
      String normalizedForm = normalizeRootAffixForm(form);
      if (normalizedForm.length() < 2 || meaning.isEmpty()) { continue; }
      String displayForm = form.endsWith("-") || form.startsWith("-") ? form : formatSharedRootAffixForm(word.toLowerCase(Locale.ROOT), normalizedForm, i, segments.length);
      String id = "root_affix_shared_" + normalizedForm;
      cards.add(new RootAffixCard(
          id,
          displayForm,
          meaning,
          word,
          gloss,
          "shared"));
    }
    return cards;
  }

  private String formatSharedRootAffixForm(String word, String normalizedForm, int index, int total) {
    if (index == 0 && total > 1 && word.startsWith(normalizedForm)) { return normalizedForm + "-"; }
    if (index + 1 == total && total > 1 && word.endsWith(normalizedForm)) { return "-" + normalizedForm; }
    return normalizedForm;
  }

  private List<RootAffixCard> parseSharedRootAffixCardsStable(String word, String gloss, String remMethod) {
    List<RootAffixCard> cards = new ArrayList<>();
    if (remMethod == null || remMethod.isEmpty() || word == null || word.length() < 3) { return cards; }
    String formula = trimBeforeArrow(remMethod);
    Matcher matcher = Pattern.compile("([A-Za-z-]{2,12})\\s*\\(([^)]+)\\)").matcher(formula);
    List<String[]> matches = new ArrayList<>();
    while (matcher.find()) {
      matches.add(new String[] {matcher.group(1), matcher.group(2)});
    }
    String lowerWord = word.toLowerCase(Locale.ROOT);
    for (int i = 0; i < matches.size(); i += 1) {
      String form = matches.get(i)[0].trim();
      String meaning = sanitizeChineseMeaning(matches.get(i)[1]);
      String normalizedForm = normalizeRootAffixForm(form);
      if (normalizedForm.length() < 2 || meaning.isEmpty()) { continue; }
      if (normalizedForm.length() >= lowerWord.length()) { continue; }
      if (!lowerWord.contains(normalizedForm) && !form.startsWith("-") && !form.endsWith("-")) { continue; }
      String displayForm = form.endsWith("-") || form.startsWith("-") ? form : formatSharedRootAffixForm(lowerWord, normalizedForm, i, matches.size());
      RootAffixCard card = new RootAffixCard(
          "root_affix_shared_" + normalizedForm,
          displayForm,
          meaning,
          word,
          gloss,
          "shared");
      card.exampleCount = 1;
      cards.add(card);
    }
    return cards;
  }

  private List<RootAffixCard> loadMedicalRootAffixCards(AssetManager assets) throws Exception {
    List<RootAffixCard> cards = new ArrayList<>();
    try (InputStream stream = assets.open(MEDICAL_ROOT_AFFIX_ASSET);
         BufferedReader reader = new BufferedReader(new InputStreamReader(stream, StandardCharsets.UTF_8))) {
      String line;
      RootAffixCard last = null;
      while ((line = reader.readLine()) != null) {
        String trimmed = line.trim();
        if (trimmed.isEmpty()) { continue; }
        if (trimmed.contains("常见前缀") || trimmed.contains("常见词根") || trimmed.contains("常见后缀") || trimmed.startsWith("四、")) {
          continue;
        }
        if (trimmed.startsWith("例：") && last != null) {
          String payload = trimmed.substring(2).trim().replace('，', ',');
          String[] examples = payload.split("；|,");
          if (examples.length > 0) { last.exampleWords = joinExamples(examples, true); }
          if (examples.length > 0) { last.exampleGlosses = joinExamples(examples, false); }
          continue;
        }
        String[] parts = trimmed.split("\\s{2,}");
        if (parts.length < 2) { continue; }
        String form = parts[0].trim();
        String meaning = parts[1].trim();
        String normalizedForm = normalizeRootAffixForm(form);
        if (normalizedForm.isEmpty() || !containsHan(meaning)) { continue; }
        last = new RootAffixCard(
            "root_affix_medical_" + normalizedForm,
            form,
            meaning,
            "",
            "",
            "medical");
        last.exampleCount = 1;
        cards.add(last);
      }
    }
    return cards;
  }

  private void mergeRootAffixCard(Map<String, RootAffixCard> byId, RootAffixCard incoming) {
    RootAffixCard existing = byId.get(incoming.id);
    if (existing == null) {
      byId.put(incoming.id, incoming);
      return;
    }
    existing.exampleWords = mergeExampleText(existing.exampleWords, incoming.exampleWords);
    existing.exampleGlosses = mergeExampleText(existing.exampleGlosses, incoming.exampleGlosses);
    existing.exampleCount += 1;
  }

  private String mergeExampleText(String current, String incoming) {
    Set<String> merged = new LinkedHashSet<>();
    for (String item : (current == null ? "" : current).split(",")) {
      String trimmed = item.trim();
      if (!trimmed.isEmpty()) {
        merged.add(trimmed);
      }
    }
    for (String item : (incoming == null ? "" : incoming).split(",")) {
      String trimmed = item.trim();
      if (!trimmed.isEmpty()) {
        merged.add(trimmed);
      }
    }
    return String.join(", ", merged);
  }

  private String trimBeforeArrow(String value) {
    int unicodeArrow = value.indexOf('\u2192');
    if (unicodeArrow >= 0) {
      return value.substring(0, unicodeArrow);
    }
    int asciiArrow = value.indexOf("->");
    if (asciiArrow >= 0) {
      return value.substring(0, asciiArrow);
    }
    return value;
  }

  private boolean containsHan(String text) {
    if (text == null || text.isEmpty()) {
      return false;
    }
    for (int i = 0; i < text.length(); i += 1) {
      if (Character.UnicodeScript.of(text.charAt(i)) == Character.UnicodeScript.HAN) {
        return true;
      }
    }
    return false;
  }

  private boolean isReliableRootAffixCard(RootAffixCard card) {
    String normalized = normalizeRootAffixForm(card.form);
    if (normalized.length() < 2 || normalized.length() > 6) {
      return false;
    }
    if ("shared".equals(card.scope) && normalized.length() > 4 && (card.form.startsWith("-") || card.form.endsWith("-"))) {
      return false;
    }
    if (!containsHan(card.meaningCn) || sanitizeChineseMeaning(card.meaningCn).isEmpty()) {
      return false;
    }
    if ("medical".equals(card.scope)) {
      return true;
    }
    boolean explicitAffixShape = card.form.startsWith("-") || card.form.endsWith("-");
    return explicitAffixShape || card.exampleCount >= 1;
  }

  private int rootAffixSelectionOffset(int poolSize) {
    if (poolSize <= 0) {
      return 0;
    }
    int daySeed = Math.abs(today().hashCode());
    int completedSeed = done(today(), "rootAffix");
    return Math.floorMod(daySeed + completedSeed, poolSize);
  }

  private String joinExamples(String[] examples, boolean words) {
    List<String> values = new ArrayList<>();
    for (String example : examples) {
      String trimmed = example.trim();
      if (trimmed.isEmpty()) { continue; }
      int open = trimmed.indexOf('（');
      int close = trimmed.indexOf('）');
      if (open > 0 && close > open) {
        values.add(words ? trimmed.substring(0, open) : trimmed.substring(open + 1, close));
      }
    }
    return String.join(", ", values);
  }

  private String normalizeRootAffixForm(String form) {
    return form
        .trim()
        .replace("/", "")
        .replace("-", "")
        .replace(" ", "")
        .replace("　", "")
        .chars()
        .filter(ch -> Character.isLetter(ch))
        .collect(StringBuilder::new, StringBuilder::appendCodePoint, StringBuilder::append)
        .toString()
        .toLowerCase(Locale.ROOT);
  }

  private List<Card> loadCardsFromAssets() {
    List<Card> cards = new ArrayList<>();
    try {
      AssetManager assets = getReactApplicationContext().getAssets();
      for (int i = 0; i < BOOK_ASSET_FILES.length; i += 1) {
        cards.addAll(loadBookCards(assets, "seed-vocab/book/" + BOOK_ASSET_FILES[i], BOOK_IDS[i]));
      }
    } catch (Exception ignored) {}
    return cards;
  }

  private List<Card> loadBookCards(AssetManager assets, String assetPath, int bookId) throws Exception {
    List<Card> cards = new ArrayList<>();
    try (InputStream stream = assets.open(assetPath);
         BufferedReader reader = new BufferedReader(new InputStreamReader(stream, StandardCharsets.UTF_8))) {
      StringBuilder builder = new StringBuilder();
      String line;
      while ((line = reader.readLine()) != null) { builder.append(line); }
      JSONArray arr = new JSONArray(builder.toString());
      for (int i = 0; i < arr.length(); i += 1) {
        JSONObject item = arr.getJSONObject(i);
        JSONObject word = item.optJSONObject("content") == null ? null : item.optJSONObject("content").optJSONObject("word");
        JSONObject content = word == null ? null : word.optJSONObject("content");
        if (word == null || content == null) { continue; }

        String displayWord = word.optString("wordHead", item.optString("headWord"));
        String sourceId = word.optString("wordId", item.optString("headWord"));
        List<MeaningDetail> meaningDetails = collectMeaningDetails(content);
        List<String> meanings = meaningTexts(meaningDetails);
        String meaning = meanings.isEmpty() ? "" : meanings.get(0);
        if (displayWord.isEmpty() || sourceId.isEmpty() || meaning.isEmpty()) { continue; }

        String phonetic = content.optString("usphone", content.optString("ukphone", ""));
        JSONObject examplePair = firstExample(content);
        cards.add(new Card(
            bookId,
            sourceId,
            displayWord,
            meaning,
            meanings,
            meaningDetails,
            frequencyScore(item, i),
            primaryPartOfSpeech(content),
            phonetic,
            examplePair.optString("en"),
            examplePair.optString("cn")));
      }
    }
    return cards;
  }

  private String primaryMeaning(JSONObject content) {
    List<String> meanings = collectMeanings(content);
    return meanings.isEmpty() ? "" : meanings.get(0);
  }

  private List<String> collectMeanings(JSONObject content) {
    return meaningTexts(collectMeaningDetails(content));
  }

  private List<String> meaningTexts(List<MeaningDetail> meaningDetails) {
    List<String> meanings = new ArrayList<>();
    for (MeaningDetail detail : meaningDetails) {
      if (!detail.meaningCn.isEmpty() && !meanings.contains(detail.meaningCn)) {
        meanings.add(detail.meaningCn);
      }
      if (meanings.size() >= 4) {
        break;
      }
    }
    return meanings;
  }

  private List<MeaningDetail> collectMeaningDetails(JSONObject content) {
    List<MeaningDetail> meanings = new ArrayList<>();
    JSONArray trans = content.optJSONArray("trans");
    if (trans != null) {
      for (int i = 0; i < trans.length() && meanings.size() < 4; i += 1) {
        JSONObject entry = trans.optJSONObject(i);
        if (entry == null) { continue; }
        String normalized = sanitizeChineseMeaning(entry.optString("tranCn"));
        if (!normalized.isEmpty() && meaningDetailIndex(meanings, normalized) < 0) {
          meanings.add(new MeaningDetail(
              normalizePartOfSpeech(entry.optString("pos")),
              normalized,
              entry.optString("tranOther")));
        }
      }
    }
    if (!meanings.isEmpty()) {
      return meanings;
    }
    JSONArray synos = content.optJSONObject("syno") == null ? null : content.optJSONObject("syno").optJSONArray("synos");
    if (synos != null) {
      for (int i = 0; i < synos.length() && meanings.size() < 4; i += 1) {
        JSONObject entry = synos.optJSONObject(i);
        if (entry == null) { continue; }
        String normalized = sanitizeChineseMeaning(entry.optString("tran"));
        if (!normalized.isEmpty() && meaningDetailIndex(meanings, normalized) < 0) {
          meanings.add(new MeaningDetail(
              normalizePartOfSpeech(entry.optString("pos")),
              normalized,
              null));
        }
      }
    }
    return meanings;
  }

  private int meaningDetailIndex(List<MeaningDetail> meanings, String meaningCn) {
    for (int i = 0; i < meanings.size(); i += 1) {
      if (meanings.get(i).meaningCn.equals(meaningCn)) {
        return i;
      }
    }
    return -1;
  }

  private double frequencyScore(JSONObject item, int rankIndex) {
    JSONObject content = item.optJSONObject("content");
    JSONObject word = content == null ? null : content.optJSONObject("word");
    int rank = 0;
    if (word != null) {
      rank = word.optInt("wordRank", 0);
    }
    if (rank <= 0) {
      rank = item.optInt("wordRank", rankIndex + 1);
    }
    if (rank <= 0) {
      rank = rankIndex + 1;
    }
    return 10000.0 / Math.max(rank, 1);
  }

  private String sanitizeChineseMeaning(String raw) {
    if (raw == null) { return ""; }
    String cleaned = raw
        .replace("；", "、")
        .replace(";", "、")
        .replace("，", "、")
        .replace(",", "、")
        .replace("：", "")
        .replace(":", "")
        .replace("<", "")
        .replace(">", "")
        .replace("[", "")
        .replace("]", "")
        .replace("(", "")
        .replace(")", "")
        .replace("（", "")
        .replace("）", "")
        .replace("…", "")
        .replace("...", "")
        .replaceAll("[A-Za-z]+$", "")
        .replaceAll("[^\\p{IsHan}、；/]", "")
        .trim();

    String[] parts = cleaned.split("[、/]");
    List<String> normalized = new ArrayList<>();
    for (String part : parts) {
      String item = part.trim();
      if (item.isEmpty()) { continue; }
      if (item.length() > 10) { continue; }
      if (!normalized.contains(item)) {
        normalized.add(item);
      }
      if (normalized.size() >= 2) { break; }
    }

    if (normalized.isEmpty()) {
      return cleaned.length() > 12 ? cleaned.substring(0, 12) : cleaned;
    }
    return String.join("、", normalized);
  }

  private String primaryPartOfSpeech(JSONObject content) {
    JSONArray trans = content.optJSONArray("trans");
    if (trans != null && trans.length() > 0) {
      JSONObject first = trans.optJSONObject(0);
      if (first != null) {
        return normalizePartOfSpeech(first.optString("pos"));
      }
    }
    JSONArray synos = content.optJSONObject("syno") == null ? null : content.optJSONObject("syno").optJSONArray("synos");
    if (synos != null && synos.length() > 0) {
      JSONObject first = synos.optJSONObject(0);
      if (first != null) {
        return normalizePartOfSpeech(first.optString("pos"));
      }
    }
    return "";
  }

  private JSONObject firstExample(JSONObject content) throws JSONException {
    JSONArray sentences = content.optJSONObject("sentence") == null ? null : content.optJSONObject("sentence").optJSONArray("sentences");
    if (sentences != null && sentences.length() > 0) {
      JSONObject first = sentences.optJSONObject(0);
      if (first != null) {
        return new JSONObject().put("en", first.optString("sContent")).put("cn", first.optString("sCn"));
      }
    }
    return new JSONObject().put("en", "").put("cn", "");
  }

  private String normalizePartOfSpeech(String raw) {
    if (raw == null) { return ""; }
    String value = raw.trim().toLowerCase(Locale.ROOT);
    if (value.startsWith("vt") || value.startsWith("vi") || value.startsWith("v")) { return "v."; }
    if (value.startsWith("n")) { return "n."; }
    if (value.startsWith("adj")) { return "adj."; }
    if (value.startsWith("adv")) { return "adv."; }
    return value;
  }

  private static final class MeaningDetail {
    final String pos; final String meaningCn; final String meaningEn;
    MeaningDetail(String pos, String meaningCn, String meaningEn) { this.pos = pos == null ? "" : pos; this.meaningCn = meaningCn; this.meaningEn = meaningEn; }
  }

  private static final class Card {
    final int bookId; final String id; final String word; final String meaning; final List<String> meanings; final List<MeaningDetail> meaningDetails; final double frequency; final String partOfSpeech; final String phonetic; final String exampleEn; final String exampleCn;
    Card(int bookId, String id, String meaning, String phonetic, String exampleEn, String exampleCn) { this(bookId, id, id, meaning, Collections.singletonList(meaning), defaultMeaningDetails(Collections.singletonList(meaning), "", id), 0.0, "", phonetic, exampleEn, exampleCn); }
    Card(int bookId, String id, String word, String meaning, String partOfSpeech, String phonetic, String exampleEn, String exampleCn) { this(bookId, id, word, meaning, Collections.singletonList(meaning), defaultMeaningDetails(Collections.singletonList(meaning), partOfSpeech, word), 0.0, partOfSpeech, phonetic, exampleEn, exampleCn); }
    Card(int bookId, String id, String word, String meaning, List<String> meanings, List<MeaningDetail> meaningDetails, double frequency, String partOfSpeech, String phonetic, String exampleEn, String exampleCn) { this.bookId = bookId; this.id = id; this.word = word; this.meaning = meaning; this.meanings = meanings; this.meaningDetails = meaningDetails; this.frequency = frequency; this.partOfSpeech = partOfSpeech; this.phonetic = phonetic; this.exampleEn = exampleEn; this.exampleCn = exampleCn; }
    private static List<MeaningDetail> defaultMeaningDetails(List<String> meanings, String partOfSpeech, String word) {
      List<MeaningDetail> details = new ArrayList<>();
      for (String meaning : meanings) {
        details.add(new MeaningDetail(partOfSpeech, meaning, word));
      }
      return details;
    }
  }

  private static final class RootAffixCard {
    final String id; final String form; final String meaningCn; String exampleWords; String exampleGlosses; final String scope; int exampleCount;
    RootAffixCard(String id, String form, String meaningCn, String exampleWords, String exampleGlosses, String scope) { this.id = id; this.form = form; this.meaningCn = meaningCn; this.exampleWords = exampleWords; this.exampleGlosses = exampleGlosses; this.scope = scope; this.exampleCount = 0; }
    Object exampleSentence() { return exampleWords == null || exampleWords.isEmpty() ? JSONObject.NULL : exampleWords; }
    Object exampleTranslation() { return exampleGlosses == null || exampleGlosses.isEmpty() ? JSONObject.NULL : exampleGlosses; }
  }

  private static final class TargetBreakdown {
    final int total; final int base; final int carryover;
    TargetBreakdown(int total, int base, int carryover) { this.total = total; this.base = base; this.carryover = carryover; }
  }
}
