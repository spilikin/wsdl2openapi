package testproj_test

import (
	"encoding/xml"
	"testing"

	"github.com/gematik/zero-lab/go/soap"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"github.com/test/testproj/kon/api/gematik/conn/authsignatureservice741"
	"github.com/test/testproj/kon/api/gematik/conn/cardservice81"
	"github.com/test/testproj/kon/api/gematik/conn/connectorcommon50"
	"github.com/test/testproj/kon/api/gematik/conn/connectorcontext20"
	"github.com/test/testproj/kon/api/gematik/conn/eventservice72"
	"github.com/test/testproj/kon/api/gematik/conn/signatureservice74"
	"github.com/test/testproj/kon/api/gematik/tel/error20"
)

func TestEventService(t *testing.T) {
	assert := assert.New(t)

	assert.NotPanics(func() {})

	env := authsignatureservice741.ExternalAuthenticateEnvelope{
		ExternalAuthenticate: &signatureservice74.ExternalAuthenticate{
			CardHandle: "1234567890",
		},
	}

	marshalled, err := soap.MarshalTypeSafeEnvelopeIndent(env, "", "  ")
	assert.NoError(err)

	t.Log(string(marshalled))
}

func TestFaultParsing(t *testing.T) {
	type weatherEnvelope struct {
		Body struct {
			GetWeatherResponse *struct {
				Temperature string `xml:"Temperature"`
			} `xml:"GetWeatherResponse"`
			Fault *struct {
				Code   string `xml:"Code"`
				String string `xml:"String"`
			} `xml:"Fault"`
		} `xml:"Body"`
	}

	var env weatherEnvelope

	faultXml := `<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
	<soap:Body>
		<soap:Fault>
			<Code>Server</Code>
			<String>Internal Server Error</String>
		</soap:Fault>
	</soap:Body>
</soap:Envelope>`

	assert := assert.New(t)
	err := soap.UnmarshalTypeSafeEnvelope([]byte(faultXml), &env)
	assert.NoError(err)
	assert.Equal("Server", env.Body.Fault.Code)
	assert.Equal("Internal Server Error", env.Body.Fault.String)

	successXml := `<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
	<soap:Body>
		<GetWeatherResponse>
			<Temperature>20°C</Temperature>
		</GetWeatherResponse>
	</soap:Body>
</soap:Envelope>`

	var successEnv weatherEnvelope

	err = soap.UnmarshalTypeSafeEnvelope([]byte(successXml), &successEnv)
	assert.NoError(err)
	assert.True(successEnv.Body.Fault == nil)
	assert.Equal("20°C", successEnv.Body.GetWeatherResponse.Temperature)
}

func TestBody(t *testing.T) {
	type Resp struct {
		Result string `xml:"Result"`
	}
	type Fault struct {
		Code string `xml:"Code"`
	}
	type env struct {
		Resp  *Resp  `xml:"Body>GetDataResponse"`
		Fault *Fault `xml:"Body>Fault"`
	}

	testMsgFault := `<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
	<soap:Body>
		<soap:Fault>
			<Code>Server</Code>
		</soap:Fault>
	</soap:Body>
</soap:Envelope>`

	var envFault env

	err := soap.UnmarshalTypeSafeEnvelope([]byte(testMsgFault), &envFault)
	assert := assert.New(t)
	assert.NoError(err)
	assert.Equal("Server", envFault.Fault.Code)
	assert.True(envFault.Resp == nil)

	testMsgSuccess := `<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
	<soap:Body>
		<GetDataResponse>
			<Result>Success</Result>
		</GetDataResponse>
	</soap:Body>
</soap:Envelope>`

	var envSuccess env

	err = soap.UnmarshalTypeSafeEnvelope([]byte(testMsgSuccess), &envSuccess)
	assert.NoError(err)
	assert.Equal("Success", envSuccess.Resp.Result)
	assert.True(envSuccess.Fault == nil)

}

// TestSubscribeEnvelopeMarshal tests marshalling a Subscribe request envelope
func TestSubscribeEnvelopeMarshal(t *testing.T) {
	assert := assert.New(t)

	env := eventservice72.SubscribeEnvelope{
		Subscribe: &eventservice72.Subscribe{
			Context: connectorcontext20.Context{
				MandantId:      "Mandant1",
				ClientSystemId: "ClientSystem1",
				WorkplaceId:    "Workplace1",
			},
			Subscription: eventservice72.Subscription{
				EventTo: "http://example.com/events",
				Topic:   "CARD/INSERTED",
			},
		},
	}

	data, err := xml.MarshalIndent(env, "", "  ")
	assert.NoError(err)

	xmlStr := string(data)
	assert.Contains(xmlStr, "http://schemas.xmlsoap.org/soap/envelope/")
	assert.Contains(xmlStr, "Subscribe")
	assert.Contains(xmlStr, "Mandant1")
	assert.Contains(xmlStr, "CARD/INSERTED")

	// IsFault should return false for request envelopes
	assert.False(env.IsFault())
}

