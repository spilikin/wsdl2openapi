import logging

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import to_json, to_yaml

logging.basicConfig(level=logging.INFO)
logging.getLogger("wsdl_parser").setLevel(logging.DEBUG)
logging.getLogger("zeep").setLevel(logging.ERROR)

builder = Builder()

builder.api.info.title = "ePA XDS Document Service"
builder.api.info.version = "3.0.5"
builder.api.info.description = "Conversion of the WSDL to OpenAPI 3.1"

builder.add_wsdl(
    "https://raw.githubusercontent.com/gematik/ePA-XDS-Document/refs/heads/ePA-3.0.5/src/schema/XDSDocumentService.wsdl",
    use_connector_conventions=False,
)

with open("XDSDocumentService.yaml", "w") as file:
    file.write(to_yaml(builder.api))

with open("XDSDocumentService.json", "w") as file:
    file.write(to_json(builder.api, indent=2))
