import SwiftUI

struct HallView: View {
    @EnvironmentObject private var session: Session
    @StateObject private var model = HallModel()
    @State private var activity: ActivityLink?

    var body: some View {
        ZStack(alignment: .topTrailing) {
            ScrollViewReader { proxy in
                ScrollView {
                    VStack(spacing: 12) {
                        PageHeader(title: "小橙有喜") { CoinBadge(coins: session.user?.coins) }
                        RainBar(model: model)
                        CheckinCard(model: model)
                        Text("今天").font(.caption).foregroundColor(Theme.hint).padding(.top, 2)
                        if model.items.isEmpty {
                            ChatBubble(text: "小橙正在赶来，马上就有惊喜…")
                        }
                        ForEach(model.items) { item in
                            HallItemView(item: item).id(item.id)
                                .transition(.move(edge: .bottom).combined(with: .opacity))
                        }
                        Color.clear.frame(height: 12).id("bottom")
                    }
                    .padding(.horizontal, 14)
                }
                .onChange(of: model.items.count) { _ in
                    withAnimation { proxy.scrollTo("bottom", anchor: .bottom) }
                }
            }
            FloatingEntries(open: { activity = $0 })
                .padding(.top, 150).padding(.trailing, 6)
        }
        .background(Theme.background.ignoresSafeArea())
        .onAppear { model.start() }
        .onDisappear { model.stop() }
        .fullScreenCover(item: $activity) { link in ActivityScreen(link: link) }
    }
}

struct CoinBadge: View {
    let coins: Int?
    var body: some View {
        HStack(spacing: 6) {
            CoinIcon(size: 26)
            Text(coins.map(String.init) ?? "--").font(.system(size: 22, weight: .bold)).foregroundColor(Theme.text)
                .monospacedDigit()
        }
    }
}

private struct RainBar: View {
    @ObservedObject var model: HallModel
    var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { ctx in
            HStack(spacing: 8) {
                Image(systemName: "gift").foregroundColor(Color(hex: 0xC98A4B))
                Text("红包雨").font(.subheadline.weight(.semibold)).foregroundColor(Theme.text)
                Spacer()
                Text(model.rainText(now: ctx.date)).font(.subheadline.weight(.semibold))
                    .foregroundColor(Theme.orange).monospacedDigit()
            }
            .padding(.horizontal, 16).frame(height: 54).card(radius: 16)
        }
    }
}

private struct CheckinCard: View {
    @ObservedObject var model: HallModel
    @EnvironmentObject private var session: Session
    @State private var busy = false

