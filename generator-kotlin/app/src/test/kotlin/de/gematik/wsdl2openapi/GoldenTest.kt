package de.gematik.wsdl2openapi

import org.junit.jupiter.api.DynamicTest
import org.junit.jupiter.api.TestFactory
import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.copyTo
import kotlin.io.path.createDirectories
import kotlin.io.path.relativeTo
import kotlin.io.path.walk
import kotlin.io.path.writeText
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/**
 * Golden / regression tests. Each fixture under `src/test/resources/fixtures/`
 * provides an `api.json` + `naming.json`; running the generator must produce
 * the file tree pinned in `<fixture>/expected/`.
 *
 * Set `-Dgolden.update=true` to (re-)materialise the expected tree from the
 * current generator output. Use this only when you've eyeballed the diff and
 * are intentionally accepting the change.
 */
class GoldenTest {

    private val fixturesRoot: Path =
        Path.of(System.getProperty("user.dir"), "src", "test", "resources", "fixtures")

    private val updateGolden: Boolean = System.getProperty("golden.update") == "true"

    @OptIn(kotlin.io.path.ExperimentalPathApi::class)
    @TestFactory
    fun `each fixture matches its expected tree`(): List<DynamicTest> {
        val fixtures = Files.list(fixturesRoot).use { stream ->
            stream.filter { Files.isDirectory(it) }.sorted().toList()
        }
        return fixtures.map { fixture ->
            DynamicTest.dynamicTest(fixture.fileName.toString()) {
                runFixture(fixture)
            }
        }
    }

    @OptIn(kotlin.io.path.ExperimentalPathApi::class)
    private fun runFixture(fixture: Path) {
        val apiFile = fixture.resolve("api.json")
        val namingFile = fixture.resolve("naming.json")
        val expectedDir = fixture.resolve("expected")

        val outDir = Files.createTempDirectory("golden-${fixture.fileName}-")
        try {
            val api = JSON.decodeFromString(Api.serializer(), Files.readString(apiFile))
            val naming = JSON.decodeFromString(NamingStrategy.serializer(), Files.readString(namingFile))
            Generator(outDir.toString(), naming, api).generate()

            if (updateGolden) {
                refreshExpected(outDir, expectedDir)
                return
            }

            assertTrue(Files.exists(expectedDir),
                "Expected tree not found at $expectedDir. Run with -Dgolden.update=true to materialise it.")
            compareTrees(outDir, expectedDir)
        } finally {
            outDir.toFile().deleteRecursively()
        }
    }

    @OptIn(kotlin.io.path.ExperimentalPathApi::class)
    private fun compareTrees(actualRoot: Path, expectedRoot: Path) {
        val actualFiles = collectRelative(actualRoot)
        val expectedFiles = collectRelative(expectedRoot)
        assertEquals(expectedFiles, actualFiles, "Generated file set differs from golden tree.")

        for (rel in actualFiles) {
            val actual = Files.readString(actualRoot.resolve(rel))
            val expected = Files.readString(expectedRoot.resolve(rel))
            assertEquals(expected, actual, "Content mismatch at $rel")
        }
    }

    @OptIn(kotlin.io.path.ExperimentalPathApi::class)
    private fun refreshExpected(actualRoot: Path, expectedRoot: Path) {
        if (Files.exists(expectedRoot)) expectedRoot.toFile().deleteRecursively()
        expectedRoot.createDirectories()
        actualRoot.walk().filter { Files.isRegularFile(it) }.forEach { src ->
            val rel = src.relativeTo(actualRoot)
            val dst = expectedRoot.resolve(rel.toString())
            dst.parent?.createDirectories()
            src.copyTo(dst, overwrite = true)
        }
    }

    @OptIn(kotlin.io.path.ExperimentalPathApi::class)
    private fun collectRelative(root: Path): Set<String> =
        if (!Files.exists(root)) emptySet()
        else root.walk()
            .filter { Files.isRegularFile(it) }
            .map { it.relativeTo(root).toString() }
            .toSortedSet()
}
