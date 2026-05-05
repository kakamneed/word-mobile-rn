#ifndef WORD_PLATFORM_MOBILE_IOS_H
#define WORD_PLATFORM_MOBILE_IOS_H

#ifdef __cplusplus
extern "C" {
#endif

char *word_mobile_ios_initialize(
    const char *app_data_dir,
    const char *app_config_dir,
    const char *app_cache_dir,
    const char *bundle_resource_dir
);

char *word_mobile_ios_get_bridge_status(void);
char *word_mobile_ios_get_bootstrap_state(void);
char *word_mobile_ios_mark_onboarding_completed(void);
char *word_mobile_ios_get_today_home_state(void);
char *word_mobile_ios_get_today_reward_state(void);
char *word_mobile_ios_draw_today_reward(const char *request_json);
char *word_mobile_ios_get_settings(void);
char *word_mobile_ios_get_resume_session_hint(void);
char *word_mobile_ios_get_sync_status(void);
char *word_mobile_ios_record_sync_result(const char *request_json);
char *word_mobile_ios_record_cloud_restore_attempt(const char *request_json);
char *word_mobile_ios_enqueue_cloud_backfill(const char *request_json);
char *word_mobile_ios_preserve_guest_local_data(void);
char *word_mobile_ios_reconcile_local_data_owner(const char *request_json);
char *word_mobile_ios_restore_cloud_data_snapshot(const char *request_json);
char *word_mobile_ios_get_ai_provider_config(void);
char *word_mobile_ios_save_ai_provider_config(const char *request_json);
char *word_mobile_ios_build_today_home_state(const char *request_json);
char *word_mobile_ios_build_reports_overview(const char *request_json);
char *word_mobile_ios_build_wrong_words(const char *request_json);
char *word_mobile_ios_build_wrong_word_detail(const char *request_json);
char *word_mobile_ios_build_today_ai_passage_context(const char *request_json);
char *word_mobile_ios_get_active_plan(void);
char *word_mobile_ios_save_plan(const char *request_json);
char *word_mobile_ios_apply_saved_plan_to_today(void);
char *word_mobile_ios_get_wordbooks(void);
char *word_mobile_ios_get_reports_overview(void);
char *word_mobile_ios_get_wrong_words(const char *filter);
char *word_mobile_ios_get_wrong_word_detail(long long entry_id);
char *word_mobile_ios_save_word_hint(const char *request_json);
char *word_mobile_ios_get_word_hint_suggestions(long long entry_id);
char *word_mobile_ios_get_today_ai_passage_context(void);
char *word_mobile_ios_toggle_wordbook(long long wordbook_id, signed char is_active);
char *word_mobile_ios_start_study_session(const char *request_json);
char *word_mobile_ios_submit_study_answer(const char *request_json);
char *word_mobile_ios_complete_study_session(const char *session_id);
char *word_mobile_ios_cancel_study_session(const char *session_id);
char *word_mobile_ios_save_ai_passage(const char *request_json);
char *word_mobile_ios_get_ai_passage_history(void);
char *word_mobile_ios_get_ai_passage(const char *passage_id);
char *word_mobile_ios_generate_ai_passage(const char *request_json);
char *word_mobile_ios_analyze_wrong_word_import(const char *request_json);
char *word_mobile_ios_commit_wrong_word_import(const char *request_json);
void word_mobile_ios_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif
