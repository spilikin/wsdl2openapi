import logging

from wsdl2openapi.builder import Builder
from wsdl2openapi.model import to_json, to_yaml

logging.basicConfig(level=logging.INFO)
logging.getLogger("wsdl_parser").setLevel(logging.DEBUG)
logging.getLogger("zeep").setLevel(logging.ERROR)

builder = Builder()
builder.add_wsdl(
    # "https://raw.githubusercontent.com/gematik/api-telematik/refs/heads/OPB5/conn/CardService_v8_1_2.wsdl",
    # "https://raw.githubusercontent.com/gematik/ePA-XDS-Document/refs/heads/ePA-3.0.5/src/schema/XDSDocumentService.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/refs/heads/OPB5/conn/EventService.wsdl",
    use_connector_conventions=True,
)

builder.add_wsdl(
    "https://raw.githubusercontent.com/gematik/api-telematik/refs/heads/publishInternalRelease-43/conn/CardService_v8_1_1.wsdl",
    use_connector_conventions=True,
)

builder.add_wsdl(
    "https://raw.githubusercontent.com/gematik/api-telematik/refs/heads/publishInternalRelease-43/conn/CardService_v8_2_1.wsdl",
    use_connector_conventions=True,
)

builder.add_xsd(
    "https://raw.githubusercontent.com/gematik/api-telematik/refs/heads/publishInternalRelease-43/conn/ServiceDirectory.xsd"
)
builder.api.info.title = "Connector API"
builder.api.info.version = "OPB5"
builder.api.info.description = "API for the Connector PTV6"

# print(to_yaml(builder.api))

with open("output.yaml", "w") as file:
    file.write(to_yaml(builder.api))

with open("output.json", "w") as file:
    file.write(to_json(builder.api, indent=2))

exit(0)
