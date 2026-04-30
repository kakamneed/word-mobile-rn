use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;

use crate::bridge::{
    apply_saved_plan_to_today, build_reports_overview, build_today_ai_passage_context,
    build_today_home_state, build_wrong_word_detail, build_wrong_words, cancel_study_session,
    complete_study_session, draw_today_reward, generate_ai_passage, get_active_plan,
    get_ai_passage, get_ai_passage_history, get_ai_provider_config, get_bootstrap_state,
    get_bridge_status, get_reports_overview, get_resume_session_hint, get_settings,
    get_sync_status, get_today_ai_passage_context, get_today_home_state, get_today_reward_state,
    get_wordbooks, get_wrong_word_detail, get_wrong_words, initialize_mobile_runtime,
    mark_onboarding_completed, save_ai_passage, save_ai_provider_config, save_plan,
    start_study_session, submit_study_answer, toggle_wordbook,
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
