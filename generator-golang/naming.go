package main

import (
	"os"
	"path/filepath"
	"strings"
)

type NamingStrategy struct {
	IdMappings  map[string]string
	BasePackage string
}

func (n NamingStrategy) NormalizeNamespaceId(namespaceId string) string {
	if customId, ok := n.IdMappings[namespaceId]; ok {
		return customId
	}
	return strings.ToLower(namespaceId)
}

func (n NamingStrategy) FilenameForTypes(outputDir string, namespaceId string) string {
	namespaceId = n.NormalizeNamespaceId(namespaceId)
	return filepath.Join(outputDir, strings.ReplaceAll(namespaceId, ".", string(os.PathSeparator)), "types.go")
}

func (n NamingStrategy) PublicFieldName(propertyName string) string {
	return strings.ToUpper(propertyName[:1]) + propertyName[1:]
}

func (n NamingStrategy) PackageName(namespaceId string) string {
	namespaceId = n.NormalizeNamespaceId(namespaceId)
	parts := strings.Split(namespaceId, ".")
	return parts[len(parts)-1]
}

func (n NamingStrategy) BuildPackagePath(namespaceId string) string {
	namespaceId = n.NormalizeNamespaceId(namespaceId)
	parts := strings.Split(namespaceId, ".")
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