// TestSubscribeResponseEnvelopeSuccess tests unmarshalling a successful Subscribe response
func TestSubscribeResponseEnvelopeSuccess(t *testing.T) {
	assert := assert.New(t)

	responseXml := `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <ns2:SubscribeResponse xmlns:ns2="http://ws.gematik.de/conn/EventService/v7.2" xmlns:ns3="http://ws.gematik.de/conn/ConnectorCommon/v5.0">
      <ns3:Status>
        <Result>OK</Result>
      </ns3:Status>
      <SubscriptionID>sub-12345</SubscriptionID>
      <TerminationTime>2026-12-31T23:59:59Z</TerminationTime>
    </ns2:SubscribeResponse>
  </soap:Body>
</soap:Envelope>`

	var env eventservice72.SubscribeResponseEnvelope
	err := soap.UnmarshalTypeSafeEnvelope([]byte(responseXml), &env)
	assert.NoError(err)

	assert.False(env.IsFault())
	assert.NotNil(env.SubscribeResponse)
	assert.Equal("OK", string(env.SubscribeResponse.Status.Result))
	assert.Equal("sub-12345", string(env.SubscribeResponse.SubscriptionID))
	assert.Equal("2026-12-31T23:59:59Z", string(env.SubscribeResponse.TerminationTime))
}

// TestSubscribeResponseEnvelopeFault tests unmarshalling a SOAP fault with custom Error detail
func TestSubscribeResponseEnvelopeFault(t *testing.T) {
	assert := assert.New(t)

	faultXml := `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <soap:Fault>
      <faultcode>soap:Server</faultcode>
      <faultstring>Internal Error</faultstring>
      <faultactor>EventService</faultactor>
      <Detail>
        <ns1:Error xmlns:ns1="http://ws.gematik.de/tel/error/v2.0">
          <MessageID>msg-001</MessageID>
          <Timestamp>2026-02-07T10:00:00Z</Timestamp>
          <ns1:Trace>
            <EventID>evt-001</EventID>
            <Instance>EventService</Instance>
            <LogReference>log-ref-001</LogReference>
            <CompType>Konnektor</CompType>
            <Code>4000</Code>
            <Severity>Error</Severity>
            <ErrorType>Technical</ErrorType>
            <ErrorText>Subscription limit exceeded</ErrorText>
          </ns1:Trace>
        </ns1:Error>
      </Detail>
    </soap:Fault>
  </soap:Body>
</soap:Envelope>`

	var env eventservice72.SubscribeResponseEnvelope
	err := soap.UnmarshalTypeSafeEnvelope([]byte(faultXml), &env)
	assert.NoError(err)

	assert.True(env.IsFault())
	assert.Nil(env.SubscribeResponse)
	assert.NotNil(env.Fault)
	assert.Equal("soap:Server", env.Fault.Code)
	assert.Equal("Internal Error", env.Fault.String)
	assert.Equal("EventService", env.Fault.Actor)

	// Check the custom fault detail (Error type)
	require.NotNil(t, env.Fault.Detail)
	assert.Equal("msg-001", env.Fault.Detail.MessageID)
	assert.Equal("2026-02-07T10:00:00Z", env.Fault.Detail.Timestamp)
	require.Len(t, env.Fault.Detail.Trace, 1)
	assert.Equal(4000, env.Fault.Detail.Trace[0].Code)
	assert.Equal("Subscription limit exceeded", env.Fault.Detail.Trace[0].ErrorText)
}

// TestGetCardsEnvelopeRoundTrip tests marshal/unmarshal round-trip for GetCards
func TestGetCardsEnvelopeRoundTrip(t *testing.T) {
	require := require.New(t)
	assert := assert.New(t)

	original := eventservice72.GetCardsEnvelope{
		GetCards: &eventservice72.GetCards{
			MandantWide: true,
			Context: connectorcontext20.Context{
				MandantId:      "TestMandant",
				ClientSystemId: "TestClient",
				WorkplaceId:    "TestWorkplace",
			},
		},
	}

	// Marshal
	data, err := soap.MarshalTypeSafeEnvelope(original)
	require.NoError(err)

	// Unmarshal
	var restored eventservice72.GetCardsEnvelope
	err = soap.UnmarshalTypeSafeEnvelope(data, &restored)
	require.NoError(err)

	assert.False(restored.IsFault())
	assert.NotNil(restored.GetCards)
	assert.True(restored.GetCards.MandantWide)
	assert.Equal("TestMandant", string(restored.GetCards.Context.MandantId))
	assert.Equal("TestClient", string(restored.GetCards.Context.ClientSystemId))
	assert.Equal("TestWorkplace", string(restored.GetCards.Context.WorkplaceId))
}

