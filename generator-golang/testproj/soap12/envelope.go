package soap12

import (
	"encoding/xml"
)

// Envelope represents a SOAP 1.2 envelope
type Envelope struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Envelope"`
	Header  *Header  `xml:"Header,omitempty"`
	Body    Body     `xml:"Body"`
	Raw     []byte   `xml:"-"`
}

func NewEnvelope() *Envelope {
	return &Envelope{
		Header: nil,
		Body:   Body{},
	}
}

// MarshalEnvelope marshals a SOAP 1.2 envelope into XML
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
	// TODO: parse headers and body elements into DOM-Like Element structs
	// empty the nested slices to remove nil values from unmarshalling the unknown elements
	if env.Header != nil {
		env.Header.Nested = make([]Element, 0)
	}
	env.Body.Nested = make([]any, 0)
	return env, nil
}

// UnmarshalEnvelopeToTypeSafe unmarshals XML data into a type-safe SOAP 1.2 envelope
func UnmarshalTypeSafeEnvelope(data []byte, v any) error {
	return xml.Unmarshal(data, v)
}

// ToTypeSafe converts a generic Envelope to a type-safe envelope
func (e *Envelope) ToTypeSafe(v any) error {
	return UnmarshalTypeSafeEnvelope(e.Raw, v)
}

// Element represents a generic XML element with attributes
type Element struct {
	XMLName    xml.Name
	Attributes []xml.Attr `xml:",any,attr"`
	CharData   string     `xml:",chardata"`
	Nested     []Element  `xml:",any"`
}

// Header represents a SOAP 1.2 header
type Header struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Header"`
	// WS-Addressing, s. https://www.w3.org/TR/2006/REC-ws-addr-core-20060509/
	To        *To
	From      *From
	ReplyTo   *ReplyTo
	FaultTo   *FaultTo
	Action    *Action
	MessageID *MessageID
	RelatesTo *RelatesTo
	// custom headers
	Nested []Element `xml:",any"`
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
