import os

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import to_yaml


def test_facets_from_xsd():
    builder = Builder()

    base_dir = os.path.dirname(__file__)
    builder.add_xsd(os.path.join(base_dir, "test_facets.xsd"))
    print(to_yaml(builder.api))


def test_facets_from_wsdl():
    builder = Builder()

    base_dir = os.path.dirname(__file__)
    builder.add_wsdl(os.path.join(base_dir, "test_facets.wsdl"))
    print(to_yaml(builder.api))
