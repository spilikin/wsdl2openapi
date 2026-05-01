import logging
import os

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import (
    ReferenceObject,
    TypeArray,
    TypeObject,
    to_yaml,
)

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def test_complex_types():
    builder = Builder()

    base_dir = os.path.dirname(__file__)
    builder.add_xsd(os.path.join(base_dir, "test_complex_types.xsd"))
    print(to_yaml(builder.api))

    schemas = builder.api.components.schemas
    complex_type = schemas["org.example.complextypes.ComplexType"]
    assert isinstance(complex_type, TypeObject)
    assert "elementNested" in complex_type.properties
    nested_type_ref = complex_type.properties["elementNested"]
    assert isinstance(nested_type_ref, ReferenceObject)

    element_int_list = complex_type.properties["elementIntList"]
    assert isinstance(element_int_list, TypeArray)
