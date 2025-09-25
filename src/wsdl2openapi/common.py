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
        """Return the unique identifier for a given target namespace to be used in the OpenAPI document.
        By default the following rules are applied:
        - if namespace is a URL
            - the host FQDN is inverted (e.g. gematik.de becomes de.gematik)
            - remove non-informative parts like "www"
            - in the path / is replaced by . and concatenated with the inverted FQDN
            - if path element contains "." replace it with underscore _
        - if namespace is URN
            - split into urn parts by ":"
            - replace "." in each urn part by underscore _
        - if last path element matches the semantic version (with or without v, the version is appended to the previous part with underscore separator
        - if the target namespace matches the connector service or types namespace, the version is replaced with the connector service version
        - if connector conventions are used:
            - the version is taken from the connector service version including patch version
            - WSDL and Types namespaces get the same identifiers (WSDL is removed from the identifier)
        """

        result = None

        if not target_namespace:
            return "default"

        if target_namespace.startswith("http"):
            url = urllib.parse.urlparse(target_namespace)
            reversed_parts = reversed(url.netloc.split("."))
            reversed_parts = (part for part in reversed_parts if part.lower() != "www")
            inverted_fqdn = ".".join(reversed_parts)

            path_elements = url.path.split("/")
            path = ".".join(elem.replace(".", "_") for elem in path_elements if elem)
            if path:
                result = f"{inverted_fqdn}.{path}"
            else:
                result = inverted_fqdn
        elif target_namespace.startswith("urn:"):
            urn_parts = target_namespace.split(":")[1:]
            result = ".".join(part.replace(".", "_") for part in urn_parts if part)
        else:
            result = target_namespace.replace("/", ".").replace(":", ".")

        # Check if the last path element matches a semantic version pattern
        version_pattern = re.compile(r"(?:\.|^)(v?\d+(_\d+)*)(?:\.|$)$")
        if version_pattern.search(result):
            match = version_pattern.search(result)
            if match:
                version = match.group(1)
                result = result.replace(f".{version}", f"_{version}")

        if ctx.use_connector_conventions:
            result = result.replace(".WSDL", "")
            # If the target namespace matches the connector service or types namespace, replace the version with the connector service version
            if ctx.connector_service_version is not None and (
                target_namespace == ctx.connector_service_namespace
                or target_namespace == ctx.connector_service_types_namespace
            ):
                result = result.replace(
                    f"_{version}",
                    f"_v{ctx.connector_service_version.replace('.', '_')}",
                )
            # Remove "WSDL" from the identifier if present

        return result


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
