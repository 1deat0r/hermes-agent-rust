//! Parity tests for `tools/schema_sanitizer.py` @ 5d59366.
//!
//! Oracle: `tests/tools/test_schema_sanitizer.py` (all cases) +
//! `tests/tools/test_schema_boolean_required.py` (both cases) + live
//! interpreter probes for recursion-table edges. Tier: `unit`.

use serde_json::{json, Value};

use hermes_tools::schema_sanitizer::{
    collapse_const_unions, sanitize_property_key, sanitize_tool_schemas, strip_nullable_unions,
    strip_pattern_and_format, strip_slash_enum, unrename_tool_args,
};

fn tool(name: &str, parameters: Value) -> Value {
    json!({"type": "function", "function": {"name": name, "parameters": parameters}})
}

fn params(out: &[Value]) -> &Value {
    &out[0]["function"]["parameters"]
}

// ── shapes from test_schema_sanitizer.py ─────────────────────────────────

#[test]
fn oracle_object_without_properties_gets_empty_properties() {
    let out = sanitize_tool_schemas(&[tool("t", json!({"type": "object"}))]);
    assert_eq!(params(&out), &json!({"type": "object", "properties": {}}));
}

#[test]
fn oracle_nested_object_without_properties_keeps_description() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "arguments": {"type": "object", "description": "free-form"},
            },
            "required": ["name"],
        }),
    )]);
    let args = &params(&out)["properties"]["arguments"];
    assert_eq!(args["type"], json!("object"));
    assert_eq!(args["properties"], json!({}));
    assert_eq!(args["description"], json!("free-form"));
}

#[test]
fn oracle_bare_string_object_value_replaced_with_schema_dict() {
    // The exact llama.cpp `Unrecognized schema: "object"` shape.
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object", "properties": {"payload": "object"}}),
    )]);
    let payload = &params(&out)["properties"]["payload"];
    assert!(payload.is_object());
    assert_eq!(payload["type"], json!("object"));
    assert_eq!(payload["properties"], json!({}));
}

#[test]
fn oracle_nullable_type_array_collapsed_to_single_string() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"maybe_name": {"type": ["string", "null"]}}}),
    )]);
    let prop = &params(&out)["properties"]["maybe_name"];
    assert_eq!(prop["type"], json!("string"));
    assert_eq!(prop.get("nullable"), Some(&json!(true)));
}

#[test]
fn oracle_multitype_array_becomes_anyof_no_branch_dropped() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"status": {"type": ["number", "string"],
                                        "description": "status filter"}}}),
    )]);
    let prop = &params(&out)["properties"]["status"];
    assert!(prop.get("type").is_none());
    assert_eq!(
        prop["anyOf"],
        json!([{"type": "number"}, {"type": "string"}])
    );
    assert!(prop.get("nullable").is_none());
    assert_eq!(prop["description"], json!("status filter"));
}

#[test]
fn oracle_all_null_type_array_becomes_null_type() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object", "properties": {"n": {"type": ["null"]}}}),
    )]);
    assert_eq!(params(&out)["properties"]["n"]["type"], json!("null"));
}

#[test]
fn oracle_single_element_type_array_unwrapped() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object", "properties": {"s": {"type": ["string"]}}}),
    )]);
    let prop = &params(&out)["properties"]["s"];
    assert_eq!(prop["type"], json!("string"));
    assert!(prop.get("nullable").is_none());
}

#[test]
fn oracle_anyof_nested_objects_sanitized() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"opt": {"anyOf": [{"type": "object"},
                                                     {"type": "string"}]}}}),
    )]);
    let variants = params(&out)["properties"]["opt"]["anyOf"].clone();
    assert_eq!(variants[0], json!({"type": "object", "properties": {}}));
    assert_eq!(variants[1], json!({"type": "string"}));
}

