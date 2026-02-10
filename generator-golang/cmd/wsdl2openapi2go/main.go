package main

import (
	"encoding/json"
	"flag"
	"fmt"
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
		fmt.Fprintln(os.Stderr, "Error: input file is required")
		os.Exit(1)
	}

	if outputDir == "" {
		fmt.Fprintln(os.Stderr, "Error: output directory is required")
		os.Exit(1)
	}

	namingStrategy := new(generator.NamingStrategy)
	if err := unmarshalJsonFile(namingFile, namingStrategy); err != nil {
		fmt.Fprintf(os.Stderr, "Error reading naming file: %v\n", err)
		os.Exit(1)
	}

	api := new(generator.Api)
	if err := unmarshalJsonFile(inputFile, api); err != nil {
		fmt.Fprintf(os.Stderr, "Error reading input file: %v\n", err)
		os.Exit(1)
	}

	gen := generator.Generator{
		OutputDir:      outputDir,
		NamingStrategy: *namingStrategy,
		Api:            api,
	}

	err := gen.Generate()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error generating code: %v\n", err)
		os.Exit(1)
	}

}

func unmarshalJsonFile(filename string, v any) error {
	data, err := os.ReadFile(filename)
	if err != nil {
		return err
	}
	return json.Unmarshal(data, v)
}
