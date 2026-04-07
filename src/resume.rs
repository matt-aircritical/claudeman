use crate::config::Config;
use anyhow::Result;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub struct ResumeOptions {
    pub session_id: String,
    pub cwd: String,
    pub fork: bool,
}

pub fn build_resume_command(config: &Config, options: &ResumeOptions) -> Command {
    let mut cmd = Command::new(&config.claude_bin);
    cmd.arg("--resume");
    cmd.arg(&options.session_id);

    if options.fork {
        cmd.arg("--fork-session");
    }

    for arg in &config.claude_args {
        cmd.arg(arg);
    }

    let cwd_path = Path::new(&options.cwd);
    if cwd_path.exists() && cwd_path.is_dir() {
        cmd.current_dir(cwd_path);
    }

    cmd
}

pub fn resume(config: &Config, options: &ResumeOptions) -> Result<()> {
    let mut cmd = build_resume_command(config, options);
    let err = cmd.exec();
    Err(anyhow::anyhow!("Failed to launch claude: {}", err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_resume_command_basic() {
        let config = Config::default();
        let options = ResumeOptions {
            session_id: "abc-123".to_string(),
            cwd: "/tmp".to_string(),
            fork: false,
        };
        let cmd = build_resume_command(&config, &options);
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, &["--resume", "abc-123"]);
        assert_eq!(cmd.get_program(), "claude");
    }

    #[test]
    fn test_build_resume_command_with_fork_and_args() {
        let config = Config {
            claude_args: vec!["--dangerously-skip-permissions".to_string()],
            claude_bin: "claude".to_string(),
            claude_dir: None,
        };
        let options = ResumeOptions {
            session_id: "abc-123".to_string(),
            cwd: "/tmp".to_string(),
            fork: true,
        };
        let cmd = build_resume_command(&config, &options);
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, &["--resume", "abc-123", "--fork-session", "--dangerously-skip-permissions"]);
    }

    #[test]
    fn test_build_resume_command_nonexistent_cwd() {
        let config = Config::default();
        let options = ResumeOptions {
            session_id: "abc-123".to_string(),
            cwd: "/nonexistent/path/that/does/not/exist".to_string(),
            fork: false,
        };
        let cmd = build_resume_command(&config, &options);
        assert!(cmd.get_current_dir().is_none());
    }
}
