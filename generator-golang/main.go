package main

import (
	"encoding/json"
	"os"
)

func main() {
	inputFile := "../XDSDocumentService.json"

	inputData, err := os.ReadFile(inputFile)
	if err != nil {
		panic(err)
	}

	api := new(Api)

	err = json.Unmarshal(inputData, api)
	if err != nil {
		panic(err)
	}

	gen := Generator{
		OutputDir: "./testproj/epa/soap",
		NamingStrategy: NamingStrategy{
			IdMappings:  map[string]string{},
			BasePackage: "github.com/test/testproj/epa/soap",
		},
		Api: api,
	}

	err = gen.Generate()
	if err != nil {
		panic(err)
	}

}
