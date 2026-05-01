# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Status

Working Kotlin port of `../generator-golang/`. CLI flags (`-f` / `-o` / `-n`) match the Go binary. Output is idiomatic Kotlin: one top-level type per file, no explicit `public` modifier, kotlinx.serialization + xmlutil annotations only (no SOAP framework dependency), and `@Serializable sealed interface` markers for `x-is-base` / `x-extends` polymorphism.

For the overall pipeline (Python WSDL→OpenAPI converter, x-wsdl extensions, naming configs) see `../CLAUDE.md`. Both this generator and `../generator-golang/` consume the same OpenAPI JSON (e.g. `../konnektor-opb6.json`).

## Build & Test

```bash
./gradlew :app:test                     # All unit + golden + smoke tests
./gradlew :app:test --tests "de.gematik.wsdl2openapi.NamingStrategyTest"
./gradlew :app:test -Dgolden.update=true   # Rematerialise golden trees
./gradlew :app:shadowJar                # Builds app/build/libs/wsdl2openapi2kotlin.jar
./gradlew :app:run --args="--file ../konnektor-opb6.json --output /tmp/out --naming naming-kon.json"
```

- Toolchain: **JDK 21**, Kotlin **2.3.0**, Gradle **9.4.1**.
- Fat jar produced by `com.gradleup.shadow` 9.x (Shadow's task class still ships under `com.github.jengelman.gradle.plugins.shadow.tasks.ShadowJar`).
- Tests use JUnit Platform + `kotlin-test`; the Gradle `test` task forwards `golden.*` system properties so `-Dgolden.update=true` reaches the test JVM.

## Architecture

| File | Role |
|------|------|
| `app/src/main/kotlin/de/gematik/wsdl2openapi/Main.kt` | Clikt CLI entry point (`MainKt`) |
| `Model.kt` | kotlinx.serialization data classes for `Api` / `WebService` / `OperationDefinition`; `TypeView` is a thin wrapper over the raw `JsonObject` so insertion order is preserved |
| `Naming.kt` | Regex-based namespace→package mapping. `buildPackagePath` tolerates Go-style `basePackage` values (slashes, hyphens) so the same `naming-kon.json` works for both generators |
| `Extract.kt` | Lifts inline `type: object` / array-items into named top-level schemas with `$ref` placeholders — mirrors `generator_extract.go` |
| `TypeGenerator.kt` | The whole emitter: parses `TypePtr`s, drives kotlinpoet to write one `<TypeName>.kt` per schema, plus `Operations.kt` + per-operation `<Op>Envelope.kt` / `<Op>ResponseEnvelope.kt` per port. `stripDefaultPublic` post-processes kotlinpoet 2.x output (which always emits `public`) |

Key choices specific to this generator:
- **One file per top-level type** (idiomatic Kotlin), not a single `Types.kt` per package like the Go side.
- **Sealed interfaces for polymorphism** — `x-is-base` emits `@Serializable sealed interface IFoo`; subtypes (`x-extends`) implement it. Combined with `@XmlSerialName` on each impl, xmlutil dispatches on element name.
- **Shared `SoapEnvelope` contract** — single `<basePackage>.soap.SoapEnvelope.kt` declares `interface SoapEnvelope { fun isFault(): Boolean }` and the `SoapOperation` data class. Every generated envelope `: SoapEnvelope`, so consumers can poll fault state without knowing the concrete operation type. This is polymorphism that the Go generator cannot express — Go uses duck-typed `IsFault()` methods.
- **Per-port service interface** — `<PortName>.kt` declares one typed function per operation (`fun foo(request: Foo): FooResponse`). Gives consumers a real Kotlin contract to implement (e.g., for an HTTP-backed transport) or to mock in tests.
- **No runtime/SOAP-framework dependency** — generated code only imports `kotlinx.serialization` and `nl.adaptivity.xmlutil.serialization`.

## Tests

- `NamingStrategyTest` — regex mappings, `basePackage` handling (incl. Go-style), keyword escaping, port mappings.
- `ExtractTest` — inline-object lifting, array items, xml-extension preservation on `$ref` placeholders.
- `GoldenTest` — TestFactory-driven, runs each fixture under `app/src/test/resources/fixtures/<name>/{api.json,naming.json}` and diffs the generated tree against `<name>/expected/`. Fixtures cover: `simple` (1 port + envelope), `enum` (`@SerialName` per value), `polymorphism` (sealed interface + impls), `attributes` (xml `attribute: true` and array elements), `multi-port` (per-port subpackage + soap12 skipped). Update with `-Dgolden.update=true`.
- `KonnektorSmokeTest` — runs the generator end-to-end against `../konnektor-opb6.json`, asserts file count and presence of representative outputs (auto-skipped if the input is not present).
