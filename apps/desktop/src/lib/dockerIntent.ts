const dockerIntentTerms = [
  "docker",
  "도커",
  "containerd",
  "container",
  "컨테이너",
  "buildx",
  "build cache",
  "build-cache",
  "빌드 캐시",
] as const;

export function isDockerManagementQuestion(message: string): boolean {
  const normalized = message.normalize("NFKC").toLocaleLowerCase("en-US");
  return dockerIntentTerms.some((term) => normalized.includes(term));
}
