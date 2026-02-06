package soap12_test

import (
	"encoding/xml"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/test/testproj/epa/soap/ihe/iti/xdsb2007/docmgmt"
	"github.com/test/testproj/epa/soap/oasis/names/tc/ebxmlregrep/query30"
	"github.com/test/testproj/epa/soap/oasis/names/tc/ebxmlregrep/rim30"
	"github.com/test/testproj/soap12"
)

const QueryIdFindDocuments = "urn:uuid:14d4debf-8f97-4251-9a74-a90016b0af0d"

type RegistryStoredQueryEnvelope struct {
	XMLName xml.Name `xml:"http://www.w3.org/2003/05/soap-envelope Envelope"`
	Header  struct {
		Action string `xml:"http://www.w3.org/2005/08/addressing Action"`
	} `xml:"Header"`
	Body struct {
		AdhocQueryRequest query30.AdhocQueryRequest
	} `xml:"Body"`
}

// implement TypeSafeEnvelope interface
func (e *RegistryStoredQueryEnvelope) GetHeaders() []any {
	return nil
}
func (e *RegistryStoredQueryEnvelope) GetBodyContent() []any {
	return []any{e.Body.AdhocQueryRequest}
}

func TestAdhocQueryRequest(t *testing.T) {
	assert := assert.New(t)

	qr := query30.AdhocQueryRequest{
		Federated:  false,
		StartIndex: 0,
		MaxResults: -1,
		ResponseOption: query30.ResponseOption{
			ReturnType:            "LeafClass",
			ReturnComposedObjects: true,
		},
		AdhocQuery: rim30.AdhocQuery{
			Id: QueryIdFindDocuments,
			Slot: []rim30.Slot{
				{
					Name: "$XDSDocumentEntryStatus",
					ValueList: rim30.ValueList{
						Value: []rim30.Value{
							"('urn:oasis:names:tc:ebxml-regrep:StatusType:Approved')",
							"('urn:oasis:names:tc:ebxml-regrep:StatusType:Deprecated')",
						},
					},
				},
				{
					Name: "$XDSDocumentEntryPatientId",
					ValueList: rim30.ValueList{
						Value: []rim30.Value{
							"('X110674241^^^&1.2.276.0.76.4.8&ISO')",
						},
					},
				},
			},
		},
	}

	typeSafeEnv := &docmgmt.DocumentRegistryRegistryStoredQueryInputEnvelope{}
	//typeSafeEnv.Header.Action = "urn:ihe:iti:2007:RegistryStoredQuery"

	data1, err := soap12.MarshalTypeSafeEnvelopeIndent(typeSafeEnv)
	if err != nil {
		t.Fatalf("Failed to marshal Header: %v", err)
	}
	t.Logf("Marshalled Header:\n%s", string(data1))

	parsedTypeSafeEnv := new(RegistryStoredQueryEnvelope)
	err = soap12.UnmarshalTypeSafeEnvelope(data1, parsedTypeSafeEnv)
	assert.NoError(err)

	assert.NotNil(parsedTypeSafeEnv.Body.AdhocQueryRequest)
	assert.Equal(qr.Federated, parsedTypeSafeEnv.Body.AdhocQueryRequest.Federated)
	assert.Equal(qr.StartIndex, parsedTypeSafeEnv.Body.AdhocQueryRequest.StartIndex)
	assert.Equal(qr.MaxResults, parsedTypeSafeEnv.Body.AdhocQueryRequest.MaxResults)
	assert.Equal(qr.ResponseOption.ReturnType, parsedTypeSafeEnv.Body.AdhocQueryRequest.ResponseOption.ReturnType)
	assert.Equal(qr.ResponseOption.ReturnComposedObjects, parsedTypeSafeEnv.Body.AdhocQueryRequest.ResponseOption.ReturnComposedObjects)
	assert.Equal(qr.AdhocQuery.Id, parsedTypeSafeEnv.Body.AdhocQueryRequest.AdhocQuery.Id)
	assert.Equal(len(qr.AdhocQuery.Slot), len(parsedTypeSafeEnv.Body.AdhocQueryRequest.AdhocQuery.Slot))
	assert.Equal(qr.AdhocQuery.Slot[0].Name, parsedTypeSafeEnv.Body.AdhocQueryRequest.AdhocQuery.Slot[0].Name)
	assert.Equal(len(qr.AdhocQuery.Slot[0].ValueList.Value), len(parsedTypeSafeEnv.Body.AdhocQueryRequest.AdhocQuery.Slot[0].ValueList.Value))

	env, err := soap12.UnmarshalEnvelope(data1)
	assert.NoError(err)
	assert.NotNil(env)
	assert.NotNil(env.Body)
	assert.Len(env.Body.Nested, 1)
	t.Logf("Unmarshalled generic Envelope:\n%#v", env.Body.Nested)
}
