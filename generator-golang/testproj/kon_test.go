package testproj_test

import (
	"testing"

	"github.com/gematik/zero-lab/go/soap"
	"github.com/stretchr/testify/assert"
	"github.com/test/testproj/kon/api/gematik/conn/authsignatureservice741"
	"github.com/test/testproj/kon/api/gematik/conn/signatureservice74"
)

func TestEventService(t *testing.T) {
	assert := assert.New(t)

	assert.NotPanics(func() {})

	env := authsignatureservice741.ExternalAuthenticateEnvelope{
		Body: signatureservice74.ExternalAuthenticate{
			CardHandle: "1234567890",
		},
	}

	marshalled, err := soap.MarshalTypeSafeEnvelopeIndent(env, "", "  ")
	assert.NoError(err)

	t.Log(string(marshalled))
}
