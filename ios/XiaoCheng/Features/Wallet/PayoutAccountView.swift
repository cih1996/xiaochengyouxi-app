import SwiftUI
import PhotosUI
import CoreImage

/// 对照 Android 0.3.24 收款账户页。微信收款码图片只在本机识别，仅上传 wxp:// 内容。
struct PayoutAccountView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var accounts: PayoutAccounts?
    @State private var wechatName = ""
    @State private var alipayAccount = ""
    @State private var alipayName = ""
    @State private var picking = false
    @State private var busy = false

    var body: some View {
        VStack(spacing: 0) {
            BackHeader(title: "收款账户")
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    Text("绑定一次，以后提现直接选择，无需每次填写。").font(.footnote).foregroundColor(Theme.secondary)
                    wechatCard
                    alipayCard
                    Text("收款信息仅用于提现打款，提交后的提现申请不受之后修改影响。")
                        .font(.caption).foregroundColor(Theme.hint)
                }
                .padding(14)
            }
        }
        .background(Color(hex: 0xEDEDED).ignoresSafeArea())
        .navigationBarHidden(true)
        .hideTabBar()
        .task { await load() }
        .sheet(isPresented: $picking) { ImagePicker { image in if let image { bindWechat(image) } } }
    }

    private var wechatCard: some View {
        VStack(alignment: .leading, spacing: 14) {
            header("wechat", "微信收款码", accounts?.wechat?.account.map { "已绑定 · \($0)" } ?? "选择微信“收款码”图片，提现打到你的微信")
            field("真实姓名（选填，便于核对）", text: $wechatName).textContentType(.name)
            Text("在微信「我 → 服务 → 收付款 → 二维码收款」保存收款码图片后选择。图片只在手机上识别，仅上传收款码内容。")
                .font(.caption).foregroundColor(Theme.hint)
            Button { picking = true } label: {
                Text(accounts?.wechat == nil ? "选择收款码图片" : "更换收款码图片")
                    .font(.system(size: 18, weight: .bold)).foregroundColor(.white)
                    .frame(maxWidth: .infinity).frame(height: 52)
                    .background(Color(hex: 0x07C160), in: RoundedRectangle(cornerRadius: 10))
            }
            .disabled(busy)
        }
        .padding(20).background(Color.white, in: RoundedRectangle(cornerRadius: 16))
    }

    private var alipayCard: some View {
        VStack(alignment: .leading, spacing: 14) {
            header("alipay", "支付宝", accounts?.alipay?.account.map { "已绑定 · \($0)" } ?? "填写支付宝账号和实名认证的姓名")
            field("支付宝账号（手机号或邮箱）", text: $alipayAccount)
                .keyboardType(.emailAddress).textInputAutocapitalization(.never).autocorrectionDisabled()
            field("真实姓名（须与支付宝实名一致）", text: $alipayName).textContentType(.name)
            Button(action: bindAlipay) {
                Text(accounts?.alipay == nil ? "绑定支付宝" : "更新支付宝")
                    .font(.system(size: 18, weight: .bold)).foregroundColor(.white)
                    .frame(maxWidth: .infinity).frame(height: 52)
                    .background(Color(hex: 0x1677FF).opacity(alipayAccount.isEmpty || alipayName.isEmpty ? 0.5 : 1),
                                in: RoundedRectangle(cornerRadius: 10))
            }
            .disabled(busy || alipayAccount.isEmpty || alipayName.isEmpty)
        }
        .padding(20).background(Color.white, in: RoundedRectangle(cornerRadius: 16))
    }

    private func header(_ ch: String, _ title: String, _ sub: String) -> some View {
        HStack(spacing: 14) {
            ChannelIcon(channel: ch)
            VStack(alignment: .leading, spacing: 3) {
                Text(title).font(.body.bold()).foregroundColor(Theme.text)
                Text(sub).font(.footnote).foregroundColor(Theme.secondary)
            }
        }
    }

    private func field(_ placeholder: String, text: Binding<String>) -> some View {
        TextField(placeholder, text: text)
            .padding(.horizontal, 18).frame(height: 52)
            .background(Color(hex: 0xF5F5F5), in: RoundedRectangle(cornerRadius: 10))
    }

    private func load() async {
        accounts = try? await APIClient.shared.get("/payout-account")
        if let n = accounts?.wechat?.name, wechatName.isEmpty { wechatName = n }
        if let a = accounts?.alipay {
            if alipayAccount.isEmpty { alipayAccount = a.account ?? "" }
            if alipayName.isEmpty { alipayName = a.name ?? "" }
        }
    }

    private func bindWechat(_ image: UIImage) {
        guard let code = QRDecoder.decode(image), code.lowercased().hasPrefix("wxp://") else {
            Toast.shared.show("没有识别到微信收款码，请选择“二维码收款”页面保存的图片")
            return
        }
        busy = true
        Task {
            defer { busy = false }
            do {
                let _: EmptyJSON = try await APIClient.shared.post("/payout-account/wechat",
                    ["code": code, "real_name": wechatName.trimmingCharacters(in: .whitespaces)])
                Toast.shared.show("微信收款码已绑定")
                await load()
            } catch { Toast.shared.show(error.localizedDescription) }
        }
    }

    private func bindAlipay() {
        busy = true
        Task {
            defer { busy = false }
            do {
                let _: EmptyJSON = try await APIClient.shared.put("/payout-account/alipay",
                    ["account": alipayAccount.trimmingCharacters(in: .whitespaces),
                     "real_name": alipayName.trimmingCharacters(in: .whitespaces)])
                Toast.shared.show("支付宝已绑定")
                await load()
            } catch { Toast.shared.show(error.localizedDescription) }
        }
    }
}

