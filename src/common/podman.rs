use crate::constants;

pub fn podman() -> tokio::process::Command {
    let sock = constants::podman_socket();
    let mut cmd = tokio::process::Command::new("podman");
    cmd.args(["--remote", "--url", &format!("unix://{sock}")]);
    cmd
}
