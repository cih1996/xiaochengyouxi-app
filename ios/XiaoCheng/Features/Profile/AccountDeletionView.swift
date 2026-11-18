import SwiftUI

/// 注销账号：提交申请 → 人工审核；审核前可撤回。（Apple 审核要求 App 内可发起注销）
struct AccountDeletionView: View {
    struct Status: Decodable { let status: String?; let createdAt: String?; let reviewNote: String? }
    @EnvironmentObject private var session: Session
    @State private var status: Status?
    @State private var reason = ""
    @State private var confirm = false
    @State private var busy = false

    private var pending: Bool { status?.status == "pending" }

    var body: some View {
        SubPage(title: "注销账号") {
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    VStack(alignment: .leading, spacing: 10) {
                        Text(pending ? "注销申请审核中" : "注销前请确认").font(.headline).foregroundColor(Theme.text)
                        Text(pending
                             ? "我们会在 15 个工作日内处理。审核通过前你可以撤回申请，账号照常使用。"
                             : "注销后账号信息、金币余额和收益记录将被清除且无法恢复，未完成的提现会一并取消。提交后需人工审核。")
                            .font(.subheadline).foregroundColor(Theme.secondary)
                    }
                    .padding(20).frame(maxWidth: .infinity, alignment: .leading).card(radius: 18)
                    if !pending {
                        TextField("注销原因（选填）", text: $reason)
                            .padding(.horizontal, 18).frame(height: 54).card(radius: 16)
                    }
                    Button { pending ? cancel() : (confirm = true) } label: {
                        Text(pending ? "撤回注销申请" : "申请注销").font(.body.bold())
                            .foregroundColor(pending ? Theme.orange : .white)
                            .frame(maxWidth: .infinity).frame(height: 54)
                            .background(pending ? Color.white : Color(hex: 0xE04848), in: RoundedRectangle(cornerRadius: 27))
                    }
                    .disabled(busy)
                    .padding(.top, 6)
                }
                .padding(14)
            }
        }
        .task { await load() }
        .confirmationDialog("确定申请注销账号？", isPresented: $confirm, titleVisibility: .visible) {
            Button("申请注销", role: .destructive) { submit() }
        }
    }

    private func load() async { status = try? await APIClient.shared.get("/account/deletion-status") }

    private func submit() {
        busy = true
        Task {
            defer { busy = false }
            do {
                let _: EmptyJSON = try await APIClient.shared.post("/account/delete", ["reason": reason])
                Toast.shared.show("注销申请已提交")
                await load()
            } catch { Toast.shared.show(error.localizedDescription) }
        }
    }

    private func cancel() {
        busy = true
        Task {
            defer { busy = false }
            do {
                let _: EmptyJSON = try await APIClient.shared.post("/account/deletion/cancel", [String: String]())
                Toast.shared.show("已撤回注销申请")
                await load()
            } catch { Toast.shared.show(error.localizedDescription) }
        }
    }
}
