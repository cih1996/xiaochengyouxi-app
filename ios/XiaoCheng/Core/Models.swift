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
}

struct AuthResponse: Decodable {
    let token: String
    let user: User
}

struct OK: Decodable { let ok: Bool? }
