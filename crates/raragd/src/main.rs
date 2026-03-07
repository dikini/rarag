fn main() {
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("raragd bootstrap binary");
        println!("Usage: raragd [--help]");
        return;
    }

    println!("raragd bootstrap binary");
}
