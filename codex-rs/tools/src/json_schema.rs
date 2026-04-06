use serde::Serialize;
use serde::Serializer;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::BTreeMap;

/// Generic JSON-Schema subset needed for our tool definitions.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonSchema {
    Boolean {
        description: Option<String>,
    },
    String {
        description: Option<String>,
    },
    /// MCP schema allows "number" | "integer" for Number.
    Number {
        description: Option<String>,
    },
    Null {
        description: Option<String>,
    },
    Array {
        items: Box<JsonSchema>,
        description: Option<String>,
    },
    Object {
        properties: BTreeMap<String, JsonSchema>,
        required: Option<Vec<String>>,
        additional_properties: Option<AdditionalProperties>,
    },
    Const {
        value: JsonValue,
        schema_type: Option<String>,
        description: Option<String>,
    },
    Enum {
        values: Vec<JsonValue>,
        schema_type: Option<String>,
        description: Option<String>,
    },
    AnyOf {
        variants: Vec<JsonSchema>,
        description: Option<String>,
    },
    OneOf {
        variants: Vec<JsonSchema>,
        description: Option<String>,
    },
    AllOf {
        variants: Vec<JsonSchema>,
        description: Option<String>,
    },
}

/// Whether additional properties are allowed, and if so, any required schema.
#[derive(Debug, Clone, PartialEq)]
pub enum AdditionalProperties {
    Boolean(bool),
    Schema(Box<JsonSchema>),
}

impl From<bool> for AdditionalProperties {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<JsonSchema> for AdditionalProperties {
    fn from(value: JsonSchema) -> Self {
        Self::Schema(Box::new(value))
    }
}

impl Serialize for JsonSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        json_schema_to_json(self).serialize(serializer)
    }
}

impl Serialize for AdditionalProperties {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Boolean(value) => value.serialize(serializer),
            Self::Schema(schema) => json_schema_to_json(schema).serialize(serializer),
        }
    }
}

/// Parse the tool `input_schema` or return an error for invalid schema.
pub fn parse_tool_input_schema(input_schema: &JsonValue) -> Result<JsonSchema, serde_json::Error> {
    let mut input_schema = input_schema.clone();
    sanitize_json_schema(&mut input_schema);
    parse_json_schema(&input_schema)
}

/// Sanitize a JSON Schema (as serde_json::Value) so it can fit our limited
/// JsonSchema enum. This function:
/// - Infers a concrete `"type"` when it is missing and the shape can be reduced
///   to our supported subset (properties => object, items => array,
///   enum/const/format => string).
/// - Preserves explicit combiners like `anyOf`/`oneOf`/`allOf` and nullable
///   unions instead of collapsing them to a single fallback type.
/// - Fills required child fields (e.g. array items, object properties) with
///   permissive defaults when absent.
fn sanitize_json_schema(value: &mut JsonValue) {
    match value {
        JsonValue::Bool(_) => {
            // JSON Schema boolean form: true/false. Coerce to an accept-all string.
            *value = json!({ "type": "string" });
        }
        JsonValue::Array(values) => {
            for value in values {
                sanitize_json_schema(value);
            }
        }
        JsonValue::Object(map) => {
            if let Some(properties) = map.get_mut("properties")
                && let Some(properties_map) = properties.as_object_mut()
            {
                for value in properties_map.values_mut() {
                    sanitize_json_schema(value);
                }
            }
            if let Some(items) = map.get_mut("items") {
                sanitize_json_schema(items);
            }
            for combiner in ["oneOf", "anyOf", "allOf", "prefixItems"] {
                if let Some(value) = map.get_mut(combiner) {
                    sanitize_json_schema(value);
                }
            }

            let mut schema_type = map
                .get("type")
                .and_then(|value| value.as_str())
                .map(str::to_string);

            if schema_type.is_none() {
                if map.contains_key("properties")
                    || map.contains_key("required")
                    || map.contains_key("additionalProperties")
                {
                    schema_type = Some("object".to_string());
                } else if map.contains_key("items") || map.contains_key("prefixItems") {
                    schema_type = Some("array".to_string());
                } else if map.contains_key("enum")
                    || map.contains_key("const")
                    || map.contains_key("format")
                {
                    schema_type = Some("string".to_string());
                } else if map.contains_key("minimum")
                    || map.contains_key("maximum")
                    || map.contains_key("exclusiveMinimum")
                    || map.contains_key("exclusiveMaximum")
                    || map.contains_key("multipleOf")
                {
                    schema_type = Some("number".to_string());
                }
            }

            if let Some(schema_type) = &schema_type {
                map.insert("type".to_string(), JsonValue::String(schema_type.clone()));
            }

            if schema_type.as_deref() == Some("object") {
                if !map.contains_key("properties") {
                    map.insert(
                        "properties".to_string(),
                        JsonValue::Object(serde_json::Map::new()),
                    );
                }
                if let Some(additional_properties) = map.get_mut("additionalProperties")
                    && !matches!(additional_properties, JsonValue::Bool(_))
                {
                    sanitize_json_schema(additional_properties);
                }
            }

            if schema_type.as_deref() == Some("array") && !map.contains_key("items") {
                map.insert("items".to_string(), json!({ "type": "string" }));
            }
        }
        _ => {}
    }
}

