import logging

from wsdl2openapi.builder import Builder
from wsdl2openapi.common import NamingStrategy
from wsdl2openapi.model import to_json, to_yaml

logging.basicConfig(level=logging.INFO)
logging.getLogger("wsdl_parser").setLevel(logging.DEBUG)
logging.getLogger("zeep").setLevel(logging.ERROR)

wsdl_list = [
    # Authentication and Signature Services
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/AuthSignatureService.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/AuthSignatureService_v7_4_1.wsdl",
    # Card Services
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/CardService.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/CardService_v8_1_1.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/CardService_v8_1_2.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/CardService_v8_2_0.wsdl",
    # Card Terminal Service
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/CardTerminalService.wsdl",
    # Certificate Services
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/CertificateService.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/CertificateService_v6_0_1.wsdl",
    # Encryption Services
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/EncryptionService.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/EncryptionService_v6_1_1.wsdl",
    # Event Service
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/EventService.wsdl",
    # Signature Services
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/SignatureService_V7_4_3.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/SignatureService_V7_5_6.wsdl",
    "https://raw.githubusercontent.com/gematik/api-telematik/OPB5/conn/SignatureService_V7_5_7.wsdl",
]
naming = NamingStrategy(
    property_name_overrides={
        # "EContent" in the Konnektor schemas stands for "enveloped content".
        # Expand it so the JSON property name carries the meaning.
        "IncludeEContent": "includeEnvelopedContent",
        # OASIS DSS InlineXMLType: "PIs" is "Processing Instructions" plural.
        # The XSD already ships in lowerCamelCase, so we expand for clarity.
        "ignorePIs": "ignoreProcessingInstructions",
        # VPN connection status fields: "TI" = Telematikinfrastruktur,
        # "SIS" = Sicherer Internet Service. The default acronym splitter
        # yields "vpntiStatus" / "vpnsisStatus" which blur the boundaries;
        # treat each acronym as its own word.
        "VPNTIStatus": "vpnTiStatus",
        "VPNSISStatus": "vpnSisStatus",
    },
)
builder = Builder(naming_strategy=naming)

builder.api.info.title = "Konnektor OPB6"
builder.api.info.version = "OPB6"
builder.api.info.description = "Conversion of the WSDL to OpenAPI 3.1"

for wsdl in wsdl_list:
    builder.add_wsdl(wsdl, use_connector_conventions=True)

# SAML is reachable only via dssx10 VerificationReport.IdentifierType in
# SignDocument/VerifyDocument outputs. Excluding it drops 100+ self-referential
# schemas and removes the SAML cycles that cause vacuum to hang. The SAML
# identifier slots become opaque XML — consumers who need them can parse the
# inner XML themselves.
builder.exclude_namespaces([
    "urn:oasis:names:tc:SAML:1.0:assertion",
    "urn:oasis:names:tc:SAML:2.0:assertion",
])

with open("konnektor-opb6.yaml", "w") as file:
    file.write(to_yaml(builder.api))

with open("konnektor-opb6.json", "w") as file:
    file.write(to_json(builder.api, indent=2))
