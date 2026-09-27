//! Generates Rust types and typed SOAP 1.1 envelopes from the OpenAPI
//! documents produced by the wsdl2openapi converter.
//!
//! The output is a module tree meant to be mounted with `mod name;`. It
//! depends on `serde` and `quick-xml` only (plus `base64` when a schema uses
//! `xs:base64Binary`).

pub mod analysis;
pub mod emit;
pub mod extract;
pub mod ir;
pub mod model;
pub mod naming;
pub mod select;
pub mod writer;

use anyhow::{Result, ensure};

use crate::analysis::Namespaces;
use crate::emit::{Emitter, SOAP_MODULE, SoapFeatures};
use crate::ir::{FieldType, ItemKind, Program};
use crate::model::Api;
use crate::naming::NamingStrategy;
pub use crate::writer::{GeneratedFile, write_files};

#[derive(Debug, Clone)]
pub struct Options {
    /// Path of the module the output is mounted at, e.g. `crate::kon`.
    pub module_root: String,
    /// Generate only these operations and the types they reach.
    pub selection: Option<select::Selection>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            module_root: "crate".to_owned(),
            selection: None,
        }
    }
}

pub fn generate(
    mut api: Api,
    naming: &NamingStrategy,
    options: &Options,
) -> Result<Vec<GeneratedFile>> {
    extract::extract_inline_objects(&mut api.components.schemas);
    let mut program = Program::build(&api, naming)?;
    if let Some(selection) = &options.selection {
        select::apply(&mut program, selection)?;
    }
    analysis::box_recursive_fields(&mut program);
    let namespaces = Namespaces::build(&program);
    let emitter = Emitter::new(&program, &namespaces, &options.module_root)?;

    let mut tree = writer::ModuleTree::default();
    for (key, item) in &program.items {
        tree.push(&item.module, emit::emit_item(&emitter, key, item));
    }
    for port in &program.ports {
        tree.push(&port.module, emit::emit_port(&emitter, port));
    }

    let features = soap_features(&program);
    let soap_path = vec![SOAP_MODULE.to_owned()];
    ensure!(
        !features.any() || !tree.contains(&soap_path),
        "a package maps to the top-level module `{SOAP_MODULE}`, which is reserved for the SOAP runtime"
    );
    if features.any() {
        tree.push(&soap_path, emit::emit_soap_module(features));
    }
    tree.into_files()
}

fn soap_features(program: &Program) -> SoapFeatures {
    let mut features = SoapFeatures {
        ports: !program.ports.is_empty(),
        no_detail: program
            .ports
            .iter()
            .flat_map(|p| &p.operations)
            .any(|op| op.faults.is_empty()),
        ..SoapFeatures::default()
    };
    for item in program.items.values() {
        match &item.kind {
            ItemKind::Enum(_) => features.enums = true,
            ItemKind::Struct(fields) => {
                for field in fields {
                    features.base64 |= field.ty == FieldType::Base64;
                    features.any_xml |= field.ty == FieldType::AnyXml;
                }
            }
            ItemKind::Alias(_) => {}
        }
    }
    features
}
