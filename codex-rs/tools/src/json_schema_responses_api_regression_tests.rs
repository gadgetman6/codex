use super::AdditionalProperties;
use super::JsonSchema;
use super::parse_tool_input_schema;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;

#[test]
fn parse_tool_input_schema_coerces_boolean_schemas() {
    let schema = parse_tool_input_schema(&serde_json::json!(true)).expect("parse schema");

    assert_eq!(schema, JsonSchema::String { description: None });
}

#[test]
fn parse_tool_input_schema_infers_object_shape_and_defaults_properties() {
    let schema = parse_tool_input_schema(&serde_json::json!({
        "properties": {
            "query": {"description": "search query"}
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::Object {
            properties: BTreeMap::from([(
                "query".to_string(),
                JsonSchema::String {
                    description: Some("search query".to_string()),
                },
            )]),
            required: None,
            additional_properties: None,
        }
    );
}

#[test]
fn parse_tool_input_schema_normalizes_integer_and_missing_array_items() {
    let schema = parse_tool_input_schema(&serde_json::json!({
        "type": "object",
        "properties": {
            "page": {"type": "integer"},
            "tags": {"type": "array"}
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::Object {
            properties: BTreeMap::from([
                ("page".to_string(), JsonSchema::Number { description: None }),
                (
                    "tags".to_string(),
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::String { description: None }),
                        description: None,
                    },
                ),
            ]),
            required: None,
            additional_properties: None,
        }
    );
}

#[test]
fn parse_tool_input_schema_sanitizes_additional_properties_schema() {
    let schema = parse_tool_input_schema(&serde_json::json!({
        "type": "object",
        "additionalProperties": {
            "required": ["value"],
            "properties": {
                "value": {"anyOf": [{"type": "string"}, {"type": "number"}]}
            }
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::Object {
            properties: BTreeMap::new(),
            required: None,
            additional_properties: Some(AdditionalProperties::Schema(Box::new(
                JsonSchema::Object {
                    properties: BTreeMap::from([(
                        "value".to_string(),
                        JsonSchema::String { description: None },
                    )]),
                    required: Some(vec!["value".to_string()]),
                    additional_properties: None,
                },
            ))),
        }
    );
}

#[test]
fn parse_tool_input_schema_preserves_boolean_additional_properties_on_inferred_object() {
    // Example schema shape:
    // {
    //   "type": "object",
    //   "properties": {
    //     "metadata": {
    //       "additionalProperties": true
    //     }
    //   }
    // }
    //
    // Expected normalization behavior:
    // - The nested `metadata` schema is inferred to be an object because it has
    //   `additionalProperties`.
    // - `additionalProperties: true` is preserved rather than rewritten.
    let schema = parse_tool_input_schema(&serde_json::json!({
        "type": "object",
        "properties": {
            "metadata": {
                "additionalProperties": true
            }
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::Object {
            properties: BTreeMap::from([(
                "metadata".to_string(),
                JsonSchema::Object {
                    properties: BTreeMap::new(),
                    required: None,
                    additional_properties: Some(AdditionalProperties::Boolean(true)),
                },
            )]),
            required: None,
            additional_properties: None,
        }
    );
}

#[test]
fn parse_tool_input_schema_preserves_nested_nullable_type_union() {
    // Example schema shape:
    // {
    //   "type": "object",
    //   "properties": {
    //     "nickname": {
    //       "type": ["string", "null"],
    //       "description": "Optional nickname"
    //     }
    //   },
    //   "required": ["nickname"],
    //   "additionalProperties": false
    // }
    //
    // Expected normalization behavior:
    // - The nested property keeps the explicit `["string", "null"]` union.
    // - The object-level `required` and `additionalProperties: false` stay intact.
    let schema = parse_tool_input_schema(&serde_json::json!({
        "type": "object",
        "properties": {
            "nickname": {
                "type": ["string", "null"],
                "description": "Optional nickname"
            }
        },
        "required": ["nickname"],
        "additionalProperties": false
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::Object {
            properties: BTreeMap::from([(
                "nickname".to_string(),
                serde_json::from_value(serde_json::json!({
                    "type": ["string", "null"],
                    "description": "Optional nickname"
                }))
                .expect("nested nullable schema"),
            )]),
            required: Some(vec!["nickname".to_string()]),
            additional_properties: Some(false.into()),
        }
    );
}

#[test]
fn parse_tool_input_schema_preserves_nested_any_of_property() {
    // Example schema shape:
    // {
    //   "type": "object",
    //   "properties": {
    //     "query": {
    //       "anyOf": [
    //         { "type": "string" },
    //         { "type": "number" }
    //       ]
    //     }
    //   }
    // }
    //
    // Expected normalization behavior:
    // - The nested `anyOf` is preserved rather than flattened into a single
    //   fallback type.
    let schema = parse_tool_input_schema(&serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "anyOf": [
                    { "type": "string" },
                    { "type": "number" }
                ]
            }
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::Object {
            properties: BTreeMap::from([(
                "query".to_string(),
                serde_json::from_value(serde_json::json!({
                    "anyOf": [
                        { "type": "string" },
                        { "type": "number" }
                    ]
                }))
                .expect("nested anyOf schema"),
            )]),
            required: None,
            additional_properties: None,
        }
    );
}
