mod port;
mod soap;
mod types;

use std::collections::{HashMap, HashSet};

use anyhow::{Result, ensure};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::analysis::Namespaces;
use crate::ir::{Field, FieldType, ModulePath, Occurs, Program, XmlNode};
use crate::select::Usage;

pub use port::emit_port;
pub use soap::{SoapFeatures, emit_soap_module};
pub use types::emit_item;

pub const SOAP_MODULE: &str = "soap";

pub fn ident(name: &str) -> Ident {
    match name.strip_prefix("r#") {
        Some(raw) => Ident::new_raw(raw, Span::call_site()),
        None => Ident::new(name, Span::call_site()),
    }
}

/// Shared lookups for the emitters: where types live, which prefix a
/// namespace uses and which prelude names a module shadows.
pub struct Emitter<'a> {
    pub program: &'a Program,
    pub namespaces: &'a Namespaces,
    root: Vec<Ident>,
    locals: HashMap<ModulePath, HashSet<String>>,
}

impl<'a> Emitter<'a> {
    pub fn new(
        program: &'a Program,
        namespaces: &'a Namespaces,
        module_root: &str,
    ) -> Result<Self> {
        let root: Vec<Ident> = module_root.split("::").map(|s| ident(s.trim())).collect();
        let mut locals: HashMap<ModulePath, HashSet<String>> = HashMap::new();
        for item in program.items.values() {
            locals
                .entry(item.module.clone())
                .or_default()
                .insert(item.ident.clone());
        }
        for port in &program.ports {
            let names = locals.entry(port.module.clone()).or_default();
            for name in port::item_names(port) {
                ensure!(
                    names.insert(name.clone()),
                    "generated item {name} for port {} clashes with another item in module {}",
                    port.trait_ident,
                    port.module.join("::")
                );
            }
        }
        Ok(Self {
            program,
            namespaces,
            root,
            locals,
        })
    }

    pub fn module_path(&self, module: &[String]) -> TokenStream {
        let root = &self.root;
        let segments = module.iter().map(|s| ident(s));
        quote!(#(#root)::* #(:: #segments)*)
    }

    pub fn soap(&self) -> TokenStream {
        self.module_path(&[SOAP_MODULE.to_owned()])
    }

    /// Path to a generated type as written from inside `from`.
    pub fn type_ref(&self, key: &str, from: &ModulePath) -> TokenStream {
        let item = &self.program.items[key];
        let name = ident(&item.ident);
        if &item.module == from {
            quote!(#name)
        } else {
            let module = self.module_path(&item.module);
            quote!(#module :: #name)
        }
    }

    /// A prelude type, fully qualified only where a generated item shadows it
    /// (DSS, for one, defines a `Result`).
    pub fn prelude(&self, from: &ModulePath, name: &str) -> TokenStream {
        let shadowed = self.locals.get(from).is_some_and(|l| l.contains(name));
        let ty = ident(name);
        if !shadowed {
            return quote!(#ty);
        }
        let module = ident(match name {
            "String" => "string",
            "Option" => "option",
            "Vec" => "vec",
            "Box" => "boxed",
            "Result" => "result",
            other => unreachable!("{other} is not a prelude type"),
        });
        quote!(::std::#module::#ty)
    }

    fn prelude_str(&self, from: &ModulePath, name: &str) -> String {
        self.prelude(from, name).to_string().replace(' ', "")
    }

    /// The `rename` part of a field's `#[serde(...)]` attribute. Qualified
    /// names are written with their prefix and matched by local name, which
    /// is all quick-xml's deserializer compares. A one-way type only carries
    /// the name for its direction.
    pub fn rename(&self, node: &XmlNode, usage: Usage) -> TokenStream {
        let (marker, qname) = match node {
            XmlNode::Text => return quote!(rename = "$text"),
            XmlNode::Element(q) => ("", q),
            XmlNode::Attribute(q) => ("@", q),
        };
        let local = format!("{marker}{}", qname.local);
        match &qname.namespace {
            None => quote!(rename = #local),
            Some(ns) => {
                let prefix = self.namespaces.prefix(ns);
                let qualified = format!("{marker}{prefix}:{}", qname.local);
                // quick-xml keeps the `xml:` prefix when deserializing attributes.
                match (prefix, usage.read, usage.write) {
                    ("xml", _, _) | (_, false, _) => quote!(rename = #qualified),
                    (_, true, false) => quote!(rename = #local),
                    (_, true, true) => quote!(rename(serialize = #qualified, deserialize = #local)),
                }
            }
        }
    }

    pub fn field(&self, field: &Field, from: &ModulePath, usage: Usage) -> TokenStream {
        let name = ident(&field.ident);
        let rename = self.rename(&field.node, usage);
        let base = match &field.ty {
            FieldType::String => self.prelude(from, "String"),
            FieldType::I32 => quote!(i32),
            FieldType::I64 => quote!(i64),
            FieldType::F64 => quote!(f64),
            FieldType::Bool => quote!(bool),
            FieldType::Base64 => {
                let soap = self.soap();
                quote!(#soap::Base64Binary)
            }
            FieldType::AnyXml => {
                let soap = self.soap();
                quote!(#soap::AnyXml)
            }
            FieldType::Named(key) => self.type_ref(key, from),
        };
        let base = if field.boxed {
            let boxed = self.prelude(from, "Box");
            quote!(#boxed<#base>)
        } else {
            base
        };
        let (ty, extra) = match field.occurs {
            Occurs::Required => (base, quote!()),
            Occurs::Optional => {
                let option = self.prelude(from, "Option");
                let skip = format!("{}::is_none", self.prelude_str(from, "Option"));
                (
                    quote!(#option<#base>),
                    quote!(, default, skip_serializing_if = #skip),
                )
            }
            Occurs::Many => {
                let vec = self.prelude(from, "Vec");
                let skip = format!("{}::is_empty", self.prelude_str(from, "Vec"));
                (
                    quote!(#vec<#base>),
                    quote!(, default, skip_serializing_if = #skip),
                )
            }
        };
        quote! {
            #[serde(#rename #extra)]
            pub #name: #ty
        }
    }
}

/// The serde derives for a type travelling in `usage`'s directions.
pub fn serde_derives(usage: Usage) -> TokenStream {
    match (usage.read, usage.write) {
        (true, true) => quote!(::serde::Serialize, ::serde::Deserialize),
        (true, false) => quote!(::serde::Deserialize),
        (false, _) => quote!(::serde::Serialize),
    }
}

pub fn doc(text: &str) -> TokenStream {
    let text = format!(" {text}");
    quote!(#[doc = #text])
}
