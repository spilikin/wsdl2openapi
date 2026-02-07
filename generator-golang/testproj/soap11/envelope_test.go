package soap11

import (
	"encoding/xml"
	"testing"

	"github.com/stretchr/testify/assert"
)

type AccessTokenHeader struct {
	XMLName        xml.Name `xml:"http://example.org/headers AccessToken"`
	Type           string   `xml:"type,attr"`
	Token          string   `xml:",chardata"`
	MustUnderstand string   `xml:"http://schemas.xmlsoap.org/soap/envelope/ mustUnderstand,attr,omitempty"`
}

type PreferencesHeader struct {
	XMLName        xml.Name `xml:"http://example.org/headers Preferences"`
	Lang           string   `xml:"http://www.w3.org/XML/1998/namespace lang,attr,omitempty"`
	Degree         string   `xml:"http://example.org/headers degree,attr,omitempty"`
	MustUnderstand string   `xml:"http://schemas.xmlsoap.org/soap/envelope/ mustUnderstand,attr,omitempty"`
}

type GetWeatherByPostalCode struct {
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
	XMLName xml.Name `xml:"http://schemas.xmlsoap.org/soap/envelope/ Envelope"`
	Header  struct {
		AccessToken AccessTokenHeader `xml:"http://example.org/headers AccessToken"`
		Preferences PreferencesHeader `xml:"http://example.org/headers Preferences"`
	} `xml:"Header"`
	Body struct {
		GetWeatherByPostalCode GetWeatherByPostalCode `xml:"http://example.org/weather GetWeatherByPostalCode"`
	} `xml:"Body"`
}

type GetWeatherResponseEnvelope struct {
	XMLName xml.Name `xml:"http://schemas.xmlsoap.org/soap/envelope/ Envelope"`
	Body    struct {
		WeatherResponse WeatherResponse `xml:"http://example.org/weather GetWeatherResponse"`
	} `xml:"Body"`
}

func TestWeatherRequest(t *testing.T) {
	assert := assert.New(t)

	reqStr := `<?xml version="1.0" encoding="UTF-8"?>
<env:Envelope xmlns:env="http://schemas.xmlsoap.org/soap/envelope/" xmlns:hdr="http://example.org/headers" xmlns:wea="http://example.org/weather">
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
	err := xml.Unmarshal([]byte(reqStr), typedENv)
	assert.NoError(err)
	assert.Equal("abcdef123456", typedENv.Header.AccessToken.Token)
	assert.Equal("en", typedENv.Header.Preferences.Lang)
	assert.Equal("Celsius", typedENv.Header.Preferences.Degree)
	assert.Equal("12345", typedENv.Body.GetWeatherByPostalCode.PostalCode)
}

func TestWeatherFault(t *testing.T) {
	assert := assert.New(t)

	faultStr := `<?xml version="1.0" encoding="UTF-8"?>
<env:Envelope xmlns:env="http://schemas.xmlsoap.org/soap/envelope/">
   <env:Body>
	  <env:Fault>
		 <faultcode>env:Client</faultcode>
		 <faultstring>Invalid postal code</faultstring>
		 <detail>Postal code must be 5 digits</detail>
	  </env:Fault>
   </env:Body>
</env:Envelope>`

	env := new(Envelope)
	err := xml.Unmarshal([]byte(faultStr), env)
	assert.NoError(err)
	assert.NotNil(env.Body.Fault)
	assert.Equal("env:Client", env.Body.Fault.FaultCode)
	assert.Equal("Invalid postal code", env.Body.Fault.FaultString)
	assert.Equal("Postal code must be 5 digits", env.Body.Fault.Detail)
}

func TestBodyShortcut(t *testing.T) {
	type bodyWithShortcutEnvelope struct {
		XMLName xml.Name               `xml:"http://schemas.xmlsoap.org/soap/envelope/ Envelope"`
		Body    GetWeatherByPostalCode `xml:"Body>GetWeatherByPostalCode"`
	}

	reqStr := `<?xml version="1.0" encoding="UTF-8"?>
<env:Envelope xmlns:env="http://schemas.xmlsoap.org/soap/envelope/" xmlns:wea="http://example.org/weather">
   <env:Body>
	  <wea:GetWeatherByPostalCode>
		 <wea:PostalCode>12345</wea:PostalCode>
	  </wea:GetWeatherByPostalCode>
   </env:Body>
</env:Envelope>`

	typedENv := new(bodyWithShortcutEnvelope)
	err := UnmarshalTypeSafeEnvelope([]byte(reqStr), typedENv)
	assert.NoError(t, err)
	assert.Equal(t, "12345", typedENv.Body.PostalCode)

	// marshal, unmarshal again and check if the content is the same
	marshaled, err := MarshalTypeSafeEnvelopeIndent(typedENv, "", "    ")
	assert.NoError(t, err)

	t.Logf("Marshaled Envelope:\n%s", string(marshaled))

	typedENv2 := new(bodyWithShortcutEnvelope)
	err = UnmarshalTypeSafeEnvelope(marshaled, typedENv2)
	assert.NoError(t, err)
	assert.Equal(t, "12345", typedENv2.Body.PostalCode)
}
