use crate::{ScanError, ScanIssue};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_CLEANUP_CANDIDATES: usize = 10_000;
const MAX_CLEANUP_ENTRIES: u64 = 5_000_000;
const MAX_CLEANUP_ISSUES: usize = 1_000;
const MAX_INSTALLED_IDENTITY_TOKENS: usize = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupCandidateKind {
    TemporaryEntry,
    AppDataDirectory,
    CacheDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupConfidence {
    LikelySafe,
    Review,
}

#[derive(Debug, Clone)]
pub struct CleanupRootSpec {
    pub path: PathBuf,
    pub label: String,
    pub kind: CleanupCandidateKind,
    pub minimum_age: Duration,
    pub protected_names: Vec<String>,
}

impl CleanupRootSpec {
    pub fn new(
        path: impl Into<PathBuf>,
        label: impl Into<String>,
        kind: CleanupCandidateKind,
        minimum_age: Duration,
    ) -> Self {
        Self {
            path: path.into(),
            label: label.into(),
            kind,
            minimum_age,
            protected_names: Vec::new(),
        }
    }

    pub fn with_protected_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.protected_names = names.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone)]
pub struct CleanupScanConfig {
    pub roots: Vec<CleanupRootSpec>,
    pub installed_identity_tokens: Vec<String>,
    pub max_candidates: usize,
    pub max_candidates_per_root: usize,
    pub max_entries: u64,
    pub max_issues: usize,
}

impl Default for CleanupScanConfig {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            installed_identity_tokens: Vec::new(),
            max_candidates: 450,
            max_candidates_per_root: 150,
            max_entries: 500_000,
            max_issues: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCandidate {
    pub kind: CleanupCandidateKind,
    pub confidence: CleanupConfidence,
    pub name: String,
    pub path: String,
    pub source_label: String,
    pub logical_bytes: u64,
    pub entry_count: u64,
    pub modified_at_unix_ms: Option<u128>,
    pub inactive_days: u64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanProgress {
    pub message: String,
    pub processed_roots: usize,
    pub total_roots: usize,
    pub processed_entries: u64,
    pub processed_bytes: u64,
    pub candidates_found: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanReport {
    pub completed_at_unix_ms: u128,
    pub duration_ms: u128,
    pub scanned_roots: usize,
    pub processed_entries: u64,
    pub processed_bytes: u64,
    pub unreadable_entries: u64,
    pub candidate_bytes: u64,
    pub candidates: Vec<CleanupCandidate>,
    pub limit_reached: bool,
    pub issues: Vec<ScanIssue>,
}

#[derive(Default)]
struct CandidateStats {
    logical_bytes: u64,
    entry_count: u64,
    latest_modified: Option<SystemTime>,
    unreadable_entries: u64,
    truncated: bool,
}

pub fn scan_cleanup_candidates<F, C>(
    config: CleanupScanConfig,
    mut on_progress: F,
    should_cancel: C,
) -> Result<CleanupScanReport, ScanError>
where
    F: FnMut(CleanupScanProgress),
    C: Fn() -> bool,
{
    let started = Instant::now();
    let now = SystemTime::now();
    let total_roots = config.roots.len();
    let max_candidates = config.max_candidates.min(MAX_CLEANUP_CANDIDATES);
    let max_candidates_per_root = config.max_candidates_per_root.min(MAX_CLEANUP_CANDIDATES);
    let max_entries = config.max_entries.min(MAX_CLEANUP_ENTRIES);
    let max_issues = config.max_issues.min(MAX_CLEANUP_ISSUES);
    let installed_tokens: Vec<String> = config
        .installed_identity_tokens
        .iter()
        .take(MAX_INSTALLED_IDENTITY_TOKENS)
        .map(|token| normalize_identity(token))
        .filter(|token| token.len() >= 3)
        .collect();
    let mut candidates = Vec::new();
    let mut issues = Vec::new();
    let mut processed_entries = 0_u64;
    let mut processed_bytes = 0_u64;
    let mut unreadable_entries = 0_u64;
    let mut scanned_roots = 0_usize;
    let mut limit_reached = false;

    'roots: for (root_index, root) in config.roots.iter().enumerate() {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }

        if !root.path.is_dir() {
            push_cleanup_issue(
                &mut issues,
                max_issues,
                Some(root.path.to_string_lossy().into_owned()),
                "정리 후보 위치가 없거나 폴더가 아닙니다".to_owned(),
            );
            continue;
        }

        let direct_entries = match fs::read_dir(&root.path) {
            Ok(entries) => entries,
            Err(error) => {
                unreadable_entries = unreadable_entries.saturating_add(1);
                push_cleanup_issue(
                    &mut issues,
                    max_issues,
                    Some(root.path.to_string_lossy().into_owned()),
                    error.to_string(),
                );
                continue;
            }
        };
        scanned_roots = scanned_roots.saturating_add(1);
        let candidates_before_root = candidates.len();

        for entry in direct_entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(error) => {
                    unreadable_entries = unreadable_entries.saturating_add(1);
                    push_cleanup_issue(
                        &mut issues,
                        max_issues,
                        Some(root.path.to_string_lossy().into_owned()),
                        error.to_string(),
                    );
                    continue;
                }
            };

            if should_cancel() {
                return Err(ScanError::Cancelled);
            }
            if candidates.len() >= max_candidates || processed_entries >= max_entries {
                limit_reached = true;
                break 'roots;
            }
            if candidates.len().saturating_sub(candidates_before_root) >= max_candidates_per_root {
                limit_reached = true;
                break;
            }

            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if root
                .protected_names
                .iter()
                .any(|protected| protected.eq_ignore_ascii_case(&name))
                || (root.kind == CleanupCandidateKind::AppDataDirectory
                    && matches_installed_identity(&name, &installed_tokens))
            {
                continue;
            }

            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    unreadable_entries = unreadable_entries.saturating_add(1);
                    push_cleanup_issue(
                        &mut issues,
                        max_issues,
                        Some(path.to_string_lossy().into_owned()),
                        error.to_string(),
                    );
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                continue;
            }
            let Some(top_modified) = metadata.modified().ok() else {
                continue;
            };
            if age_since(now, top_modified) < root.minimum_age {
                continue;
            }

            on_progress(CleanupScanProgress {
                message: format!("확인 중: {}", path.to_string_lossy()),
                processed_roots: root_index,
                total_roots,
                processed_entries,
                processed_bytes,
                candidates_found: candidates.len(),
            });

            let remaining_entries = max_entries.saturating_sub(processed_entries);
            let stats = scan_candidate(
                &path,
                metadata,
                remaining_entries,
                max_issues,
                &mut issues,
                &should_cancel,
            )?;
            processed_entries = processed_entries.saturating_add(stats.entry_count);
            processed_bytes = processed_bytes.saturating_add(stats.logical_bytes);
            unreadable_entries = unreadable_entries.saturating_add(stats.unreadable_entries);
            if stats.truncated {
                limit_reached = true;
                break 'roots;
            }

            let latest_modified = stats.latest_modified.unwrap_or(top_modified);
            let inactive = age_since(now, latest_modified);
            if inactive < root.minimum_age {
                continue;
            }
            let inactive_days = inactive.as_secs() / 86_400;
            let (confidence, evidence) = candidate_evidence(root.kind, inactive_days);
            candidates.push(CleanupCandidate {
                kind: root.kind,
                confidence,
                name,
                path: path.to_string_lossy().into_owned(),
                source_label: root.label.clone(),
                logical_bytes: stats.logical_bytes,
                entry_count: stats.entry_count,
                modified_at_unix_ms: system_time_ms(latest_modified),
                inactive_days,
                evidence,
            });
        }