#[test]
fn oracle_missing_and_non_dict_parameters_get_default_schema() {
    let out = sanitize_tool_schemas(&[json!({"type": "function", "function": {"name": "t"}})]);
    assert_eq!(params(&out), &json!({"type": "object", "properties": {}}));
    // Pathological non-dict parameters → same minimal shape.
    let out = sanitize_tool_schemas(&[tool("t", json!("object"))]);
    assert_eq!(params(&out), &json!({"type": "object", "properties": {}}));
}

#[test]
fn oracle_required_pruned_to_existing_properties() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"name": {"type": "string"}},
               "required": ["name", "missing_field"]}),
    )]);
    assert_eq!(params(&out)["required"], json!(["name"]));
}

#[test]
fn oracle_well_formed_schema_unchanged() {
    let schema = json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "File path"},
            "offset": {"type": "integer", "minimum": 1},
        },
        "required": ["path"],
    });
    let input = tool("read_file", schema.clone());
    let out = sanitize_tool_schemas(std::slice::from_ref(&input));
    // Input is never mutated; output equals the input clone.
    assert_eq!(out[0]["function"]["parameters"], schema);
    assert_eq!(input["function"]["parameters"], schema);
}

#[test]
fn oracle_additional_properties_and_items_sanitized() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"dict_field": {"type": "object",
                    "additionalProperties": {"type": "object"}}}}),
    )]);
    assert_eq!(
        params(&out)["properties"]["dict_field"]["additionalProperties"],
        json!({"type": "object", "properties": {}})
    );
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"bag": {"type": "array",
                                            "items": {"type": "object"}}}}),
    )]);
    assert_eq!(
        params(&out)["properties"]["bag"]["items"],
        json!({"type": "object", "properties": {}})
    );
}

#[test]
fn oracle_strip_mixed_openai_and_responses_formats() {
    let mut tools = vec![
        json!({"type": "function", "function": {"name": "search",
            "parameters": {"type": "object",
                "properties": {"query": {"type": "string", "pattern": "^[a-z]+$"}}}}}),
        json!({"name": "get_time",
               "parameters": {"type": "object",
                   "properties": {"tz": {"type": "string", "format": "date-time"}}},
               "type": "function"}),
    ];
    let stripped = strip_pattern_and_format(&mut tools);
    assert_eq!(stripped, 2, "1 pattern + 1 format");
    assert!(tools[0]["function"]["parameters"]["properties"]["query"]
        .get("pattern")
        .is_none());
    assert!(tools[1]["parameters"]["properties"]["tz"]
        .get("format")
        .is_none());
    assert_eq!(tools[0]["function"]["parameters"]["type"], json!("object"));
    assert_eq!(tools[1]["parameters"]["type"], json!("object"));
}

#[test]
fn oracle_sanitize_property_key_empty_falls_back() {
    assert_eq!(sanitize_property_key("~~~"), "___");
    assert_eq!(sanitize_property_key(""), "param");
}

#[test]
fn oracle_dependent_required_preserved_and_input_untouched() {
    let schema = json!({
        "type": "object",
        "properties": {
            "owner": {"type": "string"},
            "repo": {"type": "string"},
            "organization": {"type": "string"},
        },
        "dependentRequired": {
            "owner": ["repo", "organization"],
            "repo": ["owner"],
        },
    });
    let out = sanitize_tool_schemas(std::slice::from_ref(&tool("t", schema.clone())));
    let dep = &params(&out)["dependentRequired"];
    assert_eq!(dep["owner"], json!(["repo", "organization"]));
    assert_eq!(dep["repo"], json!(["owner"]));
    assert_eq!(
        params(&out)["properties"]["owner"],
        json!({"type": "string"})
    );
    // The caller's schema object is unchanged (deep-copy semantics).
    let schema2 = schema.clone();
    let _ = sanitize_tool_schemas(std::slice::from_ref(&tool("t", schema.clone())));
    assert_eq!(schema, schema2);
}

