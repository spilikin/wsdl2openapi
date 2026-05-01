package de.gematik.wsdl2openapi

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

@Serializable
enum class BindingType {
    @SerialName("soap11") SOAP11,
    @SerialName("soap12") SOAP12,
    @SerialName("http-get") HTTP_GET,
    @SerialName("http-post") HTTP_POST,
    @SerialName("other") OTHER,
}

@Serializable
data class WebServiceMessage(
    val soapBody: String,
    val soapHeaders: List<String> = emptyList(),
)

@Serializable
data class OperationDefinition(
    val name: String,
    val soapAction: String = "",
    val style: String = "",
    val input: WebServiceMessage,
    val output: WebServiceMessage,
    val faults: Map<String, WebServiceMessage> = emptyMap(),
    val inputHeaders: Map<String, WebServiceMessage> = emptyMap(),
    val outputHeaders: Map<String, WebServiceMessage> = emptyMap(),
)

@Serializable
data class WebServicePort(
    val name: String,
    val bindingType: BindingType,
    val address: String = "",
    val operations: List<OperationDefinition> = emptyList(),
)

@Serializable
data class WebService(
    val name: String,
    val targetNamespace: String,
    val targetPackage: String,
    val ports: List<WebServicePort> = emptyList(),
)

@Serializable
data class Components(
    val schemas: JsonObject = JsonObject(emptyMap()),
)

@Serializable
data class Info(
    val title: String = "",
    val version: String = "",
    val description: String = "",
)

@Serializable
data class Api(
    val openapi: String = "",
    @SerialName("x-wsdl-extension") val wsdlExtensionVersion: String = "",
    @SerialName("x-wsdl-version") val wsdlVersion: String = "",
    val info: Info? = null,
    @SerialName("x-wsdl-services") val webServices: List<WebService> = emptyList(),
    val components: Components = Components(),
)

@Serializable
data class XmlExtension(
    val name: String = "",
    val namespace: String = "",
    val prefix: String = "",
    val attribute: Boolean = false,
    val wrapped: Boolean = false,
    @SerialName("x-extends") val extends: List<String> = emptyList(),
    @SerialName("x-is-base") val isBase: Boolean = false,
)

val JSON: Json = Json {
    ignoreUnknownKeys = true
    explicitNulls = false
    encodeDefaults = true
}

const val SCHEMA_KEY_SEPARATOR = "."

/** Splits a flat schema key into (namespaceId, typeName) by the **last** separator. */
fun splitSchemaKey(key: String): Pair<String, String> {
    val idx = key.lastIndexOf(SCHEMA_KEY_SEPARATOR)
    if (idx < 0) return "" to key
    return key.substring(0, idx) to key.substring(idx + SCHEMA_KEY_SEPARATOR.length)
}

/**
 * Lightweight, lazily-parsed view over a single component schema entry. We keep
 * the raw [JsonObject] around so property ordering is preserved (LinkedHashMap)
 * and so we can resolve `$ref` chains without committing to a closed sealed
 * hierarchy upfront.
 */
data class TypeView(val raw: JsonObject) {
    val ref: String? get() = raw["\$ref"]?.stringOrNull()
    val type: String? get() = raw["type"]?.stringOrNull()
    val xml: XmlExtension? get() = raw["xml"]?.let { JSON.decodeFromJsonElement(XmlExtension.serializer(), it) }
    val description: String? get() = raw["description"]?.stringOrNull()
    val format: String? get() = raw["format"]?.stringOrNull()
    val pattern: String? get() = raw["pattern"]?.stringOrNull()
    val enum: List<String>? get() = (raw["enum"] as? JsonArray)?.mapNotNull { it.stringOrNull() }
    val properties: JsonObject? get() = raw["properties"] as? JsonObject
    val required: List<String> get() = (raw["required"] as? JsonArray)?.mapNotNull { it.stringOrNull() } ?: emptyList()
    val items: JsonElement? get() = raw["items"]
    val nullable: Boolean? get() = (raw["nullable"] as? JsonPrimitive)?.booleanOrNull
    val isPrimitive: Boolean get() = type in setOf("string", "number", "integer", "boolean")
    val isObject: Boolean get() = type == "object"
    val isArray: Boolean get() = type == "array"
}

internal fun JsonElement.stringOrNull(): String? =
    (this as? JsonPrimitive)?.takeIf { it !is kotlinx.serialization.json.JsonNull }?.content

private val JsonPrimitive.booleanOrNull: Boolean? get() = content.toBooleanStrictOrNull()