        on_progress(CleanupScanProgress {
            message: format!("{} 확인 완료", root.label),
            processed_roots: root_index.saturating_add(1),
            total_roots,
            processed_entries,
            processed_bytes,
            candidates_found: candidates.len(),
        });
    }

    candidates.sort_unstable_by(|left, right| {
        right
            .logical_bytes
            .cmp(&left.logical_bytes)
            .then_with(|| right.inactive_days.cmp(&left.inactive_days))
            .then_with(|| left.path.cmp(&right.path))
    });
    let candidate_bytes = candidates.iter().fold(0_u64, |total, candidate| {
        total.saturating_add(candidate.logical_bytes)
    });

    Ok(CleanupScanReport {
        completed_at_unix_ms: system_time_ms(now).unwrap_or_default(),
        duration_ms: started.elapsed().as_millis(),
        scanned_roots,
        processed_entries,
        processed_bytes,
        unreadable_entries,
        candidate_bytes,
        candidates,
        limit_reached,
        issues,
    })
}

fn scan_candidate<C>(
    path: &Path,
    metadata: fs::Metadata,
    max_entries: u64,
    max_issues: usize,
    issues: &mut Vec<ScanIssue>,
    should_cancel: &C,
) -> Result<CandidateStats, ScanError>
where
    C: Fn() -> bool,
{
    if metadata.is_file() {
        return Ok(CandidateStats {
            logical_bytes: metadata.len(),
            entry_count: 1,
            latest_modified: metadata.modified().ok(),
            ..CandidateStats::default()
        });
    }

    let mut stats = CandidateStats {
        latest_modified: metadata.modified().ok(),
        ..CandidateStats::default()
    };
    for item in jwalk::WalkDir::new(path)
        .follow_links(false)
        .skip_hidden(false)
        .parallelism(jwalk::Parallelism::Serial)
    {
        if should_cancel() {
            return Err(ScanError::Cancelled);
        }
        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                stats.unreadable_entries = stats.unreadable_entries.saturating_add(1);
                push_cleanup_issue(
                    issues,
                    max_issues,
                    error.path().map(|path| path.to_string_lossy().into_owned()),
                    error.to_string(),
                );
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        if stats.entry_count >= max_entries {
            stats.truncated = true;
            break;
        }
        stats.entry_count = stats.entry_count.saturating_add(1);
        if entry.file_type().is_symlink() {
            continue;
        }
        let entry_metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                stats.unreadable_entries = stats.unreadable_entries.saturating_add(1);
                push_cleanup_issue(
                    issues,
                    max_issues,
                    Some(entry.path().to_string_lossy().into_owned()),
                    error.to_string(),
                );
                continue;
            }
        };
        if entry_metadata.is_file() {
            stats.logical_bytes = stats.logical_bytes.saturating_add(entry_metadata.len());
        }
        update_latest(&mut stats.latest_modified, entry_metadata.modified().ok());
    }
    Ok(stats)
}

