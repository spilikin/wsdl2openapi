package testproj

import (
	"encoding/xml"
	"testing"

	"github.com/test/testproj/epa/soap/oasis/names/tc/ebxmlregrep/query30"
	"github.com/test/testproj/epa/soap/oasis/names/tc/ebxmlregrep/rim30"
)

func TestAdhocQueryRequest(t *testing.T) {
	federated := false
	query := &query30.AdhocQueryRequest{
		Federated:  federated,
		StartIndex: 0,
		MaxResults: -1,
		ResponseOption: query30.ResponseOption{
			ReturnComposedObjects: true,
			ReturnType:            "LeafClass",
		},
		AdhocQuery: rim30.AdhocQuery{
			Id: "urn:uuid:xxxxx",
			Slot: []rim30.Slot{
				rim30.Slot{
					Name: "$XDSDocumentEntryStatus",
					ValueList: rim30.ValueList{
						Value: []rim30.Value{
							rim30.Value("('urn:oasis:names:tc:ebxml-regrep:StatusType:Approved')"),
							rim30.Value("('urn:oasis:names:tc:ebxml-regrep:StatusType:Deprecated')"),
						},
					},
				},
			},
		},
	}

	data, err := xml.MarshalIndent(query, "", "    ")
	if err != nil {
		t.Fatalf("Failed to marshal HeaderContent: %v", err)
	}

	t.Logf("Marshalled HeaderContent:\n%s", string(data))
}
