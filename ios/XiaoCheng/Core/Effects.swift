import SwiftUI
import AVFoundation
import UIKit

/// 音效（与 Android res/raw 同款），用 ambient 类别：不打断用户正在播放的音乐，静音键下不响。
enum Sound {
    private static var players: [String: AVAudioPlayer] = [:]

    static func play(_ name: String) {
        try? AVAudioSession.sharedInstance().setCategory(.ambient, options: [.mixWithOthers])
        if players[name] == nil, let url = Bundle.main.url(forResource: name, withExtension: "wav") {
            players[name] = try? AVAudioPlayer(contentsOf: url)
            players[name]?.prepareToPlay()
        }
        players[name]?.currentTime = 0
        players[name]?.play()
    }

    static func coins() { play("coin_arrival"); Haptics.success() }
    static func packet() { play("packet_open"); Haptics.tap() }
}

enum Haptics {
    static func tap() { UIImpactFeedbackGenerator(style: .medium).impactOccurred() }
    static func success() { UINotificationFeedbackGenerator().notificationOccurred(.success) }
}

/// 金币到账庆祝（对照 Android celebrate()/flyCoins()）：中间弹出 +N，金币从中间洒向右上角金币数。
final class Celebration: ObservableObject {
    static let shared = Celebration()
    @Published var amount: Int?
    @Published var token = 0

    func show(_ coins: Int, title: String = "金币到账") {
        guard coins > 0 else { return }
        DispatchQueue.main.async {
            self.title = title
            self.token += 1
            self.amount = coins
            Sound.coins()
            let t = self.token
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.9) { if t == self.token { self.amount = nil } }
        }
    }
    var title = "金币到账"
}

struct CelebrationOverlay: ViewModifier {
    @ObservedObject var c = Celebration.shared
    func body(content: Content) -> some View {
        content.overlay {
            if let n = c.amount {
                CelebrationView(amount: n, title: c.title).id(c.token)
                    .allowsHitTesting(false)
            }
        }
    }
}

private struct CelebrationView: View {
    let amount: Int
    let title: String
    @State private var pop = false
    @State private var fly = false

    var body: some View {
        GeometryReader { geo in
            ZStack {
                Color.black.opacity(pop ? 0.18 : 0).ignoresSafeArea()
                VStack(spacing: 6) {
                    CoinIcon(size: 64)
                    Text("+\(amount)").font(.system(size: 44, weight: .heavy, design: .rounded)).foregroundColor(Color(hex: 0xFFE07A))
                        .shadow(color: Color(hex: 0xC2410C).opacity(0.6), radius: 0, x: 0, y: 2)
                    Text(title).font(.headline).foregroundColor(.white)
                }
                .padding(.horizontal, 34).padding(.vertical, 22)
                .background(Theme.hero, in: RoundedRectangle(cornerRadius: 26, style: .continuous))
                .scaleEffect(pop ? 1 : 0.4).opacity(pop ? 1 : 0)
                .opacity(fly ? 0 : 1)
                ForEach(0..<10, id: \.self) { i in
                    CoinIcon(size: 22)
                        .position(x: fly ? geo.size.width - 60 : geo.size.width / 2 + CGFloat((i % 5) - 2) * 24,
                                  y: fly ? 30 : geo.size.height / 2 + CGFloat(i / 5) * 26 - 10)
                        .opacity(fly ? 0.2 : (pop ? 1 : 0))
                        .animation(.easeIn(duration: 0.55).delay(0.75 + Double(i) * 0.04), value: fly)
                }
            }
        }
        .onAppear {
            withAnimation(.spring(response: 0.38, dampingFraction: 0.6)) { pop = true }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) { withAnimation(.easeIn(duration: 0.35)) { fly = true } }
        }
    }
}
