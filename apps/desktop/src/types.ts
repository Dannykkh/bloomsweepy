export type ViewId =
  | "overview"
  | "files"
  | "documents"
  | "cleanup"
  | "large-files"
  | "duplicates"
  | "settings";

export interface ScanConfig {
  minLargeFileBytes: number;
  minDuplicateFileBytes: number;
  maxLargeFiles: number;
  maxDuplicateGroups: number;
  maxDuplicateCandidates: number;
  maxIssues: number;
}

export const DEFAULT_SCAN_CONFIG: ScanConfig = {
  minLargeFileBytes: 100 * 1024 * 1024,
  minDuplicateFileBytes: 1024 * 1024,
  maxLargeFiles: 250,
  maxDuplicateGroups: 100,
  maxDuplicateCandidates: 250_000,
  maxIssues: 50,
};

export type ScanPhase = "discovering" | "sampling" | "verifying" | "finalizing";

export interface ScanProgress {
  phase: ScanPhase;
  message: string;
  processedFiles: number;
  processedBytes: number;
  fraction: number | null;
}

export interface FileEntry {
  name: string;
  path: string;
  logicalBytes: number;
  modifiedAtUnixMs: number | null;
}

export interface DuplicateGroup {
  contentHash: string;
  eachFileBytes: number;
  wastedBytes: number;
  files: FileEntry[];
}

export interface ScanIssue {
  path: string | null;
  message: string;
}

export interface ScanReport {
  root: string;
  completedAtUnixMs: number;
  durationMs: number;
  totalFiles: number;
  totalLogicalBytes: number;
  hardLinksSkipped: number;
  hardLinkIdentityLimitReached: boolean;
  unreadableEntries: number;
  candidateLimitReached: boolean;
  largeFiles: FileEntry[];
  duplicateGroups: DuplicateGroup[];
  duplicateWasteBytes: number;
  issues: ScanIssue[];
}

export interface VolumeInfo {
  name: string;
  mountPoint: string;
  fileSystem: string;
  totalBytes: number;
  availableBytes: number;
  removable: boolean;
  readOnly: boolean;
  isSystem: boolean;
}

export interface SystemOverview {
  platform: string;
  volumes: VolumeInfo[];
}

export type ScanUiState = "idle" | "scanning" | "success" | "cancelled" | "error";

export type StorageCategoryKind =
  | "applications"
  | "system"
  | "temporaryFiles"
  | "recycleBin"
  | "desktop"
  | "documents"
  | "downloads"
  | "photos"
  | "videos"
  | "audio"
  | "archives"
  | "developer"
  | "otherUsers"
  | "other";

export interface StorageCategory {
  kind: StorageCategoryKind;
  logicalBytes: number;
  fileCount: number;
}

export interface StorageLocation {
  name: string;
  path: string;
  logicalBytes: number;
  fileCount: number;
  dominantCategory: StorageCategoryKind;
  modifiedAtUnixMs: number | null;
}

export type DriveScanPhase = "discovering" | "finalizing";

export interface DriveScanProgress {
  phase: DriveScanPhase;
  message: string;
  processedFiles: number;
  processedBytes: number;
  unreadableEntries: number;
  categories: StorageCategory[];
}

export interface InstalledApplication {
  displayName: string;
  displayVersion: string | null;
  publisher: string | null;
  installLocation: string | null;
  estimatedBytes: number | null;
  registryScope: "machine" | "user";
}

export interface InstalledAppInventory {
  supported: boolean;
  source: "windowsRegistry" | "notAvailable";
  estimatedTotalBytes: number;
  applications: InstalledApplication[];
  issues: string[];
}

export interface DriveScanReport {
  root: string;
  completedAtUnixMs: number;
  durationMs: number;
  totalFiles: number;
  totalLogicalBytes: number;
  hardLinksSkipped: number;
  hardLinkDeduplication: boolean;
  hardLinkIdentityLimitReached: boolean;
  locationTrackingLimitReached: boolean;
  unreadableEntries: number;
  categories: StorageCategory[];
  largestLocations: StorageLocation[];
  issues: ScanIssue[];
  installedApps: InstalledAppInventory;
}

