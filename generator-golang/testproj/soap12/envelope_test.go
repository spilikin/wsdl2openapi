package soap12_test

import (
	"encoding/xml"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/test/testproj/soap12"
)

type AccessTokenHeader struct {
	XMLName xml.Name `xml:"http://example.org/headers AccessToken"`
	Type    string   `xml:"type,attr"`
	Token   string   `xml:",chardata"`
}

type PreferencesHeader struct {
	XMLName xml.Name `xml:"http://example.org/headers Preferences"`
	Lang    string   `xml:"http://www.w3.org/XML/1998/namespace lang,attr,omitempty"`
	Degree  string   `xml:"http://example.org/headers degree,attr,omitempty"`
}

type WeatherRequest struct {
	XMLName    xml.Name `xml:"http://example.org/weather GetWeatherByPostalCode"`
	PostalCode string   `xml:"http://example.org/weather PostalCode"`
}

type WeatherResponse struct {
	XMLName     xml.Name `xml:"http://example.org/weather GetWeatherResponse"`
	Locality    string   `xml:"http://example.org/weather Locality"`
	Temperature string   `xml:"http://example.org/weather Temperature"`
	Condition   string   `xml:"http://example.org/weather Condition"`
}

type GetWeatherByPostalCodeEnvelope struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Envelope"`
	Header  struct {
		AccessToken AccessTokenHeader `xml:"http://example.org/headers AccessToken"`
	} `xml:"Header"`
	Body struct {
		WeatherRequest WeatherRequest `xml:"http://example.org/weather GetWeatherByPostalCode"`
	} `xml:"Body"`
}

func (e *GetWeatherByPostalCodeEnvelope) GetHeaders() []any {
	return []any{e.Header.AccessToken}
}

func (e *GetWeatherByPostalCodeEnvelope) GetBodyContent() []any {
	return []any{e.Body.WeatherRequest}
}

type GetWeatherResponseEnvelope struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Envelope"`
	Body    struct {
		WeatherResponse WeatherResponse `xml:"http://example.org/weather GetWeatherResponse"`
	} `xml:"Body"`
}

func TestWeatherRequest(t *testing.T) {
	assert := assert.New(t)

	reqStr := `<?xml version="1.0" encoding="UTF-8"?>
<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope" xmlns:hdr="http://example.org/headers" xmlns:wea="http://example.org/weather">
   <env:Header>
	  <hdr:AccessToken type="Bearer">abcdef123456</hdr:AccessToken>
	  <hdr:Preferences xml:lang="en" hdr:degree="Celsius"/>
   </env:Header>
   <env:Body>
	  <wea:GetWeatherByPostalCode>
		 <wea:PostalCode>12345</wea:PostalCode>
	  </wea:GetWeatherByPostalCode>
   </env:Body>
</env:Envelope>`

	typedENv := new(GetWeatherByPostalCodeEnvelope)
	err := soap12.UnmarshalTypeSafeEnvelope([]byte(reqStr), typedENv)
	assert.NoError(err)

	assert.NotNil(typedENv.Header)
	assert.NotNil(typedENv.Body)
	assert.NotNil(typedENv.Header.AccessToken)
	assert.Equal("http://www.w3.org/2003/05/soap-envelope", typedENv.XMLName.Space)
	assert.Equal("Envelope", typedENv.XMLName.Local)

	assert.NotNil(typedENv.Body.WeatherRequest)
	assert.Equal("abcdef123456", typedENv.Header.AccessToken.Token)
	assert.Equal("Bearer", typedENv.Header.AccessToken.Type)
	assert.Equal("12345", typedENv.Body.WeatherRequest.PostalCode)

}

func TestFault(t *testing.T) {
	assert := assert.New(t)

	faultStr := `<?xml version="1.0" encoding="UTF-8"?>
<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope">
   <env:Body>
	  <env:Fault>
		 <env:Code>
			<env:Value>env:Sender</env:Value>
			<env:Subcode>
			   <env:Value>m:MessageTimeout</env:Value>
			</env:Subcode>
		 </env:Code>
		 <env:Reason>
			<env:Text xml:lang="en">Sender Timeout</env:Text>
			<env:Text xml:lang="de">Zeitüberschreitung beim Sender</env:Text>
		 </env:Reason>
		 <env:Node>http://example.org/node</env:Node>
		 <env:Role>http://example.org/role</env:Role>
		 <env:Detail>
			<e:MaxTime xmlns:e="http://example.org/errors">P5M</e:MaxTime>
		 </env:Detail>
	  </env:Fault>
   </env:Body>
</env:Envelope>`

	env, err := soap12.UnmarshalEnvelope([]byte(faultStr))
	assert.NoError(err)

	assert.NotNil(env)
	assert.NotNil(env.Body.Fault)

	assert.Equal("env:Sender", env.Body.Fault.Code.Value)
	assert.NotNil(env.Body.Fault.Code.Subcode)
	assert.Equal("m:MessageTimeout", env.Body.Fault.Code.Subcode.Value)

	assert.NotNil(env.Body.Fault.Reason)
	assert.Len(env.Body.Fault.Reason.Text, 2)
	assert.Equal("en", env.Body.Fault.Reason.Text[0].Lang)
	assert.Equal("Sender Timeout", env.Body.Fault.Reason.Text[0].Value)
	assert.Equal("de", env.Body.Fault.Reason.Text[1].Lang)
	assert.Equal("Zeitüberschreitung beim Sender", env.Body.Fault.Reason.Text[1].Value)

}