enum QRDecoder {
    static func decode(_ image: UIImage) -> String? {
        guard let ci = CIImage(image: image),
              let detector = CIDetector(ofType: CIDetectorTypeQRCode, context: nil,
                                        options: [CIDetectorAccuracy: CIDetectorAccuracyHigh]) else { return nil }
        return detector.features(in: ci).compactMap { ($0 as? CIQRCodeFeature)?.messageString }.first
    }

    static func make(_ text: String, scale: CGFloat = 10) -> UIImage? {
        let f = CIFilter(name: "CIQRCodeGenerator")
        f?.setValue(Data(text.utf8), forKey: "inputMessage")
        f?.setValue("M", forKey: "inputCorrectionLevel")
        guard let out = f?.outputImage?.transformed(by: CGAffineTransform(scaleX: scale, y: scale)),
              let cg = CIContext().createCGImage(out, from: out.extent) else { return nil }
        return UIImage(cgImage: cg)
    }
}

/// iOS 15 兼容的相册选择（不需要相册权限）
struct ImagePicker: UIViewControllerRepresentable {
    let done: (UIImage?) -> Void
    func makeUIViewController(context: Context) -> PHPickerViewController {
        var c = PHPickerConfiguration()
        c.filter = .images
        c.selectionLimit = 1
        let vc = PHPickerViewController(configuration: c)
        vc.delegate = context.coordinator
        return vc
    }
    func updateUIViewController(_ vc: PHPickerViewController, context: Context) {}
    func makeCoordinator() -> Coordinator { Coordinator(done: done) }
    final class Coordinator: NSObject, PHPickerViewControllerDelegate {
        let done: (UIImage?) -> Void
        init(done: @escaping (UIImage?) -> Void) { self.done = done }
        func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
            picker.dismiss(animated: true)
            guard let p = results.first?.itemProvider, p.canLoadObject(ofClass: UIImage.self) else { done(nil); return }
            p.loadObject(ofClass: UIImage.self) { obj, _ in
                DispatchQueue.main.async { self.done(obj as? UIImage) }
            }
        }
    }
}
