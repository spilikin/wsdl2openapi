import os

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import (
    ReferenceObject,
    TypeBoolean,
    TypeInteger,
    TypeObject,
    TypeString,
    to_yaml,
)


def test_simple_types():
    builder = Builder()

    base_dir = os.path.dirname(__file__)
    builder.add_xsd(os.path.join(base_dir, "test_simple_types.xsd"))
    print(to_yaml(builder.api))

    assert "org.example.schema" in builder.api.components.schemas
    components = builder.api.components.schemas["org.example.schema"]
    assert "StringType" in components
    string_type = components["StringType"]
    assert isinstance(string_type, TypeString)

    assert "BooleanType" in components
    boolean_type = components["BooleanType"]
    assert isinstance(boolean_type, TypeBoolean)

    assert "IntegerType" in components
    integer_type = components["IntegerType"]
    assert isinstance(integer_type, TypeInteger)

    assert "NonNegativeIntegerType" in components
    non_negative_integer_type = components["NonNegativeIntegerType"]
    assert isinstance(non_negative_integer_type, TypeInteger)
    assert non_negative_integer_type.minimum == 0

    assert "NegativeIntegerType" in components
    negative_integer_type = components["NegativeIntegerType"]
    assert isinstance(negative_integer_type, TypeInteger)
    assert negative_integer_type.maximum == -1

    date_time_value = components["DateTimeRefValue"]
    assert isinstance(date_time_value, ReferenceObject)

    assert "Values" in components
    values = components["Values"]
    assert isinstance(values, TypeObject)

    string_property = values.properties["string"]
    assert isinstance(string_property, TypeString)

    boolean_property = values.properties["boolean"]
    assert isinstance(boolean_property, TypeBoolean)

    assert "ValuesRef" in components
    values_ref = components["ValuesRef"]
    assert isinstance(values_ref, TypeObject)
    assert "string" in values_ref.properties
    string_property = values_ref.properties["string"]
    assert isinstance(string_property, ReferenceObject)
    assert string_property.ref == "#/components/schemas/org.example.schema/StringType"
    assert "http://example.org/schema" == string_property.xml.namespace
