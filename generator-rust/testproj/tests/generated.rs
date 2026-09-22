use std::future::Future;
use std::pin::pin;
use std::task::{Context as TaskContext, Poll, Waker};

use quick_xml::NsReader;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;

use testproj::kon::gematik::conn::cardservice81::{CardInfoType, Cards};
use testproj::kon::gematik::conn::cardservicecommon20::CardTypeType;
use testproj::kon::gematik::conn::connectorcommon50::{Result as StatusResult, Status};
use testproj::kon::gematik::conn::connectorcontext20::ContextType;
use testproj::kon::gematik::conn::eventservice72::{
    EventServicePort, GetCards, GetCardsEnvelope, GetCardsInput, GetCardsOutput, GetCardsResponse,
    GetCardsResponseEnvelope,
};
use testproj::kon::oasis::dss10core::{AnyType, ClaimedIdentity};
use testproj::kon::soap::{AnyXml, Base64Binary, Envelope, SoapRequest};

const SOAP_ENV: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const EVENT: &str = "http://ws.gematik.de/conn/EventService/v7.2";
const CONTEXT: &str = "http://ws.gematik.de/conn/ConnectorContext/v2.0";
const COMMON: &str = "http://ws.gematik.de/conn/ConnectorCommon/v5.0";
const CARD_COMMON: &str = "http://ws.gematik.de/conn/CardServiceCommon/v2.0";

fn context() -> ContextType {
    ContextType {
        mandant_id: "m1".into(),
        client_system_id: "cs1".into(),
        workplace_id: "wp1".into(),
        user_id: None,
    }
}

fn get_cards() -> GetCards {
    GetCards {
        mandant_wide: Some(false),
        context: context(),
        ct_id: None,
        slot_id: None,
        card_type: Some(CardTypeType::SmcB),
    }
}

/// Every element in `xml` as (resolved namespace, local name).
fn resolved_elements(xml: &str) -> Vec<(Option<String>, String)> {
    let mut reader = NsReader::from_str(xml);
    let mut out = Vec::new();
    loop {
        match reader.read_resolved_event().unwrap() {
            (ns, Event::Start(e) | Event::Empty(e)) => {
                let ns = match ns {
                    ResolveResult::Bound(ns) => Some(ns.as_ref().to_owned()),
                    ResolveResult::Unbound => None,
                    ResolveResult::Unknown(prefix) => {
                        panic!("undeclared prefix {prefix:?} in {xml}")
                    }
                };
                out.push((ns, e.local_name().as_ref().to_owned()));
            }
            (_, Event::Eof) => return out,
            _ => {}
        }
    }
}

