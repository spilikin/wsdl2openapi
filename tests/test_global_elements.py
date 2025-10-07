import logging
import os

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import ReferenceObject, TypeObject, TypeString, to_yaml

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def test_global_elements():
    builder = Builder()

    base_dir = os.path.dirname(__file__)
    builder.add_xsd(os.path.join(base_dir, "test_global_elements.xsd"))
    print(to_yaml(builder.api))

    assert "org.example.globalelements" in builder.api.components.schemas
    assert "Value1" in builder.api.components.schemas["org.example.globalelements"]
    value1 = builder.api.components.schemas["org.example.globalelements"]["Value1"]
    assert isinstance(value1, ReferenceObject)
    assert (
        value1.ref
        == "#/components/schemas/org.example.globalelements/Value1ElementType"
    )

    assert "Value2" in builder.api.components.schemas["org.example.globalelements"]
    value2 = builder.api.components.schemas["org.example.globalelements"]["Value2"]
    assert isinstance(value2, ReferenceObject)
    assert value2.ref == "#/components/schemas/org.example.globalelements/Value2Type"

    assert "Value3" in builder.api.components.schemas["org.example.globalelements"]
    value3 = builder.api.components.schemas["org.example.globalelements"]["Value3"]
    assert isinstance(value3, TypeObject)
    assert len(value3.properties) == 1
    assert "value3" in value3.properties
    assert isinstance(value3.properties["value3"], TypeString)

    value6 = builder.api.components.schemas["org.example.globalelements"]["Value6"]
    assert isinstance(value6, TypeString)
    assert value6.format == "xml"
