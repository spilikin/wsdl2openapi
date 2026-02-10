# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**wsdl2openapi** is a two-stage pipeline that converts WSDL/XSD specifications into modern OpenAPI 3.x definitions, then generates type-safe Go code from those definitions. Built for German healthcare telematics infrastructure (Gematik Konnektor), it eliminates the need for heavyweight SOAP frameworks — generated Go code uses only standard library XML/HTTP.

```
WSDL/XSD files → [Python converter] → OpenAPI JSON (with x-wsdl-* extensions) → [Go generator] → Go structs + SOAP envelopes
```

## Build & Test Commands

### Python (root directory — WSDL-to-OpenAPI converter)

```bash
uv sync                           # Install dependencies (Python 3.13+, uses uv)
uv run pytest tests/              # Run all Python tests
uv run pytest tests/test_soap.py  # Run a single test file
uv run python konnektor-opb6.py   # Convert Konnektor WSDLs → OpenAPI JSON/YAML
```

pytest is configured with `--maxfail=1 -s` in `pyproject.toml`.

### Go (generator-golang/ — OpenAPI-to-Go generator)

```bash
cd generator-golang
just generate-kon                  # Generate Go code from Konnektor OpenAPI spec
go test ./...                      # Run all Go tests (generator + testproj)
go test -v ./testproj              # Run integration tests on generated code
go test -run TestFoo ./testproj    # Run a single test
go run ./cmd/wsdl2openapi2go --file ../konnektor-opb6.json --output ./testproj/kon/api --naming naming-kon.json
```

The Go generator has a separate `go.mod`. The `testproj/` and `soap/` subdirectories each have their own `go.mod` as well.

## Architecture

### Python Converter (`src/wsdl2openapi/`)

| Module | Role |
|--------|------|
| `builder.py` | Main API: `Builder.add_wsdl()`, `Builder.add_xsd()` — orchestrates conversion |
| `model.py` | msgspec-based data classes (`Api`, `WebService`, `OperationDefinition`), JSON/YAML serialization |
| `schema_visitor.py` | Traverses zeep XSD schema objects → OpenAPI JSON Schema |
| `wsdl_visitor.py` | Parses WSDL definitions → services/ports/operations |
| `schema_facets_visitor.py` | Extracts XSD facets (patterns, enums, min/max) as JSON Schema constraints |
| `common.py` | `NamingStrategy`, `Context` utilities, XML loading |

The converter uses **zeep** for WSDL/XSD parsing and **msgspec** for fast serialization. Output OpenAPI includes `x-wsdl-services` and `x-wsdl-extension` custom extensions consumed by the Go generator.

### Go Generator (`generator-golang/`)

| File | Role |
|------|------|
| `generator.go` | Main orchestration: validates refs, generates `types.go` per package (parallel via goroutines) |
| `generator_soap.go` | Generates SOAP 1.1/1.2 envelopes, fault handling, `soap.go` per service |
| `model.go` | Data structures including `OrderedMap[T]` (preserves JSON key order during parsing) |
| `naming.go` | Regex-based package/type name transformations from JSON config |
| `cmd/wsdl2openapi2go/` | CLI entry point |
| `soap/` | Runtime SOAP library (envelope marshal/unmarshal, used by generated code) |
| `testproj/` | Generated code examples + integration tests |

Key patterns:
- **Code generation via jennifer** — all output is built as AST nodes, not string templates
- **Polymorphism via interfaces** — types with `x-is-base: true` generate interfaces with `Is{Type}()` marker methods
- **Two-phase generation** — types first (`GenerateTypes()`), then SOAP envelopes (`GenerateSoap()`) that reference those types
- **Naming configs** — regex-based package mappings in JSON files (e.g., `naming-kon.json`, `naming-epa.json`) transform namespace URIs like `de.gematik.ws.*` into Go package paths

### Conversion Scripts (root)

- `konnektor-opb6.py` — fetches Konnektor WSDLs from GitHub, produces `konnektor-opb6.json` / `.yaml`
- `XDSDocumentService.json` / `.yaml` — EPA service OpenAPI output

## SOAP Support

- **SOAP 1.1**: Fully implemented with typed envelopes and fault handling
- **SOAP 1.2**: Scaffolded but incomplete