    var body: some View {
        let checked = model.checkin?.checkedToday ?? (session.user?.checkedInToday ?? false)
        HStack(alignment: .center) {
            VStack(alignment: .leading, spacing: 6) {
                Text("好福利，刚刚好").font(.system(size: 22, weight: .heavy)).foregroundColor(Theme.text)
                Text("把今天的小惊喜装进口袋").font(.footnote).foregroundColor(Color(hex: 0xA0785F))
                Button(action: checkIn) {
                    HStack(spacing: 6) {
                        Image(systemName: checked ? "checkmark" : "yensign.circle")
                        Text(checked ? "今日已签到" : "签到领金币").bold()
                    }
                    .font(.subheadline)
                    .foregroundColor(checked ? Theme.secondary : Color(hex: 0x6B3A12))
                    .padding(.horizontal, 16).frame(height: 40)
                    .background(checked ? AnyShapeStyle(Color.white.opacity(0.55)) : AnyShapeStyle(Theme.goldButton),
                                in: Capsule())
                }
                .disabled(checked || busy)
                .padding(.top, 4)
            }
            Spacer() // 右侧留给悬浮的新人礼包入口，和 Android 一致
        }
        .padding(.leading, 20).padding(.vertical, 18)
        .background(Theme.soft, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
    }

    private func checkIn() {
        busy = true
        Task {
            defer { busy = false }
            do {
                let _: OK = try await APIClient.shared.post("/check-in", [String: String]())
                await model.refreshCheckin()
                await session.refreshMe()
                Toast.shared.show("签到成功")
            } catch {
                Toast.shared.show(error.localizedDescription)
            }
        }
    }
}

/// 右侧悬浮的新人礼包、幸运转盘入口（Android 大厅右上角）
private struct FloatingEntries: View {
    let open: (ActivityLink) -> Void
    var body: some View {
        VStack(spacing: 14) {
            Button { open(.newbie) } label: {
                VStack(spacing: -6) {
                    ZStack {
                        RoundedRectangle(cornerRadius: 10).fill(LinearGradient(
                            colors: [Color(hex: 0xF4504A), Color(hex: 0xC8202A)], startPoint: .top, endPoint: .bottom))
                            .frame(width: 66, height: 74)
                            .shadow(color: .red.opacity(0.3), radius: 6, y: 3)
                        Circle().fill(Theme.goldButton).frame(width: 34, height: 34)
                            .overlay(Text("開").font(.system(size: 17, weight: .heavy)).foregroundColor(Color(hex: 0x8A3B0A)))
                            .offset(y: -8)
                    }
                    EntryLabel(text: "新人礼包")
                }
            }
            Button { open(.wheel) } label: {
                VStack(spacing: -8) {
                    WheelGlyph().frame(width: 80, height: 80)
                    EntryLabel(text: "幸运转盘")
                }
            }
        }
        .buttonStyle(.plain)
    }
}

private struct EntryLabel: View {
    let text: String
    var body: some View {
        Text(text).font(.system(size: 14, weight: .heavy)).foregroundColor(Color(hex: 0x7A2E05))
            .padding(.horizontal, 10).padding(.vertical, 5)
            .background(Theme.goldButton, in: Capsule())
            .overlay(Capsule().stroke(Color.white, lineWidth: 1.5))
    }
}

private struct WheelGlyph: View {
    var body: some View {
        ZStack {
            Circle().fill(Theme.goldButton)
            ForEach(0..<8) { i in
                Wedge(start: .degrees(Double(i) * 45), end: .degrees(Double(i + 1) * 45))
                    .fill(i % 2 == 0 ? Color(hex: 0xE8343A) : .white)
                    .padding(7)
            }
            Circle().fill(Theme.goldButton).frame(width: 20, height: 20)
                .shadow(color: .black.opacity(0.2), radius: 1, y: 1)
        }
    }
}

private struct Wedge: Shape {
    let start: Angle, end: Angle
    func path(in r: CGRect) -> Path {
        var p = Path()
        let c = CGPoint(x: r.midX, y: r.midY)
        p.move(to: c)
        p.addArc(center: c, radius: r.width / 2, startAngle: start - .degrees(90), endAngle: end - .degrees(90), clockwise: false)
        p.closeSubpath()
        return p
    }
}

struct ChatBubble: View {
    let text: String
    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image("mascot").resizable().scaledToFit().frame(width: 44, height: 44)
                .background(Theme.peach, in: RoundedRectangle(cornerRadius: 12))
            Text(text).font(.body).foregroundColor(Theme.text)
                .padding(.horizontal, 16).padding(.vertical, 12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .card(radius: 16)
        }
    }
}

private struct HallItemView: View {
    let item: HallItem
    var body: some View {
        VStack(spacing: 10) {
            if let t = item.text, !t.isEmpty, item.kind != "native_ad" { ChatBubble(text: t) }
            switch item.kind {
            case "red_packet":
                if let task = item.task { PacketCard(task: task).padding(.leading, 54) }
            case "product":
                if let p = item.product { ProductCard(product: p).padding(.leading, 54) }
            default:
                EmptyView()
            }
        }
    }
}

private struct PacketCard: View {
    let task: PacketTask
    var body: some View {
        Button {
            // 激励视频与 DeviceTrust iOS SDK 接入后开放
            Toast.shared.show("暂时没有可观看的视频")
        } label: {
            VStack(alignment: .leading, spacing: 0) {
                HStack(alignment: .center) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("好运加码").font(.caption).foregroundColor(Color(hex: 0xFFE7C5))
                        Text(task.title ?? "小橙红包").font(.system(size: 22, weight: .heavy)).foregroundColor(.white)
                        Text(task.subtitle ?? "").font(.footnote).foregroundColor(Color(hex: 0xFFDDBD))
                        Text(task.amountText).font(.footnote.bold()).foregroundColor(Color(hex: 0xFFEFB9))
                            .padding(.horizontal, 12).padding(.vertical, 6)
                            .background(Color.white.opacity(0.18), in: Capsule())
                            .padding(.top, 4)
                    }
                    Spacer()
                    Image("packet").resizable().scaledToFit().frame(width: 92, height: 92)
                }
                .padding(18)
                Divider().background(Color.white.opacity(0.3)).padding(.horizontal, 18)
                HStack {
                    Text("小橙红包").font(.caption).foregroundColor(Color(hex: 0xFFDFC5))
                    Spacer()
                    Text((task.claimedToday ?? 0) > 0 ? "再领一份 ›" : "拆开看看 ›")
                        .font(.footnote.bold()).foregroundColor(.white)
                }
                .padding(.horizontal, 18).padding(.vertical, 12)
            }
            .background(Theme.hero, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
            .shadow(color: Theme.orange.opacity(0.25), radius: 10, y: 4)
        }
        .buttonStyle(.plain)
    }
}
