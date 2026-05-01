import msgspec

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import (
    ReferenceObject,
    TypeObject,
    TypeString,
    XmlExtension,
)


def test_synthesize_descriptions_uses_namespace_uri_when_known():
    """Schemas with xml.namespace recoverable get a description carrying the URI."""
    builder = Builder()
    schemas = builder.api.components.schemas

    schemas["urn.example.foo.Widget"] = TypeObject(
        xml=XmlExtension(name="Widget", namespace="urn:example:foo"),
    )
    schemas["urn.example.foo.WidgetRef"] = ReferenceObject(
        ref="#/components/schemas/urn.example.foo.Widget",
        xml=XmlExtension(name="Widget", namespace="urn:example:foo"),
    )

    builder.synthesize_descriptions()

    widget = schemas["urn.example.foo.Widget"]
    assert "urn:example:foo" in widget.description
    assert "Widget" in widget.description

    ref = schemas["urn.example.foo.WidgetRef"]
    assert "urn:example:foo" in ref.description


def test_synthesize_descriptions_recovers_uri_via_property_when_top_level_lacks_xml():
    """Even when the schema itself has no xml.namespace, synthesis picks up the
    URI from any property whose xml metadata carries it."""
    builder = Builder()
    schemas = builder.api.components.schemas

    inner = TypeString(xml=XmlExtension(name="name", namespace="urn:example:bar"))
    schemas["urn.example.bar.Holder"] = TypeObject(
        properties={"name": inner},
    )

    builder.synthesize_descriptions()

    assert "urn:example:bar" in schemas["urn.example.bar.Holder"].description


def test_synthesize_descriptions_falls_back_to_package_when_no_uri_known():
    """If no xml.namespace is reachable, fall back to the package name."""
    builder = Builder()
    schemas = builder.api.components.schemas

    schemas["urn.example.unknown.Bare"] = TypeObject()

    builder.synthesize_descriptions()

    desc = schemas["urn.example.unknown.Bare"].description
    assert "urn.example.unknown" in desc
    assert "Bare" in desc


def test_synthesize_descriptions_preserves_existing():
    """An already-set description must not be overwritten."""
    builder = Builder()
    schemas = builder.api.components.schemas

    schemas["urn.example.foo.Widget"] = TypeObject(
        description="Hand-written docs.",
        xml=XmlExtension(name="Widget", namespace="urn:example:foo"),
    )

    builder.synthesize_descriptions()

    assert schemas["urn.example.foo.Widget"].description == "Hand-written docs."


def test_synthesize_descriptions_idempotent():
    """Running twice doesn't change anything after the first pass."""
    builder = Builder()
    schemas = builder.api.components.schemas
    schemas["urn.example.foo.Widget"] = TypeObject(
        xml=XmlExtension(name="Widget", namespace="urn:example:foo"),
    )

    builder.synthesize_descriptions()
    first = schemas["urn.example.foo.Widget"].description
    builder.synthesize_descriptions()
    assert schemas["urn.example.foo.Widget"].description == first
    assert first is not msgspec.UNSET
