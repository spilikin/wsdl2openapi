package de.gematik.wsdl2openapi

import org.junit.jupiter.api.Assumptions.assumeTrue
import org.junit.jupiter.api.Test
import java.io.File
import java.nio.file.Files
import kotlin.test.assertTrue

/**
 * End-to-end smoke test against the real `../konnektor-opb6.json` shipped in
 * the repo. The fixture is gigantic (700+ schemas, 15 services), so this is
 * a structural check — does the generator complete, and does it produce the
 * top-level files we expect?
 *
 * Skipped (not failed) if the input file is absent so the suite stays green
 * for someone who has only checked out `generator-kotlin/`.
 */
class KonnektorSmokeTest {

    @Test fun `generates without errors against konnektor-opb6`() {
        // Test JVM's `user.dir` is `<repo>/generator-kotlin/app`. The OpenAPI
        // input lives at the top-level repo root; the naming JSON lives one
        // level up alongside the build files.
        val moduleRoot = File(System.getProperty("user.dir")).parentFile  // generator-kotlin
        val repoRoot = moduleRoot.parentFile                              // wsdl2openapi
        val apiFile = File(repoRoot, "konnektor-opb6.json")
        val namingFile = File(moduleRoot, "naming-kon.json")
        assumeTrue(apiFile.exists() && namingFile.exists(),
            "konnektor-opb6.json or naming-kon.json missing from repo root — skipping")

        val outDir = Files.createTempDirectory("konnektor-smoke-").toFile()
        try {
            val api = JSON.decodeFromString(Api.serializer(), apiFile.readText())
            val naming = JSON.decodeFromString(NamingStrategy.serializer(), namingFile.readText())
            Generator(outDir.path, naming, api).generate()

            val generatedFiles = outDir.walkTopDown().filter { it.isFile && it.extension == "kt" }.toList()
            assertTrue(generatedFiles.size > 500, "expected many generated files, got ${generatedFiles.size}")

            // Spot-check a few representative files exist (covers types,
            // enums-with-facets, SOAP envelopes, the shared SoapEnvelope
            // contract, and per-port service interfaces).
            val byName = generatedFiles.groupBy { it.name }
            for (expected in listOf(
                "ReadCardCertificate.kt",
                "ReadCardCertificateEnvelope.kt",
                "VerificationResultType.kt",
                "Operations.kt",
                "SoapEnvelope.kt",
                "AuthSignatureServicePort.kt",
            )) {
                assertTrue(expected in byName, "expected generated file $expected not found")
            }

            // Every envelope class should implement the shared SoapEnvelope.
            val envelopeFiles = generatedFiles.filter {
                it.name.endsWith("Envelope.kt") && it.name != "SoapEnvelope.kt"
            }
            assertTrue(envelopeFiles.isNotEmpty(), "no envelope files generated")
            for (f in envelopeFiles) {
                assertTrue(
                    f.readText().contains(": SoapEnvelope"),
                    "${f.name} does not implement the shared SoapEnvelope interface"
                )
            }
        } finally {
            outDir.deleteRecursively()
        }
    }
}
