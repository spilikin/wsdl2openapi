# 🌀 WSDL to OpenAPI to Happiness

An intermediate representation for SOAP 1.1 and 1.2 Web Services based on [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0).

XML Schema definitions are converted to [JSON Schema with OpenAPI XML annotations](https://swagger.io/docs/specification/v3_0/data-models/representing-xml/). Complex type hierarchies are **flattened** — inherited properties are inlined into each type, eliminating polymorphism. This makes the output directly consumable by code generators for native languages like **Go** and **Rust**, where you only need an XML parser, a serializer, and an HTTP client.

SOAP services and operations are represented using `x-wsdl-*` extensions since OpenAPI has no native concept of SOAP bindings.

## Why

There are practically no tools to work with WSDL and SOAP for modern languages like Go or Rust. Especially not for complex services and datatypes used in enterprise applications or in German telematics infrastructure (TI). By converting WSDL to OpenAPI, an intermediate format is created that is closer to the semantics of programming languages. From this format we can generate type-safe code with much less abstraction than, for example, Java Apache CXF. All you need is an XML parser, a serializer, and an HTTP client. For instance, in Go all required functionality is available in the standard library.

## Features

- Convert XML Schema definitions to JSON Schema with [OpenAPI XML annotations](https://swagger.io/docs/specification/v3_0/data-models/representing-xml/)
- Flatten type hierarchies — inherited properties are copied into subtypes, tracked via `x-extends`
- SOAP 1.1 and 1.2 service definitions via `x-wsdl-services` extension
- Full XML namespace preservation
- Combine multiple XSD and WSDL files into a single OpenAPI document
- XSD facets and constraints (patterns, enums, min/max) mapped to JSON Schema validation
- Support for TI-Konnektor naming and versioning convention: multiple patch-level versions of services in the same OpenAPI document

## Generated Examples

- **TI-Konnektor (OPB6):** [JSON](Konnektor-OPB6.json) | [YAML](Konnektor-OPB6.yaml)
- **EPA XDS Document Service:** [JSON](XDSDocumentService.json) | [YAML](XDSDocumentService.yaml)

## Output Format

The output is a standard OpenAPI 3.1 document with two extensions:

### `x-wsdl-services` — SOAP service definitions

Services, ports, and operations are defined at the top level. Each operation references input/output schemas via `$ref` pointers into `components/schemas`.

```yaml
x-wsdl-services:
- name: CardService
  targetNamespace: http://ws.gematik.de/conn/CardService/WSDL/v8.1
  targetPackage: de.gematik.ws.conn.CardService81
  ports:
  - name: CardServicePort
    bindingType: soap11
    address: https://ti-konnektor/cardservice
    operations:
    - name: VerifyPin
      soapAction: http://ws.gematik.de/conn/CardService/v8.1#VerifyPin
      style: document
      input:
        soapBody: '#/components/schemas/de.gematik.ws.conn.CardService81/VerifyPin'
      output:
        soapBody: '#/components/schemas/de.gematik.ws.conn.CardService81/VerifyPinResponse'
      faults:
        FaultMessage:
          soapBody: '#/components/schemas/de.gematik.ws.tel.error20/Error'
```

### `x-extends` and `x-is-base` — Flattened type hierarchy

Instead of using `allOf` composition, inherited properties are inlined directly into each type. The original inheritance chain is preserved as metadata in `x-extends` (inside the `xml` object), allowing code generators to reconstruct interfaces or marker traits if needed.

```yaml
BinaryDocumentType:
  type: object
  properties:
    id:                          # inherited from DocumentBaseType
      type: string
      xml: { name: ID, attribute: true }
    base64Data:
      $ref: '#/components/schemas/oasis.names.tc.dss10.core/Base64Data'
  xml:
    x-extends:
    - '#/components/schemas/oasis.names.tc.dss10.core/DocumentBaseType'
    x-is-base: true
```

### Schema organization — XSD complex type to OpenAPI

Schemas are grouped by XML namespace under `components/schemas`. Each namespace becomes a key, and its types are nested beneath it. The original type hierarchy from WSDL/XSD is preserved and namespaces are mapped to component paths.

**Input XSD**
```xml
<complexType name="ContextType">
  <sequence>
    <element ref="CONN:MandantId">
      <annotation>
        <documentation>Die ID des Mandanten.</documentation>
      </annotation>
    </element>
    <element ref="CONN:ClientSystemId">
      <annotation>
        <documentation>Die ID des Clientsystems, von dem bzw. für das der Aufruf
        des Konnektors erfolgt.</documentation>
      </annotation>
    </element>
    <element ref="CONN:WorkplaceId">
      <annotation>
        <documentation>Die ID des Arbeitsplatzes, von dem bzw. für den der Aufruf
        des Konnektors erfolgt.</documentation>
      </annotation>
    </element>
    <element ref="CONN:UserId" minOccurs="0">
      <annotation>
        <documentation>Die ID des Nutzers im Primärsystem.</documentation>
      </annotation>
    </element>
  </sequence>
</complexType>
<element name="Context" type="CCTX:ContextType"/>
```

**Output OpenAPI (YAML)**
```yaml
de.gematik.ws.conn.ConnectorContext20:
  Context:
    $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorContext20/ContextType'
    xml:
      name: Context
      namespace: http://ws.gematik.de/conn/ConnectorContext/v2.0
  ContextType:
    type: object
    properties:
      mandantId:
        $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorCommon50/MandantId'
      clientSystemId:
        $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorCommon50/ClientSystemId'
      workplaceId:
        $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorCommon50/WorkplaceId'
      userId:
        $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorCommon50/UserId'
        nullable: true
```

## XSD to JSON Schema type mapping

| XSD Type               | JSON Schema Type                    |
|------------------------|-------------------------------------|
| `xsd:string`           | `type: string`                      |
| `xsd:boolean`          | `type: boolean`                     |
| `xsd:integer`          | `type: integer`                     |
| `xsd:int`              | `type: integer`, `format: int32`    |
| `xsd:long`             | `type: integer`, `format: int64`    |
| `xsd:decimal`          | `type: number`                      |
| `xsd:float`            | `type: number`, `format: float`     |
| `xsd:double`           | `type: number`, `format: double`    |
| `xsd:date`             | `type: string`, `format: date`      |
| `xsd:dateTime`         | `type: string`, `format: date-time` |
| `xsd:time`             | `type: string`, `format: time`      |
| `xsd:base64Binary`     | `type: string`, `format: byte`      |
| `xsd:hexBinary`        | `type: string`, `format: hex`       |
| `xsd:anyURI`           | `type: string`, `format: uri`       |
| `xsd:complexType`      | `type: object` with `properties`    |
| `xsd:anyType`          | `type: string`, `format: xml`       |

## Usage

### Python converter (WSDL/XSD to OpenAPI)

Requires Python 3.13+ and [uv](https://docs.astral.sh/uv/).

```bash
uv sync
uv run python konnektor-opb6.py
```

This fetches Konnektor WSDLs and produces `konnektor-opb6.json` and `konnektor-opb6.yaml`.

Programmatic usage:

```python
from wsdl2openapi.builder import Builder
from wsdl2openapi.model import to_json, to_yaml

builder = Builder()
builder.api.info.title = "My Service"
builder.api.info.version = "1.0"

builder.add_wsdl("https://example.com/service.wsdl")

print(to_yaml(builder.api))
```

### Go code generator (OpenAPI to Go)

The `generator-golang/` directory contains a code generator that reads the OpenAPI output and produces type-safe Go structs with XML tags and SOAP envelope handling.

```bash
cd generator-golang
go run ./cmd/wsdl2openapi2go \
  --file ../konnektor-opb6.json \
  --output ./testproj/kon/api \
  --naming naming-kon.json
```

## Running tests

```bash
# Python tests
uv run pytest tests/

# Go generator + integration tests
cd generator-golang && go test ./...
```
