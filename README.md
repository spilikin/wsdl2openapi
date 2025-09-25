# 🌀 WSDL → OpenAPI Converter

> Convert legacy SOAP/WSDL services into modern, language-agnostic OpenAPI definitions.


## What it does

This tool takes WSDL and XSD files as input and produces an OpenAPI document. The schema definitions are converted to OpenAPI / JSON Schema, and the types are annotated with XML as described here [https://swagger.io/docs/specification/v3_0/data-models/representing-xml/](https://swagger.io/docs/specification/v3_0/data-models/representing-xml/). The web services are mapped as close as possible to the OpenAPI operations.

## Why it exists

There are practically no tools to work with WSDL and SOAP for modern languages like Golang or Rust. Especially not for complex services and datatypes used in enterprise applications or in German telematics infrastructure. By converting WSDL to OpenAPI, an intermediate format is created that is closer to the semantics of programming languages. From this format we can generate type-safe code with much less abstraction than, for example, Java Apache CXF. All you need is an XML parser, a serializer, and an HTTP client. For instance, in Golang all required functionality is available in the standard library.

## Features

- Convert XML Schema definitions to OpenAPI JSOn Schema superset incl. XML annotations
- Convert WSDL definitions into proprietary extension incl. SOAP binding of input, output and faults
- Full namespace support
- Combination of several XSD and WSDL files into one OpenAPI definition
- Support for TI-Konnektor Naming and Versioning Convention: support several for Patch-Level versions of Services in the same OpenAPI. Especially useful for clients that support different versions of Services

## Conversion rules

| XSD Type          | JSON Schema / OpenAPI Type          |
|-------------------|-------------------------------------|
| `xsd:string`      | `type: string`                      |
| `xsd:integer`     | `type: integer`                     |
| `xsd:int`         | `type: integer`, `format: int32`    |
| `xsd:long`        | `type: integer`, `format: int64`    |
| `xsd:decimal`     | `type: number`                      |
| `xsd:float`       | `type: number`, `format: float`     |
| `xsd:double`      | `type: number`, `format: double`    |
| `xsd:boolean`     | `type: boolean`                     |
| `xsd:date`        | `type: string`, `format: date`      |
| `xsd:dateTime`    | `type: string`, `format: date-time` |
| `xsd:time`        | `type: string`, `format: time`      |
| `xsd:base64Binary`| `type: string`, `format: byte`      |
| `xsd:anyURI`      | `type: string`, `format: uri`       |
| `xsd:complexType` | `type: object` with `properties`    |

---

## Example conversion

### Complex types

Note the references to the types. The original type-hierarchy from WSDL/XSD is preserved and namespaces are replaced by the components path for each schema's namespace.

**Input XSD schema**
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
        <documentation>Die ID des Clientsystems, von dem bzw. für das der Aufruf des Konnektors erfolgt.
        Unter einem Clientsystem wird hier ein einzelnes oder eine Gruppe von Systemen verstanden,
        welche im LAN des Leistungserbringers auf die Clientsystem-Schnittstelle des Konnektors zugreifen.</documentation>
      </annotation>
    </element>
    <element ref="CONN:WorkplaceId">
      <annotation>
        <documentation>Die ID des Arbeitsplatzes, von dem bzw. für den der Aufruf des Konnektors erfolgt.
        Bei fachlichen Aufrufen ist sie immer erforderlich.</documentation>
      </annotation>
    </element>
    <element ref="CONN:UserId" minOccurs="0">
      <annotation>
        <documentation>Die ID des Nutzers im Primärsystem. Ist nur dann erforderlich, falls ein HBA verwendet wird.</documentation>
      </annotation>
    </element>
  </sequence>
</complexType>
<element name="Context" type="CCTX:ContextType"/>
```

**Output OpenAPI (YAML)**
```yaml
de.gematik.ws.conn.ConnectorContext_v2_0:
  Context:
    $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorContext_v2_0/ContextType'
    xml:
      name: Context
      namespace: http://ws.gematik.de/conn/ConnectorContext/v2.0
  ContextType:
    type: object
    properties:
      mandantId:
        $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorCommon_v5_0/MandantIdType'
        xml:
          name: MandantId
          namespace: http://ws.gematik.de/conn/ConnectorCommon/v5.0
      clientSystemId:
        $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorCommon_v5_0/ClientSystemIdType'
        xml:
          name: ClientSystemId
          namespace: http://ws.gematik.de/conn/ConnectorCommon/v5.0
      workplaceId:
        $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorCommon_v5_0/WorkplaceIdType'
        xml:
          name: WorkplaceId
          namespace: http://ws.gematik.de/conn/ConnectorCommon/v5.0
      userId:
        $ref: '#/components/schemas/de.gematik.ws.conn.ConnectorCommon_v5_0/UserIdType'
        xml:
          name: UserId
          namespace: http://ws.gematik.de/conn/ConnectorCommon/v5.0
```
