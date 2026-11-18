import Foundation

struct User: Decodable {
    let id: Int
    let phone: String
    let nickname: String?
    let coins: Int
    let totalEarned: Int?
    let todayEarned: Int?
    let cashYuan: Double?
    let checkedInToday: Bool?
    let inviteCode: String?
    let invitationRequired: Bool?

    var maskedPhone: String {
        guard phone.count == 11 else { return phone }
        return phone.prefix(3) + "****" + phone.suffix(4)
    }
}

struct AuthResponse: Decodable {
    let token: String
    let user: User
}

struct OK: Decodable { let ok: Bool? }

struct CheckinStatus: Decodable {
    struct Day: Decodable { let day: Int; let reward: Int; let status: String }
    let currentDay: Int
    let streak: Int
    let days: [Day]
    var today: Day? { days.first { $0.day == currentDay } }
    var checkedToday: Bool { today?.status == "checked" }
}

struct LedgerItem: Decodable, Identifiable {
    let id: Int
    let kind: String
    let title: String
    let coins: Int
    let createdAt: String
}

struct Product: Decodable, Identifiable {
    let id: Int
    let title: String
    let category: String?
    let platform: String?
    let priceText: String?
    let rewardCoins: Int?
    let image: String?
    let url: String?

    var imageURL: URL? {
        guard let image else { return nil }
        return URL(string: image, relativeTo: AppConfig.apiBaseURL)?.absoluteURL
    }
}

struct PacketTask: Decodable {
    let id: String
    let title: String?
    let subtitle: String?
    let reward: Int?
    let rewardMin: Int?
    let rewardMax: Int?
    let claimedToday: Int?

    var amountText: String {
        let lo = rewardMin ?? reward ?? 0, hi = rewardMax ?? reward ?? 0
        return (lo == hi ? "\(lo)" : "\(lo)～\(hi)") + " 金币"
    }
}

struct HallItem: Decodable, Identifiable {
    let id: String
    let kind: String
    let text: String?
    let task: PacketTask?
    let product: Product?
}

struct RainStatus: Decodable {
    struct Round: Decodable { let id: String; let endAt: String }
    let phase: String
    let nextStartAt: String?
    let round: Round?
    let joined: Bool?
}

enum ISO {
    private static let fractional: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return f
    }()
    private static let plain = ISO8601DateFormatter()
    static func date(_ s: String?) -> Date? {
        guard let s else { return nil }
        return fractional.date(from: s) ?? plain.date(from: s)
    }
}
