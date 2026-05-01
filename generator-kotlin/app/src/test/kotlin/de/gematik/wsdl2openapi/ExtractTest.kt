package de.gematik.wsdl2openapi

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class ExtractTest {

    private val naming = NamingStrategy(basePackage = "test")

    private fun apiOf(json: String): Api =
        Json { ignoreUnknownKeys = true; explicitNulls = false }
            .decodeFromString(Api.serializer(), json)

    @Test fun `extracts inline object property to top-level type`() {
        val api = apiOf(
            """
            {
              "openapi": "3.0.0",
              "components": {
                "schemas": {
                  "demo.Parent": {
                    "type": "object",
                    "properties": {
                      "child": {
                        "type": "object",
                        "properties": {
                          "name": { "type": "string" }
                        }
                      }
                    }
                  }
                }
              }
            }
            """.trimIndent()
        )
        val rewritten = extractInlineObjects(api, naming)
        val schemas = rewritten.components.schemas
        assertTrue(schemas.containsKey("demo.ParentChild"), "extracted type was not added")
        // Original property must now be a $ref pointing at the new type.
        val parent = schemas["demo.Parent"] as JsonObject
        val props = parent["properties"] as JsonObject
        val child = props["child"] as JsonObject
        assertEquals(
            "#/components/schemas/demo.ParentChild",
            child["\$ref"]?.toString()?.trim('"')
        )
    }

    @Test fun `extracts inline array items into top-level type`() {
        val api = apiOf(
            """
            {
              "openapi": "3.0.0",
              "components": {
                "schemas": {
                  "demo.Parent": {
                    "type": "object",
                    "properties": {
                      "items": {
                        "type": "array",
                        "items": {
                          "type": "object",
                          "properties": { "id": { "type": "string" } }
                        }
                      }
                    }
                  }
                }
              }
            }
            """.trimIndent()
        )
        val rewritten = extractInlineObjects(api, naming)
        assertTrue(rewritten.components.schemas.containsKey("demo.ParentItems"))
        val parent = rewritten.components.schemas["demo.Parent"] as JsonObject
        val props = parent["properties"] as JsonObject
        val items = (props["items"] as JsonObject)["items"] as JsonObject
        assertEquals(
            "#/components/schemas/demo.ParentItems",
            items["\$ref"]?.toString()?.trim('"')
        )
    }

    @Test fun `leaves primitive properties untouched`() {
        val api = apiOf(
            """
            {
              "openapi": "3.0.0",
              "components": {
                "schemas": {
                  "demo.Parent": {
                    "type": "object",
                    "properties": {
                      "id": { "type": "string" }
                    }
                  }
                }
              }
            }
            """.trimIndent()
        )
        val rewritten = extractInlineObjects(api, naming)
        // No new keys added.
        assertEquals(setOf("demo.Parent"), rewritten.components.schemas.keys)
    }

    @Test fun `preserves xml extension on extracted ref placeholder`() {
        val api = apiOf(
            """
            {
              "openapi": "3.0.0",
              "components": {
                "schemas": {
                  "demo.Parent": {
                    "type": "object",
                    "properties": {
                      "child": {
                        "type": "object",
                        "xml": { "name": "Child", "namespace": "urn:demo" },
                        "properties": { "name": { "type": "string" } }
                      }
                    }
                  }
                }
              }
            }
            """.trimIndent()
        )
        val rewritten = extractInlineObjects(api, naming)
        val parent = rewritten.components.schemas["demo.Parent"] as JsonObject
        val props = parent["properties"] as JsonObject
        val child = props["child"] as JsonObject
        // The replacement $ref node must carry forward the xml extension so
        // the parent class can still render the expected element name.
        val xml = child["xml"] as JsonObject
        assertEquals("\"Child\"", xml["name"].toString())
    }
}