#[test]
fn oracle_dependent_schemas_still_recursively_sanitized() {
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"owner": {"type": "string"}},
               "dependentSchemas": {"owner": {"type": "object"}}}),
    )]);
    assert_eq!(
        params(&out)["dependentSchemas"]["owner"],
        json!({"type": "object", "properties": {}})
    );
}

#[test]
fn oracle_pure_and_oneof_const_unions_collapse_to_enum() {
    assert_eq!(
        collapse_const_unions(
            &json!({"anyOf": [{"const": "red"}, {"const": "green"}, {"const": "blue"}]})
        ),
        json!({"type": "string", "enum": ["red", "green", "blue"]})
    );
    assert_eq!(
        collapse_const_unions(&json!({"oneOf": [{"const": 1}, {"const": 2}, {"const": 3}]})),
        json!({"type": "integer", "enum": [1, 2, 3]})
    );
}

#[test]
fn oracle_mixed_and_non_uniform_const_unions_left_alone() {
    let schema = json!({"anyOf": [{"const": "a"}, {"type": "string", "minLength": 3}]});
    assert_eq!(collapse_const_unions(&schema), schema);
    let schema = json!({"anyOf": [{"const": "a"}, {"const": 1}]});
    assert_eq!(collapse_const_unions(&schema), schema);
}

#[test]
fn oracle_bool_consts_not_confused_with_integers() {
    // Python `bool` subclasses `int`; the port must use exact-type lookup.
    let schema = json!({"anyOf": [{"const": true}, {"const": 1}]});
    assert_eq!(collapse_const_unions(&schema), schema);
    assert_eq!(
        collapse_const_unions(&json!({"anyOf": [{"const": true}, {"const": false}]})),
        json!({"type": "boolean", "enum": [true, false]})
    );
}

#[test]
fn oracle_nested_const_unions_collapse() {
    let out = collapse_const_unions(&json!({
        "type": "object",
        "properties": {
            "mode": {"anyOf": [{"const": "fast"}, {"const": "slow"}]},
            "inner": {"type": "object",
                      "properties": {"level": {"oneOf": [{"const": 1}, {"const": 2}]}}},
        },
    }));
    assert_eq!(
        out["properties"]["mode"],
        json!({"type": "string", "enum": ["fast", "slow"]})
    );
    assert_eq!(
        out["properties"]["inner"]["properties"]["level"],
        json!({"type": "integer", "enum": [1, 2]})
    );
}

#[test]
fn oracle_outer_metadata_carried_onto_collapsed_enum() {
    let out = collapse_const_unions(&json!({
        "title": "Color", "description": "Pick a color", "default": "red",
        "anyOf": [{"const": "red"}, {"const": "blue"}],
    }));
    assert_eq!(
        out,
        json!({"type": "string", "enum": ["red", "blue"],
               "title": "Color", "description": "Pick a color", "default": "red"})
    );
}

#[test]
fn oracle_branch_metadata_does_not_block_collapse() {
    let out = collapse_const_unions(&json!({
        "anyOf": [{"const": "a", "title": "A", "description": "first"},
                  {"const": "b", "type": "string"}],
    }));
    assert_eq!(out, json!({"type": "string", "enum": ["a", "b"]}));
}

#[test]
fn oracle_branch_with_mismatched_declared_type_left_alone() {
    let schema = json!({"anyOf": [{"const": "a", "type": "integer"}, {"const": "b"}]});
    assert_eq!(collapse_const_unions(&schema), schema);
}

#[test]
fn oracle_null_plus_const_union_keeps_nullable_hint() {
    // MCP pipeline order: nullable strip leaves 2-non-null unions alone,
    // then const collapse tolerates the single null branch as nullable.
    let out = collapse_const_unions(&json!({
        "anyOf": [{"const": "fast"}, {"const": "slow"}, {"type": "null"}],
    }));
    assert_eq!(out["type"], json!("string"));
    assert_eq!(out["enum"], json!(["fast", "slow"]));
    assert_eq!(out.get("nullable"), Some(&json!(true)));
    // Two null branches block the collapse.
    let schema = json!({"anyOf": [{"const": "a"}, {"type": "null"}, {"type": "null"}]});
    assert_eq!(collapse_const_unions(&schema), schema);
}

