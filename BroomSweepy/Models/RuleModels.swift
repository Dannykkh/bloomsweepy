import Foundation

// MARK: - Organize Rule

struct OrganizeRule: Codable, Identifiable {
    let id: UUID
    var name: String
    var isEnabled: Bool
    var conditions: [RuleCondition]
    var actions: [RuleAction]

    init(id: UUID = UUID(), name: String, isEnabled: Bool = true,
         conditions: [RuleCondition], actions: [RuleAction]) {
        self.id = id
        self.name = name
        self.isEnabled = isEnabled
        self.conditions = conditions
        self.actions = actions
    }
}

// MARK: - Rule Condition

enum RuleCondition: Codable, Hashable {
    case extensionIs(String)
    case nameContains(String)
    case sizeGreaterThan(Int64) // bytes
    case olderThanDays(Int)

    var description: String {
        switch self {
        case .extensionIs(let ext): return "확장자 = .\(ext)"
        case .nameContains(let text): return "파일명에 \"\(text)\" 포함"
        case .sizeGreaterThan(let bytes): return "크기 > \(formatSize(bytes))"
        case .olderThanDays(let days): return "\(days)일 이상 오래된"
        }
    }
}

// MARK: - Rule Action

enum RuleAction: Codable, Hashable {
    case moveToFolder(String)
    case addDatePrefix
    case addTag(String)
    case moveToTrash

    var description: String {
        switch self {
        case .moveToFolder(let folder): return "\"\(folder)\" 폴더로 이동"
        case .addDatePrefix: return "날짜 접두어 추가"
        case .addTag(let tag): return "\"\(tag)\" 태그 추가"
        case .moveToTrash: return "휴지통으로 이동"
        }
    }
}

// MARK: - Default Rules

extension OrganizeRule {
    static let defaults: [OrganizeRule] = [
        OrganizeRule(
            name: "PDF를 문서 폴더로",
            conditions: [.extensionIs("pdf")],
            actions: [.moveToFolder("문서/PDF")]
        ),
        OrganizeRule(
            name: "사진 날짜별 정리",
            conditions: [.extensionIs("jpg"), .extensionIs("png"), .extensionIs("heic")],
            actions: [.addDatePrefix, .moveToFolder("사진")]
        ),
        OrganizeRule(
            name: "30일 지난 설치파일 정리",
            conditions: [.extensionIs("dmg"), .olderThanDays(30)],
            actions: [.moveToTrash]
        ),
        OrganizeRule(
            name: "스크린샷 정리",
            conditions: [.nameContains("Screenshot")],
            actions: [.moveToFolder("스크린샷"), .addDatePrefix]
        ),
    ]
}
