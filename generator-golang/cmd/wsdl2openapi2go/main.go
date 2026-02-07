package main

import (
	"encoding/json"
	"flag"
	"os"

	"generator-golang"
)

func main() {
	var inputFile string
	var outputDir string
	var namingFile string

	flag.StringVar(&inputFile, "file", "", "OpenAPI specification file")
	flag.StringVar(&inputFile, "f", "", "OpenAPI specification file (shorthand)")
	flag.StringVar(&outputDir, "output", "", "Output directory for generated code")
	flag.StringVar(&outputDir, "o", "", "Output directory for generated code (shorthand)")
	flag.StringVar(&namingFile, "naming", "", "Config file with naming strategy configuration")
	flag.StringVar(&namingFile, "n", "l", "Config file with naming strategy configuration (shorthand)")

	flag.Parse()

	if inputFile == "" {
		panic("input file is required")
	}

	if outputDir == "" {
		panic("output directory is required")
	}

	namingStrategy := new(generator.NamingStrategy)
	if err := unmarshalJsonFile(namingFile, namingStrategy); err != nil {
		panic(err)
	}

	api := new(generator.Api)
	if err := unmarshalJsonFile(inputFile, api); err != nil {
		panic(err)
	}

	gen := generator.Generator{
		OutputDir:      outputDir,
		NamingStrategy: *namingStrategy,
		Api:            api,
	}

	err := gen.Generate()
	if err != nil {
		panic(err)
	}

}

func unmarshalJsonFile(filename string, v any) error {
	data, err := os.ReadFile(filename)
	if err != nil {
		return err
	}
	return json.Unmarshal(data, v)
}

/*
func generateEpa() {
	inputFile := "../../../XDSDocumentService.json"

	api := new(generator.Api)

	err = json.Unmarshal(inputData, api)
	if err != nil {
		panic(err)
	}

	namingStrategy := generator.NamingStrategy{
		BasePackage: "github.com/test/testproj/epa/soap",

		PortMappings: map[string]string{
			"I_Document_Management":          "docmgmt",
			"I_Document_Management_Insurant": "docmgmtinsurant",
			"I_Document_Management_NCPeH":    "docmgmtncpeh",
		},
	}

	gen := generator.Generator{
		OutputDir:      "../../testproj/epa/soap",
		NamingStrategy: namingStrategy,
		Api:            api,
	}

	err = gen.Generate()
	if err != nil {
		panic(err)
	}
}

func generateKon(inputFile string, outputDir string) {
	inputData, err := os.ReadFile(inputFile)
	if err != nil {
		panic(err)
	}

	namingStrategy := generator.NamingStrategy{
		BasePackage: "github.com/test/testproj/kon/api",
		PackageMappings: []generator.PackageMapping{
			{
				Pattern:     `de\.gematik\.ws\.(.*)`,
				Replacement: "gematik.$1",
			},
			{
				Pattern:     `oasis.names.tc.([^.]+).([^.]+)`,
				Replacement: "oasis.$1$2",
			},
			{
				Pattern:     `org.(.*)`,
				Replacement: "$1",
			},
		},
	}

	gen := generator.Generator{
		OutputDir:      outputDir,
		NamingStrategy: namingStrategy,
		Api:            api,
	}

	err = gen.Generate()
	if err != nil {
		panic(err)
	}
}
*/
