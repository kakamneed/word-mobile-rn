use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;

use crate::bridge::{
    accept_disputed_meaning, analyze_wrong_word_import, apply_saved_plan_to_today,
    build_reports_overview, build_today_ai_passage_context, build_today_home_state,
    build_wrong_word_detail, build_wrong_words, cancel_study_session, commit_wrong_word_import,
    complete_study_session, create_reward_image_upload, draw_today_reward, enqueue_cloud_backfill,
    generate_ai_passage, get_active_plan, get_ai_passage, get_ai_passage_history,
    get_ai_passage_style_preference, get_ai_provider_config, get_bootstrap_state,
    get_bridge_status, get_croc_bti_profile, get_local_leaderboard, get_reports_overview,
    get_resume_session_hint, get_reward_image_upload_entitlement, get_settings, get_sync_status,
    get_today_ai_passage_context, get_today_home_state, get_today_reward_state,
    get_word_hint_suggestions, get_wordbooks, get_wrong_word_detail, get_wrong_word_graph,
    get_wrong_words, initialize_mobile_runtime, list_reward_images, mark_onboarding_completed,
    mark_study_entry_mastered, moderate_reward_image, reconcile_local_data_owner,
    record_cloud_restore_attempt, record_sync_result, refresh_local_leaderboard_summary,
    refresh_reward_image_upload_entitlement, restore_cloud_ai_passage_snapshot,
    restore_cloud_data_snapshot, save_ai_passage, save_ai_passage_style_preference,
    save_ai_provider_config, save_croc_bti_profile, save_plan, save_word_hint,
    save_wrong_word_graph_position, seed_local_leaderboard_demo,
    select_leaderboard_reward_image_tag, start_study_session, submit_study_answer,
    switch_to_guest_local_data, toggle_wordbook, vote_reward_image,
};