#[test]
fn oracle_collapse_is_deterministic_and_non_mutating() {
    let schema = json!({"anyOf": [{"const": "x"}, {"const": "y"}]});
    let snapshot = schema.clone();
    collapse_const_unions(&schema);
    assert_eq!(schema, snapshot);
    let schema = json!({"anyOf": [{"const": "b"}, {"const": "a"}]});
    let first = collapse_const_unions(&schema);
    let second = collapse_const_unions(&schema);
    assert_eq!(first, second);
    assert_eq!(first, json!({"type": "string", "enum": ["b", "a"]}));
}

// ── shapes from test_schema_boolean_required.py ──────────────────────────

#[test]
fn oracle_boolean_required_lifts_to_parent_and_prunes() {
    // Legacy property-level `required: true` lifts `url path` (renamed to
    // `url_path`) into the parent items' required list; `required: false`
    // vanishes; parent intent (`[]` or `["existing"]`) is preserved.
    for required in [json!([]), json!(["existing"])] {
        let schema = json!({"type": "object", "required": required,
        "properties": {
            "existing": {"type": "string"},
            "nested": {"type": "array", "items": {"properties": {
                "url path": {"required": true},
                "flag": {"type": "boolean", "required": false},
            }}},
        }});
        let original = schema.clone();
        let out = sanitize_tool_schemas(std::slice::from_ref(&tool("probe", schema.clone())));
        let result = params(&out);
        assert_eq!(
            result["properties"]["nested"]["items"]["required"],
            json!(["url_path"])
        );
        // Absent `required` reads as `[]` (upstream `result.get("required", [])`).
        assert_eq!(
            result.get("required").cloned().unwrap_or(json!([])),
            required
        );
        assert_eq!(schema, original, "input untouched");
    }
}

#[test]
fn oracle_boolean_required_does_not_touch_literal_data() {
    // `const`/`default`/`example`/extension payloads are literal data —
    // a `required: True` inside them is not a schema flag.
    let literal =
        json!({"type": "object", "required": true, "properties": {"x": {"required": false}}});
    let mut prop = json!({"const": literal.clone(), "default": literal.clone(),
                          "example": literal.clone(), "x-metadata": literal.clone(),
                          "enum": [literal.clone()], "examples": [literal.clone()]});
    let _ = &mut prop;
    let schema = json!({"type": "object", "properties": {"value": prop.clone()}});
    let out = sanitize_tool_schemas(std::slice::from_ref(&tool("probe", schema)));
    assert_eq!(
        out[0]["function"]["parameters"]["properties"]["value"],
        prop
    );
}

// ── probe-pinned recursion-table edges ───────────────────────────────────

#[test]
fn probe_schema_map_keys_recurse_with_rename_scope() {
    // Oracle-verified: patternProperties/$defs/definitions/dependentSchemas
    // recurse (bare objects gain properties); renames apply ONLY to
    // `properties` (a bad key under $defs keeps its name).
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "patternProperties": {"^x-": {"type": "object"}},
               "$defs": {"A": {"type": "object"}},
               "definitions": {"B": {"type": "string"}},
               "dependentSchemas": {"q": "oops-string"}}),
    )]);
    let p = params(&out);
    assert_eq!(
        p["patternProperties"]["^x-"],
        json!({"type": "object", "properties": {}})
    );
    assert_eq!(p["$defs"]["A"], json!({"type": "object", "properties": {}}));
    assert_eq!(p["definitions"]["B"], json!({"type": "string"}));
    assert_eq!(
        p["dependentSchemas"]["q"],
        json!({"type": "object", "properties": {}})
    );
}

