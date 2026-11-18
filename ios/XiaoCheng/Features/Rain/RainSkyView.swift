import SwiftUI

/// 红包雨雨层（对照 Android ensureSky / Drop / burst / showCredit / RainBackdrop）。
/// 每帧按服务器时间计算每个红包的位置：y 从 -size 线性落到 height+size，期间左右摇摆、轻微漂移；
/// 大包带光晕与呼吸缩放。
struct RainSkyView: View {
    @ObservedObject var rain: RainController
    @State private var garlandIn = false
    @State private var appear = false

    var body: some View {
        // 外层读取安全区（HUD 避开状态栏/灵动岛），内层铺满全屏
        GeometryReader { outer in
            let topInset = outer.safeAreaInsets.top
            GeometryReader { geo in
            TimelineView(.animation) { ctx in
                let date = ctx.date
                let t = date.timeIntervalSinceReferenceDate
                ZStack(alignment: .topLeading) {
                    backdrop(geo.size, t)
                    garland(geo.size, t)
                    ForEach(rain.visibleDrops(at: date)) { drop in
                        dropView(drop, size: geo.size, date: date, t: t)
                    }
                    ForEach(rain.bursts) { b in BurstView(burst: b, now: date) }
                    ForEach(rain.credits) { c in CreditView(credit: c, now: date, width: geo.size.width) }
                    hud(date).padding(.top, topInset + 10).padding(.leading, 16)
                    closeButton.frame(maxWidth: .infinity, alignment: .trailing)
                        .padding(.top, topInset + 10).padding(.trailing, 16)
                }
                .onChange(of: Int(t * 4)) { tick in
                    rain.prune(at: date)
                    rain.checkEnded(at: date)
                    #if DEBUG
                    // -XCRainAutoTap 1：预览时每 1.5 秒自动点一个落到中段的红包（截图验证特效用）
                    if rain.preview, DebugLaunch.value("XCRainAutoTap") == "1", tick % 6 == 0,
                       let d = rain.visibleDrops(at: date).first(where: { !$0.large && rain.now.timeIntervalSince($0.reveal) > 2 }) {
                        let s: CGFloat = 62
                        let total = d.expires.timeIntervalSince(d.reveal)
                        let p = rain.now.timeIntervalSince(d.reveal) / total
                        let x = CGFloat(d.hash % 10000) / 10000 * max(0, geo.size.width - s) + s / 2
                        rain.tap(d, at: CGPoint(x: x, y: -s + (geo.size.height + s * 2) * p + s / 2))
                    }
                    #endif
                }
            }
            }
            .ignoresSafeArea()
        }
        .opacity(appear ? 1 : 0)
        .onAppear {
            withAnimation(.easeOut(duration: 0.3)) { appear = true }
            withAnimation(.spring(response: 0.65, dampingFraction: 0.7)) { garlandIn = true }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("红包雨")
    }

    // 深红渐变 + 顶部暖光 + 细碎光点（RainBackdrop）
    private func backdrop(_ size: CGSize, _ t: Double) -> some View {
        ZStack {
            LinearGradient(stops: [.init(color: Color(hex: 0xB3261B), location: 0),
                                   .init(color: Color(hex: 0x8A1A12), location: 0.45),
                                   .init(color: Color(hex: 0x4A0C08), location: 1)],
                           startPoint: .top, endPoint: .bottom)
                .opacity(0.35 + rain.overlayOpacity * 0.65)
            RadialGradient(colors: [Color(hex: 0xFFCF6E, alpha: 0.4), Color(hex: 0xFF8A3D, alpha: 0.13), .clear],
                           center: UnitPoint(x: 0.5, y: 0.1), startRadius: 0, endRadius: size.width * 0.85)
            Canvas { ctx, s in
                let colors: [Color] = [Color(hex: 0xFFD166), Color(hex: 0xFFB43C), Color(hex: 0xFF7A45), Color(hex: 0xFFE9A8), Color(hex: 0xF2493A), .white]
                for i in 0..<36 {
                    let h = RainController.stableHash("confetti\(i)")
                    let speed = 30 + Double(h % 40)
                    let x = Double(h % 1000) / 1000 * s.width + sin(t * 0.8 + Double(i)) * 10
                    let y = (Double((h / 1000) % 1000) / 1000 * s.height + t * speed).truncatingRemainder(dividingBy: s.height + 20) - 10
                    let r = 1.5 + Double(h % 3)
                    ctx.fill(Path(ellipseIn: CGRect(x: x, y: y, width: r, height: r * 1.6)),
                             with: .color(colors[i % colors.count].opacity(0.55)))
                }
            }
        }
        .frame(width: size.width, height: size.height)
    }

    // 顶部云彩花环：滑入后上下轻浮、横向呼吸
    private func garland(_ size: CGSize, _ t: Double) -> some View {
        let h = size.width * 1024 / 1536
        return Image("rain_sky").resizable().frame(width: size.width, height: h)
            .scaleEffect(x: 1 + 0.0175 * (1 + sin(t * 2 * .pi / 3.4)), y: 1, anchor: .top)
            .offset(y: garlandIn ? 3.5 * (1 + sin(t * 2 * .pi / 2.6)) : -h)
            .allowsHitTesting(false)
    }

    private func dropView(_ d: RainController.Drop, size: CGSize, date: Date, t: Double) -> some View {
        let s: CGFloat = d.large ? 104 : 62
        let total = d.expires.timeIntervalSince(d.reveal)
        let progress = total > 0 ? min(1, max(0, rain.now.timeIntervalSince(d.reveal) / total)) : 1
        let distance = size.height + s * 2
        let y = -s + distance * progress
        let x = CGFloat(d.hash % 10000) / 10000 * max(0, size.width - s)
        let tilt = 9 + Double(d.hash % 7)
        let swingPeriod = 1.8 + Double(d.hash % 500) / 500
        let driftPeriod = 2.6 + Double(d.hash % 700) / 500
        let angle = tilt * sin(t * 2 * .pi / swingPeriod)
        let drift = 9 * sin(t * 2 * .pi / driftPeriod)
        let pulse = d.large ? 1 + 0.08 * sin(t * 2 * .pi / 1.2) : 1
        return ZStack {
            if d.large {
                Circle().fill(RadialGradient(colors: [Color(hex: 0xFFE27A, alpha: 0.67), Color(hex: 0xFFB12F, alpha: 0)],
                                             center: .center, startRadius: 0, endRadius: s * 0.62))
                    .frame(width: s + 36, height: s + 36)
                    .opacity(0.55 + 0.45 * (0.5 + 0.5 * sin(t * 2 * .pi)))
            }
            Image(d.large ? "rain_large" : "rain_small").resizable().scaledToFit()
                .frame(width: s, height: s)
                .rotationEffect(.degrees(angle))
                .scaleEffect(pulse)
                .offset(x: drift)
        }
        .frame(width: s, height: s)
        .contentShape(Rectangle().inset(by: -8))
        .position(x: x + s / 2, y: y + s / 2)
        .onTapGesture { rain.tap(d, at: CGPoint(x: x + s / 2 + drift, y: y + s / 2)) }
        .accessibilityLabel(d.large ? "金喜大红包，观看视频领取奖励" : "小红包，点击领取金币")
        .accessibilityAddTraits(.isButton)
    }

    private func hud(_ date: Date) -> some View {
        HStack(spacing: 5) {
            Image(systemName: "clock").font(.system(size: 13, weight: .bold)).foregroundColor(Color(hex: 0xFFE3A3))
            Text("\(rain.timeLeft(at: date))s").font(.system(size: 14, weight: .bold)).foregroundColor(Color(hex: 0xFFF1CF))
                .monospacedDigit().padding(.trailing, 7)
            CoinIcon(size: 16)
            Text("+\(rain.roundCoins)").font(.system(size: 14, weight: .bold)).foregroundColor(Color(hex: 0xFFD166))
                .monospacedDigit()
                .scaleEffect(1).id(rain.hudPulse)
                .transition(.scale(scale: 1.35).combined(with: .opacity))
                .animation(.spring(response: 0.26, dampingFraction: 0.5), value: rain.hudPulse)
        }
        .padding(.horizontal, 12).frame(height: 36)
        .background(Color(hex: 0x400A07, alpha: 0.4), in: Capsule())
    }

    private var closeButton: some View {
        Button { withAnimation(.easeOut(duration: 0.25)) { rain.collapse() } } label: {
            Image(systemName: "xmark").font(.system(size: 15, weight: .bold)).foregroundColor(Color(hex: 0xFFF1CF))
                .frame(width: 36, height: 36)
                .background(Color(hex: 0x400A07, alpha: 0.4), in: Circle())
                .overlay(Circle().stroke(Color(hex: 0xFFF1CF, alpha: 0.53), lineWidth: 1.5))
        }
        .accessibilityLabel("收起本场红包雨")
    }
}

/// 点开时的闪光 + 一圈金色火花
private struct BurstView: View {
    let burst: RainController.Burst
    let now: Date
    var body: some View {
        let p = min(1, now.timeIntervalSince(burst.at) / 0.56)
        let ease = 1 - pow(1 - p, 2)
        let size: CGFloat = burst.large ? 104 : 62
        ZStack {
            Circle().fill(Color(hex: 0xFFE9A8, alpha: 0.8))
                .frame(width: size, height: size)
                .scaleEffect(1 + 0.9 * min(1, p / 0.68))
                .opacity(1 - min(1, p / 0.68))
            ForEach(0..<10, id: \.self) { i in
                let a = Double.pi * 2 * Double(i) / 10 + Double(RainController.stableHash("\(burst.id)\(i)") % 30) / 100
                let r = (55 + Double(RainController.stableHash("r\(burst.id)\(i)") % 35)) * ease
                let d: CGFloat = i % 2 == 0 ? 10 : 7
                Circle().fill(LinearGradient(colors: [Color(hex: 0xFFEF9A), Color(hex: 0xFFB11F)], startPoint: .top, endPoint: .bottom))
                    .frame(width: d, height: d)
                    .scaleEffect(1 - 0.6 * p)
                    .offset(x: cos(a) * r, y: sin(a) * r)
                    .opacity(1 - p)
            }
        }
        .position(burst.point)
        .allowsHitTesting(false)
    }
}

/// "+N" 飘字：弹出放大上浮，再淡出
private struct CreditView: View {
    let credit: RainController.Credit
    let now: Date
    let width: CGFloat
    var body: some View {
        let e = now.timeIntervalSince(credit.at)
        let p1 = min(1, e / 0.42)
        let p2 = max(0, min(1, (e - 0.42) / 0.38))
        // 带回弹的放大：0.4 → 1.15（OvershootInterpolator 近似）
        let c = 2.0, q = p1 - 1
        let overshoot = 1 + (c + 1) * q * q * q + c * q * q
        Text("+\(credit.coins)")
            .font(.system(size: 30, weight: .heavy, design: .rounded))
            .foregroundColor(Color(hex: 0xFFE08A))
            .shadow(color: Color(hex: 0xA3260F), radius: 6, y: 2)
            .scaleEffect(0.4 + 0.75 * overshoot - 0.15 * p2)
            .opacity(1 - p2)
            .position(x: min(max(60, credit.point.x), width - 60), y: credit.point.y - 60 * p1 - 25 * p2)
            .allowsHitTesting(false)
    }
}

/// 本场结算卡片
struct RainSummaryCard: View {
    let packets: Int
    let coins: Int
    let close: () -> Void
    @State private var shown = false
    var body: some View {
        VStack(spacing: 4) {
            Image("packet").resizable().scaledToFit().frame(width: 92, height: 92)
            Text("本场接到 \(packets) 个红包").font(.system(size: 15, weight: .bold)).foregroundColor(Theme.text)
            Text("+\(coins) 金币").font(.system(size: 30, weight: .heavy)).foregroundColor(Theme.orange)
        }
        .padding(.horizontal, 24).padding(.vertical, 20)
        .frame(width: 240)
        .background(LinearGradient(colors: [Color(hex: 0xFFF3DF), Color(hex: 0xFFDCBC)], startPoint: .top, endPoint: .bottom),
                    in: RoundedRectangle(cornerRadius: 26, style: .continuous))
        .shadow(color: .black.opacity(0.2), radius: 14, y: 6)
        .scaleEffect(shown ? 1 : 0.6).opacity(shown ? 1 : 0)
        .onTapGesture(perform: close)
        .onAppear {
            withAnimation(.spring(response: 0.38, dampingFraction: 0.6)) { shown = true }
            DispatchQueue.main.asyncAfter(deadline: .now() + 2.6) { withAnimation(.easeIn(duration: 0.25)) { close() } }
        }
    }
}

/// 开场前 10 秒倒计时卡片
struct RainCountdownCard: View {
    let seconds: Int
    let close: () -> Void
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("一场小欢喜，即将洒落").font(.system(size: 17, weight: .bold)).foregroundColor(Theme.text)
                Spacer()
                Button(action: close) { Image(systemName: "xmark").foregroundColor(Theme.secondary).frame(width: 30, height: 30) }
                    .accessibilityLabel("关闭倒计时")
            }
            Image("rain_sky").resizable().scaledToFit().frame(height: 100)
            Text("\(seconds)").font(.system(size: 65, weight: .heavy, design: .rounded)).foregroundColor(Theme.orange)
                .id(seconds).transition(.scale(scale: 0.85).combined(with: .opacity))
                .animation(.spring(response: 0.28, dampingFraction: 0.6), value: seconds)
            Text("红包雨马上来啦").font(.system(size: 18, weight: .bold)).foregroundColor(Theme.text).padding(.top, 10)
            Text("接住小红包，收下一点好心情").font(.footnote).foregroundColor(Theme.secondary).padding(.top, 8)
        }
        .padding(.horizontal, 22).padding(.top, 14).padding(.bottom, 22)
        .frame(maxWidth: 370)
        .background(LinearGradient(colors: [Color(hex: 0xFFF3DF), Color(hex: 0xFFDDBD)], startPoint: .top, endPoint: .bottom),
                    in: RoundedRectangle(cornerRadius: 28, style: .continuous))
        .padding(.horizontal, 20)
    }
}
