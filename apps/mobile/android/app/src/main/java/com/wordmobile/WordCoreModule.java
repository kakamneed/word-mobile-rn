package com.wordmobile;

import android.content.Context;
import android.content.SharedPreferences;
import com.facebook.react.bridge.Promise;
import com.facebook.react.bridge.ReactApplicationContext;
import com.facebook.react.bridge.ReactContextBaseJavaModule;
import com.facebook.react.bridge.ReactMethod;
import java.io.File;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Date;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

public class WordCoreModule extends ReactContextBaseJavaModule {
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
      new Card(2, "coherent", "连贯的", "/kəʊˈhɪərənt/", "Her explanation was coherent.", "她的解释很连贯。"),
      new Card(2, "resilient", "有韧性的", "/rɪˈzɪliənt/", "Resilient teams recover quickly.", "有韧性的团队恢复得更快。"),
      new Card(3, "infer", "推断", "/ɪnˈfɜːr/", "We can infer the result from the data.", "我们可以从数据中推断结果。"),
      new Card(3, "compile", "整理", "/kəmˈpaɪl/", "Compile the report before noon.", "请在中午前整理报告。"),
      new Card(4, "clinical", "临床的", "/ˈklɪnɪkəl/", "She works in a clinical team.", "她在临床团队工作。"),
      new Card(4, "therapy", "治疗", "/ˈθerəpi/", "The therapy was effective.", "这种治疗很有效。"),
      new Card(4, "diagnosis", "诊断", "/ˌdaɪəɡˈnəʊsɪs/", "The diagnosis was confirmed yesterday.", "诊断结果昨天确认了。"),
      new Card(4, "symptom", "症状", "/ˈsɪmptəm/", "Fever is a common symptom.", "发热是常见症状。")
  };

  public WordCoreModule(ReactApplicationContext context) {
    super(context);
    ensureDefaults();
  }

  @Override
  public String getName() {
    return "WordCoreModule";
  }

  @ReactMethod
  public void getBootstrapState(Promise promise) {
    try {
      ensureDefaults();
      JSONObject o = new JSONObject();
      o.put("appReady", true);
      o.put("firstRunRequired", false);
      o.put("databaseStatus", "ready");
      o.put("snapshotStatus", "ready");
      o.put("connectivityStatus", "offline");
      o.put("aiConfigStatus", "missing");
      o.put("settingsEntryAvailable", true);
      o.put("blockingReason", JSONObject.NULL);
      promise.resolve(o.toString());
    } catch (Exception e) {
      promise.reject("BOOTSTRAP_ERROR", e);
    }
  }

  @ReactMethod
  public void getTodayHomeState(Promise promise) {
    try {
      ensureDefaults();
      String today = today();
      pruneSessions(today);
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
      JSONObject o = new JSONObject();
      o.put("aiConfigured", false);
      o.put("syncConfigured", false);
      o.put("appVersion", BuildConfig.VERSION_NAME);
      o.put("schemaVersion", 1);
      o.put("databasePath", new File(getReactApplicationContext().getFilesDir(), "word.db").getAbsolutePath());
      promise.resolve(o.toString());
    } catch (Exception e) {
      promise.reject("SETTINGS_ERROR", e);
    }
  }

  @ReactMethod
  public void getActivePlan(Promise promise) {
    promise.resolve(plan(false).toString());
  }

  @ReactMethod
  public void savePlan(String requestJson, Promise promise) {
    try {
      JSONObject input = new JSONObject(requestJson).optJSONObject("input");
      if (input == null) {
        throw new IllegalArgumentException("Missing plan input.");
      }
      JSONObject next = mergePlan(plan(false), input);
      write(KEY_SAVED_PLAN, next);
      promise.resolve(next.toString());
    } catch (Exception e) {
      promise.reject("SAVE_PLAN_ERROR", e);
    }
  }

  @ReactMethod
  public void applySavedPlanToToday(Promise promise) {
    try {
      JSONObject applied = new JSONObject(plan(false).toString());
      write(KEY_TODAY_PLAN, applied);
      write(wordbooksKey(true), new JSONObject(read(wordbooksKey(false)).toString()));
      resetTodayStudyState(today());
      promise.resolve(applied.toString());
    } catch (Exception e) {
      promise.reject("APPLY_PLAN_ERROR", e);
    }
  }

  @ReactMethod
  public void getWordbooks(Promise promise) {
    try {
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

  @ReactMethod
  public void submitStudyAnswer(String requestJson, Promise promise) {
    try {
      JSONObject req = new JSONObject(requestJson);
      String questionId = req.optString("questionId");
      String mode = questionId.contains("-session-") ? questionId.substring(0, questionId.indexOf("-session-")) : "newWord";
      JSONObject sessions = read(KEY_SESSIONS);
      JSONObject s = sessions.optJSONObject(mode);
      if (s == null) {
        throw new IllegalStateException("No active session for " + mode);
      }
      JSONArray qs = s.getJSONArray("questions");
      int index = s.optInt("nextIndex");
      JSONObject q = qs.getJSONObject(index);
      String response = req.optString("response", "");
      boolean correct = judge(q, normalize(response));
      s.put("nextIndex", index + 1);
      s.put("totalTimeMs", s.optInt("totalTimeMs") + req.optInt("responseTimeMs"));
      String counter = correct ? "correctCount" : "incorrectCount";
      s.put(counter, s.optInt(counter) + 1);
      if (!correct) {
        pushUnique(s.getJSONArray("wrongIds"), q.optString("entrySourceId"));
      }

      JSONObject result = new JSONObject();
      result.put("questionId", questionId);
      result.put("entrySourceId", q.optString("entrySourceId"));
      result.put("questionType", q.optString("questionType"));
      result.put("userResponse", response);
      result.put("normalizedResponse", normalize(response));
      result.put("correctAnswer", answerText(q));
      result.put("outcome", correct ? "correct" : "incorrect");
      result.put("responseTimeMs", req.optInt("responseTimeMs"));
      result.put("answeredAt", now());

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
        growWrongPool(s.getJSONArray("wrongIds"));
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
      for (String mode : MODES) {
        JSONObject candidate = sessions.optJSONObject(mode);
        if (candidate != null && sessionId.equals(candidate.optString("sessionId"))) {
          found = candidate;
          sessions.remove(mode);
          break;
        }
      }
      if (found == null) {
        throw new IllegalStateException("Session not found: " + sessionId);
      }
      write(KEY_SESSIONS, sessions);
      JSONObject payload = new JSONObject();
      payload.put("summary", found.optJSONObject("summary") != null ? found.optJSONObject("summary") : summary(found));
      payload.put("nextAction", nextHint(today()));
      promise.resolve(payload.toString());
    } catch (Exception e) {
      promise.reject("COMPLETE_SESSION_ERROR", e);
    }
  }

  @ReactMethod
  public void cancelStudySession(Promise promise) {
    write(KEY_SESSIONS, new JSONObject());
    promise.resolve(null);
  }

  @ReactMethod
  public void getReportsOverview(Promise promise) {
    try {
      promise.resolve(buildReportsOverview().toString());
    } catch (Exception e) {
      promise.reject("REPORTS_ERROR", e);
    }
  }

  @ReactMethod
  public void getWrongWords(String filter, Promise promise) {
    try {
      promise.resolve(buildWrongWordEntries(filter).toString());
    } catch (Exception e) {
      promise.reject("WRONG_WORDS_ERROR", e);
    }
  }

  @ReactMethod
  public void getWrongWordDetail(int entryId, Promise promise) {
    try {
      promise.resolve(buildWrongWordDetail(entryId).toString());
    } catch (Exception e) {
      promise.reject("WRONG_WORD_DETAIL_ERROR", e);
    }
  }

  @ReactMethod
  public void getTodayAiPassageContext(Promise promise) {
    try {
      String currentDate = today();
      JSONObject snapshot = snapshot(currentDate);
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
              .put("wrongWords", collectDayWrongWords(currentDate))
              .toString());
    } catch (Exception e) {
      promise.reject("TODAY_AI_CONTEXT_ERROR", e);
    }
  }

  @ReactMethod
  public void generateAiPassage(String requestJson, Promise promise) {
    try {
      JSONObject request = new JSONObject(requestJson);
      promise.resolve(createAiPassage(request).toString());
    } catch (Exception e) {
      promise.reject("AI_PASSAGE_ERROR", e);
    }
  }

  @ReactMethod
  public void getAiPassageHistory(Promise promise) {
    try {
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
  }

  private JSONObject defaultPlan() {
    JSONObject p = new JSONObject();
    try {
      p.put("id", 1); p.put("name", "Starter Plan"); p.put("newWordsPerDay", 5); p.put("reviewWordsPerDay", 6); p.put("mixedTestPerDay", 4); p.put("wrongWordTestPerDay", 3); p.put("rootAffixPerDay", 2); p.put("growthIntervalDays", 7); p.put("growthIncrement", 5);
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
    for (String key : Arrays.asList("name", "newWordsPerDay", "reviewWordsPerDay", "mixedTestPerDay", "wrongWordTestPerDay", "rootAffixPerDay", "growthIntervalDays", "growthIncrement")) {
      if (input.has(key)) { next.put(key, input.get(key)); }
    }
    return next;
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
    s.put("newWordsTarget", targetQuestions(p, "newWord")); s.put("newWordsCompleted", done(today, "newWord"));
    s.put("reviewWordsTarget", targetQuestions(p, "review")); s.put("reviewWordsCompleted", done(today, "review"));
    s.put("mixedTestTarget", targetQuestions(p, "mixedTest")); s.put("mixedTestCompleted", done(today, "mixedTest"));
    s.put("wrongWordTestTarget", targetQuestions(p, "wrongWordReinforcement")); s.put("wrongWordTestCompleted", done(today, "wrongWordReinforcement"));
    s.put("rootAffixTarget", targetQuestions(p, "rootAffix")); s.put("rootAffixCompleted", done(today, "rootAffix"));
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
      day.put("newWord", Math.min(day.optInt("newWord"), targetQuestions(p, "newWord")));
      day.put("review", Math.min(day.optInt("review"), targetQuestions(p, "review")));
      day.put("mixedTest", Math.min(day.optInt("mixedTest"), targetQuestions(p, "mixedTest")));
      day.put("wrongWordReinforcement", Math.min(day.optInt("wrongWordReinforcement"), targetQuestions(p, "wrongWordReinforcement")));
      day.put("rootAffix", Math.min(day.optInt("rootAffix"), targetQuestions(p, "rootAffix")));
      all.put(today, day); write(KEY_COMPLETED, all);
    } catch (Exception ignored) {}
  }

  private int done(String today, String mode) {
    JSONObject all = read(KEY_COMPLETED); JSONObject day = all.optJSONObject(today); int persisted = day == null ? 0 : day.optInt(mode); JSONObject live = read(KEY_SESSIONS).optJSONObject(mode);
    if (live != null && !studyBasisSignature().equals(live.optString("basisSignature"))) { live = null; }
    if (live == null || live.optBoolean("completed")) { return persisted; }
    int answered = live.optInt("nextIndex");
    return Math.min(targetQuestions(plan(true), mode), persisted + answered);
  }

  private void addDone(String today, String mode, int amount) throws JSONException {
    JSONObject all = read(KEY_COMPLETED); JSONObject day = all.optJSONObject(today); if (day == null) { day = new JSONObject(); }
    int max = targetQuestions(plan(true), mode);
    day.put(mode, Math.min(day.optInt(mode) + amount, max)); all.put(today, day); write(KEY_COMPLETED, all);
  }

  private int configuredTarget(JSONObject plan, String mode) {
    if ("newWord".equals(mode)) { return plan.optInt("newWordsPerDay"); }
    if ("review".equals(mode)) { return plan.optInt("reviewWordsPerDay"); }
    if ("mixedTest".equals(mode)) { return plan.optInt("mixedTestPerDay"); }
    if ("wrongWordReinforcement".equals(mode)) { return plan.optInt("wrongWordTestPerDay"); }
    return plan.optInt("rootAffixPerDay");
  }

  private int targetQuestions(JSONObject plan, String mode) { return Math.min(configuredTarget(plan, mode) * questionMultiplier(mode), availableQuestionCapacity(mode)); }
  private int remainingQuestions(String mode) { return Math.max(targetQuestions(plan(true), mode) - done(today(), mode), 0); }
  private int remainingBundleCount(String mode) { return Math.max(configuredTarget(plan(true), mode) - (done(today(), mode) / questionMultiplier(mode)), 0); }
  private int questionMultiplier(String mode) { return ("newWord".equals(mode) || "review".equals(mode)) ? 4 : ("rootAffix".equals(mode) ? 2 : 1); }
  private int availableQuestionCapacity(String mode) {
    if ("newWord".equals(mode)) { return newWordCards().size() * 4; }
    if ("review".equals(mode)) { return reviewCards().size() * 4; }
    if ("mixedTest".equals(mode)) { return mixedTestCards().size() * 4; }
    if ("wrongWordReinforcement".equals(mode)) { return wrongPoolCards().size() * 4; }
    return Math.min(configuredTarget(plan(true), mode), ROOTS.length) * 2;
  }
  private String studyBasisSignature() {
    return plan(true).toString() + "|" + read(wordbooksKey(true)).toString();
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
      int limit = requested != null && requested.length() > 0 ? requested.length() : Math.min(remainingBundleCount(mode), ROOTS.length);
      for (int i = 0; i < limit; i += 1) {
        int index = requested != null && requested.length() > i ? indexOf(ROOT_IDS, requested.optString(i)) : i;
        if (index < 0) { index = i; }
        arr.put(rootQuestion(ROOT_IDS[index], ROOTS[index], "glossToRootInput"));
      }
      for (int i = 0; i < limit; i += 1) {
        int index = requested != null && requested.length() > i ? indexOf(ROOT_IDS, requested.optString(i)) : i;
        if (index < 0) { index = i; }
        arr.put(rootQuestion(ROOT_IDS[index], ROOTS[index], "rootToGlossInput"));
      }
      return arr;
    }
    List<Card> selected = requestedCards(requested);
    if (selected.isEmpty()) { selected = autoCards(mode); }
    List<Card> distractors = distractors(selected);
    if ("newWord".equals(mode) || "review".equals(mode)) {
      for (Card c : selected) { arr.put(choiceQuestion("enToCnChoice", c, distractors, false)); }
      for (Card c : selected) { arr.put(choiceQuestion("exampleToCnChoice", c, distractors, true)); }
      for (Card c : selected) { arr.put(englishChoice(c, distractors)); }
      for (Card c : selected) { arr.put(inputQuestion(c)); }
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
    if (pool.isEmpty()) { pool = Arrays.asList(CARDS); }
    List<Card> list = new ArrayList<>(pool);
    if ("mixedTest".equals(mode) && list.size() > 2) { Collections.rotate(list, -2); }
    if ("mixedTest".equals(mode) || "wrongWordReinforcement".equals(mode)) {
      return list;
    }
    return new ArrayList<>(list.subList(0, Math.min(remainingBundleCount(mode), list.size())));
  }

  private List<Card> activeCards() {
    JSONObject active = read(wordbooksKey(true)); List<Card> list = new ArrayList<>();
    for (Card c : CARDS) { if (active.optBoolean(String.valueOf(c.bookId), false)) { list.add(c); } }
    return list;
  }

  private List<Card> wrongPoolCards() {
    JSONObject pool = read(KEY_WRONG_POOL); List<Card> list = new ArrayList<>();
    for (Card c : CARDS) { if (wrongMeta(pool, c.id).optInt("count", 0) > 0) { list.add(c); } }
    return list;
  }

  private List<Card> newWordCards() {
    JSONObject learned = read(KEY_LEARNED);
    List<Card> unseen = new ArrayList<>();
    for (Card c : activeCards()) {
      if (learned.optInt(c.id, 0) == 0) {
        unseen.add(c);
      }
    }
    return unseen.isEmpty() ? activeCards() : unseen;
  }

  private List<Card> reviewCards() {
    JSONObject learned = read(KEY_LEARNED);
    List<Card> reviewed = new ArrayList<>();
    for (Card c : activeCards()) {
      if (learned.optInt(c.id, 0) > 0) {
        reviewed.add(c);
      }
    }
    if (reviewed.isEmpty()) {
      reviewed.addAll(activeCards());
    }
    if (reviewed.size() > 1) {
      Collections.rotate(reviewed, -1);
    }
    return reviewed;
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
    return pool;
  }

  private List<Card> distractors(List<Card> selected) {
    Set<String> ids = new LinkedHashSet<>(); for (Card c : selected) { ids.add(c.id); }
    List<Card> list = new ArrayList<>(); for (Card c : CARDS) { if (!ids.contains(c.id)) { list.add(c); } }
    return list.isEmpty() ? Arrays.asList(CARDS) : list;
  }

  private List<JSONObject> buildSampledQuestions(String mode, List<Card> pool, List<Card> distractors) throws JSONException {
    List<JSONObject> questions = new ArrayList<>();
    if (pool.isEmpty()) { return questions; }

    List<Card> shuffled = new ArrayList<>(pool);
    Collections.shuffle(shuffled);
    List<JSONObject> candidates = new ArrayList<>();
    for (String type : Arrays.asList("enToCnChoice", "exampleToCnChoice", "cnToEnChoice", "enToCnInput")) {
      for (Card card : shuffled) {
        if ("enToCnChoice".equals(type)) {
          candidates.add(choiceQuestion(type, card, distractors, false));
        } else if ("exampleToCnChoice".equals(type)) {
          candidates.add(choiceQuestion(type, card, distractors, true));
        } else if ("cnToEnChoice".equals(type)) {
          candidates.add(englishChoice(card, distractors));
        } else {
          candidates.add(inputQuestion(card));
        }
      }
    }
    int target = Math.min(remainingQuestions(mode), candidates.size());
    for (int i = 0; i < target; i += 1) { questions.add(candidates.get(i)); }
    return questions;
  }

  private JSONObject choiceQuestion(String type, Card c, List<Card> others, boolean example) throws JSONException {
    JSONObject q = baseQuestion(c.id, example ? c.id : c.id, c.phonetic, example ? c.exampleEn : c.id, new JSONArray().put(c.meaning), example ? c.exampleEn : JSONObject.NULL, example ? c.exampleCn : JSONObject.NULL);
    q.put("questionType", type); q.put("choices", chineseChoices(c, others)); return q;
  }

  private JSONObject englishChoice(Card c, List<Card> others) throws JSONException {
    JSONObject q = baseQuestion(c.id, c.meaning, "", c.meaning, new JSONArray().put(c.meaning), JSONObject.NULL, JSONObject.NULL);
    q.put("questionType", "cnToEnChoice"); q.put("choices", englishChoices(c, others)); return q;
  }

  private JSONObject inputQuestion(Card c) throws JSONException {
    JSONObject q = baseQuestion(c.id, c.id, c.phonetic, c.id, new JSONArray().put(c.meaning), JSONObject.NULL, JSONObject.NULL);
    q.put("questionType", "enToCnInput"); q.put("choices", JSONObject.NULL); q.put("correctChoiceLabel", JSONObject.NULL); return q;
  }

  private JSONObject rootQuestion(String id, String[] root, String type) throws JSONException {
    JSONObject q = baseQuestion(id, root[0], "", root[0], new JSONArray().put(root[1]), root[2], root[3]);
    q.put("questionType", type);
    if ("glossToRootInput".equals(type)) {
      q.put("prompt", "根据例词填写该词根/词缀的中文含义");
    } else {
      q.put("prompt", "根据词根/词缀和例词填写中文含义");
    }
    q.put("choices", JSONObject.NULL);
    q.put("correctChoiceLabel", JSONObject.NULL);
    return q;
  }

  private JSONObject baseQuestion(String id, String word, String phonetic, Object prompt, JSONArray meanings, Object exampleSentence, Object exampleTranslation) throws JSONException {
    JSONObject q = new JSONObject();
    q.put("entrySourceId", id); q.put("word", word); q.put("phoneticUs", phonetic); q.put("phoneticUk", phonetic); q.put("prompt", prompt); q.put("acceptedMeanings", meanings); q.put("exampleSentence", exampleSentence); q.put("exampleTranslation", exampleTranslation);
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
        c.id,
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
    return result;
  }

  private List<String> distractorWords(Card target, List<Card> others) {
    List<String> result = new ArrayList<>();
    for (Card candidate : rankedDistractorCards(target, others)) {
      if (!result.contains(candidate.id)) {
        result.add(candidate.id);
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

  private String partOfSpeechFor(Card card) {
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

  private boolean judge(JSONObject q, String normalizedResponse) {
    JSONArray choices = q.optJSONArray("choices");
    if (choices != null) { for (int i = 0; i < choices.length(); i += 1) { if (choices.optJSONObject(i).optBoolean("correct")) { return normalizedResponse.equals(normalize(choices.optJSONObject(i).optString("label"))); } } }
    JSONArray meanings = q.optJSONArray("acceptedMeanings");
    for (int i = 0; meanings != null && i < meanings.length(); i += 1) { if (normalizedResponse.equals(normalize(meanings.optString(i)))) { return true; } }
    return false;
  }

  private String answerText(JSONObject q) {
    JSONArray choices = q.optJSONArray("choices");
    if (choices != null) { for (int i = 0; i < choices.length(); i += 1) { if (choices.optJSONObject(i).optBoolean("correct")) { return choices.optJSONObject(i).optString("text"); } } }
    JSONArray meanings = q.optJSONArray("acceptedMeanings"); return meanings != null && meanings.length() > 0 ? meanings.optString(0) : "";
  }

  private JSONObject summary(JSONObject s) throws JSONException {
    int totalQ = s.getJSONArray("questions").length(); int correct = s.optInt("correctCount"); int incorrect = s.optInt("incorrectCount");
    return new JSONObject().put("sessionId", s.optString("sessionId")).put("totalQuestions", totalQ).put("correctCount", correct).put("fuzzyCorrectCount", 0).put("incorrectCount", incorrect).put("skippedCount", 0).put("totalWords", s.optInt("totalWords")).put("wrongWordCount", s.getJSONArray("wrongIds").length()).put("accuracyPercent", totalQ == 0 ? 0 : (correct * 100.0 / totalQ)).put("totalTimeMs", s.optInt("totalTimeMs")).put("completedAt", now());
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
    } catch (JSONException ignored) {}
    return meta;
  }
  private void growWrongPool(JSONArray wrongIds) throws JSONException {
    JSONObject pool = read(KEY_WRONG_POOL);
    for (int i = 0; i < wrongIds.length(); i += 1) {
      String id = wrongIds.optString(i);
      JSONObject meta = wrongMeta(pool, id);
      meta.put("count", meta.optInt("count", 0) + 1);
      meta.put("lastWrongAt", now());
      pool.put(id, meta);
    }
    write(KEY_WRONG_POOL, pool);
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
        .put("word", card.id)
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
    for (Card card : CARDS) {
      JSONObject meta = wrongMeta(pool, card.id);
      int count = meta.optInt("count", 0);
      if (count <= 0) {
        continue;
      }
      double priority = Math.min(10.0, count * 2.0);
      JSONObject item = new JSONObject();
      item.put("entryId", cardIndex(card.id));
      item.put("word", card.id);
      item.put("phoneticUs", card.phonetic);
      item.put("phoneticUk", card.phonetic);
      item.put("meanings", new JSONArray().put(card.meaning));
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

  private JSONObject buildWrongWordDetail(int entryId) throws JSONException {
    Card card = cardByIndex(entryId);
    if (card == null) {
      return new JSONObject();
    }
    JSONObject meta = wrongMeta(read(KEY_WRONG_POOL), card.id);
    JSONArray examples = new JSONArray()
        .put(new JSONObject().put("sentenceEn", card.exampleEn).put("sentenceCn", card.exampleCn));
    JSONArray meanings = new JSONArray()
        .put(new JSONObject().put("pos", "v").put("meaningCn", card.meaning).put("meaningEn", card.id));
    JSONArray history = new JSONArray()
        .put(new JSONObject().put("date", meta.optString("lastWrongAt", today())).put("context", "mobile review"));
    JSONArray related = new JSONArray();
    for (Card candidate : CARDS) {
      if (!candidate.id.equals(card.id) && related.length() < 4) {
        related.put(candidate.id);
      }
    }
    return new JSONObject()
        .put("entryId", entryId)
        .put("word", card.id)
        .put("lemma", card.id)
        .put("phoneticUs", card.phonetic)
        .put("phoneticUk", card.phonetic)
        .put("partOfSpeech", "v")
        .put("meanings", meanings)
        .put("examples", examples)
        .put("errorHistory", history)
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
      Card card = cardById(target);
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
      Card card = cardById(target);
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
    for (int i = 0; i < CARDS.length; i += 1) {
      if (CARDS[i].id.equals(id)) {
        return i + 1;
      }
    }
    return 0;
  }

  private Card cardByIndex(int entryId) {
    int index = entryId - 1;
    return index >= 0 && index < CARDS.length ? CARDS[index] : null;
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
  private Card cardById(String id) { for (Card c : CARDS) { if (c.id.equals(id)) { return c; } } return null; }

  private static final class Card {
    final int bookId; final String id; final String meaning; final String phonetic; final String exampleEn; final String exampleCn;
    Card(int bookId, String id, String meaning, String phonetic, String exampleEn, String exampleCn) { this.bookId = bookId; this.id = id; this.meaning = meaning; this.phonetic = phonetic; this.exampleEn = exampleEn; this.exampleCn = exampleCn; }
  }
}