export interface DirectoryNode {
  name: string;
  path: string;
  logicalBytes: number;
  fileCount: number;
  directoryCount: number;
  isDirectory: boolean;
  modifiedAtUnixMs: number | null;
}

export interface DirectoryBreadcrumb {
  name: string;
  path: string;
}

export interface EmptyDirectory {
  name: string;
  path: string;
  modifiedAtUnixMs: number | null;
}

export interface DirectoryScanProgress {
  message: string;
  processedEntries: number;
  processedFiles: number;
  processedBytes: number;
  unreadableEntries: number;
}

export interface DirectoryScanReport {
  root: string;
  name: string;
  parent: string | null;
  completedAtUnixMs: number;
  durationMs: number;
  totalLogicalBytes: number;
  totalFiles: number;
  totalDirectories: number;
  directChildCount: number;
  childrenTruncated: boolean;
  trackingLimitReached: boolean;
  omittedChildCount: number;
  omittedLogicalBytes: number;
  emptyDirectoryCount: number;
  emptyDirectoriesTruncated: boolean;
  unreadableEntries: number;
  children: DirectoryNode[];
  emptyDirectories: EmptyDirectory[];
  issues: ScanIssue[];
}

export type CleanupCandidateKind =
  | "temporaryEntry"
  | "appDataDirectory"
  | "cacheDirectory";

export type CleanupConfidence = "likelySafe" | "review";

export interface CleanupCandidate {
  kind: CleanupCandidateKind;
  confidence: CleanupConfidence;
  name: string;
  path: string;
  sourceLabel: string;
  logicalBytes: number;
  entryCount: number;
  modifiedAtUnixMs: number | null;
  inactiveDays: number;
  evidence: string[];
}

export interface RegistryResidueCandidate {
  displayName: string;
  registryPath: string;
  registryScope: "machine" | "user";
  evidence: string[];
}

export interface RegistryResidueInventory {
  supported: boolean;
  candidates: RegistryResidueCandidate[];
  issues: string[];
}

export interface CleanupScanProgress {
  message: string;
  processedRoots: number;
  totalRoots: number;
  processedEntries: number;
  processedBytes: number;
  candidatesFound: number;
}

export interface CleanupScanReport {
  completedAtUnixMs: number;
  durationMs: number;
  scannedRoots: number;
  processedEntries: number;
  processedBytes: number;
  unreadableEntries: number;
  candidateBytes: number;
  candidates: CleanupCandidate[];
  limitReached: boolean;
  issues: ScanIssue[];
  registryResidues: RegistryResidueInventory;
}

export interface DuplicateTrashGroupSelection {
  contentHash: string;
  paths: string[];
}

export interface DuplicateTrashRequest {
  groups: DuplicateTrashGroupSelection[];
}

export interface CleanupTrashRequest {
  paths: string[];
  allowReviewCandidates: boolean;
}

export type TrashProgressPhase = "preflight" | "moving" | "finalizing";

export interface TrashProgress {
  phase: TrashProgressPhase;
  message: string;
  processedItems: number;
  totalItems: number;
}

export type TrashItemStatus = "moved" | "failed" | "skipped";

export interface TrashItemResult {
  path: string;
  logicalBytes: number;
  status: TrashItemStatus;
  message: string | null;
}

export interface TrashOperationResult {
  operationId: string;
  requestedCount: number;
  movedCount: number;
  movedBytes: number;
  cancelled: boolean;
  stoppedEarly: boolean;
  journalComplete: boolean;
  journalPath: string;
  items: TrashItemResult[];
}

export type RecoveryItemStatus =
  | "notStarted"
  | "originalPresent"
  | "recordedMoved"
  | "foundInTrash"
  | "recordedFailed"
  | "originalAndTrash"
  | "missing"
  | "trashLookupUnavailable"
  | "accessUnknown";

export interface RecoveryItem {
  path: string;
  logicalBytes: number;
  status: RecoveryItemStatus;
  needsAttention: boolean;
  message: string;
}

export interface RecoveryOperation {
  operationId: string;
  startedAtUnixMs: number;
  plannedCount: number;
  resolved: boolean;
  auditSaved: boolean;
  attentionCount: number;
  items: RecoveryItem[];
}