fn candidate_evidence(
    kind: CleanupCandidateKind,
    inactive_days: u64,
) -> (CleanupConfidence, Vec<String>) {
    let age = format!("최근 {inactive_days}일 동안 변경되지 않음");
    match kind {
        CleanupCandidateKind::TemporaryEntry => (
            CleanupConfidence::LikelySafe,
            vec![age, "현재 사용자 임시 폴더의 직접 항목".to_owned()],
        ),
        CleanupCandidateKind::CacheDirectory => (
            CleanupConfidence::LikelySafe,
            vec![age, "운영체제가 지정한 캐시 위치의 직접 항목".to_owned()],
        ),
        CleanupCandidateKind::AppDataDirectory => (
            CleanupConfidence::Review,
            vec![
                age,
                "설치 앱 인벤토리와 이름이 일치하지 않음".to_owned(),
                "계정·설정 데이터일 수 있어 삭제 전 확인 필요".to_owned(),
            ],
        ),
    }
}

fn normalize_identity(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn matches_installed_identity(name: &str, installed_tokens: &[String]) -> bool {
    let name = normalize_identity(name);
    name.len() >= 3
        && installed_tokens
            .iter()
            .any(|token| token.contains(&name) || name.contains(token))
}

fn age_since(now: SystemTime, modified: SystemTime) -> Duration {
    now.duration_since(modified).unwrap_or(Duration::ZERO)
}

fn update_latest(current: &mut Option<SystemTime>, candidate: Option<SystemTime>) {
    if let Some(candidate) = candidate
        && current.is_none_or(|current| candidate > current)
    {
        *current = Some(candidate);
    }
}

fn system_time_ms(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_millis())
}

fn push_cleanup_issue(
    issues: &mut Vec<ScanIssue>,
    max_issues: usize,
    path: Option<String>,
    message: String,
) {
    if issues.len() < max_issues {
        issues.push(ScanIssue { path, message });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_direct_cleanup_candidates_and_sums_nested_files() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let nested = temp.path().join("old-folder").join("nested");
        fs::create_dir_all(&nested).expect("create nested directory");
        fs::write(nested.join("cache.bin"), vec![0_u8; 32]).expect("write cache file");
        fs::write(nested.join(".hidden-cache"), vec![0_u8; 5]).expect("write hidden cache");
        fs::write(temp.path().join("old.tmp"), vec![0_u8; 8]).expect("write temp file");

        let config = CleanupScanConfig {
            roots: vec![CleanupRootSpec::new(
                temp.path(),
                "테스트 임시 폴더",
                CleanupCandidateKind::TemporaryEntry,
                Duration::ZERO,
            )],
            max_entries: 1_000,
            ..CleanupScanConfig::default()
        };
        let report = scan_cleanup_candidates(config, |_| {}, || false).expect("scan");

        assert_eq!(report.candidates.len(), 2);
        assert_eq!(report.candidate_bytes, 45);
        assert!(
            report
                .candidates
                .iter()
                .all(|candidate| candidate.confidence == CleanupConfidence::LikelySafe)
        );
    }

    #[test]
    fn protects_system_names_and_installed_app_identities() {
        let temp = tempfile::tempdir().expect("create temp directory");
        fs::create_dir(temp.path().join("Microsoft")).expect("create protected directory");
        fs::create_dir(temp.path().join("Discord")).expect("create installed directory");
        fs::create_dir(temp.path().join("OldExample")).expect("create residue directory");

        let root = CleanupRootSpec::new(
            temp.path(),
            "AppData",
            CleanupCandidateKind::AppDataDirectory,
            Duration::ZERO,
        )
        .with_protected_names(["Microsoft"]);
        let config = CleanupScanConfig {
            roots: vec![root],
            installed_identity_tokens: vec!["Discord Client".to_owned()],
            ..CleanupScanConfig::default()
        };
        let report = scan_cleanup_candidates(config, |_| {}, || false).expect("scan");

        assert_eq!(report.candidates.len(), 1);
        assert_eq!(report.candidates[0].name, "OldExample");
        assert_eq!(report.candidates[0].confidence, CleanupConfidence::Review);
    }
}
