fn main() {
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("rarag-mcp bootstrap binary");
        println!("Usage: rarag-mcp [--help]");
        return;
    }

    println!("rarag-mcp bootstrap binary");
}
