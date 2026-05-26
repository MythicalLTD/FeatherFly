fn main() {
    let output = featherfly_docgen::default_output_dir();
    featherfly_docgen::generate_minimal(&output).expect("failed to generate docs");
    println!("generated docs at {}", output.display());
}
