package de.gematik.wsdl2openapi

import kotlinx.serialization.Serializable
import java.io.File

@Serializable
data class PackageMapping(val pattern: String, val replacement: String)

@Serializable
data class NamingStrategy(
    val packageMappings: List<PackageMapping> = emptyList(),
    val portMappings: Map<String, String> = emptyMap(),
    val basePackage: String = "",
) {
    /** Lower-cased, regex-rewritten namespace identifier (matches Go's NormalizePackageName). */
    fun normalizePackageName(packageName: String): String {
        var name = packageName
        for (m in packageMappings) {
            val regex = Regex(m.pattern)
            if (regex.containsMatchIn(name)) {
                name = regex.replace(name, m.replacement)
                break
            }
        }
        return name.lowercase()
    }

    /**
     * Dotted Kotlin package built by joining basePackage and the normalized
     * namespace.
     *
     * Tolerates Go-style basePackage values (e.g.
     * `github.com/gematik/zero-lab/go/kon/api`) so the **same** naming JSON
     * file can drive both generators: slashes become dots, hyphens are
     * dropped, and segments are lower-cased.
     */
    fun buildPackagePath(packageName: String): String {
        val normalized = normalizePackageName(packageName)
        val base = basePackage
            .replace('/', '.')
            .replace('\\', '.')
            .replace("-", "")
            .lowercase()
            .trim('.')
        return if (base.isEmpty()) normalized else "$base.$normalized"
    }

    /** Filesystem path for `<outputDir>/<package>/<filename>` — splits on dots. */
    fun filePath(outputDir: String, packageName: String, filename: String): File {
        val pkg = buildPackagePath(packageName)
        val dir = pkg.split('.').fold(File(outputDir)) { acc, seg -> File(acc, seg) }
        return File(dir, filename)
    }

    fun normalizePortName(portName: String): String =
        portMappings[portName] ?: publicIdentifier(portName)

    fun packageForPort(ws: WebService, port: WebServicePort): String =
        if (ws.ports.size > 1) {
            normalizePackageName(ws.targetPackage + "." + normalizePortName(port.name))
        } else {
            normalizePackageName(ws.targetPackage)
        }

    fun publicIdentifier(name: String): String {
        var n = name
        // If all upper-case (and not all digits/symbols), lower-case first.
        if (n == n.uppercase() && n != n.lowercase()) n = n.lowercase()
        n = n.map { c -> if (c.isLetterOrDigit()) c else '_' }.joinToString("")
        return n.split('_')
            .filter { it.isNotEmpty() }
            .joinToString("") { it.replaceFirstChar(Char::uppercaseChar) }
    }

    fun enumValueName(enumName: String, valueName: String): String =
        publicIdentifier(enumName) + publicIdentifier(valueName)

    fun inputEnvelopeName(op: OperationDefinition): String = publicIdentifier(op.name) + "Envelope"
    fun outputEnvelopeName(op: OperationDefinition): String = publicIdentifier(op.name) + "ResponseEnvelope"
    fun faultStructName(op: OperationDefinition): String = publicIdentifier(op.name) + "Fault"
    fun operationVarName(op: OperationDefinition): String = "Operation" + publicIdentifier(op.name)

    /**
     * Marker function name for a base-type interface. Mirrors Go's `IsXxxYyy` —
     * derived from the schema ref `#/components/schemas/<pkg>.<TypeName>`.
     */
    fun baseTypeFuncName(ref: String): String {
        val prefix = "#/components/schemas/"
        if (!ref.startsWith(prefix)) return "IsUnknownType"
        val (pkgFull, typeName) = splitSchemaKey(ref.removePrefix(prefix))
        if (pkgFull.isEmpty() || typeName.isEmpty()) return "IsUnknownType"
        val lastSegment = pkgFull.split('.').last()
            .replaceFirstChar(Char::uppercaseChar)
        return "Is$lastSegment$typeName"
    }
}
