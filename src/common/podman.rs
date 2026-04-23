pub fn podman() -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("podman");
    if let Ok(sock) = std::env::var("PODMAN_SOCKET") {
        cmd.arg("--remote");
        cmd.arg("--url");
        cmd.arg(format!("unix://{sock}"));
    }
    cmd
}
