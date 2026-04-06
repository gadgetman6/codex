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
                ("page".to_string(), JsonSchema::Number { description: None },),
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
                        JsonSchema::AnyOf {
                            variants: vec![
                                JsonSchema::String { description: None },
                                JsonSchema::Number { description: None },
                            ],
                            description: None,
                        },
                    )]),
                    required: Some(vec!["value".to_string()]),
                    additional_properties: None,
                },
            ))),
        }
    );
}

#[test]
fn parse_tool_input_schema_preserves_web_run_shape() {
    let schema = parse_tool_input_schema(&serde_json::json!({
        "type": "object",
        "properties": {
            "open": {
                "anyOf": [
                    {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "ref_id": {"type": "string"},
                                "lineno": {"anyOf": [{"type": "integer"}, {"type": "null"}]}
                            },
                            "required": ["ref_id"],
                            "additionalProperties": false
                        }
                    },
                    {"type": "null"}
                ]
            },
            "tagged_list": {
                "anyOf": [
                    {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "kind": {"type": "const", "const": "tagged"},
                                "variant": {"type": "enum", "enum": ["alpha", "beta"]},
                                "scope": {"type": "enum", "enum": ["one", "two"]}
                            },
                            "required": ["kind", "variant", "scope"]
                        }
                    },
                    {"type": "null"}
                ]
            },
            "response_length": {
                "type": "enum",
                "enum": ["short", "medium", "long"]
            }
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::Object {
            properties: BTreeMap::from([
                (
                    "open".to_string(),
                    JsonSchema::AnyOf {
                        variants: vec![
                            JsonSchema::Array {
                                items: Box::new(JsonSchema::Object {
                                    properties: BTreeMap::from([
                                        (
                                            "lineno".to_string(),
                                            JsonSchema::AnyOf {
                                                variants: vec![
                                                    JsonSchema::Number { description: None },
                                                    JsonSchema::Null { description: None },
                                                ],
                                                description: None,
                                            },
                                        ),
                                        (
                                            "ref_id".to_string(),
                                            JsonSchema::String { description: None },
                                        ),
                                    ]),
                                    required: Some(vec!["ref_id".to_string()]),
                                    additional_properties: Some(false.into()),
                                }),
                                description: None,
                            },
                            JsonSchema::Null { description: None },
                        ],
                        description: None,
                    },
                ),
                (
                    "response_length".to_string(),
                    JsonSchema::Enum {
                        values: vec![
                            serde_json::json!("short"),
                            serde_json::json!("medium"),
                            serde_json::json!("long"),
                        ],
                        schema_type: Some("enum".to_string()),
                        description: None,
                    },
                ),
                (
                    "tagged_list".to_string(),
                    JsonSchema::AnyOf {
                        variants: vec![
                            JsonSchema::Array {
                                items: Box::new(JsonSchema::Object {
                                    properties: BTreeMap::from([
                                        (
                                            "kind".to_string(),
                                            JsonSchema::Const {
                                                value: serde_json::json!("tagged"),
                                                schema_type: Some("const".to_string()),
                                                description: None,
                                            },
                                        ),
                                        (
                                            "scope".to_string(),
                                            JsonSchema::Enum {
                                                values: vec![
                                                    serde_json::json!("one"),
                                                    serde_json::json!("two"),
                                                ],
                                                schema_type: Some("enum".to_string()),
                                                description: None,
                                            },
                                        ),
                                        (
                                            "variant".to_string(),
                                            JsonSchema::Enum {
                                                values: vec![
                                                    serde_json::json!("alpha"),
                                                    serde_json::json!("beta"),
                                                ],
                                                schema_type: Some("enum".to_string()),
                                                description: None,
                                            },
                                        ),
                                    ]),
                                    required: Some(vec![
                                        "kind".to_string(),
                                        "variant".to_string(),
                                        "scope".to_string(),
                                    ]),
                                    additional_properties: None,
                                }),
                                description: None,
                            },
                            JsonSchema::Null { description: None },
                        ],
                        description: None,
                    },
                ),
            ]),
            required: None,
            additional_properties: None,
        }
    );
}
