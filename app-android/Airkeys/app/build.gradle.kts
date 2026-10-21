plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

// core/ — the Rust workspace, next to app-android/
val coreDir: File = rootDir.resolve("../../core").canonicalFile
// the ABIs to build the core for: all three, or AIRKEYS_ABIS="arm64-v8a" for a quicker build
val abis = (providers.environmentVariable("AIRKEYS_ABIS").orNull ?: "arm64-v8a armeabi-v7a x86_64")
    .split(" ").filter { it.isNotBlank() }
// what the Rust build writes (core/scripts/build-android.sh)
val rustLibs: File = layout.buildDirectory.dir("rust/jniLibs").get().asFile
val rustKotlin: File = layout.buildDirectory.dir("generated/uniffi").get().asFile

android {
    namespace = "app.airkeys"
    compileSdk {
        version = release(37) { minorApiLevel = 1 }
    }

    defaultConfig {
        applicationId = "app.airkeys"
        // Android 8: an old spare phone makes a good cold wallet
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += abis }
    }

    // Release builds are signed with your own key when these are set (in ~/.gradle/
    // gradle.properties or the environment); otherwise the APK is left unsigned.
    val keystore = providers.gradleProperty("airkeysKeystore").orElse(providers.environmentVariable("AIRKEYS_KEYSTORE")).orNull
    signingConfigs {
        if (keystore != null) {
            create("release") {
                storeFile = file(keystore)
                storePassword = providers.gradleProperty("airkeysKeystorePassword").orElse(providers.environmentVariable("AIRKEYS_KEYSTORE_PASSWORD")).orNull
                keyAlias = providers.gradleProperty("airkeysKeyAlias").orElse(providers.environmentVariable("AIRKEYS_KEY_ALIAS")).orNull
                keyPassword = providers.gradleProperty("airkeysKeyPassword").orElse(providers.environmentVariable("AIRKEYS_KEY_PASSWORD")).orNull
            }
        }
    }

    buildTypes {
        debug {
            buildConfigField("boolean", "TEST_HOOKS", "true")
        }
        release {
            // No shrinking or obfuscation: the app is small, and its bytecode stays as written —
            // easier to audit against the source.
            optimization { enable = false }
            buildConfigField("boolean", "TEST_HOOKS", "false")
            signingConfig = signingConfigs.findByName("release")
        }
    }
    // no dependency metadata blob in the APK: builds are reproducible, and F-Droid accepts them
    dependenciesInfo {
        includeInApk = false
        includeInBundle = false
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    sourceSets {
        // the Rust core's libraries and uniffi's Kotlin for them (core/scripts/build-android.sh)
        getByName("main") {
            kotlin.directories.add(rustKotlin.path)
            jniLibs.directories.add(rustLibs.path)
        }
    }
}

java { toolchain { languageVersion = JavaLanguageVersion.of(17) } }

// The Rust core: a library per ABI and the Kotlin bindings. Up to date unless something in
// core/ (or the registry) changed. Skip with -PairkeysSkipRust when only Kotlin changed.
val cargoBuild = tasks.register<Exec>("cargoBuild") {
    group = "build"
    description = "Builds the Rust core (cargo-ndk) and generates its Kotlin bindings"
    commandLine("sh", coreDir.resolve("scripts/build-android.sh").absolutePath)
    environment("AIRKEYS_ABIS", abis.joinToString(" "))
    enabled = !providers.gradleProperty("airkeysSkipRust").isPresent
    inputs.files(fileTree(coreDir) { exclude("target/**", "e2e/**") })
    inputs.file(coreDir.resolve("../registry/networks.json"))
    inputs.property("abis", abis.joinToString(" "))
    outputs.dir(rustLibs)
    outputs.dir(rustKotlin)
}
tasks.named("preBuild") { dependsOn(cargoBuild) }

dependencies {
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.foundation)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.camera.camera2)
    implementation(libs.androidx.camera.lifecycle)
    implementation(libs.androidx.camera.view)
    implementation(libs.jna) { artifact { type = "aar" } }
}
