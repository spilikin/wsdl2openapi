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
    # print(to_yaml(builder.api))

    schemas = builder.api.components.schemas
    assert "org.example.simple.StringType" in schemas
    string_type = schemas["org.example.simple.StringType"]
    assert isinstance(string_type, TypeString)

    assert "org.example.simple.BooleanType" in schemas
    boolean_type = schemas["org.example.simple.BooleanType"]
    assert isinstance(boolean_type, TypeBoolean)

    assert "org.example.simple.IntegerType" in schemas
    integer_type = schemas["org.example.simple.IntegerType"]
    assert isinstance(integer_type, TypeInteger)

    assert "org.example.simple.NonNegativeIntegerType" in schemas
    non_negative_integer_type = schemas["org.example.simple.NonNegativeIntegerType"]
    assert isinstance(non_negative_integer_type, TypeInteger)
    assert non_negative_integer_type.minimum == 0

    assert "org.example.simple.NegativeIntegerType" in schemas
    negative_integer_type = schemas["org.example.simple.NegativeIntegerType"]
    assert isinstance(negative_integer_type, TypeInteger)
    assert negative_integer_type.maximum == -1

    date_time_value = schemas["org.example.simple.DateTimeRefValue"]
    assert isinstance(date_time_value, ReferenceObject)

    assert "org.example.simple.Values" in schemas
    values = schemas["org.example.simple.Values"]
    assert isinstance(values, TypeObject)

    string_property = values.properties["string"]
    assert isinstance(string_property, TypeString)

    boolean_property = values.properties["boolean"]
    assert isinstance(boolean_property, TypeBoolean)

    assert "org.example.simple.ValuesRef" in schemas
    values_ref = schemas["org.example.simple.ValuesRef"]
    assert isinstance(values_ref, TypeObject)
    assert "string" in values_ref.properties
    string_property = values_ref.properties["string"]
    assert isinstance(string_property, ReferenceObject)
    assert string_property.ref == "#/components/schemas/org.example.simple.StringType"
    assert "http://example.org/simple" == string_property.xml.namespace

    access_header_element = schemas["org.example.simple.AccessHeaderElement"]
    assert isinstance(access_header_element, TypeObject)
    assert "code" in access_header_element.properties
