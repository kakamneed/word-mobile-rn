import Flutter
import Photos
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  private let channelName = "com.wordmobile/rust_bridge"
  private let errorPrefix = "__WORDMOBILE_ERROR__:"

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
    case "saveImageToGallery":
      handleSaveImageToGallery(call: call, result: result)
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
    case "generateAiPassage":
      guard let request = call.arguments as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "generateAiPassage requires JSON string", details: nil))
        return
      }
      request.withCString { pointer in
        handleString(result: result) { word_mobile_ios_generate_ai_passage(pointer) }
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
