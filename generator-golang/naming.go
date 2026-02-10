package generator

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

// PackageMapping defines a mapping from a package name pattern to a replacement string.
type PackageMapping struct {
	Pattern     string `json:"pattern" yaml:"pattern"`
	Replacement string `json:"replacement" yaml:"replacement"`
}

type NamingStrategy struct {
	PackageMappings []PackageMapping  `json:"packageMappings" yaml:"packageMappings"`
	PortMappings    map[string]string `json:"portMappings" yaml:"portMappings"`
	BasePackage     string            `json:"basePackage" yaml:"basePackage"`
}

func (n NamingStrategy) NormalizePackageName(packageName string) string {

	for _, mapping := range n.PackageMappings {
		if matched, _ := regexp.MatchString(mapping.Pattern, packageName); matched {
			packageName = regexp.MustCompile(mapping.Pattern).ReplaceAllString(packageName, mapping.Replacement)
			break
		}
	}
	return strings.ToLower(packageName)
}

func (n NamingStrategy) FilePath(outputDir string, packageName string, filename string) string {
	packageName = n.NormalizePackageName(packageName)
	return filepath.Join(outputDir, strings.ReplaceAll(packageName, ".", string(os.PathSeparator)), filename)
}

func (n NamingStrategy) NormalizePortName(portName string) string {
	if customId, ok := n.PortMappings[portName]; ok {
		return customId
	}
	return n.PublicIdentifier(portName)
}

func (n NamingStrategy) PublicIdentifier(name string) string {
	// If all uppercase, convert to lowercase first
	if name == strings.ToUpper(name) && name != strings.ToLower(name) {
		name = strings.ToLower(name)
	}

	name = strings.Map(func(r rune) rune {
		if (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') {
			return r
		}
		return '_'
	}, name)

	parts := strings.Split(name, "_")
	var result []string
	for _, part := range parts {
		if len(part) > 0 {
			result = append(result, strings.ToUpper(part[:1])+part[1:])
		}
	}
	return strings.Join(result, "")
}

func (n NamingStrategy) PackageName(packageName string) string {
	packageName = n.NormalizePackageName(packageName)
	parts := strings.Split(packageName, ".")
	return parts[len(parts)-1]
}

func (n NamingStrategy) BuildPackagePath(packageName string) string {
	packageName = n.NormalizePackageName(packageName)
	parts := strings.Split(packageName, ".")
	if n.BasePackage != "" {
		parts = append([]string{n.BasePackage}, parts...)
	}
	return strings.Join(parts, "/")
}

func (n NamingStrategy) BaseTypeFuncName(ref string) string {
	parts := strings.Split(ref, "/")
	if len(parts) < 2 {
		return "IsUnknownType"
	}
	packages := strings.Split(parts[len(parts)-2], ".")
	packageName := packages[len(packages)-1]
	packageName = strings.ToUpper(packageName[:1]) + packageName[1:]
	typeName := parts[len(parts)-1]

	return "Is" + packageName + typeName
}

func (n NamingStrategy) BaseTypeInterfaceName(typeName string) string {
	return "I" + typeName
}

func (n NamingStrategy) PackageForPort(ws *WebService, port *WebServicePort) string {
	if len(ws.Ports) > 1 {
		portName := n.NormalizePortName(port.Name)
		return n.NormalizePackageName(ws.TargetPackage + "." + portName)
	}
	return n.NormalizePackageName(ws.TargetPackage)
}

func (n NamingStrategy) InputEnvelopeName(op *OperationDefinition) string {
	return n.PublicIdentifier(op.Name) + "Envelope"
}

func (n NamingStrategy) OutputEnvelopeName(op *OperationDefinition) string {
	return n.PublicIdentifier(op.Name) + "ResponseEnvelope"
}

func (n NamingStrategy) FaultStructName(op *OperationDefinition) string {
	return n.PublicIdentifier(op.Name) + "Fault"
}

func (n NamingStrategy) OperationVarName(op *OperationDefinition) string {
	return "Operation" + n.PublicIdentifier(op.Name)
}

func (n NamingStrategy) EnumValueName(enumName string, valueName string) string {
	return n.PublicIdentifier(enumName) + n.PublicIdentifier(valueName)
}
