fn main() {
    let src_dir = std::path::Path::new("src");
    let parser_c = src_dir.join("parser.c");
    println!("cargo:rerun-if-changed={}", parser_c.display());
    println!("cargo:rerun-if-changed=grammar.js");
    cc::Build::new()
        .include(src_dir)
        .file(&parser_c)
        .compile("tree-sitter-spice");
}
