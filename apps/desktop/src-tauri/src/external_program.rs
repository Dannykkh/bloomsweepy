use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExternalProgram {
    Direct(PathBuf),
    #[cfg(windows)]
    CommandScript(PathBuf),
}

impl ExternalProgram {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Direct(path) => path,
            #[cfg(windows)]
            Self::CommandScript(path) => path,
        }
    }

    pub(crate) fn command(&self) -> Command {
        match self {
            Self::Direct(path) => Command::new(path),
            #[cfg(windows)]
            Self::CommandScript(path) => {
                // Keep npm's Windows command-shim behavior in one audited place.
                let mut command = Command::new("cmd.exe");
                command.arg("/D").arg("/C").arg(path);
                command
            }
        }
    }

    #[cfg(windows)]
    pub(crate) fn is_command_script(&self) -> bool {
        matches!(self, Self::CommandScript(_))
    }

    #[cfg(not(windows))]
    pub(crate) fn is_command_script(&self) -> bool {
        false
    }
}

pub(crate) fn find_external_program(executable_name: &str) -> Option<ExternalProgram> {
    if !valid_executable_name(executable_name) {
        return None;
    }

    let directories = env::var_os("PATH")
        .map(|path| {
            env::split_paths(&path)
                .filter(|directory| directory.is_absolute())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    #[cfg(windows)]
    let mut directories = directories;
    #[cfg(windows)]
    append_windows_cli_directories(&mut directories);
    #[cfg(target_os = "macos")]
    let mut directories = directories;
    #[cfg(target_os = "macos")]
    append_macos_cli_directories(&mut directories);

    find_in_directories(executable_name, &directories)
}

#[cfg(windows)]
fn append_windows_cli_directories(directories: &mut Vec<PathBuf>) {
    let Some(app_data) = env::var_os("APPDATA") else {
        return;
    };
    let npm = PathBuf::from(app_data).join("npm");
    if npm.is_absolute() && !directories.iter().any(|candidate| candidate == &npm) {
        directories.push(npm);
    }
}

fn valid_executable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn find_in_directories(name: &str, directories: &[PathBuf]) -> Option<ExternalProgram> {
    for directory in directories {
        #[cfg(windows)]
        {
            let executable = directory.join(format!("{name}.exe"));
            if executable.is_file() {
                return Some(ExternalProgram::Direct(executable));
            }
            let command_script = directory.join(format!("{name}.cmd"));
            if command_script.is_file() {
                return Some(ExternalProgram::CommandScript(command_script));
            }
        }
        #[cfg(not(windows))]
        {
            let executable = directory.join(name);
            if executable.is_file() {
                return Some(ExternalProgram::Direct(executable));
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn append_macos_cli_directories(directories: &mut Vec<PathBuf>) {
    let mut append = |directory: PathBuf| {
        if directory.is_absolute() && !directories.iter().any(|candidate| candidate == &directory) {
            directories.push(directory);
        }
    };

    append(PathBuf::from("/opt/homebrew/bin"));
    append(PathBuf::from("/usr/local/bin"));
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        append(home.join(".local").join("bin"));
        append(home.join(".npm-global").join("bin"));
        append(home.join(".volta").join("bin"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn executable_names_cannot_escape_candidate_directories() {
        assert!(valid_executable_name("codex"));
        assert!(valid_executable_name("claude-code_2"));
        assert!(!valid_executable_name("../codex"));
        assert!(!valid_executable_name("codex.exe"));
        assert!(!valid_executable_name(""));
    }

    #[test]
    fn finds_a_direct_program_in_order() {
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        #[cfg(windows)]
        let executable_name = "probe.exe";
        #[cfg(not(windows))]
        let executable_name = "probe";
        let expected = second.path().join(executable_name);
        fs::write(&expected, b"fixture").unwrap();

        let found = find_in_directories(
            "probe",
            &[first.path().to_path_buf(), second.path().to_path_buf()],
        )
        .expect("program");
        assert_eq!(found.path(), expected);
        assert!(!found.is_command_script());
    }

    #[cfg(windows)]
    #[test]
    fn windows_prefers_exe_and_preserves_command_script_as_one_argument() {
        use std::ffi::OsStr;

        let directory = tempdir().unwrap();
        let executable = directory.path().join("probe.exe");
        let script = directory.path().join("probe.cmd");
        fs::write(&script, b"@echo off\r\n").unwrap();
        assert!(
            find_in_directories("probe", &[directory.path().to_path_buf()])
                .expect("script")
                .is_command_script()
        );

        fs::write(&executable, b"fixture").unwrap();
        assert_eq!(
            find_in_directories("probe", &[directory.path().to_path_buf()])
                .expect("exe")
                .path(),
            executable
        );

        let program = ExternalProgram::CommandScript(script.clone());
        let command = program.command();
        assert_eq!(command.get_program(), OsStr::new("cmd.exe"));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![OsStr::new("/D"), OsStr::new("/C"), script.as_os_str()]
        );
    }
}
