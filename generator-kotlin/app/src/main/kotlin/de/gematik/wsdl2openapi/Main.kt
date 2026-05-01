package de.gematik.wsdl2openapi

import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.core.main
import com.github.ajalt.clikt.parameters.options.option
import com.github.ajalt.clikt.parameters.options.required
import java.io.File
import kotlin.system.exitProcess

class Wsdl2OpenApi2Kotlin : CliktCommand(name = "wsdl2openapi2kotlin") {
    private val inputFile by option(
        "-f", "--file", help = "OpenAPI specification file"
    ).required()
    private val outputDir by option(
        "-o", "--output", help = "Output directory for generated code"
    ).required()
    private val namingFile by option(
        "-n", "--naming", help = "Config file with naming strategy configuration"
    )

    override fun run() {
        val naming = namingFile?.let { JSON.decodeFromString(NamingStrategy.serializer(), File(it).readText()) }
            ?: NamingStrategy()
        val api = JSON.decodeFromString(Api.serializer(), File(inputFile).readText())
        Generator(outputDir, naming, api).generate()
    }
}

fun main(args: Array<String>) {
    try {
        Wsdl2OpenApi2Kotlin().main(args)
    } catch (t: Throwable) {
        System.err.println("Error: ${t.message}")
        t.printStackTrace(System.err)
        exitProcess(1)
    }
}
