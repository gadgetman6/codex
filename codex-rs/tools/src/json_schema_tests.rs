use super::AdditionalProperties;
use super::JsonSchema;
use super::JsonSchemaPrimitiveType;
use super::JsonSchemaType;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;

#[test]
fn parse_tool_input_schema_coerces_boolean_schemas() {
    let schema = super::parse_tool_input_schema(&serde_json::json!(true)).expect("parse schema");

    assert_eq!(schema, JsonSchema::string(/*description*/ None));
}

#[test]
fn parse_tool_input_schema_infers_object_shape_and_defaults_properties() {
    let schema = super::parse_tool_input_schema(&serde_json::json!({
        "properties": {
            "query": {"description": "search query"}
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::object(
            BTreeMap::from([(
                "query".to_string(),
                JsonSchema::string(Some("search query".to_string())),
            )]),
            /*required*/ None,
            /*additional_properties*/ None
        )
    );
}

#[test]
fn parse_tool_input_schema_preserves_integer_and_defaults_array_items() {
    let schema = super::parse_tool_input_schema(&serde_json::json!({
        "type": "object",
        "properties": {
            "page": {"type": "integer"},
            "tags": {"type": "array"}
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::object(
            BTreeMap::from([
                (
                    "page".to_string(),
                    JsonSchema::integer(/*description*/ None),
                ),
                (
                    "tags".to_string(),
                    JsonSchema::array(
                        JsonSchema::string(/*description*/ None),
                        /*description*/ None,
                    )
                ),
            ]),
            /*required*/ None,
            /*additional_properties*/ None
        )
    );
}

#[test]
fn parse_tool_input_schema_sanitizes_additional_properties_schema() {
    let schema = super::parse_tool_input_schema(&serde_json::json!({
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
        JsonSchema::object(
            BTreeMap::new(),
            /*required*/ None,
            Some(AdditionalProperties::Schema(Box::new(JsonSchema::object(
                BTreeMap::from([(
                    "value".to_string(),
                    JsonSchema::any_of(
                        vec![
                            JsonSchema::string(/*description*/ None),
                            JsonSchema::number(/*description*/ None),
                        ],
                        /*description*/ None,
                    ),
                )]),
                Some(vec!["value".to_string()]),
                /*additional_properties*/ None,
            ))))
        )
    );
}

#[test]
fn parse_tool_input_schema_preserves_nested_nullable_any_of_shape() {
    let schema = super::parse_tool_input_schema(&serde_json::json!({
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
            }
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::object(
            BTreeMap::from([(
                "open".to_string(),
                JsonSchema::any_of(
                    vec![
                        JsonSchema::array(
                            JsonSchema::object(
                                BTreeMap::from([
                                    (
                                        "lineno".to_string(),
                                        JsonSchema::any_of(
                                            vec![
                                                JsonSchema::integer(/*description*/ None),
                                                JsonSchema::null(/*description*/ None),
                                            ],
                                            /*description*/ None,
                                        ),
                                    ),
                                    (
                                        "ref_id".to_string(),
                                        JsonSchema::string(/*description*/ None),
                                    ),
                                ]),
                                Some(vec!["ref_id".to_string()]),
                                Some(false.into()),
                            ),
                            /*description*/ None,
                        ),
                        JsonSchema::null(/*description*/ None),
                    ],
                    /*description*/ None,
                ),
            ),]),
            /*required*/ None,
            /*additional_properties*/ None
        )
    );
}

#[test]
fn parse_tool_input_schema_preserves_type_unions_without_rewriting_to_any_of() {
    let schema = super::parse_tool_input_schema(&serde_json::json!({
        "type": ["string", "null"],
        "description": "optional string"
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema {
            schema_type: Some(JsonSchemaType::Multiple(vec![
                JsonSchemaPrimitiveType::String,
                JsonSchemaPrimitiveType::Null,
            ])),
            description: Some("optional string".to_string()),
            ..Default::default()
        }
    );
}

#[test]
fn parse_tool_input_schema_preserves_string_enum_constraints() {
    let schema = super::parse_tool_input_schema(&serde_json::json!({
        "type": "object",
        "properties": {
            "response_length": {
                "type": "enum",
                "enum": ["short", "medium", "long"]
            },
            "kind": {
                "type": "const",
                "const": "tagged"
            },
            "scope": {
                "type": "enum",
                "enum": ["one", "two"]
            }
        }
    }))
    .expect("parse schema");

    assert_eq!(
        schema,
        JsonSchema::object(
            BTreeMap::from([
                (
                    "kind".to_string(),
                    JsonSchema::string_enum(
                        vec![serde_json::json!("tagged")],
                        /*description*/ None,
                    ),
                ),
                (
                    "response_length".to_string(),
                    JsonSchema::string_enum(
                        vec![
                            serde_json::json!("short"),
                            serde_json::json!("medium"),
                            serde_json::json!("long"),
                        ],
                        /*description*/ None,
                    ),
                ),
                (
                    "scope".to_string(),
                    JsonSchema::string_enum(
                        vec![serde_json::json!("one"), serde_json::json!("two")],
                        /*description*/ None,
                    ),
                ),
            ]),
            /*required*/ None,
            /*additional_properties*/ None
        )
    );
}
