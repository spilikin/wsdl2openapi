package soap12

import (
	"encoding/xml"

	"github.com/gematik/zero-lab/go/soap/wsa"
)

// MarshalEnvelope marshals a SOAP envelope into XML
func MarshalTypeSafeEnvelope(env any) ([]byte, error) {
	return xml.Marshal(env)
}

// MarshalEnvelope marshals a SOAP envelope into XML
func MarshalTypeSafeEnvelopeIndent(env any) ([]byte, error) {
	return xml.MarshalIndent(env, "", "    ")
}

// UnmarshalEnvelope unmarshals XML data into a SOAP 1.2 envelope
func UnmarshalEnvelope(data []byte) (*Envelope, error) {
	env := new(Envelope)
	err := xml.Unmarshal(data, env)
	if err != nil {
		return nil, err
	}
	env.Raw = data
	return env, nil
}

// IsFault checks if the SOAP envelope contains a fault
func (e *Envelope) IsFault() bool {
	return e.Body.Fault != nil
}

// Envelope represents a SOAP 1.2 envelope
type Envelope struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Envelope"`
	Header  *Header  `xml:"Header,omitempty"`
	Body    Body     `xml:"Body"`
	Raw     []byte   `xml:"-"`
}

// UnmarshalEnvelopeToTypeSafe unmarshals XML data into a type-safe SOAP 1.2 envelope
func UnmarshalTypeSafeEnvelope(data []byte, v any) error {
	return xml.Unmarshal(data, v)
}

// ToTypeSafe converts a generic Envelope to a type-safe envelope
func (e *Envelope) ToTypeSafe(v any) error {
	return UnmarshalTypeSafeEnvelope(e.Raw, v)
}

// Header represents a SOAP 1.2 header
type Header struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Header"`
	// WS-Addressing, s. https://www.w3.org/TR/2006/REC-ws-addr-core-20060509/
	To        *wsa.To
	From      *wsa.From
	ReplyTo   *wsa.ReplyTo
	FaultTo   *wsa.FaultTo
	Action    *wsa.Action
	MessageID *wsa.MessageID
	RelatesTo *wsa.RelatesTo
}

// Body represents a SOAP 1.2 body
type Body struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Body"`
	Fault   *Fault   `xml:"Fault,omitempty"`
	Nested  []any    `xml:",any"`
}

// Fault represents a SOAP 1.2 fault
type Fault struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Fault"`
	Code    FaultCode
	Reason  FaultReason
	Node    string       `xml:"Node,omitempty"`
	Role    string       `xml:"Role,omitempty"`
	Detail  *FaultDetail `xml:"Detail,omitempty"`
}

// FaultCode represents a SOAP 1.2 fault code
type FaultCode struct {
	Value   string        `xml:"Value"`
	Subcode *FaultSubcode `xml:"Subcode,omitempty"`
}

// FaultSubcode represents a SOAP 1.2 fault subcode
type FaultSubcode struct {
	Value   string        `xml:"Value"`
	Subcode *FaultSubcode `xml:"Subcode,omitempty"`
}

// FaultReason represents a SOAP 1.2 fault reason
type FaultReason struct {
	Text []FaultReasonText `xml:"Text"`
}

// FaultReasonText represents a SOAP 1.2 fault reason text with language
type FaultReasonText struct {
	Lang  string `xml:"xml:lang,attr"`
	Value string `xml:",chardata"`
}

// FaultDetail represents a SOAP 1.2 fault detail
type FaultDetail struct {
	Attributes []xml.Attr `xml:",any,attr"`
	// custom detail elements
	Nested []any `xml:",any"`
}
