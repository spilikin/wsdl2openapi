use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;

use super::{Emitter, doc, ident};
use crate::ir::{Field, FieldType, Occurs, Operation, Payload, Port, XmlNode};
use crate::naming::{field_ident, type_ident};

struct OperationNames {
    input: String,
    output: String,
    fault_detail: String,
    envelope: String,
    response_envelope: String,
}

impl OperationNames {
    fn new(op: &Operation) -> Self {
        let base = type_ident(&op.name);
        Self {
            input: format!("{base}Input"),
            output: format!("{base}Output"),
            fault_detail: format!("{base}FaultDetail"),
            envelope: format!("{base}Envelope"),
            response_envelope: format!("{base}ResponseEnvelope"),
        }
    }
}

/// Every item name a port adds to its module, for clash detection.
pub fn item_names(port: &Port) -> Vec<String> {
    let mut names = vec![port.trait_ident.clone()];
    for op in &port.operations {
        let n = OperationNames::new(op);
        names.extend([n.input, n.output, n.envelope, n.response_envelope]);
        if !op.faults.is_empty() {
            names.push(n.fault_detail);
        }
    }
    names
}

pub fn emit_port(emitter: &Emitter, port: &Port) -> TokenStream {
    let operations = port
        .operations
        .iter()
        .map(|op| emit_operation(emitter, port, op));
    let port_trait = emit_trait(emitter, port);
    quote!(#(#operations)* #port_trait)
}

fn emit_operation(emitter: &Emitter, port: &Port, op: &Operation) -> TokenStream {
    let module = &port.module;
    let soap = emitter.soap();
    let names = OperationNames::new(op);
    let input = ident(&names.input);
    let output = ident(&names.output);
    let envelope = ident(&names.envelope);
    let response_envelope = ident(&names.response_envelope);
    let result = emitter.prelude(module, "Result");

    let namespaces = emitter
        .namespaces
        .declarations(emitter.program, op)
        .into_iter()
        .map(|(prefix, uri)| {
            let attribute = format!("@xmlns:{prefix}");
            quote!((#attribute, #uri))
        });
    let namespaces = quote!(&[#(#namespaces),*]);

    let request_type = emitter.type_ref(&op.input.key, module);
    let request_variant = ident(&type_ident(&op.input.element.local));
    let request_rename = emitter.rename(&XmlNode::Element(op.input.element.clone()));
    let response_type = emitter.type_ref(&op.output.key, module);
    let response_variant = ident(&type_ident(&op.output.element.local));
    let response_rename = emitter.rename(&XmlNode::Element(op.output.element.clone()));

    let (detail_type, detail_struct) = if op.faults.is_empty() {
        (quote!(#soap::NoDetail), None)
    } else {
        let name = ident(&names.fault_detail);
        let mut taken = HashSet::new();
        let fields = op.faults.iter().map(|fault| {
            let field = detail_field(fault, &mut taken);
            emitter.field(&field, module)
        });
        let description = doc(&format!("Typed `detail` content of a `{}` fault.", op.name));
        let detail = quote! {
            #description
            #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
            pub struct #name {
                #(#fields,)*
            }
        };
        (quote!(#name), Some(detail))
    };

    let op_name = &op.name;
    let soap_action = &op.soap_action;
    let binding = op.binding.as_str();
    let input_doc = doc(&format!(
        "`SOAP-ENV:Body` content of a `{}` request.",
        op.name
    ));
    // A parsed response is short-lived and matched once, so boxing either
    // variant to appease the size lints would only add an allocation.
    let output_doc = doc(&format!(
        "`SOAP-ENV:Body` content of a `{}` response: the result or a SOAP fault.",
        op.name
    ));

    quote! {
        #input_doc
        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        pub enum #input {
            #[serde(#request_rename)]
            #request_variant(#request_type),
        }

        impl #soap::BodyContent for #input {
            const NAMESPACES: &'static [(&'static str, &'static str)] = #namespaces;
        }

        impl #soap::SoapRequest for #input {
            const OPERATION: #soap::SoapOperation = #soap::SoapOperation {
                name: #op_name,
                soap_action: #soap_action,
                binding_type: #binding,
            };
            type Response = #output;
        }

        impl From<#request_type> for #input {
            fn from(request: #request_type) -> Self {
                Self::#request_variant(request)
            }
        }

        impl From<#request_type> for #soap::Envelope<#input> {
            fn from(request: #request_type) -> Self {
                Self::new(request.into())
            }
        }

        #output_doc
        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        #[allow(clippy::large_enum_variant)]
        pub enum #output {
            #[serde(#response_rename)]
            #response_variant(#response_type),
            #[serde(rename(serialize = "SOAP-ENV:Fault", deserialize = "Fault"))]
            Fault(#soap::Fault<#detail_type>),
        }

        impl #output {
            #[allow(clippy::result_large_err)]
            pub fn into_result(self) -> #result<#response_type, #soap::Fault<#detail_type>> {
                match self {
                    Self::#response_variant(response) => Ok(response),
                    Self::Fault(fault) => Err(fault),
                }
            }
        }

        impl #soap::BodyContent for #output {
            const NAMESPACES: &'static [(&'static str, &'static str)] = #namespaces;

            fn is_fault(&self) -> bool {
                matches!(self, Self::Fault(_))
            }
        }

        #detail_struct

        pub type #envelope = #soap::Envelope<#input>;
        pub type #response_envelope = #soap::Envelope<#output>;
    }
}

/// One optional field per distinct fault element: `<detail>` wraps the fault
/// message element, it is not that element's content.
fn detail_field(fault: &Payload, taken: &mut HashSet<String>) -> Field {
    let mut ident = field_ident(&fault.element.local);
    let mut n = 2;
    while !taken.insert(ident.clone()) {
        ident = format!("{}{n}", field_ident(&fault.element.local));
        n += 1;
    }
    Field {
        ident,
        node: XmlNode::Element(fault.element.clone()),
        ty: FieldType::Named(fault.key.clone()),
        occurs: Occurs::Optional,
        boxed: false,
    }
}

fn emit_trait(emitter: &Emitter, port: &Port) -> TokenStream {
    let module = &port.module;
    let name = ident(&port.trait_ident);
    let result = emitter.prelude(module, "Result");
    let mut taken = HashSet::new();
    let methods = port.operations.iter().map(|op| {
        let mut method = field_ident(&op.name);
        let mut n = 2;
        while !taken.insert(method.clone()) {
            method = format!("{}{n}", field_ident(&op.name));
            n += 1;
        }
        let method = ident(&method);
        let request = emitter.type_ref(&op.input.key, module);
        let response = emitter.type_ref(&op.output.key, module);
        quote! {
            fn #method(&self, request: #request)
                -> impl ::std::future::Future<Output = #result<#response, Self::Error>> + Send;
        }
    });
    let description = doc(&format!(
        "Operations of the `{}` port, to be implemented over a SOAP transport.",
        port.trait_ident
    ));
    quote! {
        #description
        pub trait #name {
            type Error;

            #(#methods)*
        }
    }
}
