import Foundation

struct APIError: LocalizedError {
    let status: Int
    let message: String
    let code: String?
    var errorDescription: String? { message }
}

/// 对应 Android Api.java：JSON over HTTPS，`Authorization: Bearer <token>`，错误体 `{"error","code"?}`。
final class APIClient {
    static let shared = APIClient()
    var token: String?

    private let session: URLSession = {
        let c = URLSessionConfiguration.default
        c.timeoutIntervalForRequest = 20
        return URLSession(configuration: c)
    }()

    private let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.keyDecodingStrategy = .convertFromSnakeCase
        return d
    }()

    func get<T: Decodable>(_ path: String, as: T.Type = T.self) async throws -> T {
        try await send("GET", path, body: Optional<[String: String]>.none)
    }

    func post<T: Decodable, B: Encodable>(_ path: String, _ body: B, as: T.Type = T.self) async throws -> T {
        try await send("POST", path, body: body)
    }

    private func send<T: Decodable, B: Encodable>(_ method: String, _ path: String, body: B?) async throws -> T {
        var req = URLRequest(url: AppConfig.apiBaseURL.appendingPathComponent("api" + path))
        req.httpMethod = method
        req.setValue("application/json", forHTTPHeaderField: "Accept")
        req.setValue("OrangeApp-iOS/\(AppConfig.version)", forHTTPHeaderField: "User-Agent")
        if let token, !token.isEmpty {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        if let body {
            req.setValue("application/json", forHTTPHeaderField: "Content-Type")
            req.httpBody = try JSONEncoder().encode(body)
        }
        let (data, resp) = try await session.data(for: req)
        let status = (resp as? HTTPURLResponse)?.statusCode ?? 0
        guard (200..<300).contains(status) else {
            let e = try? JSONDecoder().decode([String: String].self, from: data)
            throw APIError(status: status, message: e?["error"] ?? "网络异常（\(status)）", code: e?["code"])
        }
        return try decoder.decode(T.self, from: data)
    }
}
