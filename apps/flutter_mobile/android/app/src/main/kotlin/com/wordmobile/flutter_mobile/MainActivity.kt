package com.wordmobile

import android.content.ContentValues
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import android.provider.MediaStore
import android.util.Base64
import androidx.core.content.FileProvider
import com.wordmobile.RustBridge
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import org.json.JSONObject
import java.io.File
import java.util.concurrent.Executors

class MainActivity : FlutterActivity() {
    private val channelName = "com.wordmobile/rust_bridge"
    private val wrongWordImportPickRequest = 4207
    private val maxWrongWordImportBytes = 8 * 1024 * 1024
    private val bridgeExecutor = Executors.newSingleThreadExecutor()
    private val mainHandler = Handler(Looper.getMainLooper())
    private var pendingWrongWordImportResult: MethodChannel.Result? = null
    private var pendingWrongWordImportSourceType: String = "text"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName)
            .setMethodCallHandler { call, result ->
                try {
                    when (call.method) {
                        "initialize" -> {
                            bridgeExecutor.execute {
                                try {
                                    val initResult = RustBridge.initialize(applicationContext)
                                    mainHandler.post {
                                        if (initResult.isEmpty()) {
                                            result.success(null)
                                        } else {
                                            result.error("INIT_FAILED", initResult, null)
                                        }
                                    }
                                } catch (error: Throwable) {
                                    mainHandler.post {
                                        result.error("INIT_FAILED", error.message ?: "Unknown init error", null)
                                    }
                                }
                            }
                        }
                        "getBridgeStatus" -> {
                            runBridgeCall(result) {
                                RustBridge.getBridgeStatus().toString()
                            }
                        }
                        "getBootstrapState" -> {
                            runBridgeCall(result) {
                                RustBridge.getBootstrapState()
                            }
                        }
                        "markOnboardingCompleted" -> {
                            runBridgeVoid(result) {
                                RustBridge.markOnboardingCompleted()
                            }
                        }
                        "getTodayHomeState" -> {
                            runBridgeCall(result) {
                                RustBridge.getTodayHomeState()
                            }
                        }
                        "getTodayRewardState" -> {
                            runBridgeCall(result) {
                                RustBridge.getTodayRewardState()
                            }
                        }
                        "drawTodayReward" -> {
                            runBridgeCall(result) {
                                RustBridge.drawTodayReward(call.arguments as? String ?: "")
                            }
                        }
                        "getRewardImageUploadEntitlement" -> {
                            runBridgeCall(result) {
                                RustBridge.getRewardImageUploadEntitlement()
                            }
                        }
                        "refreshRewardImageUploadEntitlement" -> {
                            runBridgeCall(result) {
                                RustBridge.refreshRewardImageUploadEntitlement(call.arguments as? String ?: "")
                            }
                        }
                        "createRewardImageUpload" -> {
                            runBridgeCall(result) {
                                RustBridge.createRewardImageUpload(call.arguments as? String ?: "")
                            }
                        }
                        "listRewardImages" -> {
                            runBridgeCall(result) {
                                RustBridge.listRewardImages(call.arguments as? String ?: "")
                            }
                        }
                        "moderateRewardImage" -> {
                            runBridgeCall(result) {
                                RustBridge.moderateRewardImage(call.arguments as? String ?: "")
                            }
                        }
                        "selectLeaderboardRewardImageTag" -> {
                            runBridgeCall(result) {
                                RustBridge.selectLeaderboardRewardImageTag(call.arguments as? String ?: "")
                            }
                        }
                        "voteRewardImage" -> {
                            runBridgeCall(result) {
                                RustBridge.voteRewardImage(call.arguments as? String ?: "")
                            }
                        }
                        "getLocalLeaderboard" -> {
                            runBridgeCall(result) {
                                RustBridge.getLocalLeaderboard(call.arguments as? String ?: "")
                            }
                        }
                        "refreshLocalLeaderboardSummary" -> {
                            runBridgeCall(result) {
                                RustBridge.refreshLocalLeaderboardSummary(call.arguments as? String ?: "")
                            }
                        }
                        "seedLocalLeaderboardDemo" -> {
                            runBridgeCall(result) {
                                RustBridge.seedLocalLeaderboardDemo()
                            }
                        }
                        "saveImageToGallery" -> {
                            runBridgeCall(result) {
                                saveImageToGallery(call.arguments as? String ?: "")
                                """{"saved":true}"""
                            }
                        }
                        "pickWrongWordImportSource" -> {
                            pickWrongWordImportSource(call.arguments as? String ?: "", result)
                        }
                        "installApk" -> {
                            installApk(call.arguments as? String ?: "")
                            result.success(null)
                        }
                        "getActivePlan" -> {
                            runBridgeCall(result) {
                                RustBridge.getActivePlan()
                            }
                        }
                        "savePlan" -> {
                            runBridgeCall(result) {
                                RustBridge.savePlan(call.arguments as? String ?: "")
                            }
                        }
                        "applySavedPlanToToday" -> {
                            runBridgeCall(result) {
                                RustBridge.applySavedPlanToToday()
                            }
                        }
                        "getCrocBtiProfile" -> {
                            runBridgeCall(result) {
                                RustBridge.getCrocBtiProfile()
                            }
                        }
                        "saveCrocBtiProfile" -> {
                            runBridgeCall(result) {
                                RustBridge.saveCrocBtiProfile(call.arguments as? String ?: "")
                            }
                        }
                        "getWordbooks" -> {
                            runBridgeCall(result) {
                                RustBridge.getWordbooks()
                            }
                        }
                        "getReportsOverview" -> {
                            runBridgeCall(result) {
                                RustBridge.getReportsOverview()
                            }
                        }
                        "getResumeSessionHint" -> {
                            runBridgeCall(result) {
                                RustBridge.getResumeSessionHint()
                            }
                        }
                        "getSyncStatus" -> {
                            runBridgeCall(result) {
                                RustBridge.getSyncStatus()
                            }
                        }
                        "recordSyncResult" -> {
                            runBridgeCall(result) {
                                RustBridge.recordSyncResult(call.arguments as? String ?: "")
                            }
                        }
                        "recordCloudRestoreAttempt" -> {
                            runBridgeCall(result) {
                                RustBridge.recordCloudRestoreAttempt(call.arguments as? String ?: "")
                            }
                        }
                        "enqueueCloudBackfill" -> {
                            runBridgeCall(result) {
                                RustBridge.enqueueCloudBackfill(call.arguments as? String ?: "")
                            }
                        }
                        "preserveGuestLocalData" -> {
                            runBridgeCall(result) {
                                RustBridge.preserveGuestLocalData()
                            }
                        }
                        "reconcileLocalDataOwner" -> {
                            runBridgeCall(result) {
                                RustBridge.reconcileLocalDataOwner(call.arguments as? String ?: "")
                            }
                        }
                        "restoreCloudDataSnapshot" -> {
                            runBridgeCall(result) {
                                RustBridge.restoreCloudDataSnapshot(call.arguments as? String ?: "")
                            }
                        }
                        "restoreCloudAiPassageSnapshot" -> {
                            runBridgeCall(result) {
                                RustBridge.restoreCloudAiPassageSnapshot(call.arguments as? String ?: "")
                            }
                        }
                        "getWrongWords" -> {
                            runBridgeCall(result) {
                                RustBridge.getWrongWords(call.arguments as? String ?: "all")
                            }
                        }
                        "getWrongWordDetail" -> {
                            val entryId = (call.arguments as? String)?.toIntOrNull()
                            if (entryId == null) {
                                result.error("INVALID_ARGS", "getWrongWordDetail requires entry id", null)
                            } else {
                                runBridgeCall(result) {
                                    RustBridge.getWrongWordDetail(entryId)
                                }
                            }
                        }
                        "getWrongWordGraph" -> {
                            runBridgeCall(result) {
                                RustBridge.getWrongWordGraph()
                            }
                        }
                        "saveWrongWordGraphPosition" -> {
                            runBridgeCall(result) {
                                RustBridge.saveWrongWordGraphPosition(call.arguments as? String ?: "")
                            }
                        }
                        "saveWordHint" -> {
                            runBridgeCall(result) {
                                RustBridge.saveWordHint(call.arguments as? String ?: "")
                            }
                        }
                        "getWordHintSuggestions" -> {
                            val entryId = (call.arguments as? String)?.toIntOrNull()
                            if (entryId == null) {
                                result.error("INVALID_ARGS", "getWordHintSuggestions requires entry id", null)
                            } else {
                                runBridgeCall(result) {
                                    RustBridge.getWordHintSuggestions(entryId)
                                }
                            }
                        }
                        "getTodayAiPassageContext" -> {
                            runBridgeCall(result) {
                                RustBridge.getTodayAiPassageContext()
                            }
                        }
                        "getAiPassageHistory" -> {
                            runBridgeCall(result) {
                                RustBridge.getAiPassageHistory()
                            }
                        }
                        "getAiPassage" -> {
                            runBridgeCall(result) {
                                RustBridge.getAiPassage(call.arguments as? String ?: "")
                            }
                        }
                        "getAiPassageStylePreference" -> {
                            runBridgeCall(result) {
                                RustBridge.getAiPassageStylePreference()
                            }
                        }
                        "saveAiPassageStylePreference" -> {
                            runBridgeCall(result) {
                                RustBridge.saveAiPassageStylePreference(call.arguments as? String ?: "")
                            }
                        }
                        "generateAiPassage" -> {
                            runBridgeCall(result) {
                                RustBridge.generateAiPassage(call.arguments as? String ?: "")
                            }
                        }
                        "analyzeWrongWordImport" -> {
                            runBridgeCall(result) {
                                RustBridge.analyzeWrongWordImport(call.arguments as? String ?: "")
                            }
                        }
                        "commitWrongWordImport" -> {
                            runBridgeCall(result) {
                                RustBridge.commitWrongWordImport(call.arguments as? String ?: "")
                            }
                        }
                        "toggleWordbook" -> {
                            val raw = call.arguments as? String ?: ""
                            val args = JSONObject(raw)
                            val wordbookId = args.optInt("wordbookId", Int.MIN_VALUE)
                            val isActive = if (args.has("isActive")) args.optBoolean("isActive") else null
                            if (wordbookId == Int.MIN_VALUE || isActive == null) {
                                result.error("INVALID_ARGS", "toggleWordbook requires wordbookId and isActive", null)
                            } else {
                                runBridgeVoid(result) {
                                    RustBridge.toggleWordbook(wordbookId, isActive)
                                }
                            }
                        }
                        "startStudySession" -> {
                            runBridgeCall(result) {
                                RustBridge.startStudySession(call.arguments as? String ?: "")
                            }
                        }
                        "submitStudyAnswer" -> {
                            runBridgeCall(result) {
                                RustBridge.submitStudyAnswer(call.arguments as? String ?: "")
                            }
                        }
                        "markStudyEntryMastered" -> {
                            runBridgeCall(result) {
                                RustBridge.markStudyEntryMastered(call.arguments as? String ?: "")
                            }
                        }
                        "acceptDisputedMeaning" -> {
                            runBridgeCall(result) {
                                RustBridge.acceptDisputedMeaning(call.arguments as? String ?: "")
                            }
                        }
                        "completeStudySession" -> {
                            runBridgeCall(result) {
                                RustBridge.completeStudySession(call.arguments as? String ?: "")
                            }
                        }
                        "cancelStudySession" -> {
                            runBridgeVoid(result) {
                                RustBridge.cancelStudySession(call.arguments as? String ?: "")
                            }
                        }
                        else -> result.notImplemented()
                    }
                } catch (error: Throwable) {
                    result.error("RUST_BRIDGE_ERROR", error.message ?: "Unknown bridge error", null)
                }
            }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != wrongWordImportPickRequest) return
        val pendingResult = pendingWrongWordImportResult ?: return
        pendingWrongWordImportResult = null
        if (resultCode != RESULT_OK || data?.data == null) {
            pendingResult.success("""{"cancelled":true}""")
            return
        }
        val uri = data.data!!
        val sourceType = pendingWrongWordImportSourceType
        bridgeExecutor.execute {
            try {
                val payload = readWrongWordImportSource(uri, sourceType)
                mainHandler.post {
                    pendingResult.success(payload)
                }
            } catch (error: Throwable) {
                mainHandler.post {
                    pendingResult.error("IMPORT_PICK_FAILED", error.message ?: "Unable to read selected file", null)
                }
            }
        }
    }

    private fun runBridgeCall(result: MethodChannel.Result, block: () -> String) {
        bridgeExecutor.execute {
            try {
                val payload = block()
                mainHandler.post {
                    result.success(payload)
                }
            } catch (error: Throwable) {
                mainHandler.post {
                    result.error("RUST_BRIDGE_ERROR", error.message ?: "Unknown bridge error", null)
                }
            }
        }
    }

    private fun runBridgeVoid(result: MethodChannel.Result, block: () -> Unit) {
        bridgeExecutor.execute {
            try {
                block()
                mainHandler.post {
                    result.success(null)
                }
            } catch (error: Throwable) {
                mainHandler.post {
                    result.error("RUST_BRIDGE_ERROR", error.message ?: "Unknown bridge error", null)
                }
            }
        }
    }

    private fun pickWrongWordImportSource(raw: String, result: MethodChannel.Result) {
        if (pendingWrongWordImportResult != null) {
            result.error("IMPORT_PICK_ACTIVE", "Another import picker is already open", null)
            return
        }
        val args = JSONObject(raw.ifBlank { "{}" })
        val sourceType = args.optString("sourceType", "text").ifBlank { "text" }
        pendingWrongWordImportSourceType = sourceType
        pendingWrongWordImportResult = result
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = if (sourceType == "image") "image/*" else "text/*"
            if (sourceType != "image") {
                putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("text/*", "text/csv", "application/json"))
            }
        }
        try {
            startActivityForResult(intent, wrongWordImportPickRequest)
        } catch (error: Throwable) {
            pendingWrongWordImportResult = null
            throw error
        }
    }

    private fun readWrongWordImportSource(uri: Uri, sourceType: String): String {
        val resolver = applicationContext.contentResolver
        val mimeType = resolver.getType(uri) ?: if (sourceType == "image") "image/*" else "text/plain"
        val bytes = resolver.openInputStream(uri)?.use { stream -> stream.readBytes() }
            ?: error("Unable to open selected file")
        require(bytes.size <= maxWrongWordImportBytes) {
            "Selected file is too large. Please choose a file under 8 MB."
        }
        val response = JSONObject()
            .put("sourceType", sourceType)
            .put("sourceName", getDisplayName(uri).ifBlank { uri.lastPathSegment ?: "wrong-word-import" })
            .put("mimeType", mimeType)
        if (sourceType == "image") {
            response.put("bytesBase64", Base64.encodeToString(bytes, Base64.NO_WRAP))
        } else {
            response.put("textContent", bytes.toString(Charsets.UTF_8))
        }
        return response.toString()
    }

    private fun installApk(path: String) {
        require(path.isNotBlank()) { "APK path is required" }
        val apkFile = File(path)
        require(apkFile.exists()) { "APK file does not exist" }
        val apkUri = FileProvider.getUriForFile(
            this,
            "${applicationContext.packageName}.fileprovider",
            apkFile,
        )
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(apkUri, "application/vnd.android.package-archive")
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        startActivity(intent)
    }

    private fun getDisplayName(uri: Uri): String {
        var cursor: Cursor? = null
        return try {
            cursor = applicationContext.contentResolver.query(uri, null, null, null, null)
            if (cursor != null && cursor.moveToFirst()) {
                val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (index >= 0) cursor.getString(index) ?: "" else ""
            } else {
                ""
            }
        } finally {
            cursor?.close()
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
