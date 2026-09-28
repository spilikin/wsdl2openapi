# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Status

Rust port of `../generator-kotlin/` and `../generator-golang/`, consuming the same OpenAPI JSON. CLI flags (`-f` / `-o` / `-n`) match the other generators, plus `--module-root`. The output is a **module tree** (root file `mod.rs`) that a crate mounts with `mod kon;`. Generated code depends only on `serde` and `quick-xml` (features `serialize`, `overlapped-lists`), plus `base64` when a schema uses `format: byte`.

For the overall pipeline see `../CLAUDE.md`.

## Build & Test

```bash
cargo test                          # unit tests + golden tests
just update-golden                  # rewrite tests/fixtures/*/expected
just generate                       # regenerate testproj/src/{kon,edge}
just test                           # generator tests + testproj round-trip tests
just lint                           # fmt --check, clippy -D warnings (generator and generated code)
cargo run -- -f ../konnektor-6.0.1.json -o /tmp/out -n naming-kon.json --module-root crate::kon
cargo run -- -f ../konnektor-6.0.1.json -o /tmp/out -n naming-kon.json --module-root crate::conn \
    --select testproj/select-konnektor.json   # only the selected operations
```

Toolchain: Rust edition 2024 (1.85+). Code is built as `proc-macro2`/`quote` token streams, parsed with `syn` and printed with `prettyplease`; there are no string templates.

## Architecture

| File | Role |
|------|------|
| `src/main.rs` | clap CLI |
| `src/lib.rs` | `generate(api, naming, options) -> Vec<GeneratedFile>`: extract → IR → boxing → namespaces → emit |
| `src/model.rs` | serde model of the input (`Api`, `WebService`, `Schema`, `XmlExtension`) |
| `src/naming.rs` | Regex package mappings (Go/Java `$1` replacements), Rust identifier rules (raw idents, `self_`) |
| `src/extract.rs` | Lifts inline objects / array items into named schemas (mirrors Go/Kotlin) |
| `src/ir.rs` | Resolved IR: items (struct / enum / alias) with module, identifier, XML node, occurrence; SOAP 1.1 ports |
| `src/select.rs` | `--select`: keep the selected operations (with service directory name, version, timeout class), prune items to those they reach, record each item's direction |
| `src/analysis.rs` | Namespace prefix table, per-operation namespace closure, Tarjan SCC boxing of recursive fields |
| `src/emit/` | Token emitters: `types.rs` (structs, enums, aliases), `port.rs` (bodies, envelopes, port trait), `soap.rs` (shared runtime module) |
| `src/writer.rs` | Module tree → files, `prettyplease`, header comment |

Key choices:
- **Namespaces via prefixes.** quick-xml's serde layer is not namespace-aware. Every qualified element/attribute is renamed `rename(serialize = "prefix:Local", deserialize = "Local")`; the deserializer matches local names only, so peers may use any prefixes. Declarations are written once on `SOAP-ENV:Envelope` (`BodyContent::NAMESPACES`, the namespaces reachable from the operation). No default `xmlns=` is ever declared, so an unprefixed name is genuinely unqualified — this is how SOAP 1.1 fault children and `elementFormDefault="unqualified"` elements are expressed. Prefixes derive from the declaring module (`signatureservice74`), `ns-` prepended for names starting with `xml`; `xml:lang` uses the predeclared `xml` prefix. Two child elements with the same local name in one type are rejected, since quick-xml could not tell them apart.
- **Enums serialize through their string value** (`as_str` / `FromStr` / hand-written serde impls), because quick-xml reads derived enums inside lists as a choice of elements.
- **Aliases.** A schema that is only a `$ref` (e.g. `Context → ContextType`) becomes `pub type Context = ContextType;`; element names live on fields, so no duplicate struct is needed.
- **`x-is-base` / `x-extends` are ignored**: base types are plain structs, as in the Kotlin generator.
- **Recursion** is broken with `Box` on every non-`Vec` field inside a strongly connected component.
- **Prelude shadowing.** Generated types named `Result`, `Option`, … make the emitter fully qualify `::std::…` paths in that module only.
- **SOAP.** Per operation: `{Op}Input` (single-variant enum, implements `soap::SoapRequest` with `OPERATION` and `Response`), `{Op}Output` (payload or `Fault(soap::Fault<{Op}FaultDetail>)`; implements `soap::SoapResponse`, whose `Success`/`Detail` types and `into_result()` let a client extract any operation's result generically), `{Op}FaultDetail` (the fault element *inside* `<detail>`), and `{Op}Envelope` / `{Op}ResponseEnvelope` aliases of the generic `soap::Envelope<C>`. Per port: an async trait (`fn op(&self, X) -> impl Future<Output = Result<Y, Self::Error>> + Send`). Ports sharing a module and name (the same WSDL service listed twice) are merged. SOAP 1.2 ports are skipped.
- **Selection (`--select FILE`).** For clients that use a handful of operations. The
  file lists, per port module (`gematik.conn.eventservice72`), the service name and
  `major.minor` version as a Konnektor's service directory advertises them, and each
  operation's timeout class (`short`/`long`); `"ports": false` drops the port traits.
  - Only the selected operations and the items they reach are generated. The Konnektor
    client set is 7k lines instead of 19k.
  - Each item gets only the derives and renames for its direction:
    - request types are `Serialize` with prefixed names;
    - response and fault types are `Deserialize` by local name;
    - types used both ways keep both.
  - `SoapOperation` then carries `service`, `version` and `timeout`, so one generic
    client call handles discovery, envelope, timeout and faults for every operation.
  - Unselected generation leaves these empty (`Timeout::Short`).
- **Known limitation.** `format: xml` maps to `soap::AnyXml`, which drops namespace prefixes of nested content; wildcard content without an element name (`xs:any`) is not captured, as in the Go and Kotlin generators.

## Tests

- Unit tests in `naming.rs`, `extract.rs`, `analysis.rs`.
- `tests/golden.rs` — runs every fixture in `../generator-kotlin/app/src/test/resources/fixtures/` plus Rust-only inputs under `tests/fixtures/<name>/api.json` (`edge-cases`: recursion, keywords, prelude shadowing, enum value names, base64/any/int32, `xml:lang`, aliases) and diffs against `tests/fixtures/<name>/expected/`.
- `tests/fixtures/selection/` — a `select.json` fixture: an unselected operation and an unreachable type are dropped, one-way types carry one-way derives.
- `testproj/tests/selected.rs` — the Konnektor client selection (`testproj/select-konnektor.json`, mounted as `crate::conn`): namespaces of written requests, the GetCards response `go/kon` pins (foreign prefixes), a fault's gematik trace from `detail`, `Base64Data` written as text, CardService 8.2 sessions, operation metadata.
- `testproj/` — separate crate that mounts the generated Konnektor tree (`crate::kon`) and the `edge-cases` tree (`crate::edge`); `tests/generated.rs` checks resolved namespaces with `NsReader`, parsing with foreign prefixes, round trips, typed fault details, `AnyXml`/`Base64Binary`, boxing, and a port-trait implementation over a generic `SoapRequest` transport.
