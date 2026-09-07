//! PDF/office parsing is isolated from the application allocator. Only the bundled
//! sibling helper is executed; never resolve an executable through PATH.
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

const MAX_INPUT: u64 = 32 * 1024 * 1024;
const MAX_OUTPUT: usize = 4 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);
const HEADER: &[u8] = b"BROOMSWEEPY_DOCUMENT_1\n";
static DOCUMENT_SLOT: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub(crate) enum WorkerError {
    Document(String),
    Infrastructure(String),
}

impl From<String> for WorkerError {
    fn from(message: String) -> Self {
        Self::Infrastructure(message)
    }
}

impl From<&str> for WorkerError {
    fn from(message: &str) -> Self {
        Self::Infrastructure(message.to_owned())
    }
}

pub(crate) fn extract_document<C>(
    path: &Path,
    format: crate::DocumentFormat,
    max_input: u64,
    max_output: usize,
    should_cancel: &C,
) -> Result<String, WorkerError>
where
    C: Fn() -> bool,
{
    let _slot = loop {
        if should_cancel() {
            return Err("문서 읽기를 취소했습니다".into());
        }
        crate::ensure_operation_memory()?;
        match DOCUMENT_SLOT.try_lock() {
            Ok(slot) => break slot,
            Err(TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(25)),
            Err(TryLockError::Poisoned(_)) => return Err("문서 작업 상태를 읽지 못했습니다".into()),
        }
    };
    crate::scan_policy::ensure_local_path(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| "문서 상태를 읽지 못했습니다")?;
    let max_input = max_input.min(MAX_INPUT);
    let max_output = max_output.min(MAX_OUTPUT);
    if !metadata.is_file() || metadata.len() > max_input || max_output == 0 {
        return Err("문서 파일이 안전한 처리 한도를 넘었거나 일반 파일이 아닙니다".into());
    }
    let helper = helper_path()?;
    let mut command = Command::new(helper);
    command
        .arg("--extract")
        .arg(path)
        .arg(max_input.to_string())
        .arg(max_output.to_string())
        .arg(match format {
            crate::DocumentFormat::Pdf => "pdf",
            crate::DocumentFormat::Word => "docx",
            crate::DocumentFormat::Spreadsheet => "xlsx",
            crate::DocumentFormat::Presentation => "pptx",
            crate::DocumentFormat::Hwpx => "hwpx",
            crate::DocumentFormat::PlainText => {
                return Err("이 문서는 로컬 텍스트 읽기를 사용합니다".into());
            }
        });
    let bytes = run_bounded(
        &mut command,
        max_output + HEADER.len(),
        TIMEOUT,
        should_cancel,
    )?;
    let payload = bytes
        .strip_prefix(HEADER)
        .ok_or("문서 도구의 응답 형식이 올바르지 않습니다")?;
    String::from_utf8(payload.to_vec()).map_err(|_| "문서 텍스트를 읽지 못했습니다".into())
}

