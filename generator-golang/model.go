package generator

import (
	"bytes"
	"encoding/json"
	"fmt"
	"strings"
)

type BindingType string

const (
	SOAP11    BindingType = "soap11"
	SOAP12    BindingType = "soap12"
	HTTP_GET  BindingType = "http-get"
	HTTP_POST BindingType = "http-post"
	OTHER     BindingType = "other"
)

type OrderedMap[T any] struct {
	Keys    []string
	Entries map[string]T
}

// UnmarshalJSON implements json.Unmarshaler interface
func (om *OrderedMap[T]) UnmarshalJSON(data []byte) error {
	dec := json.NewDecoder(bytes.NewReader(data))

	// Read opening brace
	token, err := dec.Token()
	if err != nil {
		return err
	}
	if delim, ok := token.(json.Delim); !ok || delim != '{' {
		return fmt.Errorf("expected '{', got %v", token)
	}

	om.Keys = make([]string, 0)
	om.Entries = make(map[string]T)

	for dec.More() {
		// Read key
		token, err := dec.Token()
		if err != nil {
			return err
		}
		key := token.(string)

		// Read value
		var value T
		if err := dec.Decode(&value); err != nil {
			return err
		}

		om.Keys = append(om.Keys, key)
		om.Entries[key] = value
	}

	// Read closing brace
	_, err = dec.Token()
	return err
}

// MarshalJSON implements json.Marshaler interface
func (om *OrderedMap[T]) MarshalJSON() ([]byte, error) {
	var buf bytes.Buffer
	buf.WriteString("{")

	for i, key := range om.Keys {
		if i > 0 {
			buf.WriteString(",")
		}

		// Marshal key
		keyBytes, err := json.Marshal(key)
		if err != nil {
			return nil, err
		}
		buf.Write(keyBytes)
		buf.WriteString(":")

		// Marshal value
		valBytes, err := json.Marshal(om.Entries[key])
		if err != nil {
			return nil, err
		}
		buf.Write(valBytes)
	}

	buf.WriteString("}")
	return buf.Bytes(), nil
}

type WebServiceMessage struct {
	SOAPBody    string   `json:"soapBody"`
	SOAPHeaders []string `json:"soapHeaders,omitempty"`
}

type OperationDefinition struct {
	Name          string                       `json:"name"`
	SOAPAction    string                       `json:"soapAction,omitempty"`
	Style         string                       `json:"style,omitempty"`
	Input         WebServiceMessage            `json:"input"`
	Output        WebServiceMessage            `json:"output"`
	Faults        map[string]WebServiceMessage `json:"faults,omitempty"`
	InputHeaders  map[string]WebServiceMessage `json:"inputHeaders,omitempty"`
	OutputHeaders map[string]WebServiceMessage `json:"outputHeaders,omitempty"`
}

type WebServicePort struct {
	Name        string                `json:"name"`
	BindingType BindingType           `json:"bindingType"`
	Address     string                `json:"address,omitempty"`
	Operations  []OperationDefinition `json:"operations"`
}

type WebService struct {
	Name            string           `json:"name"`
	TargetNamespace string           `json:"targetNamespace"`
	TargetPackage   string           `json:"targetPackage"`
	Ports           []WebServicePort `json:"ports"`
}

type Components struct {
	Schemas Schemas `json:"schemas,omitempty"`
}

type Schemas = OrderedMap[json.RawMessage]

// SchemaKeySeparator separates the namespace identifier from the type name in
// flat component schema keys. The last separator wins (so a key like
// "de.gematik.ws.conn.SignatureService74.SignDocument" splits into the
// namespace "de.gematik.ws.conn.SignatureService74" and the type "SignDocument").
const SchemaKeySeparator = "."

// SplitSchemaKey returns (namespace, typeName) for a flat schema key by
// splitting on the last separator. If the separator is missing it returns
// ("", key).
func SplitSchemaKey(key string) (string, string) {
	idx := strings.LastIndex(key, SchemaKeySeparator)
	if idx < 0 {
		return "", key
	}
	return key[:idx], key[idx+len(SchemaKeySeparator):]
}

type License struct {
	Name       string `json:"name"`
	Identifier string `json:"identifier,omitempty"`
	URL        string `json:"url,omitempty"`
}

type Contact struct {
	Name  string `json:"name,omitempty"`
	Email string `json:"email,omitempty"`
	URL   string `json:"url,omitempty"`
}

type Info struct {
	Title          string   `json:"title"`
	Version        string   `json:"version"`
	Description    string   `json:"description,omitempty"`
	TermsOfService string   `json:"termsOfService,omitempty"`
	License        *License `json:"license,omitempty"`
	Contact        *Contact `json:"contact,omitempty"`
}

