//! Resolved, Rust-shaped view of the OpenAPI input. Everything the emitters
//! need (identifiers, XML names, occurrence, module placement) is decided
//! here, so emission is a straight walk over plain data.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail, ensure};
use indexmap::IndexMap;

use crate::model::{
    Api, BindingType, OperationDefinition, Schema, XmlExtension, ref_key, split_schema_key,
};
use crate::naming::{NamingStrategy, field_ident, type_ident, variant_ident};
use crate::select::{OperationMeta, Usage};

pub type ModulePath = Vec<String>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QName {
    pub local: String,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlNode {
    Element(QName),
    Attribute(QName),
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occurs {
    Required,
    Optional,
    Many,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    String,
    I32,
    I64,
    F64,
    Bool,
    Base64,
    AnyXml,
    /// A generated struct, enum or alias, by schema key.
    Named(String),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub ident: String,
    pub node: XmlNode,
    pub ty: FieldType,
    pub occurs: Occurs,
    pub boxed: bool,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub ident: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Struct(Vec<Field>),
    Enum(Vec<Variant>),
    /// A schema that is only a `$ref` to another generated type.
    Alias(String),
}

#[derive(Debug, Clone)]
pub struct Item {
    pub module: ModulePath,
    pub ident: String,
    /// The namespace of the schema's own `xml` extension, if any.
    pub namespace: Option<String>,
    pub kind: ItemKind,
}

#[derive(Debug, Clone)]
pub struct Payload {
    /// Schema key of the payload type.
    pub key: String,
    pub element: QName,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub name: String,
    pub soap_action: String,
    pub binding: BindingType,
    pub input: Payload,
    pub output: Payload,
    pub faults: Vec<Payload>,
    /// Service directory name, version and timeout class, for selected operations.
    pub meta: Option<OperationMeta>,
}

#[derive(Debug, Clone)]
pub struct Port {
    pub module: ModulePath,
    pub trait_ident: String,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    /// Generated items keyed by schema key, in input order.
    pub items: IndexMap<String, Item>,
    pub ports: Vec<Port>,
    /// The direction each item travels in, after an operation selection; `None`
    /// generates every item for both directions.
    pub usage: Option<HashMap<String, Usage>>,
    /// Emit a port trait per port.
    pub ports_as_traits: bool,
}

impl Program {
    /// The directions `key` travels in.
    pub fn usage(&self, key: &str) -> Usage {
        self.usage
            .as_ref()
            .and_then(|usage| usage.get(key).copied())
            .unwrap_or(Usage::BOTH)
    }

    /// Whether requests and responses are generated one-way (after a selection).
    pub fn split(&self) -> bool {
        self.usage.is_some()
    }

    pub fn build(api: &Api, naming: &NamingStrategy) -> Result<Self> {
        let builder = Builder {
            schemas: &api.components.schemas,
            naming,
        };
        let mut program = Self {
            items: builder.declare_items()?,
            ports: Vec::new(),
            usage: None,
            ports_as_traits: true,
        };
        let generated: HashSet<String> = program.items.keys().cloned().collect();
        for (key, item) in &mut program.items {
            if let ItemKind::Struct(fields) = &mut item.kind {
                *fields = builder
                    .fields(&builder.schemas[key], &generated)
                    .with_context(|| format!("schema {key}"))?;
            }
        }
        program.ports = builder.ports(api, &generated)?;
        Ok(program)
    }

    /// Follows aliases to the struct or enum a schema key finally names.
    pub fn resolve(&self, key: &str) -> (&str, &Item) {
        let lookup = |key: &str| {
            let (key, item) = self
                .items
                .get_key_value(key)
                .expect("keys come from the program");
            (key.as_str(), item)
        };
        let (mut key, mut item) = lookup(key);
        while let ItemKind::Alias(target) = &item.kind {
            (key, item) = lookup(target);
        }
        (key, item)
    }

    pub fn fields_of(&self, key: &str) -> &[Field] {
        match &self.resolve(key).1.kind {
            ItemKind::Struct(fields) => fields,
            _ => &[],
        }
    }
}

struct Builder<'a> {
    schemas: &'a IndexMap<String, Schema>,
    naming: &'a NamingStrategy,
}

enum TopLevel {
    Struct,
    Enum,
    Alias(String),
    Inline,
}

impl<'a> Builder<'a> {
    fn target(&self, reference: &str) -> Result<(&'a str, &'a Schema)> {
        let key =
            ref_key(reference).with_context(|| format!("unsupported reference {reference}"))?;
        let (key, schema) = self
            .schemas
            .get_key_value(key)
            .with_context(|| format!("invalid reference {reference}"))?;
        Ok((key.as_str(), schema))
    }

    /// The schema at the end of a `$ref` chain.
    fn final_schema(&self, mut schema: &'a Schema) -> Result<&'a Schema> {
        for _ in 0..self.schemas.len() + 1 {
            match &schema.reference {
                Some(reference) => schema = self.target(reference)?.1,
                None => return Ok(schema),
            }
        }
        bail!("cyclic $ref chain")
    }

    fn top_level(&self, schema: &Schema) -> Result<TopLevel> {
        if let Some(reference) = &schema.reference {
            let (target, _) = self.target(reference)?;
            let last = self.final_schema(schema)?;
            return Ok(match self.top_level(last)? {
                TopLevel::Struct | TopLevel::Enum => TopLevel::Alias(target.to_owned()),
                _ => TopLevel::Inline,
            });
        }
        Ok(match schema.kind.as_deref() {
            Some("object") => TopLevel::Struct,
            Some("string") if schema.enum_values.as_ref().is_some_and(|v| !v.is_empty()) => {
                TopLevel::Enum
            }
            _ => TopLevel::Inline,
        })
    }

    fn declare_items(&self) -> Result<IndexMap<String, Item>> {
        let mut items = IndexMap::new();
        let mut taken: HashMap<ModulePath, HashSet<String>> = HashMap::new();
        for (key, schema) in self.schemas {
            let (package, name) = split_schema_key(key);
            ensure!(
                !package.is_empty(),
                "invalid schema key {key:?} (missing '.' separator)"
            );
            let kind = match self
                .top_level(schema)
                .with_context(|| format!("schema {key}"))?
            {
                TopLevel::Struct => ItemKind::Struct(Vec::new()),
                TopLevel::Enum => ItemKind::Enum(variants(schema)),
                TopLevel::Alias(target) => ItemKind::Alias(target),
                TopLevel::Inline => continue,
            };
            let module = self.naming.module_path(package);
            let ident = unique(taken.entry(module.clone()).or_default(), type_ident(name));
            let namespace = schema.xml.as_ref().and_then(|x| non_empty(&x.namespace));
            items.insert(
                key.clone(),
                Item {
                    module,
                    ident,
                    namespace,
                    kind,
                },
            );
        }
        Ok(items)
    }

    /// The `xml` extension that names an element of this schema: its own, or
    /// that of the schema its `$ref` points at.
    fn element_xml(&self, mut schema: &'a Schema) -> Result<Option<&'a XmlExtension>> {
        for _ in 0..self.schemas.len() + 1 {
            match (&schema.xml, &schema.reference) {
                (Some(xml), _) => return Ok(Some(xml)),
                (None, Some(reference)) => schema = self.target(reference)?.1,
                (None, None) => return Ok(None),
            }
        }
        bail!("cyclic $ref chain")
    }

    fn value_type(
        &self,
        schema: &Schema,
        generated: &HashSet<String>,
    ) -> Result<(FieldType, bool)> {
        if let Some(reference) = &schema.reference {
            let (key, _) = self.target(reference)?;
            if generated.contains(key) {
                return Ok((FieldType::Named(key.to_owned()), false));
            }
            return primitive(self.final_schema(schema)?);
        }
        primitive(schema)
    }

    fn fields(&self, schema: &Schema, generated: &HashSet<String>) -> Result<Vec<Field>> {
        let mut fields = Vec::new();
        let mut idents = HashSet::new();
        let mut elements: HashMap<String, Option<String>> = HashMap::new();
        let Some(properties) = &schema.properties else {
            return Ok(fields);
        };
        for (name, property) in properties {
            let (value, occurs) = if property.is_kind("array") {
                let items = property
                    .items
                    .as_deref()
                    .with_context(|| format!("array {name} has no items"))?;
                ensure!(
                    !items.is_kind("array"),
                    "nested arrays are not supported ({name})"
                );
                (items, Occurs::Many)
            } else if schema.required.contains(name) {
                (property, Occurs::Required)
            } else {
                (property, Occurs::Optional)
            };
            let (ty, text) = self
                .value_type(value, generated)
                .with_context(|| format!("property {name}"))?;
            let xml = match self.element_xml(value)? {
                Some(xml) => Some(xml),
                None => property.xml.as_ref(),
            };
            let qname = || QName {
                local: xml
                    .map(|x| x.name.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| name.clone()),
                namespace: xml.and_then(|x| non_empty(&x.namespace)),
            };
            let node = if text {
                XmlNode::Text
            } else if xml.is_some_and(|x| x.attribute) {
                XmlNode::Attribute(qname())
            } else {
                let qname = qname();
                // quick-xml matches child elements by local name only.
                if let Some(other) = elements.insert(qname.local.clone(), qname.namespace.clone()) {
                    bail!(
                        "two child elements named {:?} ({:?} and {:?}) cannot be told apart",
                        qname.local,
                        other,
                        qname.namespace
                    );
                }
                XmlNode::Element(qname)
            };
            let mut ident = field_ident(name);
            if matches!(node, XmlNode::Attribute(_)) && idents.contains(&ident) {
                ident = format!("{}_attr", crate::naming::unraw(&ident));
            }
            let ident = unique(&mut idents, ident);
            fields.push(Field {
                ident,
                node,
                ty,
                occurs,
                boxed: false,
            });
        }
        Ok(fields)
    }

    fn payload(&self, reference: &str, generated: &HashSet<String>) -> Result<Payload> {
        let (key, schema) = self.target(reference)?;
        ensure!(
            generated.contains(key),
            "SOAP body {reference} is not an object type"
        );
        let xml = self.element_xml(schema)?;
        let local = xml
            .map(|x| x.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| split_schema_key(key).1.to_owned());
        Ok(Payload {
            key: key.to_owned(),
            element: QName {
                local,
                namespace: xml.and_then(|x| non_empty(&x.namespace)),
            },
        })
    }

    fn operation(
        &self,
        op: &OperationDefinition,
        binding: BindingType,
        generated: &HashSet<String>,
    ) -> Result<Operation> {
        let output = self.payload(&op.output.soap_body, generated)?;
        ensure!(
            output.element.local != "Fault",
            "response element named Fault clashes with SOAP-ENV:Fault"
        );
        let mut faults: Vec<Payload> = Vec::new();
        for message in op.faults.values() {
            let fault = self.payload(&message.soap_body, generated)?;
            if !faults.iter().any(|f| f.key == fault.key) {
                faults.push(fault);
            }
        }
        Ok(Operation {
            name: op.name.clone(),
            soap_action: op.soap_action.clone(),
            binding,
            input: self.payload(&op.input.soap_body, generated)?,
            output,
            faults,
            meta: None,
        })
    }

    /// SOAP 1.1 ports. Ports that land in the same module under the same name
    /// (the same WSDL service listed twice) are merged by operation name.
    fn ports(&self, api: &Api, generated: &HashSet<String>) -> Result<Vec<Port>> {
        let mut ports: Vec<Port> = Vec::new();
        for service in &api.web_services {
            for port in &service.ports {
                if port.binding_type != BindingType::Soap11 {
                    continue;
                }
                let module = self
                    .naming
                    .module_path(&self.naming.package_for_port(service, port));
                let trait_ident = self.naming.port_type_name(&port.name);
                let index = match ports
                    .iter()
                    .position(|p| p.module == module && p.trait_ident == trait_ident)
                {
                    Some(index) => index,
                    None => {
                        ports.push(Port {
                            module,
                            trait_ident,
                            operations: Vec::new(),
                        });
                        ports.len() - 1
                    }
                };
                for op in &port.operations {
                    if ports[index].operations.iter().any(|o| o.name == op.name) {
                        continue;
                    }
                    let operation = self
                        .operation(op, port.binding_type, generated)
                        .with_context(|| format!("operation {}.{}", port.name, op.name))?;
                    ports[index].operations.push(operation);
                }
            }
        }
        Ok(ports)
    }
}

fn primitive(schema: &Schema) -> Result<(FieldType, bool)> {
    let format = schema.format.as_deref();
    Ok(match (schema.kind.as_deref(), format) {
        (Some("string"), Some("byte")) => (FieldType::Base64, false),
        (Some("string"), Some("xml")) => (FieldType::AnyXml, false),
        (Some("string"), Some("chardata")) => (FieldType::String, true),
        (Some("string"), _) => (FieldType::String, false),
        (Some("integer"), Some("int32")) => (FieldType::I32, false),
        (Some("integer"), _) => (FieldType::I64, false),
        (Some("number"), _) => (FieldType::F64, false),
        (Some("boolean"), _) => (FieldType::Bool, false),
        (kind, _) => bail!("unsupported inline schema type {kind:?}"),
    })
}

fn variants(schema: &Schema) -> Vec<Variant> {
    let mut idents = HashSet::new();
    schema
        .enum_values
        .iter()
        .flatten()
        .map(|value| {
            let value = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            Variant {
                ident: unique(&mut idents, variant_ident(&value)),
                value,
            }
        })
        .collect()
}

fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_owned())
}

/// Returns `ident`, or `ident2`, `ident3`, … if it is already taken.
fn unique(taken: &mut HashSet<String>, ident: String) -> String {
    let ident = if taken.contains(&ident) {
        (2..)
            .map(|n| format!("{ident}{n}"))
            .find(|candidate| !taken.contains(candidate))
            .expect("an unused identifier exists")
    } else {
        ident
    };
    taken.insert(ident.clone());
    ident
}
