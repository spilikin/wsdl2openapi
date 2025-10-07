from wsdl2openapi.common import Context, NamingStrategy


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
