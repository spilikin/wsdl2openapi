use std::sync::LazyLock;

use anyhow::{Context, Result};
use heck::{ToSnakeCase, ToUpperCamelCase};
use indexmap::IndexMap;
use regex::Regex;
use serde::Deserialize;

use crate::model::{WebService, WebServicePort};

#[derive(Debug, Clone, Deserialize)]
pub struct PackageMapping {
    pub pattern: String,
    pub replacement: String,
}

/// The naming JSON shared with the Go and Kotlin generators. `basePackage`
/// is accepted but unused: `--module-root` plays that role for Rust.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NamingConfig {
    pub package_mappings: Vec<PackageMapping>,
    pub port_mappings: IndexMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct NamingStrategy {
    package_mappings: Vec<(Regex, String)>,
    port_mappings: IndexMap<String, String>,
}

impl NamingStrategy {
    pub fn from_json(json: &str) -> Result<Self> {
        let config: NamingConfig = serde_json::from_str(json).context("invalid naming config")?;
        Self::new(config)
    }

    pub fn new(config: NamingConfig) -> Result<Self> {
        let package_mappings = config
            .package_mappings
            .into_iter()
            .map(|m| {
                let regex = Regex::new(&m.pattern)
                    .with_context(|| format!("invalid package mapping pattern {:?}", m.pattern))?;
                Ok((regex, braced_group_refs(&m.replacement)))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            package_mappings,
            port_mappings: config.port_mappings,
        })
    }

    /// Applies the first matching package mapping and lower-cases the result.
    pub fn normalize_package_name(&self, package: &str) -> String {
        let mapped = self
            .package_mappings
            .iter()
            .find(|(regex, _)| regex.is_match(package))
            .map(|(regex, replacement)| {
                regex
                    .replace_all(package, replacement.as_str())
                    .into_owned()
            })
            .unwrap_or_else(|| package.to_owned());
        mapped.to_lowercase()
    }

    /// Rust module path (relative to the module root) for a schema package.
    pub fn module_path(&self, package: &str) -> Vec<String> {
        self.normalize_package_name(package)
            .split('.')
            .filter(|segment| !segment.is_empty())
            .map(module_ident)
            .collect()
    }

    /// Unnormalized package the port's SOAP items live in: the service's
    /// target package, plus the port name when the service has several ports.
    pub fn package_for_port(&self, service: &WebService, port: &WebServicePort) -> String {
        if service.ports.len() > 1 {
            format!(
                "{}.{}",
                service.target_package,
                self.port_type_name(&port.name)
            )
        } else {
            service.target_package.clone()
        }
    }

    pub fn port_type_name(&self, port: &str) -> String {
        self.port_mappings
            .get(port)
            .cloned()
            .unwrap_or_else(|| type_ident(port))
    }
}

/// Rewrites `$1` to `${1}` so that Go/Java style replacements like `$1$2` or
/// `$1abc` keep their meaning under the `regex` crate, which would otherwise
/// read `$1abc` as a named group.
fn braced_group_refs(replacement: &str) -> String {
    static GROUP_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$(\d+)").unwrap());
    GROUP_REF.replace_all(replacement, "$${${1}}").into_owned()
}

/// Port of the Go/Kotlin `PublicIdentifier`, kept for names that must match
/// the other generators (lifted inline types).
pub fn public_identifier(name: &str) -> String {
    let lowered;
    let name = if name == name.to_uppercase() && name != name.to_lowercase() {
        lowered = name.to_lowercase();
        &lowered
    } else {
        name
    };
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect()
}

const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
    "virtual", "where", "while", "yield",
];

/// Keywords that cannot be written as raw identifiers.
const RESERVED: &[&str] = &["crate", "self", "Self", "super", "_"];

/// Makes `name` usable as an identifier: raw identifiers for keywords, a
/// trailing underscore for the few that cannot be raw.
fn escape_keyword(name: String) -> String {
    if RESERVED.contains(&name.as_str()) {
        format!("{name}_")
    } else if KEYWORDS.contains(&name.as_str()) {
        format!("r#{name}")
    } else {
        name
    }
}

fn prefix_leading_digit(name: String, prefix: &str) -> String {
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("{prefix}{name}")
    } else {
        name
    }
}

pub fn type_ident(name: &str) -> String {
    let ident = prefix_leading_digit(name.to_upper_camel_case(), "T");
    escape_keyword(if ident.is_empty() {
        "Type".into()
    } else {
        ident
    })
}