#[test]
fn request_serializes_with_resolvable_namespaces() {
    let envelope: GetCardsEnvelope = get_cards().into();
    let xml = envelope.to_xml().unwrap();

    let elements = resolved_elements(&xml);
    let expect = |ns: &str, local: &str| (Some(ns.to_owned()), local.to_owned());
    assert_eq!(
        elements,
        [
            expect(SOAP_ENV, "Envelope"),
            expect(SOAP_ENV, "Body"),
            expect(EVENT, "GetCards"),
            expect(CONTEXT, "Context"),
            expect(COMMON, "MandantId"),
            expect(COMMON, "ClientSystemId"),
            expect(COMMON, "WorkplaceId"),
            expect(CARD_COMMON, "CardType"),
        ],
        "{xml}"
    );
    assert!(xml.contains(r#"mandant-wide="false""#), "{xml}");
    assert!(xml.contains(">SMC-B<"), "{xml}");

    let parsed = GetCardsEnvelope::from_xml(&xml).unwrap();
    assert_eq!(parsed, envelope);
}

#[test]
fn response_parses_with_foreign_prefixes() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Header/>
  <soap:Body>
    <EVT:GetCardsResponse xmlns:EVT="http://ws.gematik.de/conn/EventService/v7.2"
        xmlns:CONN="http://ws.gematik.de/conn/ConnectorCommon/v5.0"
        xmlns:CARD="http://ws.gematik.de/conn/CardService/v8.1"
        xmlns:CARDCMN="http://ws.gematik.de/conn/CardServiceCommon/v2.0">
      <CONN:Status><CONN:Result>OK</CONN:Result></CONN:Status>
      <CARD:Cards>
        <CARD:Card>
          <CONN:CardHandle>c-1</CONN:CardHandle>
          <CARDCMN:CardType>EGK</CARDCMN:CardType>
          <CARDCMN:Iccsn>80276001011699901234</CARDCMN:Iccsn>
          <CARDCMN:CtId>ct-1</CARDCMN:CtId>
          <CARDCMN:SlotId>1</CARDCMN:SlotId>
          <CARD:InsertTime>2025-01-01T10:00:00Z</CARD:InsertTime>
          <CARD:CardHolderName>Erika Mustermann</CARD:CardHolderName>
        </CARD:Card>
        <CARD:Card>
          <CONN:CardHandle>c-2</CONN:CardHandle>
          <CARDCMN:CardType>SMC-B</CARDCMN:CardType>
          <CARDCMN:CtId>ct-1</CARDCMN:CtId>
          <CARDCMN:SlotId>2</CARDCMN:SlotId>
          <CARD:InsertTime>2025-01-01T10:05:00Z</CARD:InsertTime>
        </CARD:Card>
      </CARD:Cards>
    </EVT:GetCardsResponse>
  </soap:Body>
</soap:Envelope>"#;

    let envelope = GetCardsResponseEnvelope::from_xml(xml).unwrap();
    assert!(!envelope.is_fault());
    let response = envelope.into_content().into_result().unwrap();
    assert_eq!(response.status.result, StatusResult::Ok);
    let cards = &response.cards.card;
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].card_type, CardTypeType::Egk);
    assert_eq!(
        cards[0].card_holder_name.as_deref(),
        Some("Erika Mustermann")
    );
    assert_eq!(cards[1].card_type, CardTypeType::SmcB);
    assert_eq!(cards[1].slot_id, 2);
}

#[test]
fn response_round_trips() {
    let response = GetCardsResponse {
        status: Status {
            result: StatusResult::Ok,
            error: None,
        },
        cards: Cards {
            card: vec![CardInfoType {
                card_handle: "c-1".into(),
                card_type: CardTypeType::Hba,
                card_version: None,
                iccsn: None,
                ct_id: "ct".into(),
                slot_id: 3,
                insert_time: "2025-01-01T10:00:00Z".into(),
                card_holder_name: None,
                kvnr: None,
                certificate_expiration_date: None,
            }],
        },
    };
    let envelope = Envelope::new(GetCardsOutput::GetCardsResponse(response));
    let xml = envelope.to_xml().unwrap();
    resolved_elements(&xml);
    assert_eq!(GetCardsResponseEnvelope::from_xml(&xml).unwrap(), envelope);
}

