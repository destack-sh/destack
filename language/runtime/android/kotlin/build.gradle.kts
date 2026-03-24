plugins {
    id("com.android.library")
    id("org.jlleitschuh.gradle.ktlint") version "12.1.2"
}

android {
    namespace = "dev.destack.runtime.android"
    compileSdk = 36

    defaultConfig {
        minSdk = 24
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
        }
    }

    buildFeatures {
        aidl = false
        buildConfig = false
        resValues = false
        shaders = false
    }

    compileOptions {
        isCoreLibraryDesugaringEnabled = true
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        abortOnError = true
    }

    testOptions {
        unitTests.isReturnDefaultValues = true
    }
}

dependencies {
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.5")
    implementation("androidx.activity:activity:1.12.4")
    testImplementation("junit:junit:4.13.2")
}

ktlint {
    android.set(true)
    ignoreFailures.set(false)

    filter {
        exclude("**/build/**")
    }
}
