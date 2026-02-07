package main

import (
	"encoding/json"
	"os"
)

func main() {
	//generateEpa()
	generateKon()

}

func generateEpa() {
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

	namingStrategy := NewNamingStrategy()
	namingStrategy.BasePackage = "github.com/test/testproj/epa/soap"

	namingStrategy.PortMappings["I_Document_Management"] = "docmgmt"
	namingStrategy.PortMappings["I_Document_Management_Insurant"] = "docmgmtinsurant"
	namingStrategy.PortMappings["I_Document_Management_NCPeH"] = "docmgmtncpeh"

	namingStrategy.Soap11Package = "github.com/test/testproj/soap11"
	namingStrategy.Soap12Package = "github.com/test/testproj/soap12"

	gen := Generator{
		OutputDir:      "./testproj/epa/soap",
		NamingStrategy: namingStrategy,
		Api:            api,
	}

	err = gen.Generate()
	if err != nil {
		panic(err)
	}
}

func generateKon() {
	inputFile := "Konnektor-OPB6.json"

	inputData, err := os.ReadFile(inputFile)
	if err != nil {
		panic(err)
	}

	api := new(Api)

	err = json.Unmarshal(inputData, api)
	if err != nil {
		panic(err)
	}

	namingStrategy := NewNamingStrategy()
	namingStrategy.BasePackage = "github.com/test/testproj/kon/soap"

	gen := Generator{
		OutputDir:      "./testproj/kon/soap",
		NamingStrategy: namingStrategy,
		Api:            api,
	}

	err = gen.Generate()
	if err != nil {
		panic(err)
	}
}
