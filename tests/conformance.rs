//! `clihatch schema` must validate against the published clispec v0.2 JSON
//! Schema (vendored at schemas/clispec-v0.2.json).

#[test]
fn schema_conforms_to_clispec_v0_2() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/clispec-v0.2.json"))
            .expect("vendored clispec schema is valid JSON");

    let instance = clihatch::schema::contract();
    let validator = jsonschema::validator_for(&schema).expect("compile clispec schema");

    if !validator.is_valid(&instance) {
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        panic!(
            "clihatch schema does not conform to clispec v0.2:\n{}",
            errors.join("\n")
        );
    }
}

#[test]
fn schema_marks_only_new_as_mutating() {
    let v = clihatch::schema::contract();
    assert_eq!(v["name"], "clihatch");
    let commands = v["commands"].as_array().unwrap();
    let new = commands.iter().find(|c| c["name"] == "new").unwrap();
    assert_eq!(new["mutating"], true);
    for name in ["schema", "completions"] {
        let c = commands.iter().find(|c| c["name"] == name).unwrap();
        assert_eq!(c["mutating"], false, "{name} must be read-only");
    }
}
