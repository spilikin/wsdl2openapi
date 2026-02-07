module github.com/test/testproj

go 1.25.7

replace github.com/gematik/zero-lab/go/soap => ../soap

require (
	github.com/gematik/zero-lab/go/soap v0.0.0-00010101000000-000000000000
	github.com/stretchr/testify v1.11.1
)

require (
	github.com/davecgh/go-spew v1.1.1 // indirect
	github.com/pmezard/go-difflib v1.0.0 // indirect
	gopkg.in/yaml.v3 v3.0.1 // indirect
)
