//! The Konnektor client selection (`select-konnektor.json`): requests are written with
//! resolvable namespaces, responses and faults as real Konnektors send them are read,
//! and every operation carries its service directory name, version and timeout class.

use quick_xml::NsReader;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;

use testproj::conn::gematik::conn::authsignatureservice741::ExternalAuthenticateInput;
use testproj::conn::gematik::conn::cardservice821::{
    StartCardSession, StartCardSessionEnvelope, StartCardSessionInput, StartCardSessionOutput,
    StartCardSessionResponseEnvelope,
};
use testproj::conn::gematik::conn::cardservicecommon20::CardTypeType;
use testproj::conn::gematik::conn::connectorcontext20::ContextType;
use testproj::conn::gematik::conn::eventservice72::{
    GetCards, GetCardsEnvelope, GetCardsInput, GetCardsOutput, GetCardsResponseEnvelope,
};
use testproj::conn::gematik::conn::signatureservice74::{BinaryString, ExternalAuthenticate};
use testproj::conn::oasis::dss10core::Base64Data;
use testproj::conn::soap::{Envelope, SoapRequest, Timeout};

const EVENT: &str = "http://ws.gematik.de/conn/EventService/v7.2";
const CONTEXT: &str = "http://ws.gematik.de/conn/ConnectorContext/v2.0";
const COMMON: &str = "http://ws.gematik.de/conn/ConnectorCommon/v5.0";
const CARD_COMMON: &str = "http://ws.gematik.de/conn/CardServiceCommon/v2.0";
const DSS: &str = "urn:oasis:names:tc:dss:1.0:core:schema";

fn context() -> ContextType {
    ContextType {
        mandant_id: "m1".into(),
        client_system_id: "cs1".into(),
        workplace_id: "wp1".into(),
        user_id: None,
    }
}

/// Every element and its text in `xml` as (resolved namespace, local name, text).
fn elements(xml: &str) -> Vec<(Option<String>, String, String)> {
    let mut reader = NsReader::from_str(xml);
    let mut out: Vec<(Option<String>, String, String)> = Vec::new();
    loop {
        match reader.read_resolved_event().unwrap() {
            (ns, Event::Start(e) | Event::Empty(e)) => {
                let ns = match ns {
                    ResolveResult::Bound(ns) => Some(ns.as_ref().to_owned()),
                    ResolveResult::Unbound => None,
                    ResolveResult::Unknown(prefix) => panic!("undeclared prefix {prefix:?}"),
                };
                let local = e.local_name().as_ref().to_owned();
                out.push((ns, local, String::new()));
            }
            (_, Event::Text(t)) => {
                if let Some(last) = out.last_mut() {
                    last.2.push_str(t.as_ref());
                }
            }
            (_, Event::Eof) => return out,
            _ => {}
        }
    }
}

fn find<'a>(
    elements: &'a [(Option<String>, String, String)],
    local: &str,
) -> &'a (Option<String>, String, String) {
    elements
        .iter()
        .find(|(_, l, _)| l == local)
        .unwrap_or_else(|| panic!("no {local}"))
}

#[test]
fn operations_carry_directory_name_version_and_timeout() {
    let op = GetCardsInput::OPERATION;
    assert_eq!(
        (op.service, op.version, op.timeout),
        ("EventService", "7.2", Timeout::Short)
    );
    assert_eq!(
        op.soap_action,
        "http://ws.gematik.de/conn/EventService/v7.2#GetCards"
    );
    let op = ExternalAuthenticateInput::OPERATION;
    assert_eq!(
        (op.service, op.version, op.timeout),
        ("AuthSignatureService", "7.4", Timeout::Long)
    );
    let op = StartCardSessionInput::OPERATION;
    assert_eq!((op.service, op.version), ("CardService", "8.2"));
}

#[test]
fn get_cards_request_resolves_every_namespace() {
    let envelope: GetCardsEnvelope = GetCards {
        mandant_wide: None,
        context: context(),
        ct_id: None,
        slot_id: None,
        card_type: Some(CardTypeType::SmcB),
    }
    .into();
    let xml = envelope.to_xml().unwrap();
    let elements = elements(&xml);
    assert_eq!(find(&elements, "GetCards").0.as_deref(), Some(EVENT));
    assert_eq!(find(&elements, "Context").0.as_deref(), Some(CONTEXT));
    assert_eq!(find(&elements, "MandantId").0.as_deref(), Some(COMMON));
    let card_type = find(&elements, "CardType");
    assert_eq!(
        (card_type.0.as_deref(), card_type.2.as_str()),
        (Some(CARD_COMMON), "SMC-B")
    );
}

