from collections import OrderedDict

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import (
    ReferenceObject,
    TypeArray,
    TypeObject,
    TypeString,
    XmlExtension,
)


def test_exclude_namespace_rewrites_refs_and_drops_schemas():
    """Refs into an excluded namespace become opaque XML; the namespace's own
    schemas vanish; xml-binding and required-ness on the parent are preserved.
    """
    builder = Builder()
    schemas = builder.api.components.schemas

    # Excluded namespace: two schemas under it.
    schemas["example.saml.NameID"] = TypeString(
        xml=XmlExtension(name="NameID", namespace="urn:example:saml"),
    )
    schemas["example.saml.Assertion"] = TypeObject(
        xml=XmlExtension(name="Assertion", namespace="urn:example:saml"),
    )

    # Kept namespace with one type that references both excluded schemas.
    parent_props = OrderedDict()
    parent_props["x509Data"] = ReferenceObject(
        ref="#/components/schemas/example.kept.X509Data",
    )
    parent_props["samlIdentifier"] = ReferenceObject(
        ref="#/components/schemas/example.saml.NameID",
        xml=XmlExtension(name="SAMLIdentifier", namespace="urn:example:dssx"),
    )
    parent_props["assertions"] = TypeArray(
        items=ReferenceObject(
            ref="#/components/schemas/example.saml.Assertion",
        ),
    )
    schemas["example.dssx.IdentifierType"] = TypeObject(
        properties=parent_props,
        required=["x509Data"],  # samlIdentifier is optional
    )

    schemas["example.kept.X509Data"] = TypeString()

    builder.exclude_namespaces(["urn:example:saml"])

    schemas = builder.api.components.schemas

    # Excluded namespace schemas dropped.
    assert "example.saml.NameID" not in schemas
    assert "example.saml.Assertion" not in schemas

    # Kept namespace untouched.
    assert "example.kept.X509Data" in schemas

    # Parent type still has all three properties — none removed.
    parent = schemas["example.dssx.IdentifierType"]
    assert isinstance(parent, TypeObject)
    assert list(parent.properties.keys()) == ["x509Data", "samlIdentifier", "assertions"]

    # x509Data: untouched ref.
    assert isinstance(parent.properties["x509Data"], ReferenceObject)

    # samlIdentifier: rewritten to opaque XML, xml-binding preserved.
    saml_ident = parent.properties["samlIdentifier"]
    assert isinstance(saml_ident, TypeString)
    assert saml_ident.format == "xml"
    assert saml_ident.xml.name == "SAMLIdentifier"
    assert saml_ident.xml.namespace == "urn:example:dssx"

    # assertions: array items rewritten to opaque XML.
    arr = parent.properties["assertions"]
    assert isinstance(arr, TypeArray)
    assert isinstance(arr.items, TypeString)
    assert arr.items.format == "xml"

    # Required list preserved exactly.
    assert parent.required == ["x509Data"]
