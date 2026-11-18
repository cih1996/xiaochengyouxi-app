import SwiftUI
import WebKit

/// 对应 Android WebActivity：只加载 AppConfig.activityHosts 内的页面，并注入 window.OrangeApp。
/// Android 的 JS 接口是同步的（token()/version() 直接返回值）；WKWebView 无法同步回传，
/// 所以在 documentStart 注入一个同形状对象：version/token 为预置值，其余方法经 messageHandler 转发。
struct WebActivityView: UIViewRepresentable {
    let url: URL
    @EnvironmentObject private var session: Session

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        let controller = WKUserContentController()
        controller.addUserScript(WKUserScript(
            source: Self.bridgeScript(token: session.token ?? ""),
            injectionTime: .atDocumentStart, forMainFrameOnly: true))
        controller.add(context.coordinator, name: "orangeApp")
        config.userContentController = controller
        let web = WKWebView(frame: .zero, configuration: config)
        web.navigationDelegate = context.coordinator
        web.customUserAgent = nil
        web.evaluateJavaScript("navigator.userAgent") { ua, _ in
            if let ua = ua as? String { web.customUserAgent = ua + " OrangeApp/\(AppConfig.version) OrangeApp-iOS" }
            web.load(URLRequest(url: url))
        }
        context.coordinator.web = web
        return web
    }

    func updateUIView(_ uiView: WKWebView, context: Context) {}

    static func trusted(_ url: URL?) -> Bool {
        guard let url, url.scheme == "https", let host = url.host else { return false }
        return AppConfig.activityHosts.contains(host)
    }

    private static func bridgeScript(token: String) -> String {
        func js(_ s: String) -> String {
            let data = try! JSONSerialization.data(withJSONObject: [s])
            return String(String(data: data, encoding: .utf8)!.dropFirst().dropLast())
        }
        return """
        (function(){
          var post=function(m,a){window.webkit.messageHandlers.orangeApp.postMessage({method:m,args:a||[]});};
          window.OrangeApp={
            platform:function(){return 'ios';},
            version:function(){return \(js(AppConfig.version));},
            token:function(){return \(js(token));},
            toast:function(t){post('toast',[String(t)]);},
            open:function(t){post('open',[String(t)]);},
            share:function(t){post('share',[String(t)]);},
            sharePoster:function(t,p,x){post('sharePoster',[String(t),String(p),String(x||'')]);},
            hasWeChat:function(){return false;},
            close:function(){post('close',[]);}
          };
        })();
        """
    }

    final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
        weak var web: WKWebView?

        func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction,
                     decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
            if action.targetFrame?.isMainFrame == false { decisionHandler(.allow); return }
            decisionHandler(WebActivityView.trusted(action.request.url) ? .allow : .cancel)
        }

        func userContentController(_ c: WKUserContentController, didReceive message: WKScriptMessage) {
            guard WebActivityView.trusted(message.frameInfo.request.url ?? web?.url),
                  let body = message.body as? [String: Any],
                  let method = body["method"] as? String else { return }
            let args = body["args"] as? [String] ?? []
            switch method {
            case "share":
                guard let text = args.first else { return }
                present(UIActivityViewController(activityItems: [text], applicationActivities: nil))
            case "open":
                // 与 Android 一致：只在本站页面间跳转
                if let s = args.first, let u = URL(string: s, relativeTo: web?.url),
                   WebActivityView.trusted(u.absoluteURL) {
                    web?.load(URLRequest(url: u.absoluteURL))
                }
            default:
                // toast / sharePoster / close 后续实现
                break
            }
        }

        private func present(_ vc: UIViewController) {
            let root = UIApplication.shared.connectedScenes
                .compactMap { ($0 as? UIWindowScene)?.keyWindow }.first?.rootViewController
            var top = root
            while let p = top?.presentedViewController { top = p }
            top?.present(vc, animated: true)
        }
    }
}