fn to_java_string(env: &mut JNIEnv, value: &str) -> jstring {
    env.new_string(value)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

fn string_arg(env: &mut JNIEnv, input: JString) -> Result<String, String> {
    env.get_string(&input)
        .map(|value| value.into())
        .map_err(|error| format!("Failed to read JNI string: {error}"))
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
    app_data_dir: JString,
    app_config_dir: JString,
    app_cache_dir: JString,
    bundle_resource_dir: JString,
) -> jstring {
    let result = (|| -> Result<(), String> {
        initialize_mobile_runtime(
            string_arg(&mut env, app_data_dir)?,
            string_arg(&mut env, app_config_dir)?,
            string_arg(&mut env, app_cache_dir)?,
            string_arg(&mut env, bundle_resource_dir)?,
        )
    })();

    match result {
        Ok(()) => to_java_string(&mut env, ""),
        Err(error) => to_java_string(&mut env, &error),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetBridgeStatus(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_bridge_status() {
        Ok(status) => to_java_string(&mut env, &status),
        Err(error) => to_java_string(&mut env, &error),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetBootstrapState(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_bootstrap_state() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetTodayHomeState(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_today_home_state() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetTodayRewardState(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_today_reward_state() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeDrawTodayReward(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(draw_today_reward) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetRewardImageUploadEntitlement(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_reward_image_upload_entitlement() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeRefreshRewardImageUploadEntitlement(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(refresh_reward_image_upload_entitlement) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeCreateRewardImageUpload(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(create_reward_image_upload) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeListRewardImages(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(list_reward_images) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeModerateRewardImage(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(moderate_reward_image) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSelectLeaderboardRewardImageTag(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(select_leaderboard_reward_image_tag) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeVoteRewardImage(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(vote_reward_image) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetLocalLeaderboard(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(get_local_leaderboard) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeRefreshLocalLeaderboardSummary(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(refresh_local_leaderboard_summary) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSeedLocalLeaderboardDemo(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match seed_local_leaderboard_demo() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeMarkOnboardingCompleted(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match mark_onboarding_completed() {
        Ok(()) => to_java_string(&mut env, ""),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeBuildTodayHomeState(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(build_today_home_state) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeBuildReportsOverview(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(build_reports_overview) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeBuildWrongWords(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(build_wrong_words) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeBuildWrongWordDetail(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(build_wrong_word_detail) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeBuildTodayAiPassageContext(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(build_today_ai_passage_context) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetActivePlan(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_active_plan() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSavePlan(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(save_plan) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeApplySavedPlanToToday(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match apply_saved_plan_to_today() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetCrocBtiProfile(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_croc_bti_profile() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSaveCrocBtiProfile(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(save_croc_bti_profile) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetWordbooks(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_wordbooks() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetReportsOverview(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_reports_overview() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetWrongWords(
    mut env: JNIEnv,
    _class: JClass,
    filter: JString,
) -> jstring {
    match string_arg(&mut env, filter).and_then(get_wrong_words) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetWrongWordGraph(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_wrong_word_graph() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSaveWrongWordGraphPosition(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(save_wrong_word_graph_position) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}
#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetWrongWordDetail(
    mut env: JNIEnv,
    _class: JClass,
    entry_id: i32,
) -> jstring {
    match get_wrong_word_detail(entry_id as i64) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSaveWordHint(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(save_word_hint) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetWordHintSuggestions(
    mut env: JNIEnv,
    _class: JClass,
    entry_id: i32,
) -> jstring {
    match get_word_hint_suggestions(entry_id as i64) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetResumeSessionHint(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_resume_session_hint() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}
#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetTodayAiPassageContext(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_today_ai_passage_context() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetSettings(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_settings() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetSyncStatus(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_sync_status() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeRecordSyncResult(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(record_sync_result) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeRecordCloudRestoreAttempt(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(record_cloud_restore_attempt) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeEnqueueCloudBackfill(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(enqueue_cloud_backfill) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativePreserveGuestLocalData(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match switch_to_guest_local_data() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeReconcileLocalDataOwner(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(reconcile_local_data_owner) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeRestoreCloudDataSnapshot(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(restore_cloud_data_snapshot) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeRestoreCloudAiPassageSnapshot(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(restore_cloud_ai_passage_snapshot) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetAiProviderConfig(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_ai_provider_config() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSaveAiProviderConfig(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(save_ai_provider_config) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeToggleWordbook(
    mut env: JNIEnv,
    _class: JClass,
    wordbook_id: i32,
    is_active: bool,
) -> jstring {
    match toggle_wordbook(wordbook_id as i64, is_active) {
        Ok(()) => to_java_string(&mut env, ""),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSaveAiPassage(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(save_ai_passage) {
        Ok(()) => to_java_string(&mut env, ""),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetAiPassageHistory(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_ai_passage_history() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetAiPassage(
    mut env: JNIEnv,
    _class: JClass,
    passage_id: JString,
) -> jstring {
    match string_arg(&mut env, passage_id).and_then(get_ai_passage) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGetAiPassageStylePreference(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match get_ai_passage_style_preference() {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSaveAiPassageStylePreference(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(save_ai_passage_style_preference) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeGenerateAiPassage(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(generate_ai_passage) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeAnalyzeWrongWordImport(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(analyze_wrong_word_import) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeCommitWrongWordImport(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(commit_wrong_word_import) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeStartStudySession(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(start_study_session) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeSubmitStudyAnswer(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(submit_study_answer) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeMarkStudyEntryMastered(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(mark_study_entry_mastered) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeAcceptDisputedMeaning(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    match string_arg(&mut env, request_json).and_then(accept_disputed_meaning) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeCompleteStudySession(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
) -> jstring {
    match string_arg(&mut env, session_id).and_then(complete_study_session) {
        Ok(payload) => to_java_string(&mut env, &payload),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wordmobile_RustBridge_nativeCancelStudySession(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
) -> jstring {
    match string_arg(&mut env, session_id).and_then(cancel_study_session) {
        Ok(()) => to_java_string(&mut env, ""),
        Err(error) => to_java_string(&mut env, &format!("ERROR:{error}")),
    }
}