pub fn field_ident(name: &str) -> String {
    let ident = prefix_leading_digit(name.to_snake_case(), "_");
    escape_keyword(if ident.is_empty() {
        "value".into()
    } else {
        ident
    })
}

pub fn variant_ident(value: &str) -> String {
    let ident = prefix_leading_digit(value.to_upper_camel_case(), "V");
    escape_keyword(if ident.is_empty() {
        "Empty".into()
    } else {
        ident
    })
}

/// Module segments are already lower-cased by the package mappings; only
/// characters Rust rejects are replaced, so `dss10core` stays `dss10core`.
pub fn module_ident(segment: &str) -> String {
    let sanitized: String = segment
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    escape_keyword(prefix_leading_digit(sanitized, "_"))
}

/// Strips the `r#` marker from an escaped identifier.
pub fn unraw(ident: &str) -> &str {
    ident.strip_prefix("r#").unwrap_or(ident)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BindingType;

    fn kon() -> NamingStrategy {
        NamingStrategy::from_json(
            r#"{
              "basePackage": "github.com/test/testproj/kon/api",
              "packageMappings": [
                {"pattern": "de\\.gematik\\.ws\\.(.*)", "replacement": "gematik.$1"},
                {"pattern": "oasis\\.names\\.tc\\.([^.]+)\\.([^.]+)", "replacement": "oasis.$1$2"},
                {"pattern": "org\\.(.*)", "replacement": "$1"}
              ]
            }"#,
        )
        .unwrap()
    }

    fn service(ports: &[&str]) -> WebService {
        WebService {
            name: "Svc".into(),
            target_package: "pkg.X".into(),
            ports: ports
                .iter()
                .map(|name| WebServicePort {
                    name: (*name).into(),
                    binding_type: BindingType::Soap11,
                    operations: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn applies_first_matching_mapping() {
        assert_eq!(
            kon().normalize_package_name("de.gematik.ws.conn.CertificateService60"),
            "gematik.conn.certificateservice60"
        );
    }

    #[test]
    fn lowercases_unmatched_packages() {
        assert_eq!(kon().normalize_package_name("Foo.Bar.Baz"), "foo.bar.baz");
    }

    #[test]
    fn adjacent_group_refs_expand() {
        assert_eq!(
            kon().normalize_package_name("oasis.names.tc.dss10.core"),
            "oasis.dss10core"
        );
    }

    #[test]
    fn module_path_splits_and_escapes() {
        assert_eq!(
            kon().module_path("de.gematik.ws.conn.CertificateService60"),
            ["gematik", "conn", "certificateservice60"]
        );
        assert_eq!(
            NamingStrategy::default().module_path("a.type.1x"),
            ["a", "r#type", "_1x"]
        );
    }

    #[test]
    fn port_mappings_override_default() {
        let naming = NamingStrategy::new(NamingConfig {
            port_mappings: [("MySoap11Port".to_owned(), "Default".to_owned())].into(),
            ..NamingConfig::default()
        })
        .unwrap();
        assert_eq!(naming.port_type_name("MySoap11Port"), "Default");
        assert_eq!(naming.port_type_name("OtherPort"), "OtherPort");
    }

    #[test]
    fn package_for_port_appends_port_only_for_multi_port_services() {
        let naming = NamingStrategy::default();
        let multi = service(&["a", "b"]);
        assert_eq!(naming.package_for_port(&multi, &multi.ports[1]), "pkg.X.B");
        let single = service(&["only"]);
        assert_eq!(naming.package_for_port(&single, &single.ports[0]), "pkg.X");
    }

    #[test]
    fn public_identifier_matches_other_generators() {
        assert_eq!(public_identifier("foo_bar-baz"), "FooBarBaz");
        assert_eq!(public_identifier("URL"), "Url");
        assert_eq!(public_identifier("optionalInputs"), "OptionalInputs");
    }

    #[test]
    fn identifiers_are_valid_rust() {
        assert_eq!(type_ident("signDocument"), "SignDocument");
        assert_eq!(type_ident("Self"), "Self_");
        assert_eq!(field_ident("CardHandle"), "card_handle");
        assert_eq!(field_ident("type"), "r#type");
        assert_eq!(field_ident("self"), "self_");
        assert_eq!(
            variant_ident("urn:oasis:names:tc:dss:1.0:resultmajor:Success"),
            "UrnOasisNamesTcDss10ResultmajorSuccess"
        );
        assert_eq!(variant_ident("1.0"), "V10");
        assert_eq!(variant_ident("-"), "Empty");
    }
}
