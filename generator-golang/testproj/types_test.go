package testproj

import (
	"encoding/xml"
	"testing"

	"github.com/test/testproj/epa/soap/oasis/names/tc/ebxmlregrep/query30"
)

func TestAdhocQueryRequest(t *testing.T) {
	federated := false
	query := &query30.AdhocQueryRequest{
		Federated:  federated,
		StartIndex: 0,
		MaxResults: -1,
	}

	data, err := xml.MarshalIndent(query, "", "    ")
	if err != nil {
		t.Fatalf("Failed to marshal HeaderContent: %v", err)
	}

	t.Logf("Marshalled HeaderContent:\n%s", string(data))
}