type Operation struct {
	Summary     string   `json:"summary,omitempty"`
	Description string   `json:"description,omitempty"`
	OperationID string   `json:"operationId,omitempty"`
	Tags        []string `json:"tags,omitempty"`
}

type PathItem struct {
	Post    *Operation `json:"post,omitempty"`
	Get     *Operation `json:"get,omitempty"`
	Put     *Operation `json:"put,omitempty"`
	Delete  *Operation `json:"delete,omitempty"`
	Options *Operation `json:"options,omitempty"`
	Head    *Operation `json:"head,omitempty"`
	Patch   *Operation `json:"patch,omitempty"`
	Trace   *Operation `json:"trace,omitempty"`
}

type Api struct {
	OpenAPI              string              `json:"openapi"`
	WSDLExtensionVersion string              `json:"x-wsdl-extension"`
	Info                 Info                `json:"info"`
	WebServices          []WebService        `json:"x-wsdl-services,omitempty"`
	Paths                map[string]PathItem `json:"paths,omitempty"`
	Components           Components          `json:"components"`
}

type XmlExtension struct {
	Name      string   `json:"name"`
	Namespace string   `json:"namespace,omitempty"`
	Prefix    string   `json:"prefix,omitempty"`
	Attribute bool     `json:"attribute,omitempty"`
	Wrapped   bool     `json:"wrapped,omitempty"`
	Extends   []string `json:"x-extends,omitempty"`
	IsBase    bool     `json:"x-is-base,omitempty"`
}

type ReferenceObject struct {
	Ref         string        `json:"$ref"`
	Xml         *XmlExtension `json:"xml,omitempty"`
	Summary     *string       `json:"summary,omitempty"`
	Description *string       `json:"description,omitempty"`
	Nullable    *bool         `json:"nullable,omitempty"`
}

type TypeDefinition interface {
	GetType() Type
	IsPrimitive() bool
}

// TypeHasFacets returns true if the given type has facets (e.g. enum values, pattern, etc.)
// that should be preserved in the generated code, even if the type is a primitive type
// at the moment we only support enums as facets for primitive types,
// but this can be extended in the future to support other facets as well
func TypeHasFacets(t Type) bool {
	return t.Enum != nil
}

type Type struct {
	Xml         *XmlExtension `json:"xml,omitempty"`
	Type        string        `json:"type"`
	Description *string       `json:"description,omitempty"`
	Nullable    *bool         `json:"nullable,omitempty"`
	Enum        []string      `json:"enum,omitempty"`
}

type TypeObject struct {
	Type
	Properties OrderedMap[json.RawMessage] `json:"properties,omitempty"`
	Required   []string                    `json:"required,omitempty"`
}

func (t TypeObject) GetType() Type {
	return t.Type
}

func (t TypeObject) IsPrimitive() bool {
	return false
}

type TypeString struct {
	Type
	Pattern          string `json:"pattern,omitempty"`
	MinLength        int    `json:"minLength,omitempty"`
	MaxLength        int    `json:"maxLength,omitempty"`
	Format           string `json:"format,omitempty"`
	ContentMediaType string `json:"contentMediaType,omitempty"`
	ContentEncoding  string `json:"contentEncoding,omitempty"`
}

func (t TypeString) GetType() Type {
	return t.Type
}

func (t TypeString) IsPrimitive() bool {
	return true
}

type TypeNumber struct {
	Type
	MultipleOf       float64 `json:"multipleOf,omitempty"`
	Maximum          float64 `json:"maximum,omitempty"`
	ExclusiveMaximum float64 `json:"exclusiveMaximum,omitempty"`
	Minimum          float64 `json:"minimum,omitempty"`
	ExclusiveMinimum float64 `json:"exclusiveMinimum,omitempty"`
}

func (t TypeNumber) GetType() Type {
	return t.Type
}

func (t TypeNumber) IsPrimitive() bool {
	return true
}

type TypeBoolean struct {
	Type
}

func (t TypeBoolean) GetType() Type {
	return t.Type
}

func (t TypeBoolean) IsPrimitive() bool {
	return true
}

type TypeInteger struct {
	TypeNumber
}

func (t TypeInteger) GetType() Type {
	return t.Type
}

func (t TypeInteger) IsPrimitive() bool {
	return true
}

type TypeNull struct {
	Type
}

func (t TypeNull) GetType() Type {
	return t.Type
}

func (t TypeNull) IsPrimitive() bool {
	return true
}

type TypeArray struct {
	Type
	Items json.RawMessage `json:"items"`
}

func (t TypeArray) GetType() Type {
	return t.Type
}

func (t TypeArray) IsPrimitive() bool {
	return false
}
