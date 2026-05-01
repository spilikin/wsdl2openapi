package de.gematik.wsdl2openapi

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/**
 * Walk all schema entries and lift inline `type: object` properties (and array
 * items) into separately-named global types, replacing the inline definition
 * with a `$ref`. Mirrors `ExtractInlineObjects` in the Go generator.
 *
 * Returns a new schemas [JsonObject] with the rewrites applied; insertion order
 * is preserved and newly-created types are appended at the end.
 */
fun extractInlineObjects(api: Api, naming: NamingStrategy): Api {
    val schemas = LinkedHashMap<String, JsonObject>()
    api.components.schemas.entries.forEach { (k, v) -> schemas[k] = v as JsonObject }

    // Use index-based iteration so newly appended types also get processed.
    val keys = ArrayList(schemas.keys)
    var i = 0
    while (i < keys.size) {
        val key = keys[i]
        val (pkgName, parentTypeName) = splitSchemaKey(key)
        val updated = extractInlineProps(pkgName, parentTypeName, schemas[key]!!, schemas, keys, naming)
        if (updated != null) schemas[key] = updated
        i++
    }

    return api.copy(
        components = Components(
            schemas = JsonObject(schemas)
        )
    )
}

private fun extractInlineProps(
    pkgName: String,
    parentTypeName: String,
    typeJson: JsonObject,
    schemas: LinkedHashMap<String, JsonObject>,
    keysOrder: ArrayList<String>,
    naming: NamingStrategy,
): JsonObject? {
    if (typeJson["type"]?.stringOrNull() != "object") return null
    val props = typeJson["properties"] as? JsonObject ?: return null

    val newProps = LinkedHashMap<String, kotlinx.serialization.json.JsonElement>()
    var changed = false

    for ((propName, propEl) in props.entries) {
        val propObj = propEl as? JsonObject
        if (propObj == null) {
            newProps[propName] = propEl
            continue
        }

        val propType = propObj["type"]?.stringOrNull()
        when (propType) {
            "object" -> {
                if (propObj["properties"] !is JsonObject) {
                    newProps[propName] = propObj
                    continue
                }
                val newTypeName = parentTypeName + naming.publicIdentifier(propName)
                val newKey = pkgName + SCHEMA_KEY_SEPARATOR + newTypeName
                val ref = "#/components/schemas/$newKey"

                schemas[newKey] = propObj
                keysOrder.add(newKey)

                newProps[propName] = buildJsonObject {
                    put("\$ref", ref)
                    propObj["xml"]?.let { put("xml", it) }
                }
                changed = true
            }
            "array" -> {
                val items = propObj["items"] as? JsonObject
                if (items == null || items["type"]?.stringOrNull() != "object" || items["properties"] !is JsonObject) {
                    newProps[propName] = propObj
                    continue
                }
                val newTypeName = parentTypeName + naming.publicIdentifier(propName)
                val newKey = pkgName + SCHEMA_KEY_SEPARATOR + newTypeName
                val ref = "#/components/schemas/$newKey"

                schemas[newKey] = items
                keysOrder.add(newKey)

                val newPropObj = JsonObject(
                    propObj.toMutableMap().apply {
                        this["items"] = buildJsonObject { put("\$ref", ref) }
                    }
                )
                newProps[propName] = newPropObj
                changed = true
            }
            else -> newProps[propName] = propObj
        }
    }

    if (!changed) return null

    return JsonObject(
        typeJson.toMutableMap().apply { this["properties"] = JsonObject(newProps) }
    )
}
