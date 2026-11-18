import SwiftUI

struct HallView: View {
    @EnvironmentObject private var session: Session
    @EnvironmentObject private var reminders: Reminders
    @StateObject private var model = HallModel()
    @State private var activity: ActivityLink?
    @State private var showCheckin = false
    @State private var debugProduct = false
    @State private var newbieAfterSheet = false

    var body: some View {
        VStack(spacing: 0) {
            // 固定区：标题 + 金币、红包雨条，不随消息滚动
            VStack(spacing: 10) {
                PageHeader(title: "小橙有喜") { CoinBadge(coins: session.user?.coins) }
                RainBar(model: model).padding(.horizontal, 14)
                if !model.notice.isEmpty {
                    HStack(alignment: .top, spacing: 6) {
                        Image(systemName: "megaphone").font(.caption)
                        Text(model.notice).font(.caption).lineLimit(2)
                        Spacer(minLength: 0)
                    }
                    .foregroundColor(Color(hex: 0xA96A3E))
                    .padding(.horizontal, 12).padding(.vertical, 8)
                    .background(Color(hex: 0xFFEAD5), in: RoundedRectangle(cornerRadius: 12))
                    .padding(.horizontal, 14)
                    .transition(.opacity)
                }
            }
            .padding(.bottom, 10)
            .background(Theme.background)
            .zIndex(1)

            ScrollViewReader { proxy in
                ScrollView {
                    VStack(spacing: 12) {
                        CheckinCard(model: model, open: { showCheckin = true })
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
                    .padding(.top, 2)
                }
                .onChange(of: model.items.count) { _ in
                    withAnimation { proxy.scrollTo("bottom", anchor: .bottom) }
                }
            }
            .overlay(alignment: .topTrailing) {
                // 悬浮入口固定在屏幕上（不随列表滚动），位置与 Android 相同：签到卡右侧
                FloatingEntries(newbieBadge: model.newbieBadge, wheelBadge: model.wheelBadge, open: { activity = $0 })
                    .padding(.top, 6).padding(.trailing, 10)
            }
        }
        .background(Theme.background.ignoresSafeArea())
        .onAppear {
            model.userId = session.user?.id
            model.start()
            route(reminders.pendingTarget)
            #if DEBUG
            if DebugLaunch.screen == "checkin" { showCheckin = true }
            if DebugLaunch.screen == "product" { debugProduct = true }
            #endif
        }
        .onChange(of: model.newbiePopup) { pop in
            guard pop else { return }
            model.newbiePopup = false
            // 签到弹窗开着时不能叠全屏页：等它关闭后再弹
            if showCheckin { newbieAfterSheet = true } else if activity == nil { activity = .newbie }
        }
        .onChange(of: model.checkin?.currentDay) { _ in maybePromptCheckin() }
        .onChange(of: session.user?.id) { uid in
            // 用户信息晚于大厅加载：拿到 id 后再判断新人礼包自动弹出与每日签到提示
            guard let uid, model.userId != uid else { return }
            model.userId = uid
            Task { await model.refreshBadges() }
            maybePromptCheckin()
        }
        .sheet(isPresented: $showCheckin, onDismiss: {
            if newbieAfterSheet { newbieAfterSheet = false; DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) { activity = .newbie } }
        }) { CheckinSheet(model: model).compactSheet() }
        #if DEBUG
        .background(EmptyView().sheet(isPresented: $debugProduct) {
            if let p = model.items.compactMap(\.product).first { ProductDetailSheet(product: p) }
        })
        #endif

        .onDisappear { model.stop() }
        .onChange(of: reminders.pendingTarget) { route($0) }
        .fullScreenCover(item: $activity, onDismiss: { Task { await model.refreshBadges() } }) { link in
            ActivityScreen(link: link)
        }
    }
}

extension HallView {
    /// 点通知进入：转盘直接打开活动页；签到/红包雨停在大厅（签到卡在顶部）
    func route(_ t: Reminders.Target?) {
        guard let t else { return }
        reminders.pendingTarget = nil
        if t == .wheel { activity = .wheel }
        if t == .checkin { showCheckin = true }
    }

