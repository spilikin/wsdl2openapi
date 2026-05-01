import pytest

from wsdl2openapi.common import Context, NamingStrategy, split_identifier_words


def test_naming_namespace_id():
    strat = NamingStrategy()
    ctx = Context()

    id = strat.namespace_identifier(
        ctx, "http://ws.gematik.de/conn/EventService/WSDL/v7.2"
    )
    assert id == "de.gematik.ws.conn.EventService72"

    ctx.use_connector_conventions = True
    ctx.connector_service_namespace = "http://ws.gematik.de/conn/EventService/WSDL/v7.2"
    ctx.connector_service_types_namespace = (
        "http://ws.gematik.de/conn/EventService/v7.2"
    )
    ctx.connector_service_version = "7.2.9"
    id = strat.namespace_identifier(ctx, "http://ws.gematik.de/conn/EventService/v7.2")
    assert id == "de.gematik.ws.conn.EventService729"

    id = strat.namespace_identifier(ctx, "30.60")
    assert id == "n3060"

    id = strat.namespace_identifier(ctx, "urn:oasis:names:tc:ebxml-regrep:xsd:lcm:3.0")
    assert id == "oasis.names.tc.ebxmlregrep.lcm30"

    id = strat.namespace_identifier(ctx, "http://www.w3.org/2000/09/xmldsig#")
    assert id == "org.w3200009.xmldsig"

    id = strat.namespace_identifier(
        ctx, "http://ws.gematik.de/epa-xds-document/I_Document_Management/v1.0"
    )
    assert id == "de.gematik.ws.epaxdsdocument.IDocumentManagement10"


@pytest.mark.parametrize(
    "name,expected",
    [
        ("foo", ["foo"]),
        ("fooBar", ["foo", "Bar"]),
        ("FooBar", ["Foo", "Bar"]),
        ("CRLRefs", ["CRL", "Refs"]),
        ("MessageID", ["Message", "ID"]),
        ("HMACOutputLength", ["HMAC", "Output", "Length"]),
        ("XMLTimeStamp", ["XML", "Time", "Stamp"]),
        ("IPV4Address", ["IPV4", "Address"]),
        ("IPAddress", ["IP", "Address"]),
        ("IS_PHYSICAL", ["IS", "PHYSICAL"]),
        ("RFC3161TimeStampToken", ["RFC3161", "Time", "Stamp", "Token"]),
        ("SPKISexp", ["SPKI", "Sexp"]),
        ("CardPTPersVersion", ["Card", "PT", "Pers", "Version"]),
        ("camelCase-with-dash", ["camel", "Case", "with", "dash"]),
    ],
)
def test_split_identifier_words(name, expected):
    assert split_identifier_words(name) == expected


@pytest.mark.parametrize(
    "name,expected",
    [
        # Simple cases that already worked
        ("foo", "foo"),
        ("fooBar", "fooBar"),
        ("FooBar", "fooBar"),
        # Leading acronyms — the regression these tests guard against.
        ("CRLRefs", "crlRefs"),
        ("OCSPRefs", "ocspRefs"),
        ("XMLTimeStamp", "xmlTimeStamp"),
        ("HMACOutputLength", "hmacOutputLength"),
        ("IPAddress", "ipAddress"),
        ("IPV4Address", "ipv4Address"),
        ("IPV6Address", "ipv6Address"),
        ("RSAKeyValue", "rsaKeyValue"),
        ("DSAKeyValue", "dsaKeyValue"),
        ("PGPData", "pgpData"),
        ("SPKISexp", "spkiSexp"),
        # Trailing acronyms
        ("MessageID", "messageId"),
        ("RequestID", "requestId"),
        ("EventID", "eventId"),
        ("RefURI", "refUri"),
        # Snake-case / all-caps
        ("IS_PHYSICAL", "isPhysical"),
        ("FOO", "foo"),
        # Digit-bearing acronyms
        ("RFC3161TimeStampToken", "rfc3161TimeStampToken"),
    ],
)
def test_format_property_name(name, expected):
    assert NamingStrategy().format_property_name(name) == expected


def test_format_property_name_override():
    strat = NamingStrategy(
        property_name_overrides={
            "IncludeEContent": "includeEnvelopedContent",
            "VPNTIStatus": "vpnTIStatus",
        }
    )
    assert strat.format_property_name("IncludeEContent") == "includeEnvelopedContent"
    assert strat.format_property_name("VPNTIStatus") == "vpnTIStatus"
    # Names not in the override fall through to the default algorithm.
    assert strat.format_property_name("CRLRefs") == "crlRefs"