export interface ActionRecoveryReport {
  checkedAtUnixMs: number;
  journalPath: string;
  trashLookupSupported: boolean;
  trashLookupPerformed: boolean;
  incompleteOperations: RecoveryOperation[];
  issues: string[];
}

export type DocumentIndexPhase = "discovering" | "indexing" | "finalizing";

export interface DocumentIndexProgress {
  phase: DocumentIndexPhase;
  message: string;
  scannedFiles: number;
  candidateDocuments: number;
  indexedDocuments: number;
  reusedDocuments: number;
  processedBytes: number;
  skippedDocuments: number;
  unreadableEntries: number;
}

export interface DocumentIndexStatus {
  root: string;
  completedAtUnixMs: number;
  durationMs: number;
  indexedDocuments: number;
  indexedBytes: number;
  supportedExtensions: string[];
}

export interface DocumentIndexIssue {
  path: string | null;
  message: string;
}

export interface DocumentIndexReport extends DocumentIndexStatus {
  scannedFiles: number;
  candidateDocuments: number;
  updatedDocuments: number;
  reusedDocuments: number;
  removedDocuments: number;
  skippedDocuments: number;
  unsupportedDocuments: number;
  unreadableEntries: number;
  documentLimitReached: boolean;
  issues: DocumentIndexIssue[];
}

export interface DocumentSearchRequest {
  query: string;
  extensions: string[];
  maxResults: number;
}

export type DocumentFormat =
  | "plainText"
  | "pdf"
  | "word"
  | "spreadsheet"
  | "presentation"
  | "hwpx";

export type DocumentMatchSource = "content" | "name" | "path";

export interface DocumentSnippetPart {
  text: string;
  highlighted: boolean;
}

export interface DocumentSearchResult {
  name: string;
  path: string;
  extension: string;
  format: DocumentFormat;
  logicalBytes: number;
  modifiedAtUnixMs: number | null;
  matchSource: DocumentMatchSource;
  snippet: DocumentSnippetPart[];
}

export interface DocumentSearchReport {
  root: string;
  query: string;
  searchedDocuments: number;
  totalMatches: number;
  resultsTruncated: boolean;
  results: DocumentSearchResult[];
}

export type FileCatalogPhase = "discovering" | "applyingChanges" | "finalizing";
export type FileCatalogProvider = "portableWalk" | "windowsNtfs";
export type FileCatalogRefreshMode = "full" | "incremental";
export type FileCatalogEntryKind = "file" | "directory" | "symlink" | "other";
export type FileCatalogSort = "relevance" | "name" | "largest" | "modified";
export type FileCatalogMatchSource = "name" | "path";

export interface FileCatalogProgress {
  phase: FileCatalogPhase;
  message: string;
  scannedEntries: number;
  indexedEntries: number;
  indexedFiles: number;
  indexedDirectories: number;
  processedBytes: number;
  unreadableEntries: number;
}

export interface FileCatalogStatus {
  root: string;
  completedAtUnixMs: number;
  durationMs: number;
  indexedEntries: number;
  indexedFiles: number;
  indexedDirectories: number;
  indexedSymlinks: number;
  indexedBytes: number;
  unreadableEntries: number;
  entryLimitReached: boolean;
  provider: FileCatalogProvider;
  refreshMode: FileCatalogRefreshMode;
}

export interface FileCatalogIssue {
  path: string | null;
  message: string;
}

export interface FileCatalogReport extends FileCatalogStatus {
  scannedEntries: number;
  removedEntries: number;
  issues: FileCatalogIssue[];
}

export interface FileCatalogSearchRequest {
  query: string;
  kind: FileCatalogEntryKind | null;
  extensions: string[];
  minBytes: number | null;
  maxBytes: number | null;
  timezoneOffsetMinutes: number;
  sort: FileCatalogSort;
  maxResults: number;
}

export interface FileCatalogSearchResult {
  name: string;
  path: string;
  parent: string;
  extension: string;
  kind: FileCatalogEntryKind;
  logicalBytes: number;
  modifiedAtUnixMs: number | null;
  matchSource: FileCatalogMatchSource;
}

export interface FileCatalogSearchReport {
  root: string;
  query: string;
  indexedEntries: number;
  searchDurationMs: number;
  resultsTruncated: boolean;
  results: FileCatalogSearchResult[];
}
