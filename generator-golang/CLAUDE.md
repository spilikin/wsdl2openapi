# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **WSDL-to-Go Code Generator** that converts OpenAPI specifications (extended with WSDL metadata) into idiomatic Go source code. It generates type-safe Go structures and SOAP envelope handling from WSDL/OpenAPI definitions, primarily for German healthcare web services (Gematik).

## Build Commands

This project uses **Just** (a modern Make alternative) as its build system.

```bash
just info              # Display project info
just generate-kon      # Generate from Konnektor spec
go test ./...          # Run all tests
go test -v ./testproj  # Run integration tests on generated code
```

## Architecture

### Data Flow

```
OpenAPI JSON (with x-wsdl-* extensions)
    ↓
cmd/wsdl2openapi2go (CLI entry point)
    ↓
Generator
    ├── NamingStrategy → transforms package/type names via regex
    ├── GenerateTypes() → Creates types.go files with Go structs
    └── GenerateSoap() → Creates SOAP envelope code with fault handling
    ↓
Generated Go code
```

### Core Components

| File | Purpose |
|------|---------|
| `generator.go` | Main orchestration, type generation, schema parsing |
| `generator_soap.go` | SOAP 1.1/1.2 envelope and fault generation |
| `model.go` | Data structures for OpenAPI/WSDL extensions (`Api`, `WebService`, `OperationDefinition`, etc.) |
| `naming.go` | Regex-based package/type name transformations |
| `soap/` | Runtime SOAP envelope marshaling/unmarshaling |
| `cmd/wsdl2openapi2go/` | CLI entry point |
| `testproj/` | Example generated code and integration tests |

### Key Methods

- `Generator.GenerateTypes()` - Generates Go struct files from JSON schemas
- `Generator.GenerateSoap()` - Generates SOAP envelope structures per operation
- `parseType()` - Recursively parses JSON schema types (handles primitives, objects, arrays, $ref)
- `renderProperty()` - Converts schema properties to Go fields with XML tags
- `NamingStrategy.NormalizePackageName()` - Applies regex package mappings

### Input/Output

**Input:** OpenAPI 3.x JSON with `x-wsdl-extension` metadata + naming config JSON

**Output:** Go source files with:
- `types.go` - Struct definitions with XML tags
- `soap.go` - Typed SOAP envelopes and operation stubs

## Key Patterns

- **Code generation uses [jennifer](https://github.com/dave/jennifer)** - all generated code is built via AST, not string templates
- **Polymorphism via interfaces** - base types (marked `x-is-base`) generate interfaces with `Is{Type}()` methods
- **Custom OrderedMap[T]** - preserves JSON key order during schema parsing
- **Namespace preservation** - XML namespaces are tracked and emitted in generated struct tags
- **Two-phase generation** - types first, then SOAP envelopes that reference those types

## Naming Configuration

Package names are transformed via regex patterns in JSON config files (e.g., `naming-kon.json`):

```json
{
  "basePackage": "github.com/test/testproj/kon/api",
  "packageMappings": [
    {"pattern": "de\\.gematik\\.ws\\.(.*)", "replacement": "gematik.$1"}
  ]
}
```

## SOAP Support Status

- **SOAP 1.1**: Fully implemented with fault handling
- **SOAP 1.2**: Scaffolded but incomplete
