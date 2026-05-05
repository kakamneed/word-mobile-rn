import java.io.ByteArrayOutputStream
import java.util.Properties

plugins {
    id("com.android.application")
    id("kotlin-android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

val rustCrateDir = file("../../../../crates/platform-mobile")
val rustTargetDir = file("${rustCrateDir}/target")
val rustLibraryName = "word_platform_mobile"
val rustGeneratedJniDir = file("$buildDir/generated/rust/jniLibs")
val rustTargetAbi = "arm64-v8a"
val rustTargetTriple = "aarch64-linux-android"
val rustLinkerTriple = "aarch64-linux-android"
val androidMinSdkLevel = 24

val localProperties = Properties()
val localPropertiesFile = rootProject.file("local.properties")
if (localPropertiesFile.exists()) {
    localPropertiesFile.inputStream().use { localProperties.load(it) }
}

val androidSdkDir = localProperties.getProperty("sdk.dir")?.let(::file)

val configuredNdkDir = androidSdkDir?.let {
    val flutterNdkVersion = providers.gradleProperty("flutter.ndkVersion").orNull
    if (flutterNdkVersion.isNullOrBlank()) null else file("${it.absolutePath}/ndk/$flutterNdkVersion")
}

val fallbackNdkDir = androidSdkDir
    ?.resolve("ndk")
    ?.takeIf { it.exists() }
    ?.listFiles()
    ?.filter { it.isDirectory }
    ?.sortedByDescending { it.name }
    ?.firstOrNull()

val rustNdkDir = when {
    configuredNdkDir?.exists() == true -> configuredNdkDir
    fallbackNdkDir?.exists() == true -> fallbackNdkDir
    else -> null
}

val cargoExecutableCandidates = listOf(
    file("D:/msys64/home/clf20/.cargo/bin/cargo.exe"),
    file("D:/msys64/home/clf20/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe"),
)
val cargoExecutable = cargoExecutableCandidates.firstOrNull { it.exists() }

val rustupExecutableCandidates = listOf(
    file("D:/msys64/home/clf20/.cargo/bin/rustup.exe"),
)
val rustupExecutable = rustupExecutableCandidates.firstOrNull { it.exists() }

fun hasRustAndroidTarget(): Boolean {
    val rustup = rustupExecutable ?: return false
    return try {
        val output = ByteArrayOutputStream()
        exec {
            commandLine(rustup.absolutePath, "target", "list", "--installed")
            standardOutput = output
            errorOutput = ByteArrayOutputStream()
            isIgnoreExitValue = true
        }
        output.toString().lineSequence().any { it.trim() == rustTargetTriple }
    } catch (_: Exception) {
        false
    }
}

fun rustBuildEnabled(): Boolean {
    return rustNdkDir != null && cargoExecutable != null && hasRustAndroidTarget()
}

fun rustToolchainBinDir(): File? {
    return rustNdkDir?.resolve("toolchains/llvm/prebuilt/windows-x86_64/bin")
}

fun configureRustToolchainEnv(task: Exec) {
    val binDir = rustToolchainBinDir()
        ?: throw GradleException("Android NDK LLVM toolchain directory is unavailable")
    val linker = File(binDir, "$rustLinkerTriple${androidMinSdkLevel}-clang.cmd")
    val cc = File(binDir, "$rustLinkerTriple${androidMinSdkLevel}-clang.cmd")
    val cxx = File(binDir, "$rustLinkerTriple${androidMinSdkLevel}-clang++.cmd")
    val ar = File(binDir, "llvm-ar.exe")
    val ranlib = File(binDir, "llvm-ranlib.exe")

    task.environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", linker.absolutePath)
    task.environment("CC", cc.absolutePath)
    task.environment("CXX", cxx.absolutePath)
    task.environment("AR", ar.absolutePath)
    task.environment("RANLIB", ranlib.absolutePath)
    task.environment("TARGET_CC", cc.absolutePath)
    task.environment("TARGET_CXX", cxx.absolutePath)
    task.environment("TARGET_AR", ar.absolutePath)
    task.environment("TARGET_RANLIB", ranlib.absolutePath)
    task.environment("CC_aarch64_linux_android", cc.absolutePath)
    task.environment("CXX_aarch64_linux_android", cxx.absolutePath)
    task.environment("AR_aarch64_linux_android", ar.absolutePath)
    task.environment("RANLIB_aarch64_linux_android", ranlib.absolutePath)
    task.environment("ANDROID_NDK_HOME", rustNdkDir!!.absolutePath)
    task.environment("ANDROID_NDK_ROOT", rustNdkDir.absolutePath)
    task.environment("CARGO_TARGET_DIR", rustTargetDir.absolutePath)

    val cargoBinDir = cargoExecutable?.parentFile?.absolutePath ?: ""
    task.environment(
        "PATH",
        listOf(binDir.absolutePath, cargoBinDir, System.getenv("PATH"))
            .filter { it.isNotBlank() }
            .joinToString(";"),
    )

    task.doFirst {
        listOf(linker, cc, cxx, ar, ranlib).forEach { tool ->
            if (!tool.exists()) {
                throw GradleException("Rust Android tool not found: ${tool.absolutePath}")
            }
        }
    }
}

android {
    namespace = "com.wordmobile"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_17.toString()
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "com.wordmobile"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    signingConfigs {
        getByName("debug") {
            storeFile = file("../../../mobile/android/app/debug.keystore")
            storePassword = "android"
            keyAlias = "androiddebugkey"
            keyPassword = "android"
        }
    }

    buildTypes {
        debug {
            signingConfig = signingConfigs.getByName("debug")
        }
        release {
            // Match the legacy RN debug signing key so upgrade installs preserve data.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir(rustGeneratedJniDir)
            assets.srcDir(file("../../../mobile/android/app/src/main/assets"))
        }
    }
}

flutter {
    source = "../.."
}

tasks.register<Exec>("buildRustDebugArm64") {
    onlyIf { rustBuildEnabled() }
    workingDir = rustCrateDir
    commandLine(
        cargoExecutable!!.absolutePath,
        "build",
        "--manifest-path",
        file("${rustCrateDir.absolutePath}/Cargo.toml").absolutePath,
        "--target",
        rustTargetTriple,
    )
    configureRustToolchainEnv(this)
}

tasks.register<Copy>("copyRustDebugArm64") {
    dependsOn("buildRustDebugArm64")
    val sourceSo = file("${rustTargetDir.absolutePath}/${rustTargetTriple}/debug/lib${rustLibraryName}.so")
    onlyIf { sourceSo.exists() }
    from(sourceSo)
    into(file("${rustGeneratedJniDir.absolutePath}/${rustTargetAbi}"))
    doFirst {
        if (!sourceSo.exists()) {
            throw GradleException("Rust debug JNI library not found: ${sourceSo.absolutePath}")
        }
    }
}

tasks.register<Exec>("buildRustReleaseArm64") {
    onlyIf { rustBuildEnabled() }
    workingDir = rustCrateDir
    commandLine(
        cargoExecutable!!.absolutePath,
        "build",
        "--release",
        "--manifest-path",
        file("${rustCrateDir.absolutePath}/Cargo.toml").absolutePath,
        "--target",
        rustTargetTriple,
    )
    configureRustToolchainEnv(this)
}

tasks.register<Copy>("copyRustReleaseArm64") {
    dependsOn("buildRustReleaseArm64")
    val sourceSo = file("${rustTargetDir.absolutePath}/${rustTargetTriple}/release/lib${rustLibraryName}.so")
    onlyIf { sourceSo.exists() }
    from(sourceSo)
    into(file("${rustGeneratedJniDir.absolutePath}/${rustTargetAbi}"))
    doFirst {
        if (!sourceSo.exists()) {
            throw GradleException("Rust release JNI library not found: ${sourceSo.absolutePath}")
        }
    }
}

afterEvaluate {
    tasks.matching {
        it.name == "mergeDebugJniLibFolders" || it.name == "mergeDebugNativeLibs"
    }.configureEach {
        dependsOn("copyRustDebugArm64")
    }
    tasks.matching {
        it.name == "mergeReleaseJniLibFolders" || it.name == "mergeReleaseNativeLibs"
    }.configureEach {
        dependsOn("copyRustReleaseArm64")
    }
}