/// The response `go/kon/cards_test.go` pins: foreign `ns2`…`ns5` prefixes and
/// unqualified children.
#[test]
fn get_cards_response_reads_with_foreign_prefixes() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <ns4:GetCardsResponse xmlns:ns4="http://ws.gematik.de/conn/EventService/v7.2"
                          xmlns:ns2="http://ws.gematik.de/conn/ConnectorCommon/v5.0"
                          xmlns:ns3="http://ws.gematik.de/conn/CardService/v8.1"
                          xmlns:ns5="http://ws.gematik.de/conn/CardServiceCommon/v2.0">
      <ns2:Status><Result>OK</Result></ns2:Status>
      <ns3:Cards>
        <ns3:Card>
          <ns2:CardHandle>card-smcb-1</ns2:CardHandle>
          <ns5:CardType>SMC-B</ns5:CardType>
          <ns5:Iccsn>80276123456789010001</ns5:Iccsn>
          <ns5:CtId>CT_ID_1</ns5:CtId>
          <ns5:SlotId>1</ns5:SlotId>
          <InsertTime>2024-01-15T10:30:00Z</InsertTime>
          <CardHolderName>Test Practice</CardHolderName>
        </ns3:Card>
        <ns3:Card>
          <ns2:CardHandle>card-hba-1</ns2:CardHandle>
          <ns5:CardType>HBA</ns5:CardType>
          <ns5:Iccsn>80276123456789020001</ns5:Iccsn>
          <ns5:CtId>CT_ID_1</ns5:CtId>
          <ns5:SlotId>2</ns5:SlotId>
          <InsertTime>2024-01-15T11:00:00Z</InsertTime>
          <CardHolderName>Dr. Test</CardHolderName>
        </ns3:Card>
      </ns3:Cards>
    </ns4:GetCardsResponse>
  </soap:Body>
</soap:Envelope>"#;
    let GetCardsOutput::GetCardsResponse(response) = GetCardsResponseEnvelope::from_xml(xml)
        .unwrap()
        .into_content()
    else {
        panic!("expected a response");
    };
    let cards = response.cards.card;
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].card_handle, "card-smcb-1");
    assert_eq!(cards[0].card_type, CardTypeType::SmcB);
    assert_eq!(cards[1].card_holder_name.as_deref(), Some("Dr. Test"));
}

/// A SOAP 1.1 fault carries the gematik error in the lowercase `detail`.
#[test]
fn fault_detail_carries_the_gematik_trace() {
    let xml = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <soap:Fault>
      <faultcode>soap:Server</faultcode>
      <faultstring>Karte nicht vorhanden</faultstring>
      <detail>
        <err:Error xmlns:err="http://ws.gematik.de/tel/error/v2.0">
          <err:MessageID>msg-1</err:MessageID>
          <err:Timestamp>2026-01-01T00:00:00Z</err:Timestamp>
          <err:Trace>
            <err:EventID>ev-1</err:EventID>
            <err:Instance>konnektor</err:Instance>
            <err:LogReference>log-1</err:LogReference>
            <err:CompType>Konnektor</err:CompType>
            <err:Code>4011</err:Code>
            <err:Severity>Error</err:Severity>
            <err:ErrorType>Technical</err:ErrorType>
            <err:ErrorText>Karte nicht vorhanden</err:ErrorText>
          </err:Trace>
        </err:Error>
      </detail>
    </soap:Fault>
  </soap:Body>
</soap:Envelope>"#;
    let envelope = GetCardsResponseEnvelope::from_xml(xml).unwrap();
    assert!(envelope.is_fault());
    let fault = envelope.into_content().into_result().unwrap_err();
    assert_eq!(fault.faultstring, "Karte nicht vorhanden");
    let trace = &fault.detail.unwrap().error.unwrap().trace[0];
    assert_eq!(
        (trace.code, trace.error_text.as_str()),
        (4011, "Karte nicht vorhanden")
    );
}

/// `Base64Data` is simpleContent: its value is element text, which the Go generator
/// got wrong (a `<chardata>` child), forcing `go/kon` to hand-write this request.
#[test]
fn external_authenticate_writes_base64_data_as_text() {
    let envelope: Envelope<ExternalAuthenticateInput> = ExternalAuthenticate {
        card_handle: "card-smcb-1".into(),
        context: context(),
        optional_inputs: None,
        binary_string: BinaryString {
            id: None,
            ref_uri: None,
            ref_type: None,
            schema_refs: None,
            base64_data: Base64Data {
                mime_type: Some("application/octet-stream".into()),
                char_data: "AAECAw==".into(),
            },
        },
    }
    .into();
    let xml = envelope.to_xml().unwrap();
    let elements = elements(&xml);
    let data = find(&elements, "Base64Data");
    assert_eq!(
        (data.0.as_deref(), data.2.as_str()),
        (Some(DSS), "AAECAw==")
    );
    assert!(!xml.contains("chardata"), "{xml}");
    assert!(
        xml.contains(r#"MimeType="application/octet-stream""#),
        "{xml}"
    );
}

#[test]
fn card_sessions_round_trip() {
    let envelope: StartCardSessionEnvelope = StartCardSession {
        context: context(),
        card_handle: "egk-1".into(),
    }
    .into();
    let xml = envelope.to_xml().unwrap();
    assert_eq!(
        find(&elements(&xml), "StartCardSession").0.as_deref(),
        Some("http://ws.gematik.de/conn/CardService/v8.2")
    );
    let response = r#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body>
      <c:StartCardSessionResponse xmlns:c="http://ws.gematik.de/conn/CardService/v8.2"
          xmlns:cc="http://ws.gematik.de/conn/ConnectorCommon/v5.0">
        <cc:Status><cc:Result>OK</cc:Result></cc:Status>
        <c:SessionId>session-42</c:SessionId>
      </c:StartCardSessionResponse></S:Body></S:Envelope>"#;
    let StartCardSessionOutput::StartCardSessionResponse(started) =
        StartCardSessionResponseEnvelope::from_xml(response)
            .unwrap()
            .into_content()
    else {
        panic!("expected a response");
    };
    assert_eq!(started.session_id, "session-42");
}