    /// 当天未签到时，每天自动弹一次签到（对照 Android maybePromptCheckin）
    func maybePromptCheckin() {
        guard let c = model.checkin, !c.checkedToday, let uid = session.user?.id, activity == nil else { return }
        let f = DateFormatter()
        f.timeZone = TimeZone(identifier: "Asia/Shanghai")
        f.dateFormat = "yyyy-MM-dd"
        let key = "checkin_prompt_\(uid)", today = f.string(from: Date())
        guard UserDefaults.standard.string(forKey: key) != today else { return }
        UserDefaults.standard.set(today, forKey: key)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) { if activity == nil { showCheckin = true } }
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
    let open: () -> Void
    @EnvironmentObject private var session: Session
    @State private var jump = false

    var body: some View {
        let checked = model.checkin?.checkedToday ?? (session.user?.checkedInToday ?? false)
        HStack(alignment: .center) {
            VStack(alignment: .leading, spacing: 6) {
                Text("好福利，刚刚好").font(.system(size: 22, weight: .heavy)).foregroundColor(Theme.text)
                Text("把今天的小惊喜装进口袋").font(.footnote).foregroundColor(Color(hex: 0xA0785F))
                Button(action: open) {
                    HStack(spacing: 6) {
                        Image(systemName: checked ? "checkmark" : "yensign.circle")
                        Text(checked ? "今日已签到" : "签到领金币").bold()
                    }
                    .font(.subheadline)
                    .foregroundColor(checked ? Theme.secondary : Color(hex: 0x6B3A12))
                    .padding(.horizontal, 16).frame(height: 40)
                    .background(checked ? AnyShapeStyle(Color.white.opacity(0.55)) : AnyShapeStyle(Theme.goldButton),
                                in: Capsule())
                    // 未签到：按钮轻跳提示（Android 同款），签到后停止
                    .offset(y: !checked && jump ? -4 : 0)
                }
                .padding(.top, 4)
            }
            Spacer() // 右侧留给悬浮的新人礼包入口，和 Android 一致
        }
        .padding(.leading, 20).padding(.vertical, 18)
        .background(Theme.soft, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
        .task(id: checked) {
            guard !checked else { jump = false; return }
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 1_600_000_000)
                withAnimation(.spring(response: 0.22, dampingFraction: 0.45)) { jump = true }
                try? await Task.sleep(nanoseconds: 220_000_000)
                withAnimation(.spring(response: 0.3, dampingFraction: 0.5)) { jump = false }
            }
        }
    }
}

/// 右侧悬浮的新人礼包、幸运转盘入口（Android 大厅右上角）。
/// 比 Android 0.3.24 缩小约 1/3；红包每隔几秒轻晃、转盘慢转、整体轻浮动，有可领次数时显示呼吸角标。
private struct FloatingEntries: View {
    let newbieBadge: Int
    let wheelBadge: Int
    let open: (ActivityLink) -> Void
    @State private var float = false

    var body: some View {
        VStack(spacing: 10) {
            EntryButton(label: "新人礼包", badge: newbieBadge) { open(.newbie) } art: { NewbieGlyph() }
                .offset(y: float ? -3 : 3)
            EntryButton(label: "幸运转盘", badge: wheelBadge) { open(.wheel) } art: { SpinningWheel() }
                .offset(y: float ? 3 : -3)
        }
        .onAppear {
            withAnimation(.easeInOut(duration: 1.8).repeatForever(autoreverses: true)) { float = true }
        }
    }
}

private struct EntryButton<Art: View>: View {
    let label: String
    let badge: Int
    let action: () -> Void
    @ViewBuilder var art: Art

    var body: some View {
        Button(action: action) {
            VStack(spacing: -5) {
                art.frame(width: 54, height: 54)
                    .overlay(alignment: .topTrailing) { if badge > 0 { BadgeDot(count: badge).offset(x: 6, y: -4) } }
                EntryLabel(text: label)
            }
        }
        .buttonStyle(PressScale())
        .accessibilityLabel(badge > 0 ? "\(label)，\(badge) 次可领" : label)
    }
}

private struct PressScale: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.88 : 1)
            .animation(.spring(response: 0.25, dampingFraction: 0.6), value: configuration.isPressed)
    }
}

