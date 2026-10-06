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
    find_external_programs(executable_name).into_iter().next()
}

/// Enumerate candidates so a caller can check health before accepting a launcher.
pub(crate) fn find_external_programs(executable_name: &str) -> Vec<ExternalProgram> {
    if !valid_executable_name(executable_name) {
        return Vec::new();
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
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let mut directories = directories;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    append_unix_cli_directories(&mut directories, env::var_os("HOME").map(PathBuf::from));

    programs_in_directories(executable_name, &directories)
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

#[cfg(test)]
fn find_in_directories(name: &str, directories: &[PathBuf]) -> Option<ExternalProgram> {
    programs_in_directories(name, directories)
        .into_iter()
        .next()
}

fn programs_in_directories(name: &str, directories: &[PathBuf]) -> Vec<ExternalProgram> {
    let mut programs = Vec::new();
    for directory in directories {
        #[cfg(windows)]
        {
            let executable = directory.join(format!("{name}.exe"));
            if executable.is_file() {
                programs.push(ExternalProgram::Direct(executable));
            }
            let command_script = directory.join(format!("{name}.cmd"));
            if command_script.is_file() {
                programs.push(ExternalProgram::CommandScript(command_script));
            }
        }
        #[cfg(not(windows))]
        {
            let executable = directory.join(name);
            if executable.is_file() {
                programs.push(ExternalProgram::Direct(executable));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    programs.retain(|program| {
        seen.insert(
            program
                .path()
                .canonicalize()
                .unwrap_or_else(|_| program.path().to_path_buf()),
        )
    });
    programs
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn append_unix_cli_directories(directories: &mut Vec<PathBuf>, home: Option<PathBuf>) {
    let mut append = |directory: PathBuf| {
        if directory.is_absolute() && !directories.iter().any(|candidate| candidate == &directory) {
            directories.push(directory);
        }
    };

    #[cfg(target_os = "macos")]
    {
        append(PathBuf::from("/opt/homebrew/bin"));
        append(PathBuf::from("/usr/local/bin"));
    }
    if let Some(home) = home {
        #[cfg(target_os = "macos")]
        {
            append(home.join(".local").join("bin"));
            append(home.join(".npm-global").join("bin"));
            append(home.join(".volta").join("bin"));
        }
        append(home.join(".grok").join("bin"));
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

    #[test]
    fn enumerates_all_distinct_candidates_in_search_order() {
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        #[cfg(windows)]
        let name = "probe.exe";
        #[cfg(not(windows))]
        let name = "probe";
        fs::write(first.path().join(name), b"fixture").unwrap();
        fs::write(second.path().join(name), b"fixture").unwrap();
        let programs = programs_in_directories(
            "probe",
            &[
                first.path().to_path_buf(),
                first.path().to_path_buf(),
                second.path().to_path_buf(),
            ],
        );
        assert_eq!(programs.len(), 2);
        assert_eq!(programs[0].path(), first.path().join(name));
        assert_eq!(programs[1].path(), second.path().join(name));
        assert!(find_external_programs("../probe").is_empty());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn finds_grok_in_official_unix_install_directory_without_shell_path() {
        let home = tempdir().unwrap();
        let binary_directory = home.path().join(".grok").join("bin");
        fs::create_dir_all(&binary_directory).unwrap();
        let executable = binary_directory.join("grok");
        fs::write(&executable, b"fixture").unwrap();

        let mut directories = Vec::new();
        append_unix_cli_directories(&mut directories, Some(home.path().to_path_buf()));
        assert!(
            programs_in_directories("grok", &directories)
                .iter()
                .any(|program| program.path() == executable)
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn official_unix_install_directory_preserves_path_order_and_absolute_boundary() {
        let home = tempdir().unwrap();
        let existing = home.path().join("existing-bin");
        let grok = home.path().join(".grok").join("bin");
        let mut directories = vec![existing.clone(), grok.clone()];
        append_unix_cli_directories(&mut directories, Some(home.path().to_path_buf()));
        assert_eq!(directories[0], existing);
        assert_eq!(directories[1], grok);
        assert_eq!(directories.iter().filter(|path| **path == grok).count(), 1);

        let mut directories = Vec::new();
        append_unix_cli_directories(&mut directories, Some(PathBuf::from("relative-home")));
        assert!(directories.iter().all(|path| path.is_absolute()));
        assert!(!directories.iter().any(|path| path.ends_with(".grok/bin")));
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
