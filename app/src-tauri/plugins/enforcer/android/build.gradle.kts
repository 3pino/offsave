import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import java.util.Properties

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "dev.offsave.enforcer"
    compileSdk = 36

    defaultConfig {
        minSdk = 26
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    testOptions {
        unitTests.isReturnDefaultValues = true
        // Rust と共通の試験データ（crates/core/tests/vectors）
        unitTests.all {
            it.systemProperty("vectors.dir", file("../../../../../crates/core/tests/vectors").absolutePath)
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

// 版は plugins/gradle-versions.properties
val versions = Properties().apply { file("../../gradle-versions.properties").inputStream().use { load(it) } }

dependencies {
    implementation("androidx.core:core-ktx:${versions.getProperty("androidx.core")}")
    // WebSocket クライアント（Android の標準ライブラリにはない）
    implementation("com.squareup.okhttp3:okhttp:${versions.getProperty("okhttp")}")
    // Shizuku があるとき、「設定の書き込みの権限」をワンボタンで付ける
    implementation("dev.rikka.shizuku:api:${versions.getProperty("shizuku")}")
    implementation("dev.rikka.shizuku:provider:${versions.getProperty("shizuku")}")
    implementation(project(":tauri-android"))
    testImplementation("junit:junit:${versions.getProperty("junit")}")
    // JVM の試験では Android の org.json が使えないので、同じ API の実装を入れる
    testImplementation("org.json:json:${versions.getProperty("json")}")
}
