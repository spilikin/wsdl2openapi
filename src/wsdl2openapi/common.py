import re
import urllib
from abc import ABC, abstractmethod
from dataclasses import dataclass

from lxml import etree
from lxml.etree import QName
from zeep.transports import Transport


class Context:
    use_connector_conventions: bool = False
    connector_service_version: str | None = None
    connector_service_types_namespace: str | None = None
    connector_service_namespace: str | None = None


class NamespaceNamingStrategy(ABC):
    @abstractmethod
    def namespace_identifier(self, ctx: Context, target_namespace: str) -> str | None:
        pass


class DefaultNamespaceNaming(NamespaceNamingStrategy):
    def namespace_identifier(self, ctx: Context, target_namespace: str) -> str:
        if not target_namespace:
            return "default"

        parts = []

        if target_namespace.startswith("http"):
            # R1 inverse fqdn in URLs
            url = urllib.parse.urlparse(target_namespace)
            reversed_fqdn_parts = list(reversed(url.netloc.split(".")))
            # R2 add all paths as list
            parts = reversed_fqdn_parts + url.path.split("/")
        elif target_namespace.startswith("urn:"):
            # R3 split urn parts by ":"
            parts = target_namespace.split(":")[1:]
        else:
            parts = [target_namespace]

        # R4 remove trivial parts
        trivial_parts = {"www", "WSDL", "schema", "uri", "xsd"}
        parts = list(
            filter(lambda p: re.sub(r"[^\w]", "", p) not in trivial_parts, parts)
        )

        # R6 is connector convention, replace version in last part with connector service version
        if (
            ctx.use_connector_conventions
            and ctx.connector_service_version is not None
            and (
                target_namespace == ctx.connector_service_namespace
                or target_namespace == ctx.connector_service_types_namespace
            )
        ):
            parts[-1] = "v" + ctx.connector_service_version.replace(".", "")

        # R7 if any part matches the version pattern, remove "v" and dots and append to previous part
        version_pattern = re.compile(r"^v?\d+(\.\d+)*$")
        for i in range(1, len(parts)):
            if version_pattern.match(parts[i]) is not None:
                version = parts[i]
                parts[i - 1] = parts[i - 1] + version.replace("v", "").replace(".", "")
                parts[i] = ""
        parts = [part for part in parts if part]

        # R8 remove punctuation characters or underscores
        parts = [re.sub(r"[^\w]", "", part) for part in parts]
        parts = [part.replace("_", "") for part in parts]

        # R9 is part starts with digit, append it to the previous part
        i = 1
        while i < len(parts):
            if parts[i] and parts[i][0].isdigit():
                parts[i - 1] = parts[i - 1] + parts[i]
                parts.pop(i)
            else:
                i += 1
        parts = [part for part in parts if part]
        # R9.1 if part 0 starts with digit, prepend "n"
        if parts[0][0].isdigit():
            parts[0] = "n" + parts[0]

        return ".".join(parts)


@dataclass
class NamingStrategy:
    namespace_naming: list[NamespaceNamingStrategy] = (DefaultNamespaceNaming(),)

    def namespace_identifier(self, ctx: Context, target_namespace: str) -> str:
        for strategy in self.namespace_naming:
            id = strategy.namespace_identifier(ctx, target_namespace)
            return id
        return "default"

    def char_data_property_name(self) -> str:
        """Return the property name for inner XML content."""
        return "charData"

    def format_property_name(self, name: str) -> str:
        """Convert a name to a valid json property name lower camelCase convention."""
        if name.isupper():
            return name.lower()
        parts = name.replace("-", "_").split("_")
        if len(parts) == 1:
            new_name = parts[0]
        else:
            new_name = "".join(part.capitalize() for part in parts)
        new_name = new_name[0].lower() + new_name[1:]  # Lowercase the first character
        return new_name

    def format_type_name(self, name: str) -> str:
        """Convert a name to a valid type name UpperCamelCase convention."""
        parts = name.replace("-", "_").split("_")
        new_name = "".join(part[0].upper() + part[1:] if part else "" for part in parts)
        return new_name

    def reference_to_type(self, ctx: Context, qname: QName) -> str:
        """Return the schema reference for a given QName."""
        id = self.namespace_identifier(ctx, qname.namespace)

        return (
            "#/components/schemas/" + id + "/" + self.format_type_name(qname.localname)
        )

    def unknown_content_property_name(self) -> str:
        """Return the property name for unknown XML attributes or elements."""
        return "unknownContent"

    def unknown_attribute_property_name(self) -> str:
        """Return the property name for unknown XML attributes."""
        return "unknownAttributes"


_xml_cache = dict()


def load_xml_document(url: str, transport: Transport, use_cache: bool = True):
    """Load an XML document from a URL.
    :param url: The URL of the XML document.
    :param transport: The transport object to use for loading the document.
    :param use_cache: Whether to use a cache for the XML document.
    :return: The parsed XML document as an lxml Element.
    """
    if use_cache and url in _xml_cache:
        return _xml_cache[url]

    content = transport.load(url)
    etree_parser = etree.XMLParser(remove_blank_text=True)
    document = etree.fromstring(content, parser=etree_parser)

    if use_cache:
        _xml_cache[url] = document

    return document
