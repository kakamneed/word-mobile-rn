use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::bridge::{
    analyze_exam_paper_import, analyze_exam_question_vocabulary, analyze_wrong_word_import,
    apply_saved_plan_to_today, build_reports_overview, build_today_ai_passage_context,
    build_today_home_state, build_wrong_word_detail, build_wrong_words, cancel_study_session,
    commit_wrong_word_import, complete_study_session, create_reward_image_upload,
    draw_today_reward, enqueue_cloud_backfill, generate_ai_passage, get_active_plan,
    get_ai_passage, get_ai_passage_history, get_ai_passage_style_preference,
    get_ai_provider_config, get_bootstrap_state, get_bridge_status, get_exam_annotation_state,
    get_local_leaderboard, get_reports_overview, get_reward_image_upload_entitlement, get_settings,
    get_sync_status, get_today_ai_passage_context, get_today_home_state, get_today_reward_state,
    get_word_hint_suggestions, get_wordbooks, get_wrong_word_detail, get_wrong_word_graph,
    get_wrong_words, initialize_mobile_runtime, list_reward_images, mark_onboarding_completed,
    moderate_reward_image, reconcile_local_data_owner, record_cloud_restore_attempt,
    record_sync_result, refresh_local_leaderboard_summary, refresh_reward_image_upload_entitlement,
    restore_cloud_data_snapshot, save_ai_passage, save_ai_passage_style_preference,
    save_ai_provider_config, save_exam_annotation, save_plan, save_word_hint,
    save_wrong_word_graph_position, seed_local_leaderboard_demo,
    select_leaderboard_reward_image_tag, start_study_session, submit_study_answer,
    switch_to_guest_local_data, toggle_wordbook, vote_reward_image,
};

const IOS_ERROR_PREFIX: &str = "__WORDMOBILE_ERROR__:";

