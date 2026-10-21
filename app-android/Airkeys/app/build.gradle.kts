plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

// core/ — the Rust workspace, next to app-android/
val coreDir: File = rootDir.resolve("../../core").canonicalFile

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
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
    }

    buildTypes {
        debug {
            buildConfigField("boolean", "TEST_HOOKS", "true")
        }
        release {
            optimization { enable = false }
            buildConfigField("boolean", "TEST_HOOKS", "false")
            // Signed with the debug key so it installs as is; set up your own key to publish.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    sourceSets {
        // uniffi-generated Kotlin (core/scripts/build-android.sh)
        getByName("main") { kotlin.directories.add(layout.buildDirectory.dir("generated/uniffi").get().asFile.path) }
    }
    packaging { jniLibs { useLegacyPackaging = false } }
}

java { toolchain { languageVersion = JavaLanguageVersion.of(17) } }

// The Rust core: a library per ABI and the Kotlin bindings. Up to date unless core/ changed.
// Skip with -PairkeysSkipRust when only Kotlin changed.
val cargoBuild = tasks.register<Exec>("cargoBuild") {
    group = "build"
    description = "Builds the Rust core (cargo-ndk) and generates its Kotlin bindings"
    commandLine("sh", coreDir.resolve("scripts/build-android.sh").absolutePath)
    enabled = !providers.gradleProperty("airkeysSkipRust").isPresent
    inputs.dir(coreDir.resolve("wallet/src"))
    inputs.dir(coreDir.resolve("app/src"))
    inputs.files(coreDir.resolve("Cargo.toml"), coreDir.resolve("Cargo.lock"), coreDir.resolve("../registry/networks.json"))
    outputs.dir(projectDir.resolve("src/main/jniLibs"))
    outputs.dir(layout.buildDirectory.dir("generated/uniffi"))
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
    debugImplementation(libs.androidx.compose.ui.tooling)
    testImplementation(libs.junit)
}