#[test]
fn fault_detail_wraps_the_typed_error() {
    let xml = r#"<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/">
  <SOAP-ENV:Body>
    <SOAP-ENV:Fault>
      <faultcode>SOAP-ENV:Server</faultcode>
      <faultstring>Karte nicht vorhanden</faultstring>
      <detail>
        <GERROR:Error xmlns:GERROR="http://ws.gematik.de/tel/error/v2.0">
          <GERROR:MessageID>m-1</GERROR:MessageID>
          <GERROR:Timestamp>2025-01-01T10:00:00Z</GERROR:Timestamp>
          <GERROR:Trace>
            <GERROR:EventID>e-1</GERROR:EventID>
            <GERROR:Instance>i-1</GERROR:Instance>
            <GERROR:LogReference>l-1</GERROR:LogReference>
            <GERROR:CompType>Konnektor</GERROR:CompType>
            <GERROR:Code>4008</GERROR:Code>
            <GERROR:Severity>Error</GERROR:Severity>
            <GERROR:ErrorType>Technical</GERROR:ErrorType>
            <GERROR:ErrorText>Karte nicht als gesteckt identifiziert</GERROR:ErrorText>
            <GERROR:Detail Encoding="text/plain">slot 2</GERROR:Detail>
          </GERROR:Trace>
        </GERROR:Error>
      </detail>
    </SOAP-ENV:Fault>
  </SOAP-ENV:Body>
</SOAP-ENV:Envelope>"#;

    let envelope = GetCardsResponseEnvelope::from_xml(xml).unwrap();
    assert!(envelope.is_fault());
    let fault = envelope.clone().into_content().into_result().unwrap_err();
    assert_eq!(fault.faultcode, "SOAP-ENV:Server");
    assert_eq!(
        fault.to_string(),
        "SOAP fault SOAP-ENV:Server: Karte nicht vorhanden"
    );
    let error = fault
        .detail
        .as_ref()
        .and_then(|d| d.error.as_ref())
        .expect("typed error detail");
    let trace = &error.trace[0];
    assert_eq!(trace.code, 4008);
    let detail = trace.detail.as_ref().unwrap();
    assert_eq!(
        (detail.encoding.as_deref(), detail.char_data.as_str()),
        (Some("text/plain"), "slot 2")
    );

    // SOAP 1.1 fault children are unqualified; the detail payload keeps its namespace.
    let written = envelope.to_xml().unwrap();
    let elements = resolved_elements(&written);
    for local in ["faultcode", "faultstring", "detail"] {
        assert!(
            elements.contains(&(None, local.to_owned())),
            "{local} in {written}"
        );
    }
    assert!(elements.contains(&(
        Some("http://ws.gematik.de/tel/error/v2.0".into()),
        "Error".into()
    )));
    assert_eq!(
        GetCardsResponseEnvelope::from_xml(&written).unwrap(),
        envelope
    );
}

#[test]
fn any_xml_and_base64_round_trip() {
    let identity = ClaimedIdentity {
        name: AnyXml {
            attributes: vec![("Format".into(), "urn:x".into())],
            children: vec![(
                "Part".into(),
                AnyXml {
                    text: Some("Praxis Dr. Test".into()),
                    ..AnyXml::default()
                },
            )],
            text: None,
        },
        supporting_info: None,
    };
    let xml = quick_xml::se::to_string_with_root("ClaimedIdentity", &identity).unwrap();
    assert_eq!(
        quick_xml::de::from_str::<ClaimedIdentity>(&xml).unwrap(),
        identity
    );

    let empty: AnyType = quick_xml::de::from_str("<AnyType/>").unwrap();
    assert_eq!(empty.unknown_content, None);

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Digest {
        #[serde(rename = "DigestValue")]
        value: Base64Binary,
    }
    let wrapped = "<Digest><DigestValue>AAEC\n  /w==</DigestValue></Digest>";
    let digest: Digest = quick_xml::de::from_str(wrapped).unwrap();
    assert_eq!(digest.value.as_ref(), [0, 1, 2, 255]);
    assert_eq!(
        quick_xml::se::to_string(&digest).unwrap(),
        "<Digest><DigestValue>AAEC/w==</DigestValue></Digest>"
    );
}

