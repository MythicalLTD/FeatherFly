fn main() {
    let output = featherfly_docgen::default_output_dir();
    let openapi = featherfly::api_spec::build_openapi("FeatherFly");
    featherfly_docgen::generate_all(&output, &openapi).expect("failed to generate docs");
    println!("generated docs at {}", output.display());
}
