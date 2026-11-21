use command_group::{CommandGroup, GroupChild};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_STDIN_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug)]
pub struct ProcessRequest {
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Option<Vec<u8>>,
    pub timeout_ms: u64,
    pub max_output_bytes: usize,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct ProcessOutput {
    pub status: i32,
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    pub truncated: bool,
}

fn spawn_reader<R: Read + Send + 'static>(
    reader: R,
    cap: usize,
) -> thread::JoinHandle<(Vec<u8>, bool)> {
    thread::spawn(move || read_capped(reader, cap))
}

fn read_capped(mut reader: impl Read, cap: usize) -> (Vec<u8>, bool) {
    let mut output = Vec::new();
    let mut truncated = false;
    let mut buffer = [0_u8; 8192];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if output.len() >= cap {
                    truncated = true;
                    continue;
                }
                let take = read.min(cap - output.len());
                output.extend_from_slice(&buffer[..take]);
                if take < read {
                    truncated = true;
                }
            }
            Err(_) => break,
        }
    }

    (output, truncated)
}

fn inherited_env_keys() -> &'static [&'static str] {
    &[
        "PATH",
        "PATHEXT",
        "HOME",
        "USERPROFILE",
        "TEMP",
        "TMP",
        "TMPDIR",
        "SYSTEMROOT",
        "WINDIR",
        "LANG",
        "LC_ALL",
    ]
}

fn poll_child(
    child: &mut GroupChild,
    deadline: Instant,
) -> Result<Option<std::process::ExitStatus>, String> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {}
            Err(error) => return Err(format!("failed to wait for process: {}", error)),
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }

        thread::sleep(Duration::from_millis(10));
    }
}

pub fn execute(request: ProcessRequest) -> Result<ProcessOutput, String> {
    if request.program.trim().is_empty() {
        return Err("program must not be empty".to_string());
    }
    if request.program.contains('\0') || request.args.iter().any(|arg| arg.contains('\0')) {
        return Err("program and arguments must not contain NUL bytes".to_string());
    }
    if request.args.len() > 256 {
        return Err("too many arguments (maximum is 256)".to_string());
    }
    if request.args.iter().any(|arg| arg.len() > 64 * 1024) {
        return Err("an argument exceeds the 64 KiB limit".to_string());
    }
    if let Some(stdin) = &request.stdin
        && stdin.len() > MAX_STDIN_BYTES
    {
        return Err(format!(
            "stdin is {} bytes, maximum is {}",
            stdin.len(),
            MAX_STDIN_BYTES
        ));
    }

    let mut command = std::process::Command::new(&request.program);
    command
        .args(&request.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();

    for key in inherited_env_keys() {
        if let Ok(value) = std::env::var(key) {
            command.env(key, value);
        }
    }
    for (key, value) in &request.env {
        command.env(key, value);
    }
    if let Some(cwd) = &request.cwd {
        command.current_dir(cwd);
    }

    let mut child = command
        .group_spawn()
        .map_err(|e| format!("failed to start '{}': {}", request.program, e))?;

    let stdout = child
        .inner()
        .stdout
        .take()
        .ok_or_else(|| "failed to capture stdout".to_string())?;
    let stderr = child
        .inner()
        .stderr
        .take()
        .ok_or_else(|| "failed to capture stderr".to_string())?;
    let stdin = child.inner().stdin.take();

    let cap = request.max_output_bytes.clamp(1024, MAX_OUTPUT_BYTES);
    let stdout_reader = spawn_reader(stdout, cap);
    let stderr_reader = spawn_reader(stderr, cap);

    let stdin_writer = if let Some(data) = request.stdin {
        let mut stdin = stdin;
        Some(thread::spawn(move || {
            if let Some(stdin) = stdin.as_mut() {
                let _ = stdin.write_all(&data);
            }
        }))
    } else {
        drop(stdin);
        None
    };

    let deadline = Instant::now() + Duration::from_millis(request.timeout_ms.max(1));
    let status = poll_child(&mut child, deadline)?;
    let timed_out = status.is_none();

    if let Some(writer) = stdin_writer {
        let _ = writer.join();
    }

    let (stdout, stdout_truncated) = stdout_reader
        .join()
        .map_err(|_| "stdout reader panicked".to_string())?;
    let (stderr, stderr_truncated) = stderr_reader
        .join()
        .map_err(|_| "stderr reader panicked".to_string())?;

    let (status, success) = match status {
        Some(status) => (status.code().unwrap_or(-1), status.success()),
        None => (-1, false),
    };

    Ok(ProcessOutput {
        status,
        success,
        stdout,
        stderr,
        timed_out,
        truncated: stdout_truncated || stderr_truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo_command() -> (String, Vec<String>) {
        if cfg!(windows) {
            (
                "cmd".to_string(),
                vec!["/C".to_string(), "echo hello".to_string()],
            )
        } else {
            (
                "sh".to_string(),
                vec!["-c".to_string(), "echo hello".to_string()],
            )
        }
    }

    #[test]
    fn runs_a_short_command_and_captures_output() {
        let (program, args) = echo_command();
        let output = execute(ProcessRequest {
            program,
            args,
            stdin: None,
            timeout_ms: 5_000,
            max_output_bytes: 1024,
            cwd: None,
            env: Vec::new(),
        })
        .expect("command should run");

        assert!(output.success);
        assert!(String::from_utf8_lossy(&output.stdout).contains("hello"));
    }

    #[test]
    fn timeout_kills_the_process() {
        let (program, args) = if cfg!(windows) {
            (
                "cmd".to_string(),
                vec!["/C".to_string(), "ping -n 10 127.0.0.1 >nul".to_string()],
            )
        } else {
            (
                "sh".to_string(),
                vec!["-c".to_string(), "sleep 5".to_string()],
            )
        };

        let output = execute(ProcessRequest {
            program,
            args,
            stdin: None,
            timeout_ms: 100,
            max_output_bytes: 1024,
            cwd: None,
            env: Vec::new(),
        })
        .expect("command should start");

        assert!(output.timed_out);
        assert!(!output.success);
    }
}
