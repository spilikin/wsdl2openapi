package generator_test

import (
	"encoding/xml"
	"testing"
)

type IFilter interface {
	isFilter()
}

func (sf SimpleFilter) isFilter() {}

func (cf ComplexFilter) isFilter() {}

type SimpleFilter struct {
	Negate bool   `xml:"negate,attr"`
	Value  string `xml:",chardata"`
}

type ComplexFilter struct {
	Negate     bool           `xml:"negate,attr"`
	SubFilters []SimpleFilter `xml:"SimpleFilter"`
}

func TestPolymorhicTypes(t *testing.T) {

	sf := SimpleFilter{
		Negate: true,
		Value:  "example",
	}

	data, err := xml.MarshalIndent(sf, "", "  ")
	if err != nil {
		t.Fatalf("Failed to marshal XML: %v", err)
	}

	t.Logf("Marshalled XML:\n%s", data)

	cf := ComplexFilter{
		Negate: false,
		SubFilters: []SimpleFilter{
			{Negate: true, Value: "sub1"},
			{Negate: false, Value: "sub2"},
		},
	}

	LogFilter(t, cf)

	cf.SubFilters = nil

	LogFilter(t, cf)

}

func LogFilter(t *testing.T, f IFilter) {
	data, err := xml.MarshalIndent(f, "", "  ")
	if err != nil {
		t.Fatalf("Failed to marshal XML: %v", err)
	}
	t.Logf("Marshalled XML:\n%s", data)
}
