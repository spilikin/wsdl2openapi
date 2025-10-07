import logging

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import to_json, to_yaml

logging.basicConfig(level=logging.INFO)
logging.getLogger("wsdl_parser").setLevel(logging.DEBUG)
logging.getLogger("zeep").setLevel(logging.ERROR)

wsdl_list = [
    "https://raw.githubusercontent.com/gematik/api-telematik/refs/heads/publishInternalRelease-43/conn/EventService.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/refs/heads/publishInternalRelease-43/conn/AuthSignatureService_v7_4_1.wsdl",
]
builder = Builder()

builder.api.info.title = "Konnektor OPB6"
builder.api.info.version = "OPB6"
builder.api.info.description = "Conversion of the WSDL to OpenAPI 3.1"

for wsdl in wsdl_list:
    builder.add_wsdl(wsdl, use_connector_conventions=True)

with open("Konnektor-OPB6.yaml", "w") as file:
    file.write(to_yaml(builder.api))

with open("Konnektor-OPB6.json", "w") as file:
    file.write(to_json(builder.api, indent=2))
