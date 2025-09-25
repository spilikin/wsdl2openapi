import os

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import to_yaml


def test_soap_operations():
    builder = Builder()

    base_dir = os.path.dirname(__file__)
    builder.add_wsdl(
        os.path.join(base_dir, "test_soap.wsdl"), use_connector_conventions=False
    )
    print()
    print(to_yaml(builder.api))

    # assert len(builder.api.soap_operations) == 2
    # operation_names = {op.name for op in builder.api.soap_operations}
    # assert "GetCityWeatherByZIP" in operation_names
    # assert "GetCityForecastByZIP" in operation_names
