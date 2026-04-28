use std::path::PathBuf;

const LABEL: &str = "com.focustrace.agent";

pub fn plist_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
}

pub fn enable() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_str}</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><false/>
</dict>
</plist>
"#);
    let path = plist_path();
    if let Some(d) = path.parent() { std::fs::create_dir_all(d)?; }
    std::fs::write(&path, plist)?;
    Ok(())
}

pub fn disable() -> std::io::Result<()> {
    let path = plist_path();
    if path.exists() { std::fs::remove_file(path)?; }
    Ok(())
}
