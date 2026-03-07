fn main() {
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("rarag bootstrap binary");
        println!("Usage: rarag [--help]");
        return;
    }

    println!("rarag bootstrap binary");
}
