import decimal
import logging
from dataclasses import dataclass

import msgspec
from lxml import etree
from lxml.etree import QName
from zeep.transports import Transport

from wsdl2openapi.model import (
    Api,
    Type,
    TypeArray,
    TypeNumber,
    TypeObject,
    TypeString,
    schema_key,
)

from .common import Context, NamingStrategy

xml_cache = dict()


@dataclass
class SchemaFacetsVisitor:
    api: Api
    ctx: Context
    naming_strategy: NamingStrategy
    transport: Transport

    def visit_schema_document(self, location: str):
        xsd_document = load_xml_document(location, self.transport)
        target_namespace = determine_target_namespace(xsd_document)
        logging.debug(
            f"Visiting schema document: {location} with target namespace {target_namespace}"
        )
        # find all elements called simpleType which contain the restriction child element
        for restriction in xsd_document.findall(
            ".//{http://www.w3.org/2001/XMLSchema}simpleType/{http://www.w3.org/2001/XMLSchema}restriction"
        ):
            simple_type = restriction.find("..")
            qname = determine_simple_type(target_namespace, simple_type)
            if qname is None:
                logging.debug(
                    "Found simpleType without name, skipping: %s",
                    etree.tostring(simple_type, pretty_print=True).decode("utf-8"),
                )
                continue
            type_definition = self.find_target_type(target_namespace, qname.localname)
            if type_definition is None:
                logging.warning(
                    "Type %s not found in API components, cannot add facets.",
                    qname,
                )
                continue

            for facet in restriction:
                self.visit_restriction(facet, type_definition)

    def visit_restriction(self, restriction: etree.Element, type_definition: Type):
        tag = str(restriction.tag)

        if tag.endswith("minInclusive") and isinstance(type_definition, TypeNumber):
            type_definition.minimum = float(restriction.get("value"))
        elif tag.endswith("maxInclusive") and isinstance(type_definition, TypeNumber):
            type_definition.maximum = float(restriction.get("value"))
        elif tag.endswith("minExclusive") and isinstance(type_definition, TypeNumber):
            type_definition.exclusiveMinimum = float(restriction.get("value"))
        elif tag.endswith("maxExclusive") and isinstance(type_definition, TypeNumber):
            type_definition.exclusiveMaximum = float(restriction.get("value"))
        elif tag.endswith("length") and isinstance(type_definition, TypeString):
            type_definition.maxLength = int(restriction.get("value"))
            type_definition.minLength = int(restriction.get("value"))
        elif tag.endswith("minLength") and isinstance(type_definition, TypeString):
            type_definition.minLength = int(restriction.get("value"))
        elif tag.endswith("maxLength") and isinstance(type_definition, TypeString):
            type_definition.maxLength = int(restriction.get("value"))
        elif tag.endswith("enumeration"):
            self.visit_enumeration(restriction, type_definition)
        elif tag.endswith("pattern"):
            self.visit_pattern(restriction, type_definition)

    def visit_enumeration(self, enumeration: etree.Element, type_definition: Type):
        if type_definition.enum == msgspec.UNSET:
            type_definition.enum = []
        value = enumeration.get("value")
        if value not in type_definition.enum:
            type_definition.enum.append(value)

    def visit_pattern(self, pattern: etree.Element, type_definition: TypeString):
        pattern_value = pattern.get("value")
        if type_definition.pattern == msgspec.UNSET:
            type_definition.pattern = pattern_value
        else:
            type_definition.pattern += f"|{pattern_value}"

    def find_target_type(self, target_namespace: str, localname: str) -> Type | None:
        namespace_id = self.naming_strategy.namespace_identifier(
            self.ctx, target_namespace
        )
        type_name = self.naming_strategy.format_type_name(localname)
        key = schema_key(namespace_id, type_name)
        if key in self.api.components.schemas:
            return self.api.components.schemas[key]

        # if type not found in global types, traverse properties of all types to find it
        for t in self.api.components.schemas.values():
            found = find_property(t, QName(f"{{{target_namespace}}}{localname}"))
            if found is not None:
                return found

        return None


def load_xml_document(url: str, transport: Transport, use_cache: bool = True):
    """Load an XML document from a URL.
    :param url: The URL of the XML document.
    :param transport: The transport object to use for loading the document.
    :param use_cache: Whether to use a cache for the XML document.
    :return: The parsed XML document as an lxml Element.
    """
    if use_cache and url in xml_cache:
        return xml_cache[url]

    content = transport.load(url)
    etree_parser = etree.XMLParser(remove_blank_text=True)
    document = etree.fromstring(content, parser=etree_parser)

    if use_cache:
        xml_cache[url] = document

    return document


def determine_target_namespace(xml_tree: etree.Element) -> str:
    """Determine the target namespace of the XML tree."""
    if xml_tree.tag.endswith("schema"):
        return xml_tree.get("targetNamespace", "")
    elif xml_tree.tag.endswith("definitions"):
        types = xml_tree.find("{http://schemas.xmlsoap.org/wsdl/}types")
        if types is not None:
            schema = types.find("{http://www.w3.org/2001/XMLSchema}schema")
            if schema is not None:
                return schema.get("targetNamespace", "")
        return xml_tree.get("targetNamespace", "")
    return ""


def determine_simple_type(target_namespace: str, simple_type: any) -> QName:
    """Determine the QName of a simple type based on its target namespace and name."""
    name = simple_type.get("name")
    if name is not None:
        return QName(f"{{{target_namespace}}}{name}")
    # see if simple type is defined inline for the element
    el = simple_type.find("..")
    if el is not None and el.tag.endswith("element"):
        el_name = el.get("name")
        if el_name is not None:
            return QName(f"{{{target_namespace}}}{el_name}")

    return None


def find_property(t: Type, qname: QName) -> Type | None:
    if (
        t.xml != msgspec.UNSET
        and t.xml.name == qname.localname
        and t.xml.namespace == qname.namespace
    ):
        return t
    if isinstance(t, TypeObject):
        for property_type in t.properties.values():
            found = find_property(property_type, qname)
            if found is not None:
                return found
    elif isinstance(t, TypeArray):
        found = find_property(t.items, qname)
        if found is not None:
            return found

    return None
