//! Isolated Feature 1 replay tool; never launches plugins or reads account credentials.
use proxyweave_plugin::Registry;
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err(
            "usage: local_registry <profile> <list|import|settings> [manifest|id] [JSON]".into(),
        );
    }
    let registry = Registry::open(&args[1])?;
    match args[2].as_str() {
        "list" => println!(
            "{}",
            serde_json::to_string_pretty(&registry.list()?).map_err(|_| "serialize list")?
        ),
        "import" => {
            let source = args.get(3).ok_or("manifest path required")?;
            let result = registry.import(source)?;
            println!(
                "id={} already_registered={} fingerprint={}",
                result.record.manifest.metadata.id,
                result.already_registered,
                result.record.fingerprint
            );
        }
        "settings" => {
            let id = args.get(3).ok_or("plugin id required")?;
            let json = args.get(4).ok_or("non-sensitive settings JSON required")?;
            let settings = serde_json::from_str(json).map_err(|_| "invalid settings JSON")?;
            let result = registry.save_settings(id, settings)?;
            println!("settings saved for {}", result.manifest.metadata.id);
        }
        _ => return Err("unsupported command".into()),
    }
    Ok(())
}
