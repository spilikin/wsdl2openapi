import logging
import re
from dataclasses import dataclass, field

import msgspec
from zeep import Client, Settings
from zeep.transports import Transport
from zeep.xsd.schema import Schema

from wsdl2openapi.schema_facets_visitor import SchemaFacetsVisitor
from wsdl2openapi.wsdl_visitor import WsdlVisitor

from .common import Context, NamingStrategy, load_xml_document
from .model import (
    Api,
    ReferenceObject,
    TypeArray,
    TypeObject,
    TypeString,
)
from .schema_visitor import XmlSchemaVisitor

soap_settings = Settings()
soap_settings.forbid_entities = False


# Upstream WSDL/XSD documents from gematik occasionally ship with bugs that
# zeep refuses to parse. We patch the raw bytes as they're loaded.
#
# CardService_v8_2_0.xsd: declares `type="dss:Base64Data"`, but in the OASIS
# DSS schema `Base64Data` is an element, not a type. Its content model is
# `xs:base64Binary` with an optional `MimeType` attribute, so substituting
# `xs:base64Binary` keeps the on-the-wire shape compatible.
_XSD_PATCHES: dict[str, list[tuple[bytes, bytes]]] = {
    "CardService_v8_2_0.xsd": [
        (b'type="dss:Base64Data"', b'type="base64Binary"'),
    ],
}


class PatchingTransport(Transport):
    def load(self, url):
        content = super().load(url)
        for needle, patches in _XSD_PATCHES.items():
            if url.endswith(needle):
                for old, new in patches:
                    content = content.replace(old, new)
                break
        return content


@dataclass
class Builder:
    api: Api = field(default_factory=Api)
    naming_strategy: NamingStrategy = field(default_factory=NamingStrategy)

    def add_wsdl(self, wsdl_location, use_connector_conventions: bool = False):
        logging.info("Adding WSDL: %s", wsdl_location)
        client = Client(
            wsdl=str(wsdl_location),
            settings=soap_settings,
            transport=PatchingTransport(),
        )

        wsdl_definition = next(iter(client.wsdl._definitions.values()), None)

        ctx = Context()

        if use_connector_conventions:
            ctx.use_connector_conventions = True
            version_match = re.search(r"_v(\d+)_(\d+)_(\d+)\.wsdl$", wsdl_location)
            if version_match:
                major, minor, patch = version_match.groups()
                ctx.connector_service_version = (
                    f"{int(major)}.{int(minor)}.{int(patch)}"
                )

            ctx.connector_service_namespace = wsdl_definition.target_namespace
            ctx.connector_service_types_namespace = (
                ctx.connector_service_namespace.replace("/WSDL/v", "/v")
            )

        logging.debug("Using context: %s", ctx)

        schema_visitor = XmlSchemaVisitor(
            naming_strategy=self.naming_strategy,
            api=self.api,
            ctx=ctx,
        )

        schema_visitor.visit_schema(client.wsdl.types)

        wsdl_visitor = WsdlVisitor(
            naming_strategy=self.naming_strategy,
            api=self.api,
            ctx=ctx,
        )

        wsdl_visitor.visit_definition(wsdl_definition)

        schema_facets_visitor = SchemaFacetsVisitor(
            api=self.api,
            ctx=ctx,
            naming_strategy=self.naming_strategy,
            transport=client.transport,
        )

        for doc in client.wsdl.types.documents.values():
            if doc._location is None:
                continue
            schema_facets_visitor.visit_schema_document(doc._location)

    def exclude_namespaces(
        self,
        namespace_uris: list[str],
        ctx: Context | None = None,
    ) -> None:
        """Drop schemas in the given XML namespaces and rewrite refs into them
        as opaque XML strings (``type: string, format: xml``). Property names,
        their xml-binding, and required-ness on parent objects are preserved —
        only the property type changes.

        Useful to cut out subgraphs that pull in large or self-referential
        external standards (SAML, etc.) when the consumer is happy to receive
        those parts as raw XML.
        """
        ctx = ctx or Context()
        excluded_ids = {
            self.naming_strategy.namespace_identifier(ctx, uri)
            for uri in namespace_uris
        }
        excluded_prefixes = tuple(f"{ns_id}." for ns_id in excluded_ids)
        ref_prefix = "#/components/schemas/"

        def is_excluded_key(key: str) -> bool:
            return key.startswith(excluded_prefixes)

        def rewrite_ref(ref_obj: ReferenceObject) -> TypeString:
            xml = (
                ref_obj.xml
                if ref_obj.xml is not msgspec.UNSET
                else msgspec.UNSET
            )
            description = (
                ref_obj.description
                if ref_obj.description is not msgspec.UNSET
                else msgspec.UNSET
            )
            replacement = TypeString(format="xml")
            if xml is not msgspec.UNSET:
                replacement.xml = xml
            if description is not msgspec.UNSET:
                replacement.description = description
            return replacement

        def rewrite(value):
            if isinstance(value, ReferenceObject):
                if value.ref.startswith(ref_prefix) and is_excluded_key(
                    value.ref[len(ref_prefix) :]
                ):
                    return rewrite_ref(value)
                return value
            if isinstance(value, TypeObject):
                for prop_name in list(value.properties.keys()):
                    value.properties[prop_name] = rewrite(value.properties[prop_name])
            elif isinstance(value, TypeArray):
                value.items = rewrite(value.items)
            return value

        # Drop excluded schemas; rewrite refs in everything else.
        schemas = self.api.components.schemas
        kept = type(schemas)()
        for key, val in schemas.items():
            if is_excluded_key(key):
                continue
            kept[key] = rewrite(val)
        self.api.components.schemas = kept

        # Warn if any operation message body now points to an excluded schema —
        # that operation would dangle. (Not a concern for the SAML case today,
        # but useful diagnostic for other namespaces.)
        if self.api.wsdl_services and self.api.wsdl_services is not msgspec.UNSET:
            for service in self.api.wsdl_services:
                for port in service.ports:
                    for op in port.operations:
                        for label, message in (
                            ("input", op.input),
                            ("output", op.output),
                        ):
                            body = getattr(message, "soapBody", "") or ""
                            if body.startswith(ref_prefix) and is_excluded_key(
                                body[len(ref_prefix) :]
                            ):
                                logging.warning(
                                    "Operation %s.%s %s body refers to excluded namespace: %s",
                                    service.name,
                                    op.name,
                                    label,
                                    body,
                                )

    def add_xsd(self, xsd_location):
        logging.info("Adding XSD: %s", xsd_location)
        transport = PatchingTransport()
        xsd_tree = load_xml_document(xsd_location, transport)
        schema = Schema(transport=transport)
        schema.add_documents([xsd_tree], xsd_location)
        logging.debug("Parsed schema: %s", schema)
        ctx = Context()
        schema_visitor = XmlSchemaVisitor(
            naming_strategy=self.naming_strategy,
            api=self.api,
            ctx=ctx,
        )
        schema_visitor.visit_schema(schema)

        schema_facets_visitor = SchemaFacetsVisitor(
            api=self.api,
            ctx=ctx,
            naming_strategy=self.naming_strategy,
            transport=transport,
        )
        schema_facets_visitor.visit_schema_document(xsd_location)
