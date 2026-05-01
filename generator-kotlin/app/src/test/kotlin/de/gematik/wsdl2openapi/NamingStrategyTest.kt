package de.gematik.wsdl2openapi

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

class NamingStrategyTest {

    private val konMappings = NamingStrategy(
        basePackage = "de.gematik.zerolab.kon",
        packageMappings = listOf(
            PackageMapping("""de\.gematik\.ws\.(.*)""", "gematik.\$1"),
            PackageMapping("""oasis\.names\.tc\.([^.]+)\.([^.]+)""", "oasis.\$1\$2"),
            PackageMapping("""org\.(.*)""", "\$1"),
        )
    )

    @Test fun `applies first matching package mapping`() {
        assertEquals(
            "gematik.conn.certificateservice60",
            konMappings.normalizePackageName("de.gematik.ws.conn.CertificateService60")
        )
    }

    @Test fun `lowercases unmatched package names`() {
        assertEquals("foo.bar.baz", konMappings.normalizePackageName("Foo.Bar.Baz"))
    }

    @Test fun `oasis pattern collapses two segments`() {
        assertEquals(
            "oasis.dss10core",
            konMappings.normalizePackageName("oasis.names.tc.dss10.core")
        )
    }

    @Test fun `buildPackagePath prefixes basePackage`() {
        assertEquals(
            "de.gematik.zerolab.kon.gematik.conn.certificateservice60",
            konMappings.buildPackagePath("de.gematik.ws.conn.CertificateService60")
        )
    }

    @Test fun `buildPackagePath tolerates Go-style basePackage`() {
        // Same naming JSON used for the Go generator should still produce a
        // valid Kotlin package: slashes → dots, hyphens dropped, lowercased.
        val goStyle = NamingStrategy(
            basePackage = "github.com/gematik/zero-lab/go/kon/api",
            packageMappings = konMappings.packageMappings,
        )
        assertEquals(
            "github.com.gematik.zerolab.go.kon.api.gematik.conn.certificateservice60",
            goStyle.buildPackagePath("de.gematik.ws.conn.CertificateService60")
        )
    }

    @Test fun `filePath splits dots into directories`() {
        val file = konMappings.filePath("/tmp/out", "de.gematik.ws.conn.CertificateService60", "Foo.kt")
        assertEquals(
            "/tmp/out/de/gematik/zerolab/kon/gematik/conn/certificateservice60/Foo.kt",
            file.path
        )
    }

    @Test fun `publicIdentifier capitalizes parts`() {
        assertEquals("FooBarBaz", konMappings.publicIdentifier("foo_bar-baz"))
    }

    @Test fun `publicIdentifier downcases all-caps inputs first`() {
        assertEquals("Url", konMappings.publicIdentifier("URL"))
    }

    @Test fun `enumValueName concatenates type and value identifier`() {
        assertEquals("VerificationResultTypeValid", konMappings.enumValueName("VerificationResultType", "VALID"))
    }

    @Test fun `portMappings overrides default identifier`() {
        val n = NamingStrategy(portMappings = mapOf("MySoap11Port" to "Default"))
        assertEquals("Default", n.normalizePortName("MySoap11Port"))
        assertEquals("OtherPort", n.normalizePortName("OtherPort"))
    }

    @Test fun `packageForPort appends port name when service has multiple ports`() {
        val ws = WebService(
            name = "Svc", targetNamespace = "ns", targetPackage = "pkg.X",
            ports = listOf(
                WebServicePort("a", BindingType.SOAP11),
                WebServicePort("b", BindingType.SOAP11),
            )
        )
        val n = NamingStrategy()
        assertEquals("pkg.x.a", n.packageForPort(ws, ws.ports[0]))
        assertEquals("pkg.x.b", n.packageForPort(ws, ws.ports[1]))
    }

    @Test fun `packageForPort uses target package only when single port`() {
        val ws = WebService(
            name = "Svc", targetNamespace = "ns", targetPackage = "pkg.X",
            ports = listOf(WebServicePort("only", BindingType.SOAP11)),
        )
        val n = NamingStrategy()
        assertEquals("pkg.x", n.packageForPort(ws, ws.ports[0]))
    }

    @Test fun `baseTypeFuncName derives marker name from ref`() {
        assertEquals(
            "IsCertificateService60Document",
            konMappings.baseTypeFuncName("#/components/schemas/de.gematik.ws.conn.CertificateService60.Document")
        )
    }

    @Test fun `splitSchemaKey returns last segment as type`() {
        assertEquals(
            "de.gematik.ws.conn.SignatureService74" to "SignDocument",
            splitSchemaKey("de.gematik.ws.conn.SignatureService74.SignDocument")
        )
    }
}
