package soap12

import "encoding/xml"

type To struct {
	XMLName        xml.Name `xml:"http://www.w3.org/2005/08/addressing To"`
	MustUnderstand bool     `xml:"http://www.w3.org/2003/05/soap-envelope mustUnderstand,attr,omitempty"`
	Value          string   `xml:",chardata"`
}

type From struct {
	XMLName        xml.Name `xml:"http://www.w3.org/2005/08/addressing From"`
	MustUnderstand bool     `xml:"http://www.w3.org/2003/05/soap-envelope mustUnderstand,attr,omitempty"`
	Value          string   `xml:",chardata"`
}

type ReplyTo struct {
	XMLName        xml.Name `xml:"http://www.w3.org/2005/08/addressing ReplyTo"`
	MustUnderstand bool     `xml:"http://www.w3.org/2003/05/soap-envelope mustUnderstand,attr,omitempty"`
	Value          string   `xml:",chardata"`
}

type FaultTo struct {
	XMLName        xml.Name `xml:"http://www.w3.org/2005/08/addressing FaultTo"`
	MustUnderstand bool     `xml:"http://www.w3.org/2003/05/soap-envelope mustUnderstand,attr,omitempty"`
	Value          string   `xml:",chardata"`
}

type Action struct {
	XMLName        xml.Name `xml:"http://www.w3.org/2005/08/addressing Action"`
	MustUnderstand bool     `xml:"http://www.w3.org/2003/05/soap-envelope mustUnderstand,attr,omitempty"`
	Value          string   `xml:",chardata"`
}

type MessageID struct {
	XMLName        xml.Name `xml:"http://www.w3.org/2005/08/addressing MessageID"`
	MustUnderstand bool     `xml:"http://www.w3.org/2003/05/soap-envelope mustUnderstand,attr,omitempty"`
	Value          string   `xml:",chardata"`
}

type RelatesTo struct {
	XMLName          xml.Name `xml:"http://www.w3.org/2005/08/addressing RelatesTo"`
	MustUnderstand   bool     `xml:"http://www.w3.org/2003/05/soap-envelope mustUnderstand,attr,omitempty"`
	RelationshipType string   `xml:"RelationshipType,attr,omitempty"`
	Value            string   `xml:",chardata"`
}
