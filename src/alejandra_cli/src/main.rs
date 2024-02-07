fn main() -> std::io::Result<()> {
    env_logger::builder()
        .format_timestamp(None)
        .init();
    alejandra_cli::cli::main()
}
