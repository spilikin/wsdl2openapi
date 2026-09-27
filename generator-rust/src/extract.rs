use indexmap::IndexMap;

use crate::model::{Schema, split_schema_key};
use crate::naming::public_identifier;

/// Lifts inline `type: object` properties and inline object array items into
/// named component schemas, replacing them with `$ref`s. Mirrors
/// `ExtractInlineObjects` in the Go generator and `extractInlineObjects` in
/// the Kotlin one; lifted types are appended and processed in turn, so nested
/// inline objects are lifted as well.
pub fn extract_inline_objects(schemas: &mut IndexMap<String, Schema>) {
    let mut index = 0;
    while index < schemas.len() {
        let (key, schema) = schemas.get_index(index).expect("index is in bounds");
        let (package, parent) = split_schema_key(key);
        let (package, parent) = (package.to_owned(), parent.to_owned());
        let mut schema = schema.clone();
        let mut lifted = Vec::new();

        if schema.is_kind("object")
            && let Some(properties) = schema.properties.as_mut()
        {
            for (name, property) in properties.iter_mut() {
                let is_array = property.is_kind("array");
                let target = match property {
                    p if p.is_kind("object") && p.properties.is_some() => p,
                    p if is_array => match p.items.as_deref_mut() {
                        Some(items) if items.is_kind("object") && items.properties.is_some() => {
                            items
                        }
                        _ => continue,
                    },
                    _ => continue,
                };
                let new_key = unique_key(
                    schemas,
                    &lifted,
                    &package,
                    &format!("{parent}{}", public_identifier(name)),
                );
                let xml = target.xml.clone();
                let inline = std::mem::take(target);
                // An inline object keeps its element name on the property; a lifted
                // array item carries it on the new schema, as Go and Kotlin do.
                *target = Schema::reference_to(&new_key, if is_array { None } else { xml });
                lifted.push((new_key, inline));
            }
        }

        if !lifted.is_empty() {
            schemas[index] = schema;
            schemas.extend(lifted);
        }
        index += 1;
    }
}

fn unique_key(
    schemas: &IndexMap<String, Schema>,
    pending: &[(String, Schema)],
    package: &str,
    name: &str,
) -> String {
    let taken = |key: &str| schemas.contains_key(key) || pending.iter().any(|(k, _)| k == key);
    let base = format!("{package}.{name}");
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}{n}"))
        .find(|key| !taken(key))
        .expect("an unused key exists")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schemas(json: &str) -> IndexMap<String, Schema> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn lifts_inline_object_property() {
        let mut s = schemas(
            r#"{"demo.Parent": {"type": "object", "properties": {
                "child": {"type": "object", "xml": {"name": "Child"}, "properties": {"name": {"type": "string"}}}
            }}}"#,
        );
        extract_inline_objects(&mut s);

        let child = &s["demo.Parent"].properties.as_ref().unwrap()["child"];
        assert_eq!(
            child.reference.as_deref(),
            Some("#/components/schemas/demo.ParentChild")
        );
        assert_eq!(child.xml.as_ref().unwrap().name, "Child");
        assert!(
            s["demo.ParentChild"]
                .properties
                .as_ref()
                .unwrap()
                .contains_key("name")
        );
    }

    #[test]
    fn lifts_inline_array_items() {
        let mut s = schemas(
            r#"{"demo.Parent": {"type": "object", "properties": {
                "items": {"type": "array", "items": {"type": "object", "xml": {"name": "Item"}, "properties": {"id": {"type": "string"}}}}
            }}}"#,
        );
        extract_inline_objects(&mut s);

        let items = &s["demo.Parent"].properties.as_ref().unwrap()["items"];
        assert!(items.is_kind("array"));
        let item_ref = items.items.as_ref().unwrap();
        assert_eq!(
            item_ref.reference.as_deref(),
            Some("#/components/schemas/demo.ParentItems")
        );
        assert_eq!(item_ref.xml, None);
        assert_eq!(s["demo.ParentItems"].xml.as_ref().unwrap().name, "Item");
    }

    #[test]
    fn lifts_nested_inline_objects() {
        let mut s = schemas(
            r#"{"demo.A": {"type": "object", "properties": {
                "b": {"type": "object", "properties": {"c": {"type": "object", "properties": {"x": {"type": "string"}}}}}
            }}}"#,
        );
        extract_inline_objects(&mut s);
        assert_eq!(
            s.keys().collect::<Vec<_>>(),
            ["demo.A", "demo.AB", "demo.ABC"]
        );
    }

    #[test]
    fn avoids_overwriting_existing_schemas() {
        let mut s = schemas(
            r#"{
              "demo.Parent": {"type": "object", "properties": {"child": {"type": "object", "properties": {}}}},
              "demo.ParentChild": {"type": "string"}
            }"#,
        );
        extract_inline_objects(&mut s);
        assert!(s["demo.ParentChild"].is_kind("string"));
        assert!(s.contains_key("demo.ParentChild2"));
    }
}
