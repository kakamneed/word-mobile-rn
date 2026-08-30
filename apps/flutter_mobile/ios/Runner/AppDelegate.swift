import Flutter
import Photos
import UIKit
import UniformTypeIdentifiers

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate, UIDocumentPickerDelegate {
  private let channelName = "com.wordmobile/rust_bridge"
  private let errorPrefix = "__WORDMOBILE_ERROR__:"
  private let maxWrongWordImportBytes = 8 * 1024 * 1024
  private var pendingWrongWordImportResult: FlutterResult?
  private var pendingWrongWordImportSourceType = "text"

  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    let launched = super.application(application, didFinishLaunchingWithOptions: launchOptions)

    if let controller = window?.rootViewController as? FlutterViewController {
      let channel = FlutterMethodChannel(name: channelName, binaryMessenger: controller.binaryMessenger)
      channel.setMethodCallHandler { [weak self] call, result in
        self?.handle(call: call, result: result)
      }
    }

    return launched
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
  }

  private func handle(call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "initialize":
      handleInitialize(result: result)
    case "getBridgeStatus":
      handleString(result: result) { word_mobile_ios_get_bridge_status() }
    case "getBootstrapState":
      handleString(result: result) { word_mobile_ios_get_bootstrap_state() }
    case "markOnboardingCompleted":
      handleVoid(result: result) { word_mobile_ios_mark_onboarding_completed() }
    case "getTodayHomeState":
      handleString(result: result) { word_mobile_ios_get_today_home_state() }
    case "getTodayRewardState":
      handleString(result: result) { word_mobile_ios_get_today_reward_state() }
    case "drawTodayReward":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "drawTodayReward requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_draw_today_reward(pointer) }
      }
    case "getRewardImageUploadEntitlement":
      handleString(result: result) { word_mobile_ios_get_reward_image_upload_entitlement() }
    case "refreshRewardImageUploadEntitlement":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "refreshRewardImageUploadEntitlement requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_refresh_reward_image_upload_entitlement(pointer) }
      }
    case "createRewardImageUpload":
      handleJsonStringCall(call: call, result: result, name: "createRewardImageUpload") { pointer in
        word_mobile_ios_create_reward_image_upload(pointer)
      }
    case "listRewardImages":
      handleJsonStringCall(call: call, result: result, name: "listRewardImages") { pointer in
        word_mobile_ios_list_reward_images(pointer)
      }
    case "moderateRewardImage":
      handleJsonStringCall(call: call, result: result, name: "moderateRewardImage") { pointer in
        word_mobile_ios_moderate_reward_image(pointer)
      }
    case "selectLeaderboardRewardImageTag":
      handleJsonStringCall(call: call, result: result, name: "selectLeaderboardRewardImageTag") { pointer in
        word_mobile_ios_select_leaderboard_reward_image_tag(pointer)
      }
    case "voteRewardImage":
      handleJsonStringCall(call: call, result: result, name: "voteRewardImage") { pointer in
        word_mobile_ios_vote_reward_image(pointer)
      }
    case "getLocalLeaderboard":
      handleJsonStringCall(call: call, result: result, name: "getLocalLeaderboard") { pointer in
        word_mobile_ios_get_local_leaderboard(pointer)
      }
    case "refreshLocalLeaderboardSummary":
      handleJsonStringCall(call: call, result: result, name: "refreshLocalLeaderboardSummary") { pointer in
        word_mobile_ios_refresh_local_leaderboard_summary(pointer)
      }
    case "seedLocalLeaderboardDemo":
      handleString(result: result) { word_mobile_ios_seed_local_leaderboard_demo() }
    case "saveImageToGallery":
      handleSaveImageToGallery(call: call, result: result)
    case "pickWrongWordImportSource":
      handlePickWrongWordImportSource(call: call, result: result)
    case "getActivePlan":
      handleString(result: result) { word_mobile_ios_get_active_plan() }
    case "savePlan":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "savePlan requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_save_plan(pointer) }
      }
    case "applySavedPlanToToday":
      handleString(result: result) { word_mobile_ios_apply_saved_plan_to_today() }
    case "getWordbooks":
      handleString(result: result) { word_mobile_ios_get_wordbooks() }
    case "getReportsOverview":
      handleString(result: result) { word_mobile_ios_get_reports_overview() }
    case "getResumeSessionHint":
      handleString(result: result) { word_mobile_ios_get_resume_session_hint() }
    case "getSyncStatus":
      handleString(result: result) { word_mobile_ios_get_sync_status() }
    case "recordSyncResult":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "recordSyncResult requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_record_sync_result(pointer) }
      }
    case "recordCloudRestoreAttempt":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "recordCloudRestoreAttempt requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_record_cloud_restore_attempt(pointer) }
      }
    case "enqueueCloudBackfill":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "enqueueCloudBackfill requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_enqueue_cloud_backfill(pointer) }
      }
    case "preserveGuestLocalData":
      handleString(result: result) { word_mobile_ios_preserve_guest_local_data() }
    case "reconcileLocalDataOwner":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "reconcileLocalDataOwner requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_reconcile_local_data_owner(pointer) }
      }
    case "restoreCloudDataSnapshot":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "restoreCloudDataSnapshot requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_restore_cloud_data_snapshot(pointer) }
      }
    case "getWrongWords":
      let filter = (call.arguments as? String) ?? "all"
      filter.withCString { pointer in
        handleString(result: result) { word_mobile_ios_get_wrong_words(pointer) }
      }
    case "getWrongWordDetail":
      guard let entryId = Int64((call.arguments as? String) ?? "") else {
        result(FlutterError(code: "INVALID_ARGS", message: "getWrongWordDetail requires entry id", details: nil))
        return
      }
      handleString(result: result) { word_mobile_ios_get_wrong_word_detail(entryId) }
    case "getWrongWordGraph":
      handleString(result: result) { word_mobile_ios_get_wrong_word_graph() }
    case "getExamCatalog":
      handleString(result: result) { word_mobile_ios_get_exam_catalog() }
    case "getExamPaper":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "getExamPaper requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_get_exam_paper(pointer) }
      }
    case "analyzeExamPaperImport":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "analyzeExamPaperImport requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_analyze_exam_paper_import(pointer) }
      }
    case "saveUserExamPaper":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "saveUserExamPaper requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_save_user_exam_paper(pointer) }
      }
    case "getExamVocabularyPriority":
      handleString(result: result) { word_mobile_ios_get_exam_vocabulary_priority() }
    case "getExamPracticeReport":
      handleString(result: result) { word_mobile_ios_get_exam_practice_report() }
    case "analyzeExamQuestionVocabulary":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "analyzeExamQuestionVocabulary requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_analyze_exam_question_vocabulary(pointer) }
      }
    case "analyzeExamSectionVocabulary":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "analyzeExamSectionVocabulary requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_analyze_exam_section_vocabulary(pointer) }
      }
    case "saveExamAnalysisTask":
      handleJsonStringCall(call: call, result: result, name: "saveExamAnalysisTask", action: word_mobile_ios_save_exam_analysis_task)
    case "getExamAnalysisTasks":
      handleString(result: result) { word_mobile_ios_get_exam_analysis_tasks() }
    case "markExamAnalysisTasksRead":
      handleString(result: result) { word_mobile_ios_mark_exam_analysis_tasks_read() }
    case "saveExamAttempt":
      handleJsonStringCall(call: call, result: result, name: "saveExamAttempt", action: word_mobile_ios_save_exam_attempt)
    case "getExamAttempt":
      handleJsonStringCall(call: call, result: result, name: "getExamAttempt", action: word_mobile_ios_get_exam_attempt)
    case "tokenizeExamText":
      handleJsonStringCall(call: call, result: result, name: "tokenizeExamText", action: word_mobile_ios_tokenize_exam_text)
    case "inspectExamWord":
      handleJsonStringCall(call: call, result: result, name: "inspectExamWord", action: word_mobile_ios_inspect_exam_word)
    case "getExamAnnotationState":
      handleJsonStringCall(call: call, result: result, name: "getExamAnnotationState", action: word_mobile_ios_get_exam_annotation_state)
    case "saveExamAnnotation":
      handleJsonStringCall(call: call, result: result, name: "saveExamAnnotation", action: word_mobile_ios_save_exam_annotation)
    case "saveWrongWordGraphPosition":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "saveWrongWordGraphPosition requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_save_wrong_word_graph_position(pointer) }
      }
    case "saveWordHint":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "saveWordHint requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_save_word_hint(pointer) }
      }
    case "getWordHintSuggestions":
      guard let entryId = Int64((call.arguments as? String) ?? "") else {
        result(FlutterError(code: "INVALID_ARGS", message: "getWordHintSuggestions requires entry id", details: nil))
        return
      }
      handleString(result: result) { word_mobile_ios_get_word_hint_suggestions(entryId) }
    case "getTodayAiPassageContext":
      handleString(result: result) { word_mobile_ios_get_today_ai_passage_context() }
    case "getAiPassageHistory":
      handleString(result: result) { word_mobile_ios_get_ai_passage_history() }
    case "getAiPassage":
      guard let passageId = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "getAiPassage requires passage id", details: nil))
        return
      }
      passageId.withCString { pointer in
        handleString(result: result) { word_mobile_ios_get_ai_passage(pointer) }
      }
    case "getAiPassageStylePreference":
      handleString(result: result) { word_mobile_ios_get_ai_passage_style_preference() }
    case "saveAiPassageStylePreference":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "saveAiPassageStylePreference requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_save_ai_passage_style_preference(pointer) }
      }
    case "generateAiPassage":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "generateAiPassage requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_generate_ai_passage(pointer) }
      }
    case "analyzeWrongWordImport":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "analyzeWrongWordImport requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_analyze_wrong_word_import(pointer) }
      }
    case "commitWrongWordImport":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "commitWrongWordImport requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_commit_wrong_word_import(pointer) }
      }
    case "toggleWordbook":
      guard
        let raw = call.arguments as? String,
        let data = raw.data(using: .utf8),
        let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
        let wordbookId = object["wordbookId"] as? NSNumber,
        let isActive = object["isActive"] as? Bool
      else {
        result(FlutterError(code: "INVALID_ARGS", message: "toggleWordbook requires JSON payload", details: nil))
        return
      }
      handleVoid(result: result) {
        word_mobile_ios_toggle_wordbook(wordbookId.int64Value, isActive ? 1 : 0)
      }
    case "startStudySession":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "startStudySession requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_start_study_session(pointer) }
      }
    case "submitStudyAnswer":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "submitStudyAnswer requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_submit_study_answer(pointer) }
      }
    case "markStudyEntryMastered":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "markStudyEntryMastered requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_mark_study_entry_mastered(pointer) }
      }
    case "acceptDisputedMeaning":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "acceptDisputedMeaning requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_accept_disputed_meaning(pointer) }
      }
    case "completeStudySession":
      guard let sessionId = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "completeStudySession requires session id", details: nil))
        return
      }
      sessionId.withCString { pointer in
        handleString(result: result) { word_mobile_ios_complete_study_session(pointer) }
      }
    case "cancelStudySession":
      guard let sessionId = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "cancelStudySession requires session id", details: nil))
        return
      }
      sessionId.withCString { pointer in
        handleVoid(result: result) { word_mobile_ios_cancel_study_session(pointer) }
      }
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  private func handleInitialize(result: @escaping FlutterResult) {
    guard
      let filesDir = NSSearchPathForDirectoriesInDomains(.documentDirectory, .userDomainMask, true).first,
      let appSupportDir = NSSearchPathForDirectoriesInDomains(.applicationSupportDirectory, .userDomainMask, true).first,
      let cacheDir = NSSearchPathForDirectoriesInDomains(.cachesDirectory, .userDomainMask, true).first,
      let bundleDir = Bundle.main.resourcePath
    else {
      result(FlutterError(code: "INIT_FAILED", message: "Unable to resolve iOS runtime directories", details: nil))
      return
    }

    filesDir.withCString { filesPtr in
      appSupportDir.withCString { supportPtr in
        cacheDir.withCString { cachePtr in
          bundleDir.withCString { bundlePtr in
            handleVoid(result: result) {
              word_mobile_ios_initialize(filesPtr, supportPtr, cachePtr, bundlePtr)
            }
          }
        }
      }
    }
  }

  private func handleSaveImageToGallery(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard
      let raw = call.arguments as? String,
      let data = raw.data(using: .utf8),
      let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
      let bytesBase64 = object["bytesBase64"] as? String,
      let imageData = Data(base64Encoded: bytesBase64),
      let image = UIImage(data: imageData)
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "saveImageToGallery requires image bytes", details: nil))
      return
    }

    PHPhotoLibrary.shared().performChanges({
      PHAssetChangeRequest.creationRequestForAsset(from: image)
    }) { success, error in
      DispatchQueue.main.async {
        if success {
          result("{\"saved\":true}")
        } else {
          result(
            FlutterError(
              code: "SAVE_FAILED",
              message: error?.localizedDescription ?? "Unable to save image",
              details: nil
            )
          )
        }
      }
    }
  }

  private func handlePickWrongWordImportSource(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard pendingWrongWordImportResult == nil else {
      result(FlutterError(code: "IMPORT_PICK_ACTIVE", message: "Another import picker is already open", details: nil))
      return
    }
    let raw = (call.arguments as? String) ?? "{}"
    let object = raw.data(using: .utf8).flatMap {
      try? JSONSerialization.jsonObject(with: $0) as? [String: Any]
    }
    let sourceType = (object?["sourceType"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? "text"
    pendingWrongWordImportSourceType = sourceType
    pendingWrongWordImportResult = result
    let types: [UTType] = sourceType == "image"
      ? [.image]
      : [.plainText, .text, .commaSeparatedText, .json]
    let picker = UIDocumentPickerViewController(forOpeningContentTypes: types, asCopy: true)
    picker.delegate = self
    picker.allowsMultipleSelection = false
    window?.rootViewController?.present(picker, animated: true)
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    pendingWrongWordImportResult?("{\"cancelled\":true}")
    pendingWrongWordImportResult = nil
  }

  func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
    guard let result = pendingWrongWordImportResult else { return }
    pendingWrongWordImportResult = nil
    guard let url = urls.first else {
      result("{\"cancelled\":true}")
      return
    }
    do {
      let data = try Data(contentsOf: url)
      guard data.count <= maxWrongWordImportBytes else {
        result(
          FlutterError(
            code: "IMPORT_FILE_TOO_LARGE",
            message: "Selected file is too large. Please choose a file under 8 MB.",
            details: nil
          )
        )
        return
      }
      var response: [String: Any] = [
        "sourceType": pendingWrongWordImportSourceType,
        "sourceName": url.lastPathComponent,
        "mimeType": mimeType(for: url, sourceType: pendingWrongWordImportSourceType),
      ]
      if pendingWrongWordImportSourceType == "image" {
        response["bytesBase64"] = data.base64EncodedString()
      } else {
        response["textContent"] = String(data: data, encoding: .utf8) ?? ""
      }
      let responseData = try JSONSerialization.data(withJSONObject: response)
      result(String(data: responseData, encoding: .utf8) ?? "{\"cancelled\":true}")
    } catch {
      result(FlutterError(code: "IMPORT_PICK_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  private func mimeType(for url: URL, sourceType: String) -> String {
    if let type = UTType(filenameExtension: url.pathExtension),
       let mimeType = type.preferredMIMEType {
      return mimeType
    }
    return sourceType == "image" ? "image/*" : "text/plain"
  }

  private func handleString(
    result: @escaping FlutterResult,
    call: () -> UnsafeMutablePointer<CChar>?
  ) {
    guard let decoded = decodeRustString(call()) else {
      result(FlutterError(code: "RUST_BRIDGE_ERROR", message: "Rust bridge returned no value", details: nil))
      return
    }
    if decoded.hasPrefix(errorPrefix) {
      result(
        FlutterError(
          code: "RUST_BRIDGE_ERROR",
          message: String(decoded.dropFirst(errorPrefix.count)),
          details: nil
        )
      )
      return
    }
    result(decoded)
  }

  private func handleJsonStringCall(
    call: FlutterMethodCall,
    result: @escaping FlutterResult,
    name: String,
    action: (UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
  ) {
    guard let request = call.arguments as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "\(name) requires JSON string", details: nil))
      return
    }
    request.withCString { pointer in
      handleString(result: result) { action(pointer) }
    }
  }

  private func handleVoid(
    result: @escaping FlutterResult,
    call: () -> UnsafeMutablePointer<CChar>?
  ) {
    let decoded = decodeRustString(call()) ?? ""
    if decoded.hasPrefix(errorPrefix) {
      result(
        FlutterError(
          code: "RUST_BRIDGE_ERROR",
          message: String(decoded.dropFirst(errorPrefix.count)),
          details: nil
        )
      )
      return
    }
    if !decoded.isEmpty {
      result(FlutterError(code: "RUST_BRIDGE_ERROR", message: decoded, details: nil))
      return
    }
    result(nil)
  }

  private func decodeRustString(_ pointer: UnsafeMutablePointer<CChar>?) -> String? {
    guard let pointer else { return nil }
    let value = String(cString: pointer)
    word_mobile_ios_string_free(pointer)
    return value
  }
}
