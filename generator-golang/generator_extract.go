package generator

import (
	"encoding/json"
	"fmt"
)

// ExtractInlineObjects is a pre-processing step that walks all schema types and
// extracts inline object properties into separate named global types. An inline
// object property (type: "object" with properties) gets replaced with a $ref to
// the newly created type. This also handles arrays whose items are inline objects.
//
// Must be called before Validate/GenerateTypes.
func (g *Generator) ExtractInlineObjects() {
	for _, packageName := range g.Api.Components.Schemas.Keys {
		schema := g.Api.Components.Schemas.Entries[packageName]

		// Use index-based iteration so newly appended types also get processed.
		processed := 0
		for processed < len(schema.Keys) {
			typeName := schema.Keys[processed]
			typeRaw := schema.Entries[typeName]

			if updated := g.extractInlineProps(packageName, typeName, typeRaw, &schema); updated != nil {
				schema.Entries[typeName] = updated
			}

			processed++
		}

		g.Api.Components.Schemas.Entries[packageName] = schema
	}
}

// extractInlineProps processes a single type definition and extracts any inline
// object properties into separate named types. Returns the modified type JSON,
// or nil if no changes were made.
func (g *Generator) extractInlineProps(packageName, parentTypeName string, typeRaw json.RawMessage, schema *OrderedMap[json.RawMessage]) json.RawMessage {
	var obj map[string]json.RawMessage
	if err := json.Unmarshal(typeRaw, &obj); err != nil {
		return nil
	}

	propsRaw, hasProps := obj["properties"]
	if !hasProps {
		return nil
	}

	if typeStr, ok := jsonString(obj["type"]); !ok || typeStr != "object" {
		return nil
	}

	var props OrderedMap[json.RawMessage]
	if err := json.Unmarshal(propsRaw, &props); err != nil {
		return nil
	}

	changed := false
	for _, propName := range props.Keys {
		propRaw := props.Entries[propName]

		var propObj map[string]json.RawMessage
		if err := json.Unmarshal(propRaw, &propObj); err != nil {
			continue
		}

		propType, ok := jsonString(propObj["type"])
		if !ok {
			continue
		}

		switch propType {
		case "object":
			if _, hasInnerProps := propObj["properties"]; !hasInnerProps {
				continue
			}
			newTypeName := parentTypeName + g.NamingStrategy.PublicIdentifier(propName)
			ref := fmt.Sprintf("#/components/schemas/%s/%s", packageName, newTypeName)

			// New global type: everything except nullable
			newType := make(map[string]json.RawMessage, len(propObj))
			for k, v := range propObj {
				if k != "nullable" {
					newType[k] = v
				}
			}
			newTypeJSON, _ := json.Marshal(newType)
			schema.Keys = append(schema.Keys, newTypeName)
			schema.Entries[newTypeName] = json.RawMessage(newTypeJSON)

			// Replacement $ref property: preserve xml and nullable
			replacement := map[string]json.RawMessage{
				"$ref": mustMarshalJSON(ref),
			}
			if v, ok := propObj["xml"]; ok {
				replacement["xml"] = v
			}
			if v, ok := propObj["nullable"]; ok {
				replacement["nullable"] = v
			}
			replacementJSON, _ := json.Marshal(replacement)
			props.Entries[propName] = json.RawMessage(replacementJSON)
			changed = true

		case "array":
			itemsRaw, hasItems := propObj["items"]
			if !hasItems {
				continue
			}

			var itemsObj map[string]json.RawMessage
			if err := json.Unmarshal(itemsRaw, &itemsObj); err != nil {
				continue
			}

			itemsType, ok := jsonString(itemsObj["type"])
			if !ok || itemsType != "object" {
				continue
			}

			if _, hasItemProps := itemsObj["properties"]; !hasItemProps {
				continue
			}

			newTypeName := parentTypeName + g.NamingStrategy.PublicIdentifier(propName)
			ref := fmt.Sprintf("#/components/schemas/%s/%s", packageName, newTypeName)

			// New global type from items (without nullable)
			newType := make(map[string]json.RawMessage, len(itemsObj))
			for k, v := range itemsObj {
				if k != "nullable" {
					newType[k] = v
				}
			}
			newTypeJSON, _ := json.Marshal(newType)
			schema.Keys = append(schema.Keys, newTypeName)
			schema.Entries[newTypeName] = json.RawMessage(newTypeJSON)

			// Replace items with $ref
			newItemsJSON, _ := json.Marshal(map[string]json.RawMessage{
				"$ref": mustMarshalJSON(ref),
			})
			propObj["items"] = json.RawMessage(newItemsJSON)
			propJSON, _ := json.Marshal(propObj)
			props.Entries[propName] = json.RawMessage(propJSON)
			changed = true
		}
	}

	if !changed {
		return nil
	}

	newPropsJSON, _ := json.Marshal(&props)
	obj["properties"] = json.RawMessage(newPropsJSON)
	result, _ := json.Marshal(obj)
	return json.RawMessage(result)
}

func jsonString(raw json.RawMessage) (string, bool) {
	if raw == nil {
		return "", false
	}
	var s string
	if err := json.Unmarshal(raw, &s); err != nil {
		return "", false
	}
	return s, true
}

func mustMarshalJSON(v any) json.RawMessage {
	b, err := json.Marshal(v)
	if err != nil {
		panic(err)
	}
	return json.RawMessage(b)
}