#[test]
fn probe_schema_child_keys_recurse_and_literals_pass_through() {
    // Oracle-verified: unevaluatedProperties/contains/if/then/prefixItems
    // recurse (strings become object schemas); propertyNames/else pass
    // through the literal arm (else-string → object schema via the
    // catch-all object/array branch; propertyNames dict recurses by key).
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"a": {"type": "string"}},
               "unevaluatedProperties": {"type": "object"},
               "contains": "oops",
               "propertyNames": {"type": "string"},
               "if": {"type": "object"},
               "then": "oops2",
               "else": {"type": "string"},
               "prefixItems": [{"type": "object"}],
               "not": "oops3"}),
    )]);
    let p = params(&out);
    assert_eq!(
        p["unevaluatedProperties"],
        json!({"type": "object", "properties": {}})
    );
    assert_eq!(p["contains"], json!({"type": "object", "properties": {}}));
    assert_eq!(p["if"], json!({"type": "object", "properties": {}}));
    assert_eq!(p["then"], json!({"type": "object", "properties": {}}));
    assert_eq!(p["else"], json!({"type": "string"}));
    assert_eq!(
        p["prefixItems"],
        json!([{"type": "object", "properties": {}}])
    );
}

#[test]
fn probe_dependencies_dict_and_lift_then_prune() {
    // Oracle-verified: list-valued dependencies pass through; dict values
    // recurse; property-level `required: true` lifts then prunes against
    // the renamed properties (non-list parent required + stray types drop).
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "dependencies": {"a": ["b"], "c": {"type": "object"}}}),
    )]);
    assert_eq!(params(&out)["dependencies"]["a"], json!(["b"]));
    assert_eq!(
        params(&out)["dependencies"]["c"],
        json!({"type": "object", "properties": {}})
    );
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"a": {"type": "string", "required": true, "x": 1}},
               "required": "oops"}),
    )]);
    assert_eq!(
        params(&out)["properties"]["a"],
        json!({"type": "string", "x": 1})
    );
    assert_eq!(params(&out)["required"], json!(["a"]));
    // Required entries of non-string type prune away entirely.
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"q": {"type": "object", "required": [123, "q"]}}}),
    )]);
    assert!(params(&out)["properties"]["q"].get("required").is_none());
}

#[test]
fn probe_rename_collision_and_required_remap() {
    // Oracle-verified: `bad key!` → `bad_key_`; required remaps then the
    // prune drops the non-string entry (`5` is not a property name);
    // with no required, no key is added.
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object",
               "properties": {"bad key!": {"type": "string"}},
               "required": ["bad key!", 5]}),
    )]);
    assert!(params(&out)["properties"].get("bad_key_").is_some());
    assert_eq!(params(&out)["required"], json!(["bad_key_"]));
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object", "properties": {"bad key!": {"type": "string"}}}),
    )]);
    assert!(params(&out)["properties"].get("bad_key_").is_some());
    assert!(params(&out).get("required").is_none());
}

#[test]
fn probe_type_array_edges_and_bare_strings() {
    // Oracle-verified: a TOP-level multi-type array normalizes to anyOf
    // inside `_sanitize_node`, but the single-tool pass then forces
    // `type: object` + strips top-level combinators — the anyOf does NOT
    // survive at top level (stays `nullable` + sibling keys only).
    // Nested multi-type arrays (see the oracle anyOf test) keep branches.
    let out = sanitize_tool_schemas(&[tool("t", json!({"type": ["a", "b", "null"], "x": 1}))]);
    assert!(params(&out).get("anyOf").is_none());
    assert_eq!(params(&out)["type"], json!("object"));
    assert_eq!(params(&out).get("nullable"), Some(&json!(true)));
    assert_eq!(params(&out)["x"], json!(1));
    let out = sanitize_tool_schemas(&[tool("t", json!({"type": [1, 2]}))]);
    assert_eq!(params(&out)["type"], json!("object"));
    let out = sanitize_tool_schemas(&[tool("t", json!("oops"))]);
    assert_eq!(params(&out), &json!({"type": "object", "properties": {}}));
    let out = sanitize_tool_schemas(&[tool("t", json!(["string", 5]))]);
    assert_eq!(params(&out), &json!({"type": "object", "properties": {}}));
}

