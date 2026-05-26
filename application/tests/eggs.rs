use featherfly::eggs::{EggDefinition, import_pterodactyl_egg, substitute_placeholders};
use std::collections::HashMap;

#[test]
fn featherfly_egg_resolves_startup_variables() {
    let egg = EggDefinition {
        id: "demo".into(),
        name: "Demo".into(),
        description: None,
        author: None,
        docker_image: "alpine:3".into(),
        startup: "echo {{MESSAGE}}".into(),
        entrypoint: None,
        port: 80,
        workdir: "/home/container".into(),
        features: vec!["web".into()],
        file_denylist: vec![],
        variables: vec![featherfly::eggs::EggVariable {
            name: "MESSAGE".into(),
            description: None,
            default_value: "hello".into(),
            user_viewable: true,
            user_editable: true,
            rules: None,
        }],
        install: None,
        config: None,
    };

    let runtime = egg.resolve(&HashMap::new()).unwrap();
    assert_eq!(runtime.startup, "echo hello");
    assert_eq!(
        runtime.env.get("STARTUP").map(String::as_str),
        Some("echo hello")
    );
}

#[test]
fn example_generic_web_egg_loads() {
    let raw = include_str!("../../examples/eggs/generic-web.json");
    let egg: EggDefinition = serde_json::from_str(raw).expect("example egg JSON");
    assert_eq!(egg.id, "generic-web");
    let runtime = egg.resolve(&HashMap::new()).unwrap();
    assert!(runtime.startup.contains("nginx"));
}

#[test]
fn pterodactyl_ptdl_import_maps_docker_image() {
    let raw = r#"{
        "meta": { "version": "PTDL_v2" },
        "name": "Node Server",
        "docker_images": { "main": "ghcr.io/pterodactyl/games:nodejs" },
        "startup": "node /home/container/index.js",
        "variables": []
    }"#;
    let egg = import_pterodactyl_egg("node-server", raw).unwrap();
    assert_eq!(egg.docker_image, "ghcr.io/pterodactyl/games:nodejs");
    assert_eq!(
        substitute_placeholders(
            "{{env.MESSAGE}}",
            &HashMap::from([("MESSAGE".into(), "ok".into())])
        ),
        "ok"
    );
}
