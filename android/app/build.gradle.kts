plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// The version comes from Cargo.toml, so it is kept in one place
// "0.4.0-beta.1" for a pre-release: its numbers make the version code, the whole is the name
val cargoVersion = Regex("""(?m)^version = "((\d+)\.(\d+)\.(\d+)(-[0-9A-Za-z.]+)?)"""")
    .find(rootDir.parentFile.resolve("Cargo.toml").readText())!!.groupValues

android {
    namespace = "io.github.wobi848.bacnetio"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "io.github.wobi848.bacnetio"
        // Android 10: saving to Downloads without storage permission
        minSdk = 29
        targetSdk = 36
        versionCode = cargoVersion[2].toInt() * 10000 + cargoVersion[3].toInt() * 100 + cargoVersion[4].toInt()
        versionName = cargoVersion[1]
        ndk { abiFilters += "arm64-v8a" }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // bacnetio runs as its own process from nativeLibraryDir, so the file must be unpacked
    packaging { jniLibs { useLegacyPackaging = true } }
}

kotlin { jvmToolchain(17) }

// bacnetio (Rust) built for Android and shipped as libbacnetio.so: only then may the app run it
val cargoBuild by tasks.registering(Exec::class) {
    val bin = android.ndkDirectory.resolve("toolchains/llvm/prebuilt/linux-x86_64/bin")
    val clang = bin.resolve("aarch64-linux-android29-clang").path
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", clang)
    environment("CC_aarch64_linux_android", clang)
    environment("AR_aarch64_linux_android", bin.resolve("llvm-ar").path)
    commandLine(
        rootDir.resolve("build-core.sh").path,
        projectDir.resolve("src/main/jniLibs/arm64-v8a/libbacnetio.so").path,
    )
}
tasks.named("preBuild") { dependsOn(cargoBuild) }