#[test]
fn probe_top_level_combinators_and_ref_siblings_stripped() {
    // Oracle-verified: every top-level combinator drops (validity kept —
    // handlers re-validate); forbidden `default` beside `$ref` drops while
    // sibling type/description survive; outer union meta carries except
    // default-onto-ref.
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object", "properties": {},
               "anyOf": [{"type": "string"}], "allOf": [1],
               "enum": ["a"], "not": {}}),
    )]);
    for key in ["anyOf", "allOf", "enum", "not"] {
        assert!(params(&out).get(key).is_none());
    }
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"$ref": "#/x", "default": "d", "type": "string", "description": "keep"}),
    )]);
    assert!(params(&out).get("default").is_none());
    assert_eq!(params(&out)["description"], json!("keep"));
    // Nullable-union collapse keeps the survivor default, copies outer
    // title/examples, skips outer default onto $ref, and honors
    // keep_nullable_hint=false.
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"anyOf": [{"type": "null"},
                        {"type": "string", "default": "x"}],
               "title": "T", "default": "outer", "examples": [1]}),
    )]);
    assert_eq!(params(&out)["type"], json!("string"));
    assert_eq!(params(&out)["default"], json!("x"));
    assert_eq!(params(&out)["title"], json!("T"));
    assert_eq!(
        strip_nullable_unions(
            &json!({"anyOf": [{"type": "null"}, {"type": "string"}]}),
            false
        ),
        json!({"type": "string"})
    );
}

#[test]
fn probe_unrename_recursion_and_passthrough() {
    // Oracle-verified: sanitized keys map back recursively through
    // objects and array items; unknown keys pass through; non-dict
    // schemas/args pass through untouched.
    let schema = json!({"type": "object",
                        "properties": {"a~b": {"type": "string"}}});
    assert_eq!(
        unrename_tool_args(&schema, &json!({"a_b": 1, "zzz": 2})),
        json!({"a~b": 1, "zzz": 2})
    );
    assert_eq!(
        unrename_tool_args(&json!({"properties": "oops"}), &json!({"a": 1})),
        json!({"a": 1})
    );
    assert_eq!(
        unrename_tool_args(&json!({}), &json!([1, 2])),
        json!([1, 2])
    );
    // Long keys truncate to 64 chars; `~~~` → `___`.
    assert_eq!(sanitize_property_key(&"x".repeat(100)).len(), 64);
    assert_eq!(sanitize_property_key("~~~"), "___");
}

#[test]
fn probe_rename_suffix_chain_and_unrename_round_trip() {
    // Oracle-verified: `a!`→`a__2`, `a?`→`a__3` (base `a_` pre-taken by a
    // conforming key); unrename maps the suffixed keys back exactly.
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object", "properties": {
            "a!": {"type": "string"}, "a?": {"type": "string"}, "a_": {"type": "string"}}}),
    )]);
    let props = &params(&out)["properties"];
    assert!(props.get("a_").is_some());
    assert!(props.get("a__2").is_some());
    assert!(props.get("a__3").is_some());
    let schema = json!({"type": "object",
        "properties": {"a!": {"type": "string"}, "a?": {"type": "string"}, "a_": {"type": "string"}}});
    assert_eq!(
        unrename_tool_args(&schema, &json!({"a_": 1, "a__2": 2, "a__3": 3})),
        json!({"a_": 1, "a!": 2, "a?": 3})
    );
    // Oracle-verified: nested objects and array items recurse on unrename.
    let schema = json!({"type": "object", "properties": {
        "o~": {"type": "object", "properties": {"i~": {"type": "string"}}},
        "arr~": {"type": "array",
                 "items": {"type": "object", "properties": {"x~": {"type": "number"}}}}}});
    assert_eq!(
        unrename_tool_args(&schema, &json!({"o_": {"i_": 1, "extra": 2}})),
        json!({"o~": {"i~": 1, "extra": 2}})
    );
    assert_eq!(
        unrename_tool_args(&schema, &json!({"arr_": [{"x_": 1}, 5, "s"]})),
        json!({"arr~": [{"x~": 1}, 5, "s"]})
    );
}

