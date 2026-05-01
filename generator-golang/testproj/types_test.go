package testproj

import (
	"bytes"
	"encoding/xml"
	"testing"

	"github.com/test/testproj/epa/soap/oasis/names/tc/ebxmlregrep/query30"
	"github.com/test/testproj/epa/soap/oasis/names/tc/ebxmlregrep/rim30"
	"github.com/test/testproj/kon/api/gematik/conn/certificateservicecommon20"
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

func TestBase64BytesRoundTrip(t *testing.T) {
	raw := []byte("hello world")

	in := &certificateservicecommon20.X509DataInfoListTypeX509DataInfoX509Data{
		X509IssuerSerial: certificateservicecommon20.X509DataInfoListTypeX509DataInfoX509DataX509IssuerSerial{
			X509IssuerName:   "CN=Test",
			X509SerialNumber: "1",
		},
		X509SubjectName: "CN=Subject",
		X509Certificate: certificateservicecommon20.Base64Bytes(raw),
	}

	encoded, err := xml.Marshal(in)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	if !bytes.Contains(encoded, []byte("aGVsbG8gd29ybGQ=")) {
		t.Fatalf("expected base64 of %q in output, got: %s", raw, encoded)
	}

	var out certificateservicecommon20.X509DataInfoListTypeX509DataInfoX509Data
	if err := xml.Unmarshal(encoded, &out); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if !bytes.Equal(out.X509Certificate, raw) {
		t.Fatalf("round-trip mismatch: got %q, want %q", out.X509Certificate, raw)
	}
}
