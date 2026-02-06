package main

import (
	"os"
	"path/filepath"
	"strings"
)

type NamingStrategy struct {
	PackageMappings map[string]string
	ServiceMappings map[string]string
	PortMappings    map[string]string
	BasePackage     string
	Soap11Package   string
	Soap12Package   string
}

func NewNamingStrategy() NamingStrategy {
	return NamingStrategy{
		PackageMappings: map[string]string{},
		ServiceMappings: map[string]string{},
		PortMappings:    map[string]string{},
	}
}

func (n NamingStrategy) NormalizePackageName(packageName string) string {
	if customId, ok := n.PackageMappings[packageName]; ok {
		return customId
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
	return n.PublicIdentifier(op.Name) + "InputEnvelope"
}

func (n NamingStrategy) InputeHeadersName(op *OperationDefinition) string {
	return n.PublicIdentifier(op.Name) + "InputHeaders"
}

func (n NamingStrategy) InputBodyName(op *OperationDefinition) string {
	return n.PublicIdentifier(op.Name) + "InputBody"
}

func (n NamingStrategy) OutputEnvelopeName(op *OperationDefinition) string {
	return n.PublicIdentifier(op.Name) + "OutputEnvelope"
}

func (n NamingStrategy) OutpuHeadersName(op *OperationDefinition) string {
	return n.PublicIdentifier(op.Name) + "OutputHeaders"
}

func (n NamingStrategy) OutputBodyName(op *OperationDefinition) string {
	return n.PublicIdentifier(op.Name) + "OutputBody"
}