private struct BadgeDot: View {
    let count: Int
    @State private var pulse = false
    var body: some View {
        Text(count > 9 ? "9+" : "\(count)")
            .font(.system(size: 10, weight: .bold)).foregroundColor(.white)
            .frame(minWidth: 17, minHeight: 17)
            .background(Color(hex: 0xE8343A), in: Capsule())
            .overlay(Capsule().stroke(Color.white, lineWidth: 1.5))
            .scaleEffect(pulse ? 1.12 : 0.95)
            .onAppear { withAnimation(.easeInOut(duration: 0.9).repeatForever(autoreverses: true)) { pulse = true } }
    }
}

/// 红包：每 3 秒左右晃一下，带一道扫光
private struct NewbieGlyph: View {
    @State private var angle: Double = 0
    @State private var shine = false
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(LinearGradient(colors: [Color(hex: 0xF4504A), Color(hex: 0xC8202A)], startPoint: .top, endPoint: .bottom))
                .frame(width: 44, height: 50)
                .overlay(
                    LinearGradient(colors: [.clear, .white.opacity(0.45), .clear], startPoint: .leading, endPoint: .trailing)
                        .frame(width: 14).rotationEffect(.degrees(20))
                        .offset(x: shine ? 34 : -34)
                        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                )
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                .shadow(color: .red.opacity(0.3), radius: 4, y: 2)
            Circle().fill(Theme.goldButton).frame(width: 22, height: 22)
                .overlay(Text("開").font(.system(size: 11, weight: .heavy)).foregroundColor(Color(hex: 0x8A3B0A)))
                .offset(y: -6)
        }
        .rotationEffect(.degrees(angle), anchor: .bottom)
        .task {
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 2_600_000_000)
                for a in [-9.0, 8, -6, 4, 0] {
                    withAnimation(.easeInOut(duration: 0.09)) { angle = a }
                    try? await Task.sleep(nanoseconds: 90_000_000)
                }
                shine = false
                withAnimation(.easeInOut(duration: 0.7)) { shine = true }
            }
        }
    }
}

/// 转盘：慢速持续旋转，中心金色指针不动
private struct SpinningWheel: View {
    @State private var spin = false
    var body: some View {
        ZStack {
            WheelGlyph()
                .rotationEffect(.degrees(spin ? 360 : 0))
                .animation(.linear(duration: 8).repeatForever(autoreverses: false), value: spin)
            Circle().fill(Theme.goldButton).frame(width: 14, height: 14)
                .shadow(color: .black.opacity(0.2), radius: 1, y: 1)
            Triangle().fill(Theme.goldButton).frame(width: 10, height: 9).offset(y: -27)
        }
        .onAppear { spin = true }
    }
}

private struct Triangle: Shape {
    func path(in r: CGRect) -> Path {
        var p = Path()
        p.move(to: CGPoint(x: r.midX, y: r.maxY))
        p.addLine(to: CGPoint(x: r.minX, y: r.minY))
        p.addLine(to: CGPoint(x: r.maxX, y: r.minY))
        p.closeSubpath()
        return p
    }
}

private struct EntryLabel: View {
    let text: String
    var body: some View {
        Text(text).font(.system(size: 11, weight: .heavy)).foregroundColor(Color(hex: 0x7A2E05))
            .padding(.horizontal, 7).padding(.vertical, 3)
            .background(Theme.goldButton, in: Capsule())
            .overlay(Capsule().stroke(Color.white, lineWidth: 1.2))
    }
}

private struct WheelGlyph: View {
    var body: some View {
        ZStack {
            Circle().fill(Theme.goldButton)
            ForEach(0..<8) { i in
                Wedge(start: .degrees(Double(i) * 45), end: .degrees(Double(i + 1) * 45))
                    .fill(i % 2 == 0 ? Color(hex: 0xE8343A) : .white)
                    .padding(5)
            }
        }
        .shadow(color: .black.opacity(0.12), radius: 3, y: 2)
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
                // 与 Android 0.3.25 一致：大厅商品卡固定 220 宽、正方形主图，左侧留头像位
                if let p = item.product {
                    ProductCard(product: p).frame(width: 220).padding(.leading, 54)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
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