fn helper_path() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|_| "문서 도구 위치를 찾지 못했습니다")?;
    let parent = executable
        .parent()
        .ok_or("문서 도구 위치를 찾지 못했습니다")?;
    let name = if cfg!(windows) {
        "bloomsweepy-document-worker.exe"
    } else {
        "bloomsweepy-document-worker"
    };
    let sibling = parent.join(name);
    if sibling.is_file() {
        return Ok(sibling);
    }
    // Cargo test binaries live in target/{profile}/deps. This is not a PATH search.
    if parent.file_name().is_some_and(|name| name == "deps")
        && let Some(profile) = parent.parent()
    {
        let candidate = profile.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("문서 안전 처리 도구가 없습니다. 앱 설치 파일을 확인해 주세요".into())
}

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        // Reap only this worker, including on cancellation and early returns.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run_bounded<C>(
    command: &mut Command,
    max_output: usize,
    timeout: Duration,
    should_cancel: &C,
) -> Result<Vec<u8>, WorkerError>
where
    C: Fn() -> bool,
{
    if should_cancel() {
        return Err("문서 읽기를 취소했습니다".into());
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = OwnedChild(
        command
            .spawn()
            .map_err(|_| "문서 처리 도구를 시작하지 못했습니다")?,
    );
    let stdout = child
        .0
        .stdout
        .take()
        .ok_or("문서 응답 통로를 열지 못했습니다")?;
    let reader = thread::Builder::new()
        .name("document-output".into())
        .spawn(move || {
            let mut bytes = Vec::new();
            stdout.take(max_output as u64 + 1).read_to_end(&mut bytes)?;
            Ok::<_, std::io::Error>(bytes)
        })
        .map_err(|_| "문서 응답 작업을 시작하지 못했습니다")?;
    let started = Instant::now();
    let result = loop {
        if should_cancel() {
            break Err(WorkerError::from("문서 읽기를 취소했습니다"));
        }
        if let Err(message) = crate::ensure_operation_memory() {
            break Err(WorkerError::from(message));
        }
        if started.elapsed() >= timeout {
            break Err(WorkerError::from(
                "문서 처리 시간 한도를 넘었습니다. 이전 색인을 보존하고 중단합니다",
            ));
        }
        match child.0.try_wait() {
            Ok(Some(status)) if status.success() => break Ok(()),
            Ok(Some(status)) if status.code() == Some(3) => {
                break Err(WorkerError::Document(
                    "문서 형식을 읽지 못했습니다".to_owned(),
                ));
            }
            Ok(Some(_)) => {
                break Err(WorkerError::from(
                    "문서 안전 처리 도구가 중단됐습니다. 이전 색인을 보존하고 중단합니다",
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => break Err(WorkerError::from("문서 처리 상태를 확인하지 못했습니다")),
        }
    };
    drop(child);
    let bytes = reader
        .join()
        .map_err(|_| "문서 응답 작업이 중단됐습니다")?
        .map_err(|_| "문서 응답을 읽지 못했습니다")?;
    result?;
    if bytes.len() > max_output {
        return Err("문서 텍스트가 안전한 출력 한도를 넘었습니다".into());
    }
    Ok(bytes)
}

/// Entry point for the dedicated executable. The binary installs its own
/// allocation budget before calling this; the GUI never installs that allocator.
pub fn run_document_worker() -> i32 {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() == 1 && arguments[0] == "--version" {
        println!("bloomsweepy-document-worker {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    if arguments.len() != 5 || arguments[0] != "--extract" {
        return 2;
    }
    let parse_limit = |index: usize| {
        arguments[index]
            .to_str()
            .and_then(|text| text.parse::<u64>().ok())
    };
    let (Some(max_input), Some(max_output)) = (parse_limit(2), parse_limit(3)) else {
        return 2;
    };
    if max_input == 0 || max_input > MAX_INPUT || max_output == 0 || max_output > MAX_OUTPUT as u64
    {
        return 2;
    }
    let path = Path::new(&arguments[1]);
    if crate::scan_policy::ensure_local_path(path).is_err() {
        return 2;
    }
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 2;
    };
    if !metadata.is_file() || metadata.len() > max_input {
        return 2;
    }
    let mut text = if arguments[4] == "pdf" {
        let Ok(file) = crate::open_read_shared(path) else {
            return 2;
        };
        let mut bytes = Vec::new();
        if file.take(max_input + 1).read_to_end(&mut bytes).is_err()
            || bytes.len() as u64 > max_input
        {
            return 2;
        }
        let Ok(text) = pdf_extract::extract_text_from_mem(&bytes) else {
            return 3;
        };
        text
    } else {
        let format = match arguments[4].to_str() {
            Some("docx") => crate::DocumentFormat::Word,
            Some("xlsx") => crate::DocumentFormat::Spreadsheet,
            Some("pptx") => crate::DocumentFormat::Presentation,
            Some("hwpx") => crate::DocumentFormat::Hwpx,
            _ => return 2,
        };
        let config = crate::DocumentIndexConfig {
            max_file_bytes: max_input,
            max_extracted_bytes: max_output as usize,
            ..crate::DocumentIndexConfig::default()
        };
        let Ok(text) =
            crate::document_search::extract_archive_text(path, format, &config, &|| false)
        else {
            return 3;
        };
        text
    };
    let mut end = text.len().min(max_output as usize);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    let mut stdout = std::io::stdout().lock();
    if stdout
        .write_all(HEADER)
        .and_then(|()| stdout.write_all(text.as_bytes()))
        .is_err()
    {
        return 4; // IPC failure is not evidence that the document is unreadable.
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_document_is_distinct_from_worker_infrastructure_failure() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("broken.pdf");
        fs::write(&path, b"not a PDF").unwrap();
        assert!(matches!(
            extract_document(&path, crate::DocumentFormat::Pdf, 1024, 1024, &|| false),
            Err(WorkerError::Document(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_output_timeout_and_cancel_reap_only_owned_child() {
        let mut output = Command::new("/usr/bin/printf");
        output.arg("123456789");
        assert!(run_bounded(&mut output, 4, Duration::from_secs(2), &|| false).is_err());
        let mut sleep = Command::new("/bin/sleep");
        sleep.arg("1");
        let started = Instant::now();
        assert!(run_bounded(&mut sleep, 32, Duration::from_millis(50), &|| false).is_err());
        assert!(started.elapsed() < Duration::from_millis(800));
        let mut never = Command::new("/not-a-real-worker");
        assert!(matches!(run_bounded(&mut never, 32, TIMEOUT, &|| true),
            Err(WorkerError::Infrastructure(message)) if message.contains("취소")));
        assert!(matches!(
            run_bounded(&mut never, 32, TIMEOUT, &|| false),
            Err(WorkerError::Infrastructure(_))
        ));
    }
}