fn sanitize_cstring_value(value: String) -> CString {
    let sanitized = value.replace('\0', " ");
    CString::new(sanitized).expect("CString sanitization must remove interior NUL bytes")
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_analyze_exam_paper_import(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(analyze_exam_paper_import);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_user_exam_paper(request_json: *const c_char) -> *mut c_char {
    let result =
        decode_arg("request_json", request_json).and_then(crate::bridge::save_user_exam_paper);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_exam_vocabulary_priority() -> *mut c_char {
    encode_string_result(crate::bridge::get_exam_vocabulary_priority())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_exam_practice_report() -> *mut c_char {
    encode_string_result(crate::bridge::get_exam_practice_report())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_analyze_exam_question_vocabulary(
    request_json: *const c_char,
) -> *mut c_char {
    let result =
        decode_arg("request_json", request_json).and_then(analyze_exam_question_vocabulary);
    encode_string_result(result)
}

fn into_owned_c_string(value: String) -> *mut c_char {
    sanitize_cstring_value(value).into_raw()
}

fn decode_arg(name: &str, value: *const c_char) -> Result<String, String> {
    if value.is_null() {
        return Err(format!("{name} was null"));
    }

    unsafe {
        CStr::from_ptr(value)
            .to_str()
            .map(|text| text.to_owned())
            .map_err(|error| format!("{name} was not valid UTF-8: {error}"))
    }
}

fn encode_string_result(result: Result<String, String>) -> *mut c_char {
    match result {
        Ok(payload) => into_owned_c_string(payload),
        Err(error) => into_owned_c_string(format!("{IOS_ERROR_PREFIX}{error}")),
    }
}

fn encode_void_result(result: Result<(), String>) -> *mut c_char {
    encode_string_result(result.map(|_| String::new()))
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    unsafe {
        let _ = CString::from_raw(value);
    }
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_initialize(
    app_data_dir: *const c_char,
    app_config_dir: *const c_char,
    app_cache_dir: *const c_char,
    bundle_resource_dir: *const c_char,
) -> *mut c_char {
    let result = (|| {
        initialize_mobile_runtime(
            decode_arg("app_data_dir", app_data_dir)?,
            decode_arg("app_config_dir", app_config_dir)?,
            decode_arg("app_cache_dir", app_cache_dir)?,
            decode_arg("bundle_resource_dir", bundle_resource_dir)?,
        )?;
        Ok(String::new())
    })();

    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_bridge_status() -> *mut c_char {
    encode_string_result(get_bridge_status())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_bootstrap_state() -> *mut c_char {
    encode_string_result(get_bootstrap_state())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_mark_onboarding_completed() -> *mut c_char {
    encode_void_result(mark_onboarding_completed())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_today_home_state() -> *mut c_char {
    encode_string_result(get_today_home_state())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_today_reward_state() -> *mut c_char {
    encode_string_result(get_today_reward_state())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_draw_today_reward(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(draw_today_reward);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_reward_image_upload_entitlement() -> *mut c_char {
    encode_string_result(get_reward_image_upload_entitlement())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_refresh_reward_image_upload_entitlement(
    request_json: *const c_char,
) -> *mut c_char {
    let result =
        decode_arg("request_json", request_json).and_then(refresh_reward_image_upload_entitlement);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_create_reward_image_upload(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(create_reward_image_upload);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_list_reward_images(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(list_reward_images);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_moderate_reward_image(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(moderate_reward_image);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_select_leaderboard_reward_image_tag(
    request_json: *const c_char,
) -> *mut c_char {
    let result =
        decode_arg("request_json", request_json).and_then(select_leaderboard_reward_image_tag);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_vote_reward_image(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(vote_reward_image);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_local_leaderboard(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(get_local_leaderboard);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_refresh_local_leaderboard_summary(
    request_json: *const c_char,
) -> *mut c_char {
    let result =
        decode_arg("request_json", request_json).and_then(refresh_local_leaderboard_summary);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_seed_local_leaderboard_demo() -> *mut c_char {
    encode_string_result(seed_local_leaderboard_demo())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_resume_session_hint() -> *mut c_char {
    encode_string_result(get_resume_session_hint())
}
#[no_mangle]
pub extern "C" fn word_mobile_ios_get_settings() -> *mut c_char {
    encode_string_result(get_settings())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_sync_status() -> *mut c_char {
    encode_string_result(get_sync_status())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_record_sync_result(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(record_sync_result);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_record_cloud_restore_attempt(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(record_cloud_restore_attempt);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_enqueue_cloud_backfill(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(enqueue_cloud_backfill);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_preserve_guest_local_data() -> *mut c_char {
    encode_string_result(switch_to_guest_local_data())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_reconcile_local_data_owner(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(reconcile_local_data_owner);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_restore_cloud_data_snapshot(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(restore_cloud_data_snapshot);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_ai_provider_config() -> *mut c_char {
    encode_string_result(get_ai_provider_config())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_ai_provider_config(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(save_ai_provider_config);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_build_today_home_state(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(build_today_home_state);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_build_reports_overview(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(build_reports_overview);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_build_wrong_words(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(build_wrong_words);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_build_wrong_word_detail(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(build_wrong_word_detail);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_build_today_ai_passage_context(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(build_today_ai_passage_context);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_active_plan() -> *mut c_char {
    encode_string_result(get_active_plan())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_plan(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(save_plan);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_apply_saved_plan_to_today() -> *mut c_char {
    encode_string_result(apply_saved_plan_to_today())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_wordbooks() -> *mut c_char {
    encode_string_result(get_wordbooks())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_reports_overview() -> *mut c_char {
    encode_string_result(get_reports_overview())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_wrong_words(filter: *const c_char) -> *mut c_char {
    let result = decode_arg("filter", filter).and_then(get_wrong_words);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_wrong_word_graph() -> *mut c_char {
    encode_string_result(get_wrong_word_graph())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_exam_catalog() -> *mut c_char {
    encode_string_result(get_exam_catalog())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_exam_paper(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(get_exam_paper);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_exam_attempt(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(save_exam_attempt);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_exam_attempt(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(get_exam_attempt);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_tokenize_exam_text(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(tokenize_exam_text);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_inspect_exam_word(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(inspect_exam_word);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_exam_annotation_state(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(get_exam_annotation_state);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_exam_annotation(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(save_exam_annotation);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_wrong_word_graph_position(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(save_wrong_word_graph_position);
    encode_string_result(result)
}
#[no_mangle]
pub extern "C" fn word_mobile_ios_get_wrong_word_detail(entry_id: i64) -> *mut c_char {
    encode_string_result(get_wrong_word_detail(entry_id))
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_word_hint(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(save_word_hint);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_word_hint_suggestions(entry_id: i64) -> *mut c_char {
    encode_string_result(get_word_hint_suggestions(entry_id))
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_today_ai_passage_context() -> *mut c_char {
    encode_string_result(get_today_ai_passage_context())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_toggle_wordbook(wordbook_id: i64, is_active: i8) -> *mut c_char {
    encode_void_result(toggle_wordbook(wordbook_id, is_active != 0))
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_start_study_session(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(start_study_session);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_submit_study_answer(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(submit_study_answer);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_mark_study_entry_mastered(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(mark_study_entry_mastered);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_accept_disputed_meaning(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(accept_disputed_meaning);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_complete_study_session(session_id: *const c_char) -> *mut c_char {
    let result = decode_arg("session_id", session_id).and_then(complete_study_session);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_cancel_study_session(session_id: *const c_char) -> *mut c_char {
    let result = decode_arg("session_id", session_id).and_then(cancel_study_session);
    encode_void_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_ai_passage(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(save_ai_passage);
    encode_void_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_ai_passage_history() -> *mut c_char {
    encode_string_result(get_ai_passage_history())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_ai_passage(passage_id: *const c_char) -> *mut c_char {
    let result = decode_arg("passage_id", passage_id).and_then(get_ai_passage);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_get_ai_passage_style_preference() -> *mut c_char {
    encode_string_result(get_ai_passage_style_preference())
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_save_ai_passage_style_preference(
    request_json: *const c_char,
) -> *mut c_char {
    let result =
        decode_arg("request_json", request_json).and_then(save_ai_passage_style_preference);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_generate_ai_passage(request_json: *const c_char) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(generate_ai_passage);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_analyze_wrong_word_import(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(analyze_wrong_word_import);
    encode_string_result(result)
}

#[no_mangle]
pub extern "C" fn word_mobile_ios_commit_wrong_word_import(
    request_json: *const c_char,
) -> *mut c_char {
    let result = decode_arg("request_json", request_json).and_then(commit_wrong_word_import);
    encode_string_result(result)
}
