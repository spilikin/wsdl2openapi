use indexmap::IndexMap;
use serde::Deserialize;

pub const SCHEMA_REF_PREFIX: &str = "#/components/schemas/";

#[derive(Debug, Clone, Deserialize)]
pub struct Api {
    #[serde(rename = "x-wsdl-services", default)]
    pub web_services: Vec<WebService>,
    #[serde(default)]
    pub components: Components,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Components {
    #[serde(default)]
    pub schemas: IndexMap<String, Schema>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebService {
    pub name: String,
    pub target_package: String,
    #[serde(default)]
    pub ports: Vec<WebServicePort>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebServicePort {
    pub name: String,
    pub binding_type: BindingType,
    #[serde(default)]
    pub operations: Vec<OperationDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BindingType {
    #[serde(rename = "soap11")]
    Soap11,
    #[serde(rename = "soap12")]
    Soap12,
    #[serde(rename = "http-get")]
    HttpGet,
    #[serde(rename = "http-post")]
    HttpPost,
    #[serde(other)]
    Other,
}

impl BindingType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Soap11 => "soap11",
            Self::Soap12 => "soap12",
            Self::HttpGet => "http-get",
            Self::HttpPost => "http-post",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDefinition {
    pub name: String,
    #[serde(default)]
    pub soap_action: String,
    pub input: WebServiceMessage,
    pub output: WebServiceMessage,
    #[serde(default)]
    pub faults: IndexMap<String, WebServiceMessage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebServiceMessage {
    pub soap_body: String,
}

/// One JSON Schema fragment as emitted by the Python converter. Keys the
/// generator does not act on (facets such as `pattern` or `maxLength`) are
/// ignored.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Schema {
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub format: Option<String>,
    pub xml: Option<XmlExtension>,
    pub properties: Option<IndexMap<String, Schema>>,
    #[serde(default)]
    pub required: Vec<String>,
    pub items: Option<Box<Schema>>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

impl Schema {
    pub fn is_kind(&self, kind: &str) -> bool {
        self.kind.as_deref() == Some(kind)
    }

    pub fn has_format(&self, format: &str) -> bool {
        self.format.as_deref() == Some(format)
    }

    pub fn reference_to(key: &str, xml: Option<XmlExtension>) -> Self {
        Self {
            reference: Some(format!("{SCHEMA_REF_PREFIX}{key}")),
            xml,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct XmlExtension {
    pub name: String,
    pub namespace: String,
    pub prefix: String,
    pub attribute: bool,
}

/// Splits a flat schema key into `(package, type)` at the **last** dot.
pub fn split_schema_key(key: &str) -> (&str, &str) {
    key.rsplit_once('.').unwrap_or(("", key))
}

/// The component key a `$ref` points at, or `None` for non-local refs.
pub fn ref_key(reference: &str) -> Option<&str> {
    reference.strip_prefix(SCHEMA_REF_PREFIX)
}
