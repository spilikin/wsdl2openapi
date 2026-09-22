use proc_macro2::TokenStream;
use quote::quote;

use super::{Emitter, ident};
use crate::ir::{Item, ItemKind, Variant};

pub fn emit_item(emitter: &Emitter, item: &Item) -> TokenStream {
    let name = ident(&item.ident);
    match &item.kind {
        ItemKind::Struct(fields) => {
            let fields = fields.iter().map(|f| emitter.field(f, &item.module));
            quote! {
                #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
                pub struct #name {
                    #(#fields,)*
                }
            }
        }
        ItemKind::Enum(variants) => emit_enum(emitter, item, variants),
        ItemKind::Alias(target) => {
            let target = emitter.type_ref(target, &item.module);
            quote!(pub type #name = #target;)
        }
    }
}

/// Enumerations (de)serialize through their string value rather than serde's
/// derived enum representation, which quick-xml would read as a choice of
/// child elements inside lists.
fn emit_enum(emitter: &Emitter, item: &Item, variants: &[Variant]) -> TokenStream {
    let name = ident(&item.ident);
    let type_name = &item.ident;
    let soap = emitter.soap();
    let string = emitter.prelude(&item.module, "String");
    let result = emitter.prelude(&item.module, "Result");
    let idents: Vec<_> = variants.iter().map(|v| ident(&v.ident)).collect();
    let values: Vec<_> = variants.iter().map(|v| &v.value).collect();
    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #name {
            #(#idents,)*
        }

        impl #name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    #(Self::#idents => #values,)*
                }
            }
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::std::str::FromStr for #name {
            type Err = #soap::UnknownEnumValue;

            fn from_str(value: &str) -> #result<Self, Self::Err> {
                match value {
                    #(#values => Ok(Self::#idents),)*
                    _ => Err(#soap::UnknownEnumValue {
                        type_name: #type_name,
                        value: value.to_owned(),
                    }),
                }
            }
        }

        impl ::serde::Serialize for #name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> #result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> #result<Self, D::Error> {
                let value = <#string as ::serde::Deserialize>::deserialize(deserializer)?;
                value.parse().map_err(::serde::de::Error::custom)
            }
        }
    }
}