#[test]
fn probe_additional_items_forms_and_empty_top() {
    // Oracle-verified: additionalItems/unevaluatedItems recurse like
    // schema children; `{}` top gains the object shape; non-dict
    // `function` passes through untouched.
    let out = sanitize_tool_schemas(&[tool(
        "t",
        json!({"type": "object", "additionalItems": {"type": "object"},
               "unevaluatedItems": "oops"}),
    )]);
    assert_eq!(
        params(&out)["additionalItems"],
        json!({"type": "object", "properties": {}})
    );
    assert_eq!(
        params(&out)["unevaluatedItems"],
        json!({"type": "object", "properties": {}})
    );
    let out = sanitize_tool_schemas(&[tool("t", json!({}))]);
    assert_eq!(params(&out), &json!({"type": "object", "properties": {}}));
    let out = sanitize_tool_schemas(&[json!({"function": "oops"})]);
    assert_eq!(out, vec![json!({"function": "oops"})]);
    // Oracle-verified: top-level boolean `required` drops (legacy flag).
    let out = sanitize_tool_schemas(&[tool("t", json!({"type": "object", "required": true}))]);
    assert_eq!(params(&out), &json!({"type": "object", "properties": {}}));
}

#[test]
fn probe_strip_and_collapse_noop_edges() {
    // Oracle-verified: a property NAMED `pattern` keeps its schema (the
    // strip only fires beside type/combinator markers); slash-free enums
    // stay; impure const branches / empty unions / non-list unions pass
    // through; a lone null branch does not collapse.
    let mut tools = vec![tool(
        "t",
        json!({"properties": {"pattern": {"type": "string", "pattern": "x"}}}),
    )];
    // No `type` at top of parameters — the nested node still has one.
    assert_eq!(strip_pattern_and_format(&mut tools), 1);
    let mut tools = vec![tool(
        "t",
        json!({"type": "object",
               "properties": {"m": {"enum": ["a", "b"]}}}),
    )];
    assert_eq!(strip_slash_enum(&mut tools), 0);
    assert!(tools[0]["function"]["parameters"]["properties"]["m"]
        .get("enum")
        .is_some());
    let schema = json!({"anyOf": [{"const": "a", "extra": 1}]});
    assert_eq!(collapse_const_unions(&schema), schema);
    let schema = json!({"anyOf": []});
    assert_eq!(collapse_const_unions(&schema), schema);
    let schema = json!({"anyOf": "oops"});
    assert_eq!(strip_nullable_unions(&schema, true), schema);
    let schema = json!({"anyOf": [{"type": "null"}]});
    assert_eq!(strip_nullable_unions(&schema, true), schema);
}

#[test]
fn probe_strip_helpers_tolerate_foreign_shapes() {
    // Oracle-verified: non-dict tools and Responses-format tools pass the
    // reactive strips without damage; slash-enum strips only enums with `/`.
    let mut tools = vec![
        tool(
            "t",
            json!({"type": "object",
               "properties": {"x": {"type": "string", "pattern": "p", "format": "f"}}}),
        ),
        json!({"nonsense": 1}),
        json!("str"),
    ];
    assert_eq!(strip_pattern_and_format(&mut tools), 2);
    let mut tools2 = vec![
        json!({"name": "r", "parameters": {"type": "object",
            "properties": {"m": {"type": "string", "enum": ["a/b", "c"]}}}}),
        json!({"type": "function"}),
    ];
    assert_eq!(strip_slash_enum(&mut tools2), 1);
    assert!(tools2[0]["parameters"]["properties"]["m"]
        .get("enum")
        .is_none());
}
