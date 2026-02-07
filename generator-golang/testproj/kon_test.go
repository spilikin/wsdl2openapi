package testproj_test

import (
	"testing"

	"github.com/gematik/zero-lab/go/soap"
	"github.com/stretchr/testify/assert"
	"github.com/test/testproj/kon/api/gematik/conn/authsignatureservice741"
	"github.com/test/testproj/kon/api/gematik/conn/signatureservice74"
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
