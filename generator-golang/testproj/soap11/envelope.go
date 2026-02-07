package soap11

import "encoding/xml"

// MarshalEnvelope marshals a SOAP envelope into XML
func MarshalTypeSafeEnvelope(env any) ([]byte, error) {
	return xml.Marshal(env)
}

// MarshalEnvelope marshals a SOAP envelope into XML
func MarshalTypeSafeEnvelopeIndent(env any, prefix, indent string) ([]byte, error) {
	return xml.MarshalIndent(env, prefix, indent)
}

// UnmarshalEnvelopeToTypeSafe unmarshals XML data into a type-safe SOAP 1.1 envelope
func UnmarshalTypeSafeEnvelope(data []byte, v any) error {
	return xml.Unmarshal(data, v)
}

// ToTypeSafe converts a generic Envelope to a type-safe envelope
func (e *Envelope) ToTypeSafe(v any) error {
	return UnmarshalTypeSafeEnvelope(e.Raw, v)
}

// UnmarshalEnvelope unmarshals XML data into a SOAP 1.1 envelope
func UnmarshalEnvelope(data []byte) (*Envelope, error) {
	env := new(Envelope)
	err := xml.Unmarshal(data, env)
	if err != nil {
		return nil, err
	}
	env.Raw = data
	return env, nil
}

type Envelope struct {
	XMLName xml.Name `xml:"http://schemas.xmlsoap.org/soap/envelope/ Envelope"`
	Body    Body     `xml:"Body"`
	Raw     []byte   `xml:"-"`
}

type Fault struct {
	FaultCode   string `xml:"faultcode"`
	FaultString string `xml:"faultstring"`
	FaultActor  string `xml:"faultactor,omitempty"`
	Detail      string `xml:"detail,omitempty"`
}

type Body struct {
	Fault *Fault `xml:"Fault,omitempty"`
}