#[test]
fn enums_reject_unknown_values() {
    assert_eq!("SMC-B".parse::<CardTypeType>(), Ok(CardTypeType::SmcB));
    let err = "SMC-X".parse::<CardTypeType>().unwrap_err();
    assert_eq!(err.to_string(), r#"unknown CardTypeType value "SMC-X""#);
}

/// A transport written once for every operation, driven by `SoapRequest`.
struct Recorded {
    response: &'static str,
}

impl Recorded {
    async fn call<R: SoapRequest>(
        &self,
        request: Envelope<R>,
    ) -> Result<Envelope<R::Response>, String> {
        let body = request.to_xml().map_err(|e| e.to_string())?;
        assert!(body.contains(R::OPERATION.name));
        assert!(
            R::NAMESPACES
                .iter()
                .all(|(attr, _)| attr.starts_with("@xmlns:"))
        );
        Envelope::from_xml(self.response).map_err(|e| e.to_string())
    }
}

impl EventServicePort for Recorded {
    type Error = String;

    async fn get_cards(&self, request: GetCards) -> Result<GetCardsResponse, String> {
        let response = self.call::<GetCardsInput>(request.into()).await?;
        response
            .into_content()
            .into_result()
            .map_err(|fault| fault.to_string())
    }

    async fn get_card_terminals(
        &self,
        _: testproj::kon::gematik::conn::eventservice72::GetCardTerminals,
    ) -> Result<testproj::kon::gematik::conn::eventservice72::GetCardTerminalsResponse, String>
    {
        unimplemented!()
    }

    async fn subscribe(
        &self,
        _: testproj::kon::gematik::conn::eventservice72::Subscribe,
    ) -> Result<testproj::kon::gematik::conn::eventservice72::SubscribeResponse, String> {
        unimplemented!()
    }

    async fn unsubscribe(
        &self,
        _: testproj::kon::gematik::conn::eventservice72::Unsubscribe,
    ) -> Result<testproj::kon::gematik::conn::eventservice72::UnsubscribeResponse, String> {
        unimplemented!()
    }

    async fn renew_subscriptions(
        &self,
        _: testproj::kon::gematik::conn::eventservice72::RenewSubscriptions,
    ) -> Result<testproj::kon::gematik::conn::eventservice72::RenewSubscriptionsResponse, String>
    {
        unimplemented!()
    }

    async fn get_subscription(
        &self,
        _: testproj::kon::gematik::conn::eventservice72::GetSubscription,
    ) -> Result<testproj::kon::gematik::conn::eventservice72::GetSubscriptionResponse, String> {
        unimplemented!()
    }

    async fn get_resource_information(
        &self,
        _: testproj::kon::gematik::conn::eventservice72::GetResourceInformation,
    ) -> Result<testproj::kon::gematik::conn::eventservice72::GetResourceInformationResponse, String>
    {
        unimplemented!()
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    match future
        .as_mut()
        .poll(&mut TaskContext::from_waker(Waker::noop()))
    {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the recorded transport never waits"),
    }
}

#[test]
fn port_trait_is_implementable_over_a_generic_transport() {
    let port = Recorded {
        response: r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body>
            <e:GetCardsResponse xmlns:e="http://ws.gematik.de/conn/EventService/v7.2" xmlns:c="http://ws.gematik.de/conn/ConnectorCommon/v5.0" xmlns:k="http://ws.gematik.de/conn/CardService/v8.1">
              <c:Status><c:Result>Warning</c:Result></c:Status><k:Cards/>
            </e:GetCardsResponse></s:Body></s:Envelope>"#,
    };
    let response = block_on(port.get_cards(get_cards())).unwrap();
    assert_eq!(response.status.result, StatusResult::Warning);
    assert!(response.cards.card.is_empty());
    assert_eq!(
        GetCardsInput::OPERATION.soap_action,
        "http://ws.gematik.de/conn/EventService/v7.2#GetCards"
    );
}

#[test]
fn recursive_types_are_boxed() {
    use testproj::edge::demo::tree::{Link, Node};

    let leaf = |t: &str| Node {
        r#type: t.into(),
        self_: None,
        parent: None,
        children: vec![],
        link: None,
    };
    let node = Node {
        parent: Some(Box::new(leaf("parent"))),
        children: vec![leaf("a"), leaf("b")],
        link: Some(Box::new(Link {
            target: Box::new(leaf("target")),
            lang: Some("de".into()),
        })),
        ..leaf("root")
    };
    let xml = quick_xml::se::to_string_with_root("tree:Node", &node).unwrap();
    assert!(xml.contains(r#"<tree:Link xml:lang="de">"#), "{xml}");
    assert_eq!(quick_xml::de::from_str::<Node>(&xml).unwrap(), node);
}
