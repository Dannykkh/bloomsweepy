import Foundation

// MARK: - AI Classification Result

struct AIClassification {
    let fileName: String
    let category: String
    let suggestedFolder: String
    let reasoning: String
}

// MARK: - AI Classifier (Claude API)

final class AIClassifier {
    static let shared = AIClassifier()

    private let apiKeyKey = "com.broomsweepy.claude-api-key"

    var apiKey: String? {
        get { UserDefaults.standard.string(forKey: apiKeyKey) }
        set { UserDefaults.standard.set(newValue, forKey: apiKeyKey) }
    }

    var isConfigured: Bool { apiKey != nil && !(apiKey?.isEmpty ?? true) }

    // MARK: - Classify Files

    func classify(fileNames: [String]) async throws -> [AIClassification] {
        guard let key = apiKey, !key.isEmpty else {
            return fallbackClassify(fileNames: fileNames)
        }

        let fileList = fileNames.enumerated().map { "\($0.offset + 1). \($0.element)" }.joined(separator: "\n")

        let requestBody: [String: Any] = [
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 2048,
            "messages": [
                [
                    "role": "user",
                    "content": """
                    다음 파일들을 분류해주세요. 각 파일에 대해 JSON 배열로 응답하세요.

                    파일 목록:
                    \(fileList)

                    응답 형식 (JSON 배열만, 다른 텍스트 없이):
                    [{"fileName": "파일명", "category": "카테고리", "suggestedFolder": "추천/폴더/경로", "reasoning": "이유"}]

                    카테고리: 사진, 동영상, 문서, 음악, 압축파일, 설치파일, 스크린샷, 개발, 기타
                    """
                ]
            ]
        ]

        let jsonData = try JSONSerialization.data(withJSONObject: requestBody)

        var request = URLRequest(url: URL(string: "https://api.anthropic.com/v1/messages")!)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "content-type")
        request.setValue("2023-06-01", forHTTPHeaderField: "anthropic-version")
        request.setValue(key, forHTTPHeaderField: "x-api-key")
        request.httpBody = jsonData

        let (data, response) = try await URLSession.shared.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse,
              httpResponse.statusCode == 200 else {
            return fallbackClassify(fileNames: fileNames)
        }

        return parseResponse(data: data, fileNames: fileNames)
    }

    // MARK: - Parse Response

    private func parseResponse(data: Data, fileNames: [String]) -> [AIClassification] {
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let content = json["content"] as? [[String: Any]],
              let textBlock = content.first(where: { $0["type"] as? String == "text" }),
              let text = textBlock["text"] as? String else {
            return fallbackClassify(fileNames: fileNames)
        }

        // Extract JSON from response
        guard let jsonStart = text.firstIndex(of: "["),
              let jsonEnd = text.lastIndex(of: "]") else {
            return fallbackClassify(fileNames: fileNames)
        }

        let jsonStr = String(text[jsonStart...jsonEnd])
        guard let jsonData = jsonStr.data(using: .utf8),
              let results = try? JSONSerialization.jsonObject(with: jsonData) as? [[String: String]] else {
            return fallbackClassify(fileNames: fileNames)
        }

        return results.map { dict in
            AIClassification(
                fileName: dict["fileName"] ?? "",
                category: dict["category"] ?? "기타",
                suggestedFolder: dict["suggestedFolder"] ?? "기타",
                reasoning: dict["reasoning"] ?? ""
            )
        }
    }

    // MARK: - Fallback (No API)

    func fallbackClassify(fileNames: [String]) -> [AIClassification] {
        fileNames.map { name in
            let ext = (name as NSString).pathExtension.lowercased()
            let (cat, folder) = classifyByExtension(ext)
            return AIClassification(
                fileName: name,
                category: cat,
                suggestedFolder: folder,
                reasoning: "확장자 기반 분류"
            )
        }
    }

    private func classifyByExtension(_ ext: String) -> (String, String) {
        let map: [(String, Set<String>, String)] = [
            ("사진", ["jpg", "jpeg", "png", "gif", "heic", "raw", "webp"], "사진"),
            ("동영상", ["mp4", "mov", "mkv", "avi", "wmv", "m4v"], "동영상"),
            ("문서", ["pdf", "doc", "docx", "xlsx", "pptx", "hwp", "txt", "csv"], "문서"),
            ("음악", ["mp3", "wav", "flac", "aac", "m4a"], "음악"),
            ("압축파일", ["zip", "rar", "7z", "tar", "gz"], "압축파일"),
            ("설치파일", ["dmg", "pkg", "iso"], "설치파일"),
            ("개발", ["swift", "py", "js", "ts", "html", "css", "json"], "개발"),
        ]
        for (cat, exts, folder) in map {
            if exts.contains(ext) { return (cat, folder) }
        }
        return ("기타", "기타")
    }
}
