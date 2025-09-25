import logging
import re
from dataclasses import dataclass, field

from zeep import Client, Settings
from zeep.transports import Transport
from zeep.xsd.schema import Schema

from wsdl2openapi.schema_facets_visitor import SchemaFacetsVisitor
from wsdl2openapi.wsdl_visitor import WsdlVisitor

from .common import Context, NamingStrategy, load_xml_document
from .model import Api
from .schema_visitor import XmlSchemaVisitor

soap_settings = Settings()
soap_settings.forbid_entities = False


@dataclass
class Builder:
    api: Api = field(default_factory=Api)
    naming_strategy: NamingStrategy = field(default_factory=NamingStrategy)

    def add_wsdl(self, wsdl_location, use_connector_conventions: bool = False):
        logging.info("Adding WSDL: %s", wsdl_location)
        client = Client(
            wsdl=str(wsdl_location),
            settings=soap_settings,
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

    def add_xsd(self, xsd_location):
        logging.info("Adding XSD: %s", xsd_location)
        transport = Transport()
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
