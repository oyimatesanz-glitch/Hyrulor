use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use nix::{
    sys::signal::{killpg, Signal},
    unistd::Pid,
};
use serde::Serialize;
use tokio::{process::Command, time::timeout};
use tracing::warn;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("tool directory is not a directory: {0}")]
    InvalidDirectory(PathBuf),
    #[error("tool path escapes configured directory")]
    InvalidToolPath,
    #[error("sandbox executable `bwrap` was not found")]
    MissingBubblewrap,
    #[error("sandbox failed to start: {0}")]
    Start(#[source] std::io::Error),
    #[error("sandbox timed out after 5 seconds")]
    Timeout,
    #[error("sandbox exited unsuccessfully: {status}; stderr: {stderr}")]
    Failed { status: String, stderr: String },
    #[error("sandbox output was not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub tool: String,
    pub output: String,
}

pub struct ToolSandbox {
    tools_dir: PathBuf,
    tools: HashMap<&'static str, &'static str>,
}

impl ToolSandbox {
    pub fn new(tools_dir: impl Into<PathBuf>) -> Result<Self, ToolError> {
        let tools_dir = tools_dir.into();
        if !tools_dir.is_dir() {
            return Err(ToolError::InvalidDirectory(tools_dir));
        }

        Ok(Self {
            tools_dir,
            tools: HashMap::from([
                ("battery_status", "battery_tool.sh"),
                ("gpu_diag", "gpu_diag.sh"),
                ("net_privacy", "net_privacy.sh"),
                ("check_disk_space", "check_disk_space.sh"),
                ("check_memory", "check_memory.sh"),
                ("list_processes", "list_processes.sh"),
                ("read_file", "read_file.sh"),
                ("search_code", "search_code.sh"),
                ("system_info", "system_info.sh"),
                ("date_time", "date_time.sh"),
                ("ping_host", "ping_host.sh"),
                ("offensive_security_action", "offensive_security_action.sh"),
            ]),
        })
    }

    pub async fn exec_tool(&self, name: &str, args: &str) -> Result<ToolResult, ToolError> {
        let script_name = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::UnknownTool(name.to_string()))?;
        let script = self.tools_dir.join(script_name);
        ensure_child_path(&self.tools_dir, &script)?;
        self.exec_with_timeout(name, script_name, args).await
    }

    async fn exec_with_timeout(
        &self,
        name: &str,
        script_name: &str,
        args: &str,
    ) -> Result<ToolResult, ToolError> {
        let mut command = self.build_command(name, script_name, args)?;
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        let child = command.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ToolError::MissingBubblewrap
            } else {
                ToolError::Start(error)
            }
        })?;
        let pid = child.id().map(|value| Pid::from_raw(value as i32));
        let result = timeout(Duration::from_secs(5), child.wait_with_output()).await;

        let output = match result {
            Ok(result) => result.map_err(ToolError::Start)?,
            Err(_) => {
                if let Some(pid) = pid {
                    if let Err(error) = killpg(pid, Signal::SIGKILL) {
                        warn!(%error, "failed to kill timed-out tool process group");
                    }
                }
                return Err(ToolError::Timeout);
            }
        };
        if !output.status.success() {
            return Err(ToolError::Failed {
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let output = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(ToolResult {
            tool: name.to_string(),
            output,
        })
    }

    fn build_command(
        &self,
        name: &str,
        script_name: &str,
        args: &str,
    ) -> Result<Command, ToolError> {
        let mut command = Command::new("bwrap");
        command.args([
            "--ro-bind",
            "/usr",
            "/usr",
            // /bin y /lib son symlinks a /usr/* en la mayoría de distros.
            // Crear los symlinks evita fallos al bindear el symlink original.
            "--symlink",
            "usr/bin",
            "/bin",
            "--symlink",
            "usr/lib",
            "/lib",
            "--ro-bind",
            "/etc",
            "/etc",
            "--dev",
            "/dev",
            "--proc",
            "/proc",
            "--tmpfs",
            "/tmp",
            "--ro-bind",
        ]);
        command.arg(&self.tools_dir).arg("/tools");
        if Path::new("/usr/lib64").is_dir() {
            command.args(["--symlink", "usr/lib64", "/lib64"]);
        }
        let mut unshare_args = vec![
            "--unshare-user",
            "--unshare-uts",
            "--unshare-ipc",
            "--die-with-parent",
        ];
        // `ps` necesita ver el PID namespace del host para listar procesos reales.
        if name != "list_processes" {
            unshare_args.push("--unshare-pid");
        }
        let is_network_tool = matches!(
            name,
            "ping_host" | "curl_url" | "dns_lookup" | "public_ip" | "check_website"
        );
        if !is_network_tool {
            unshare_args.push("--unshare-net");
        }
        command.args(unshare_args);
        command.args([
            "--new-session",
            "--cap-drop",
            "ALL",
            "--clearenv",
            "--setenv",
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            "--chdir",
            "/tmp",
        ]);
        if (name == "battery_status" || name == "gpu_diag") && Path::new("/sys").is_dir() {
            command.args(["--ro-bind", "/sys", "/sys"]);
        }
        command
            .arg("/bin/sh")
            .arg(format!("/tools/{script_name}"))
            .arg(args);
        command.kill_on_drop(true);

        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(command)
    }
}

fn ensure_child_path(root: &Path, child: &Path) -> Result<(), ToolError> {
    let root = root
        .canonicalize()
        .map_err(|_| ToolError::InvalidToolPath)?;
    let child = child
        .canonicalize()
        .map_err(|_| ToolError::InvalidToolPath)?;
    if child.parent() != Some(root.as_path()) {
        return Err(ToolError::InvalidToolPath);
    }
    Ok(())
}
