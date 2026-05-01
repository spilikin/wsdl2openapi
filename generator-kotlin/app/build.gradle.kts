import com.github.jengelman.gradle.plugins.shadow.tasks.ShadowJar

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.shadow)
    application
}

repositories {
    mavenCentral()
}

dependencies {
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.xmlutil.serialization)
    implementation(libs.kotlinpoet)
    implementation(libs.clikt)

    testImplementation(kotlin("test"))
    testImplementation(libs.junit.jupiter.api)
    testRuntimeOnly(libs.junit.jupiter.engine)
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

java {
    toolchain { languageVersion = JavaLanguageVersion.of(21) }
}

application {
    mainClass = "de.gematik.wsdl2openapi.MainKt"
}

tasks.named<Test>("test") {
    useJUnitPlatform()
    // Forward `-Dgolden.update=true` (and any other golden.* prop) into the
    // test JVM so the regression test can rematerialise its expected trees.
    systemProperties.putAll(System.getProperties().filterKeys { it.toString().startsWith("golden.") }.mapKeys { it.key.toString() })
}

tasks.named<ShadowJar>("shadowJar") {
    archiveBaseName.set("wsdl2openapi2kotlin")
    archiveClassifier.set("")
    archiveVersion.set("")
    mergeServiceFiles()
}
