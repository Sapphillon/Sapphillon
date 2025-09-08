fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../proto/browser_bridge.proto",
                "../proto/browser_info.proto",
                "../proto/webscraper.proto",
                "../proto/tab_manager.proto",
            ],
            &["../proto"],
        )?;
    Ok(())
}