fn parse_json_schema(value: &JsonValue) -> Result<JsonSchema, serde_json::Error> {
    match value {
        JsonValue::Bool(_) => Ok(JsonSchema::String { description: None }),
        JsonValue::Object(map) => {
            let description = map
                .get("description")
                .and_then(JsonValue::as_str)
                .map(str::to_string);

            if let Some(value) = map.get("const") {
                return Ok(JsonSchema::Const {
                    value: value.clone(),
                    schema_type: map
                        .get("type")
                        .and_then(JsonValue::as_str)
                        .map(str::to_string),
                    description,
                });
            }

            if let Some(values) = map.get("enum").and_then(JsonValue::as_array) {
                return Ok(JsonSchema::Enum {
                    values: values.clone(),
                    schema_type: map
                        .get("type")
                        .and_then(JsonValue::as_str)
                        .map(str::to_string),
                    description,
                });
            }

            if let Some(variants) = map.get("anyOf").and_then(JsonValue::as_array) {
                return Ok(JsonSchema::AnyOf {
                    variants: variants
                        .iter()
                        .map(parse_json_schema)
                        .collect::<Result<Vec<_>, _>>()?,
                    description,
                });
            }

            if let Some(variants) = map.get("oneOf").and_then(JsonValue::as_array) {
                return Ok(JsonSchema::OneOf {
                    variants: variants
                        .iter()
                        .map(parse_json_schema)
                        .collect::<Result<Vec<_>, _>>()?,
                    description,
                });
            }

            if let Some(variants) = map.get("allOf").and_then(JsonValue::as_array) {
                return Ok(JsonSchema::AllOf {
                    variants: variants
                        .iter()
                        .map(parse_json_schema)
                        .collect::<Result<Vec<_>, _>>()?,
                    description,
                });
            }

            if let Some(types) = map.get("type").and_then(JsonValue::as_array) {
                return Ok(JsonSchema::AnyOf {
                    variants: types
                        .iter()
                        .filter_map(JsonValue::as_str)
                        .map(|schema_type| {
                            parse_json_schema(&json!({
                                "type": schema_type,
                            }))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    description,
                });
            }

            match map
                .get("type")
                .and_then(JsonValue::as_str)
                .unwrap_or("string")
            {
                "boolean" => Ok(JsonSchema::Boolean { description }),
                "string" => Ok(JsonSchema::String { description }),
                "number" | "integer" => Ok(JsonSchema::Number { description }),
                "null" => Ok(JsonSchema::Null { description }),
                "array" => Ok(JsonSchema::Array {
                    items: Box::new(parse_json_schema(
                        map.get("items").unwrap_or(&json!({ "type": "string" })),
                    )?),
                    description,
                }),
                "object" => {
                    let properties = map
                        .get("properties")
                        .and_then(JsonValue::as_object)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(name, value)| Ok((name, parse_json_schema(&value)?)))
                        .collect::<Result<BTreeMap<_, _>, serde_json::Error>>()?;
                    let required = map
                        .get("required")
                        .and_then(JsonValue::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(JsonValue::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        });
                    let additional_properties = map
                        .get("additionalProperties")
                        .map(parse_additional_properties)
                        .transpose()?;
                    Ok(JsonSchema::Object {
                        properties,
                        required,
                        additional_properties,
                    })
                }
                _ => Ok(JsonSchema::String { description }),
            }
        }
        _ => Ok(JsonSchema::String { description: None }),
    }
}

fn parse_additional_properties(
    value: &JsonValue,
) -> Result<AdditionalProperties, serde_json::Error> {
    match value {
        JsonValue::Bool(flag) => Ok(AdditionalProperties::Boolean(*flag)),
        _ => Ok(AdditionalProperties::Schema(Box::new(parse_json_schema(
            value,
        )?))),
    }
}

fn json_schema_to_json(schema: &JsonSchema) -> JsonValue {
    match schema {
        JsonSchema::Boolean { description } => {
            let mut map = serde_json::Map::new();
            map.insert("type".to_string(), JsonValue::String("boolean".to_string()));
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::String { description } => {
            let mut map = serde_json::Map::new();
            map.insert("type".to_string(), JsonValue::String("string".to_string()));
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::Number { description } => {
            let mut map = serde_json::Map::new();
            map.insert("type".to_string(), JsonValue::String("number".to_string()));
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::Null { description } => {
            let mut map = serde_json::Map::new();
            map.insert("type".to_string(), JsonValue::String("null".to_string()));
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::Array { items, description } => {
            let mut map = serde_json::Map::new();
            map.insert("type".to_string(), JsonValue::String("array".to_string()));
            map.insert("items".to_string(), json_schema_to_json(items));
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::Object {
            properties,
            required,
            additional_properties,
        } => {
            let mut map = serde_json::Map::new();
            map.insert("type".to_string(), JsonValue::String("object".to_string()));
            map.insert(
                "properties".to_string(),
                JsonValue::Object(
                    properties
                        .iter()
                        .map(|(name, value)| (name.clone(), json_schema_to_json(value)))
                        .collect(),
                ),
            );
            if let Some(required) = required {
                map.insert(
                    "required".to_string(),
                    JsonValue::Array(required.iter().cloned().map(JsonValue::String).collect()),
                );
            }
            if let Some(additional_properties) = additional_properties {
                map.insert(
                    "additionalProperties".to_string(),
                    match additional_properties {
                        AdditionalProperties::Boolean(flag) => JsonValue::Bool(*flag),
                        AdditionalProperties::Schema(schema) => json_schema_to_json(schema),
                    },
                );
            }
            JsonValue::Object(map)
        }
        JsonSchema::Const {
            value,
            schema_type,
            description,
        } => {
            let mut map = serde_json::Map::new();
            map.insert("const".to_string(), value.clone());
            if let Some(schema_type) = schema_type {
                map.insert("type".to_string(), JsonValue::String(schema_type.clone()));
            }
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::Enum {
            values,
            schema_type,
            description,
        } => {
            let mut map = serde_json::Map::new();
            map.insert("enum".to_string(), JsonValue::Array(values.clone()));
            if let Some(schema_type) = schema_type {
                map.insert("type".to_string(), JsonValue::String(schema_type.clone()));
            }
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::AnyOf {
            variants,
            description,
        } => {
            let mut map = serde_json::Map::new();
            map.insert(
                "anyOf".to_string(),
                JsonValue::Array(variants.iter().map(json_schema_to_json).collect()),
            );
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::OneOf {
            variants,
            description,
        } => {
            let mut map = serde_json::Map::new();
            map.insert(
                "oneOf".to_string(),
                JsonValue::Array(variants.iter().map(json_schema_to_json).collect()),
            );
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
        JsonSchema::AllOf {
            variants,
            description,
        } => {
            let mut map = serde_json::Map::new();
            map.insert(
                "allOf".to_string(),
                JsonValue::Array(variants.iter().map(json_schema_to_json).collect()),
            );
            insert_description(&mut map, description.as_deref());
            JsonValue::Object(map)
        }
    }
}

fn insert_description(map: &mut serde_json::Map<String, JsonValue>, description: Option<&str>) {
    if let Some(description) = description {
        map.insert(
            "description".to_string(),
            JsonValue::String(description.to_string()),
        );
    }
}

#[cfg(test)]
#[path = "json_schema_tests.rs"]
mod tests;
