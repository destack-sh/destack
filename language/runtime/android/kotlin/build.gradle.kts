plugins {
    id("com.android.library")
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
    implementation("androidx.activity:activity:1.12.4")
    testImplementation("junit:junit:4.13.2")
}
