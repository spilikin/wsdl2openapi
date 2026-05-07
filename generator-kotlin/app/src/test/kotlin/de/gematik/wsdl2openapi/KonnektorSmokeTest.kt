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
            assertTrue(generatedFiles.size > 50, "expected many generated files, got ${generatedFiles.size}")

            // Spot-check a few representative files exist (covers per-port
            // grouped envelopes, the shared SoapEnvelope contract, per-port
            // service interfaces, and per-package grouped schemas).
            val byName = generatedFiles.groupBy { it.name }
            for (expected in listOf(
                "Envelopes.kt",
                "Operations.kt",
                "SoapEnvelope.kt",
                "AuthSignatureServicePort.kt",
                "Schemas.kt",
            )) {
                assertTrue(expected in byName, "expected generated file $expected not found")
            }

            // Schema types now live inside per-package Schemas.kt — confirm
            // representative request/enum types are emitted somewhere.
            val schemasContent = (byName["Schemas.kt"] ?: emptyList())
                .joinToString("\n") { it.readText() }
            for (expected in listOf("ReadCardCertificate", "VerificationResultType")) {
                assertTrue(
                    schemasContent.contains(expected),
                    "expected type $expected not found in any Schemas.kt"
                )
            }

            // Every envelope class declared inside Envelopes.kt should implement
            // the shared SoapEnvelope contract.
            val envelopeFiles = byName["Envelopes.kt"] ?: emptyList()
            assertTrue(envelopeFiles.isNotEmpty(), "no Envelopes.kt files generated")
            for (f in envelopeFiles) {
                val text = f.readText()
                assertTrue(
                    text.contains(": SoapEnvelope"),
                    "${f.parentFile.name}/${f.name} has no class implementing SoapEnvelope"
                )
            }
        } finally {
            outDir.deleteRecursively()
        }
    }
}
