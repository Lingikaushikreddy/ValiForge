use crate::SchemaError;
use openapiv3::{OpenAPI, PathItem, ReferenceOr, StatusCode};
use valiforge_core::engine::{EndpointParam, ParamLocation, ParsedEndpoint};

/// Parse an `OpenAPI` 3.x spec string into endpoints.
///
/// # Errors
/// Returns error if the YAML/JSON is invalid or the `OpenAPI` version is unsupported.
pub fn parse_openapi_string(content: &str) -> Result<Vec<ParsedEndpoint>, SchemaError> {
    let spec: OpenAPI = serde_yaml::from_str(content)
        .or_else(|_| serde_json::from_str::<OpenAPI>(content).map_err(SchemaError::JsonParse))?;

    if !spec.openapi.starts_with("3.0") && !spec.openapi.starts_with("3.1") {
        return Err(SchemaError::UnsupportedVersion {
            version: spec.openapi.clone(),
        });
    }

    let mut endpoints = Vec::new();

    for (path, path_item_ref) in &spec.paths.paths {
        let path_item = match path_item_ref {
            ReferenceOr::Item(item) => item,
            ReferenceOr::Reference { .. } => continue,
        };

        extract_operations(path, path_item, &mut endpoints);
    }

    Ok(endpoints)
}

fn extract_operations(path: &str, item: &PathItem, endpoints: &mut Vec<ParsedEndpoint>) {
    let ops = [
        ("GET", &item.get),
        ("POST", &item.post),
        ("PUT", &item.put),
        ("DELETE", &item.delete),
        ("PATCH", &item.patch),
        ("HEAD", &item.head),
        ("OPTIONS", &item.options),
    ];

    for (method, op) in ops {
        if let Some(operation) = op {
            let params: Vec<EndpointParam> = operation
                .parameters
                .iter()
                .filter_map(|p| match p {
                    ReferenceOr::Item(param) => Some(EndpointParam {
                        name: param_name(param),
                        location: param_location(param),
                        required: param_required(param),
                        schema: param_schema(param),
                    }),
                    ReferenceOr::Reference { .. } => None,
                })
                .collect();

            let (expected_status, response_schema) = extract_response_info(&operation.responses);

            endpoints.push(ParsedEndpoint {
                method: method.to_string(),
                path: path.to_string(),
                parameters: params,
                expected_status,
                response_schema,
            });
        }
    }
}

fn extract_response_info(responses: &openapiv3::Responses) -> (u16, Option<serde_json::Value>) {
    for code in [200, 201, 204] {
        let key = StatusCode::Code(code);
        if let Some(ReferenceOr::Item(resp)) = responses.responses.get(&key) {
            if let Some(content) = resp.content.get("application/json") {
                if let Some(ReferenceOr::Item(schema)) = &content.schema {
                    if let Ok(json) = serde_json::to_value(schema) {
                        return (code, Some(json));
                    }
                }
            }
            return (code, None);
        }
    }
    (200, None)
}

fn param_name(param: &openapiv3::Parameter) -> String {
    match param {
        openapiv3::Parameter::Query { parameter_data, .. }
        | openapiv3::Parameter::Header { parameter_data, .. }
        | openapiv3::Parameter::Path { parameter_data, .. }
        | openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data.name.clone(),
    }
}

fn param_location(param: &openapiv3::Parameter) -> ParamLocation {
    match param {
        openapiv3::Parameter::Query { .. } => ParamLocation::Query,
        openapiv3::Parameter::Header { .. } => ParamLocation::Header,
        openapiv3::Parameter::Path { .. } => ParamLocation::Path,
        openapiv3::Parameter::Cookie { .. } => ParamLocation::Cookie,
    }
}

fn param_required(param: &openapiv3::Parameter) -> bool {
    match param {
        openapiv3::Parameter::Path { .. } => true,
        openapiv3::Parameter::Query { parameter_data, .. }
        | openapiv3::Parameter::Header { parameter_data, .. }
        | openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data.required,
    }
}

fn param_schema(param: &openapiv3::Parameter) -> Option<serde_json::Value> {
    let data = match param {
        openapiv3::Parameter::Query { parameter_data, .. }
        | openapiv3::Parameter::Header { parameter_data, .. }
        | openapiv3::Parameter::Path { parameter_data, .. }
        | openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data,
    };
    match &data.format {
        openapiv3::ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) => {
            serde_json::to_value(schema).ok()
        }
        _ => None,
    }
}