// TestGetCardsResponseWithCards tests unmarshalling a GetCards response with card data
func TestGetCardsResponseWithCards(t *testing.T) {
	assert := assert.New(t)

	responseXml := `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <ns2:GetCardsResponse xmlns:ns2="http://ws.gematik.de/conn/EventService/v7.2"
                          xmlns:ns3="http://ws.gematik.de/conn/ConnectorCommon/v5.0"
                          xmlns:ns4="http://ws.gematik.de/conn/CardService/v8.1"
                          xmlns:ns5="http://ws.gematik.de/conn/CardServiceCommon/v2.0">
      <ns3:Status>
        <Result>OK</Result>
      </ns3:Status>
      <ns4:Cards>
        <ns4:Card>
          <ns3:CardHandle>card-handle-001</ns3:CardHandle>
          <ns5:CardType>EGK</ns5:CardType>
          <ns5:CtId>ct-001</ns5:CtId>
          <ns5:SlotId>1</ns5:SlotId>
          <InsertTime>2026-02-07T09:00:00Z</InsertTime>
        </ns4:Card>
        <ns4:Card>
          <ns3:CardHandle>card-handle-002</ns3:CardHandle>
          <ns5:CardType>HBA</ns5:CardType>
          <ns5:CtId>ct-001</ns5:CtId>
          <ns5:SlotId>2</ns5:SlotId>
          <InsertTime>2026-02-07T09:05:00Z</InsertTime>
        </ns4:Card>
      </ns4:Cards>
    </ns2:GetCardsResponse>
  </soap:Body>
</soap:Envelope>`

	var env eventservice72.GetCardsResponseEnvelope
	err := soap.UnmarshalTypeSafeEnvelope([]byte(responseXml), &env)
	require.NoError(t, err)

	assert.False(env.IsFault())
	require.NotNil(t, env.GetCardsResponse)
	assert.Equal("OK", string(env.GetCardsResponse.Status.Result))
	require.Len(t, env.GetCardsResponse.Cards.Card, 2)

	card1 := env.GetCardsResponse.Cards.Card[0]
	assert.Equal("card-handle-001", string(card1.CardHandle))
	assert.Equal("EGK", string(card1.CardType))

	card2 := env.GetCardsResponse.Cards.Card[1]
	assert.Equal("card-handle-002", string(card2.CardHandle))
	assert.Equal("HBA", string(card2.CardType))
}

// TestCardInfoTypeInterface tests that CardInfoType and CardInfoErrType implement ICardInfoType
func TestCardInfoTypeInterface(t *testing.T) {
	assert := assert.New(t)

	// Both types should implement ICardInfoType interface
	var _ cardservice81.ICardInfoType = cardservice81.CardInfoType{}
	var _ cardservice81.ICardInfoType = cardservice81.CardInfoErrType{}

	// Create instances and verify they work as interface types
	cardInfo := cardservice81.CardInfoType{
		CardHandle: "handle-123",
		CtId:       "ct-001",
		SlotId:     1,
		InsertTime: "2026-02-07T10:00:00Z",
	}

	cardInfoErr := cardservice81.CardInfoErrType{
		CardHandle: "handle-456",
		CtId:       "ct-002",
		SlotId:     2,
		InsertTime: "2026-02-07T11:00:00Z",
		Error: &error20.Error{
			MessageID: "err-001",
			Timestamp: "2026-02-07T11:00:00Z",
		},
	}

	// Use interface to accept both types
	checkCardInfo := func(c cardservice81.ICardInfoType) {
		c.IsCardService81CardInfoType()
	}

	assert.NotPanics(func() {
		checkCardInfo(cardInfo)
		checkCardInfo(cardInfoErr)
	})
}

// TestStatusWithError tests Status struct with embedded Error
func TestStatusWithError(t *testing.T) {
	assert := assert.New(t)

	status := connectorcommon50.Status{
		Result: "WARNING",
		Error: &error20.Error{
			MessageID: "warn-001",
			Timestamp: "2026-02-07T12:00:00Z",
		},
	}

	data, err := xml.MarshalIndent(status, "", "  ")
	assert.NoError(err)

	xmlStr := string(data)
	assert.Contains(xmlStr, "WARNING")
	assert.Contains(xmlStr, "warn-001")

	// Unmarshal and verify
	var restored connectorcommon50.Status
	err = xml.Unmarshal(data, &restored)
	assert.NoError(err)
	assert.Equal("WARNING", string(restored.Result))
	assert.NotNil(restored.Error)
	assert.Equal("warn-001", restored.Error.MessageID)
}

