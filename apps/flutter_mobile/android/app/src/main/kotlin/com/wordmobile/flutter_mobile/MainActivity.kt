package com.wordmobile

import android.content.ContentValues
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.util.Base64
import com.wordmobile.RustBridge
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import org.json.JSONObject

class MainActivity : FlutterActivity() {
    private val channelName = "com.wordmobile/rust_bridge"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName)
            .setMethodCallHandler { call, result ->
                try {
                    when (call.method) {
                        "initialize" -> {
                            val initResult = RustBridge.initialize(applicationContext)
                            if (initResult.isEmpty()) {
                                result.success(null)
                            } else {
                                result.error("INIT_FAILED", initResult, null)
                            }
                        }
                        "getBridgeStatus" -> {
                            result.success(RustBridge.getBridgeStatus().toString())
                        }
                        "getBootstrapState" -> {
                            result.success(RustBridge.getBootstrapState())
                        }
                        "markOnboardingCompleted" -> {
                            RustBridge.markOnboardingCompleted()
                            result.success(null)
                        }
                        "getTodayHomeState" -> {
                            result.success(RustBridge.getTodayHomeState())
                        }
                        "getTodayRewardState" -> {
                            result.success(RustBridge.getTodayRewardState())
                        }
                        "drawTodayReward" -> {
                            result.success(RustBridge.drawTodayReward(call.arguments as? String ?: ""))
                        }
                        "saveImageToGallery" -> {
                            saveImageToGallery(call.arguments as? String ?: "")
                            result.success("""{"saved":true}""")
                        }
                        "getActivePlan" -> {
                            result.success(RustBridge.getActivePlan())
                        }
                        "savePlan" -> {
                            result.success(RustBridge.savePlan(call.arguments as? String ?: ""))
                        }
                        "applySavedPlanToToday" -> {
                            result.success(RustBridge.applySavedPlanToToday())
                        }
                        "getWordbooks" -> {
                            result.success(RustBridge.getWordbooks())
                        }
                        "getReportsOverview" -> {
                            result.success(RustBridge.getReportsOverview())
                        }
                        "getResumeSessionHint" -> {
                            result.success(RustBridge.getResumeSessionHint())
                        }
                        "getSyncStatus" -> {
                            result.success(RustBridge.getSyncStatus())
                        }
                        "getWrongWords" -> {
                            result.success(RustBridge.getWrongWords(call.arguments as? String ?: "all"))
                        }
                        "getWrongWordDetail" -> {
                            val entryId = (call.arguments as? String)?.toIntOrNull()
                            if (entryId == null) {
                                result.error("INVALID_ARGS", "getWrongWordDetail requires entry id", null)
                            } else {
                                result.success(RustBridge.getWrongWordDetail(entryId))
                            }
                        }
                        "getTodayAiPassageContext" -> {
                            result.success(RustBridge.getTodayAiPassageContext())
                        }
                        "getAiPassageHistory" -> {
                            result.success(RustBridge.getAiPassageHistory())
                        }
                        "getAiPassage" -> {
                            result.success(RustBridge.getAiPassage(call.arguments as? String ?: ""))
                        }
                        "generateAiPassage" -> {
                            result.success(RustBridge.generateAiPassage(call.arguments as? String ?: ""))
                        }
                        "toggleWordbook" -> {
                            val raw = call.arguments as? String ?: ""
                            val args = JSONObject(raw)
                            val wordbookId = args.optInt("wordbookId", Int.MIN_VALUE)
                            val isActive = if (args.has("isActive")) args.optBoolean("isActive") else null
                            if (wordbookId == Int.MIN_VALUE || isActive == null) {
                                result.error("INVALID_ARGS", "toggleWordbook requires wordbookId and isActive", null)
                            } else {
                                RustBridge.toggleWordbook(wordbookId, isActive)
                                result.success(null)
                            }
                        }
                        "startStudySession" -> {
                            result.success(RustBridge.startStudySession(call.arguments as? String ?: ""))
                        }
                        "submitStudyAnswer" -> {
                            result.success(RustBridge.submitStudyAnswer(call.arguments as? String ?: ""))
                        }
                        "completeStudySession" -> {
                            result.success(RustBridge.completeStudySession(call.arguments as? String ?: ""))
                        }
                        "cancelStudySession" -> {
                            RustBridge.cancelStudySession(call.arguments as? String ?: "")
                            result.success(null)
                        }
                        else -> result.notImplemented()
                    }
                } catch (error: Throwable) {
                    result.error("RUST_BRIDGE_ERROR", error.message ?: "Unknown bridge error", null)
                }
            }
    }

    private fun saveImageToGallery(raw: String) {
        val args = JSONObject(raw)
        val fileName = args.optString("fileName").ifBlank { "word_mobile_reward.webp" }
        val mimeType = args.optString("mimeType").ifBlank { "image/webp" }
        val bytesBase64 = args.optString("bytesBase64")
        require(bytesBase64.isNotBlank()) { "bytesBase64 is required" }

        val bytes = Base64.decode(bytesBase64, Base64.DEFAULT)
        val resolver = applicationContext.contentResolver
        val values = ContentValues().apply {
            put(MediaStore.Images.Media.DISPLAY_NAME, fileName)
            put(MediaStore.Images.Media.MIME_TYPE, mimeType)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                put(
                    MediaStore.Images.Media.RELATIVE_PATH,
                    "${Environment.DIRECTORY_PICTURES}/WordMobile",
                )
                put(MediaStore.Images.Media.IS_PENDING, 1)
            }
        }
        val uri = resolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values)
            ?: error("Unable to create gallery image")
        try {
            resolver.openOutputStream(uri)?.use { stream ->
                stream.write(bytes)
            } ?: error("Unable to open gallery image")
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                values.clear()
                values.put(MediaStore.Images.Media.IS_PENDING, 0)
                resolver.update(uri, values, null, null)
            }
        } catch (error: Throwable) {
            resolver.delete(uri, null, null)
            throw error
        }
    }
}