// TestAllEventServiceOperations tests that all EventService envelopes can be instantiated
func TestAllEventServiceOperations(t *testing.T) {
	assert := assert.New(t)

	ctx := connectorcontext20.Context{
		MandantId:      "M1",
		ClientSystemId: "CS1",
		WorkplaceId:    "WP1",
	}

	// Test all input envelopes have IsFault() returning false
	inputEnvelopes := []interface {
		IsFault() bool
	}{
		&eventservice72.SubscribeEnvelope{Subscribe: &eventservice72.Subscribe{Context: ctx}},
		&eventservice72.UnsubscribeEnvelope{Unsubscribe: &eventservice72.Unsubscribe{Context: ctx}},
		&eventservice72.GetSubscriptionEnvelope{GetSubscription: &eventservice72.GetSubscription{Context: ctx}},
		&eventservice72.GetResourceInformationEnvelope{GetResourceInformation: &eventservice72.GetResourceInformation{Context: ctx}},
		&eventservice72.GetCardTerminalsEnvelope{GetCardTerminals: &eventservice72.GetCardTerminals{Context: ctx}},
		&eventservice72.GetCardsEnvelope{GetCards: &eventservice72.GetCards{Context: ctx}},
		&eventservice72.RenewSubscriptionsEnvelope{RenewSubscriptions: &eventservice72.RenewSubscriptions{Context: ctx}},
	}

	for _, env := range inputEnvelopes {
		assert.False(env.IsFault(), "Input envelope should not be a fault")
	}

	// Test all response envelopes without fault
	responseEnvelopes := []interface {
		IsFault() bool
	}{
		&eventservice72.SubscribeResponseEnvelope{SubscribeResponse: &eventservice72.SubscribeResponse{}},
		&eventservice72.UnsubscribeResponseEnvelope{UnsubscribeResponse: &eventservice72.UnsubscribeResponse{}},
		&eventservice72.GetSubscriptionResponseEnvelope{GetSubscriptionResponse: &eventservice72.GetSubscriptionResponse{}},
		&eventservice72.GetResourceInformationResponseEnvelope{GetResourceInformationResponse: &eventservice72.GetResourceInformationResponse{}},
		&eventservice72.GetCardTerminalsResponseEnvelope{GetCardTerminalsResponse: &eventservice72.GetCardTerminalsResponse{}},
		&eventservice72.GetCardsResponseEnvelope{GetCardsResponse: &eventservice72.GetCardsResponse{}},
		&eventservice72.RenewSubscriptionsResponseEnvelope{RenewSubscriptionsResponse: &eventservice72.RenewSubscriptionsResponse{}},
	}

	for _, env := range responseEnvelopes {
		assert.False(env.IsFault(), "Response envelope without fault should not be a fault")
	}
}

// TestNestedStructMarshal tests marshalling of deeply nested structures
func TestNestedStructMarshal(t *testing.T) {
	assert := assert.New(t)

	event := eventservice72.Event{
		Topic:          "CARD/INSERTED",
		Type:           "Info",
		Severity:       "Info",
		SubscriptionID: "sub-123",
		Message: struct {
			XMLName   xml.Name `xml:"http://ws.gematik.de/conn/EventService/v7.2 Message"`
			Parameter []struct {
				XMLName xml.Name `xml:"http://ws.gematik.de/conn/EventService/v7.2 Parameter"`
				Key     string   `xml:"Key"`
				Value   string   `xml:"Value"`
			} `xml:"Parameter"`
		}{
			Parameter: []struct {
				XMLName xml.Name `xml:"http://ws.gematik.de/conn/EventService/v7.2 Parameter"`
				Key     string   `xml:"Key"`
				Value   string   `xml:"Value"`
			}{
				{Key: "CardHandle", Value: "card-001"},
				{Key: "CardType", Value: "EGK"},
			},
		},
	}

	data, err := xml.MarshalIndent(event, "", "  ")
	assert.NoError(err)

	xmlStr := string(data)
	assert.Contains(xmlStr, "CARD/INSERTED")
	assert.Contains(xmlStr, "CardHandle")
	assert.Contains(xmlStr, "card-001")
	assert.Contains(xmlStr, "EGK")

	// Unmarshal and verify
	var restored eventservice72.Event
	err = xml.Unmarshal(data, &restored)
	assert.NoError(err)
	assert.Equal("CARD/INSERTED", string(restored.Topic))
	assert.Len(restored.Message.Parameter, 2)
}
