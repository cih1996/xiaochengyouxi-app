import SwiftUI

struct ShopView: View {
    @EnvironmentObject private var session: Session
    @State private var products: [Product] = []
    @State private var category = "全部"
    @State private var loaded = false

    private var categories: [String] {
        var seen = ["全部"]
        for p in products { if let c = p.category, !c.isEmpty, !seen.contains(c) { seen.append(c) } }
        return seen
    }
    private var shown: [Product] { category == "全部" ? products : products.filter { $0.category == category } }

    var body: some View {
        VStack(spacing: 0) {
        // 固定标题栏
        PageHeader(title: "捡漏好物") { CoinBadge(coins: session.user?.coins) }
            .padding(.bottom, 8)
            .background(Theme.background)
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("认真生活，聪明买").font(.system(size: 22, weight: .heavy)).foregroundColor(Theme.text)
                    Text("心动好物，把金币也带回家").font(.footnote).foregroundColor(Theme.secondary)
                    Text("每日精选 · 好价发现").font(.caption.bold()).foregroundColor(Theme.orange).padding(.top, 2)
                }
                .padding(20).frame(maxWidth: .infinity, alignment: .leading)
                .background(Theme.soft, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
                .padding(.horizontal, 14)
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 10) {
                        ForEach(categories, id: \.self) { c in
                            Button { category = c } label: {
                                Text(c).font(.subheadline.weight(.semibold))
                                    .foregroundColor(c == category ? .white : Theme.text)
                                    .padding(.horizontal, 20).frame(height: 40)
                                    .background(c == category ? Theme.orange : Color.white, in: Capsule())
                            }
                        }
                    }
                    .padding(.horizontal, 14)
                }
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 12), GridItem(.flexible(), spacing: 12)], spacing: 12) {
                    ForEach(shown) { ProductCard(product: $0) }
                }
                .padding(.horizontal, 14)
                if loaded && shown.isEmpty {
                    Text("好物正在上架，稍后再来看看").font(.footnote).foregroundColor(Theme.hint)
                        .frame(maxWidth: .infinity).padding(.top, 40)
                }
            }
            .padding(.bottom, 16)
        }
        }
        .background(Theme.background.ignoresSafeArea())
        .refreshable { await load() }
        .task { if !loaded { await load() } }
    }

    private func load() async {
        if let list: [Product] = try? await APIClient.shared.get("/products") { products = list }
        loaded = true
    }
}

struct ProductCard: View {
    let product: Product
    @State private var detail = false

    var body: some View {
        Button { detail = true } label: {
            VStack(alignment: .leading, spacing: 0) {
                Color.clear.aspectRatio(1, contentMode: .fit)
                    .overlay {
                        AsyncImage(url: product.imageURL) { img in img.resizable().scaledToFill() }
                            placeholder: { Theme.peach }
                    }
                    .clipped()
                VStack(alignment: .leading, spacing: 6) {
                    Text(product.platform ?? "").font(.caption).foregroundColor(Theme.secondary)
                    Text(product.title).font(.subheadline).foregroundColor(Theme.text)
                        .lineLimit(2).multilineTextAlignment(.leading)
                        .frame(maxWidth: .infinity, minHeight: 40, alignment: .topLeading)
                    Text("¥" + (product.priceText ?? "")).font(.system(size: 20, weight: .bold)).foregroundColor(Theme.orange)
                    if let c = product.rewardCoins, c > 0 {
                        Text("预计返 \(c) 金币").font(.caption.bold()).foregroundColor(Color(hex: 0xB0622A))
                            .padding(.horizontal, 8).padding(.vertical, 4)
                            .background(Theme.cream, in: RoundedRectangle(cornerRadius: 6))
                    }
                }
                .padding(12)
            }
            .card(radius: 18)
            .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
        }
        .buttonStyle(.plain)
        .sheet(isPresented: $detail) { ProductDetailSheet(product: product) }
    }
}

/// 好物详情（对照 Android productDetail）：大图 + 平台 + 标题 + 价格 + 预计返金币 + "去 X 看看"。
/// 打开时记一次 detail 点击；跳转前重新读取有效详情，已下架则提示。
struct ProductDetailSheet: View {
    let product: Product
    @Environment(\.dismiss) private var dismiss
    @Environment(\.openURL) private var openURL
    @State private var busy = false

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("好物详情").font(.headline).foregroundColor(Theme.text)
                Spacer()
                Button { dismiss() } label: {
                    Image(systemName: "xmark").font(.system(size: 14, weight: .bold)).foregroundColor(Theme.secondary)
                        .frame(width: 30, height: 30).background(Color.black.opacity(0.05), in: Circle())
                }
                .accessibilityLabel("关闭商品详情")
            }
            .padding(.horizontal, 20).padding(.top, 18).padding(.bottom, 10)
            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    AsyncImage(url: product.imageURL) { img in img.resizable().scaledToFit() } placeholder: { Theme.peach.aspectRatio(1, contentMode: .fit) }
                        .frame(maxWidth: .infinity)
                        .background(Color.white)
                        .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
                    Text(product.platform ?? "").font(.caption).foregroundColor(Theme.orange)
                    Text(product.title).font(.system(size: 19, weight: .bold)).foregroundColor(Theme.text)
                    Text("¥" + (product.priceText ?? "")).font(.system(size: 29, weight: .bold)).foregroundColor(Theme.orange)
                    if let c = product.rewardCoins, c > 0 {
                        Text("预计返 \(c) 金币").font(.subheadline.bold()).foregroundColor(Color(hex: 0xB5753D))
                    }
                    Text("预计返还以平台实际结算为准。").font(.caption).foregroundColor(Theme.hint)
                }
                .padding(.horizontal, 20)
            }
            Button(action: go) {
                ZStack {
                    if busy { ProgressView().tint(.white) }
                    else { Text("去\(product.platform ?? "购买平台")看看").font(.system(size: 17, weight: .bold)) }
                }
                .foregroundColor(.white).frame(maxWidth: .infinity).frame(height: 54)
                .background(Theme.button, in: RoundedRectangle(cornerRadius: 27))
            }
            .disabled(busy)
            .padding(20)
        }
        .background(Theme.background.ignoresSafeArea())
        .task { let _: EmptyJSON? = try? await APIClient.shared.post("/products/\(product.id)/click", ["kind": "detail"]) }
    }

    private func go() {
        busy = true
        Task {
            defer { busy = false }
            let fresh: Product? = try? await APIClient.shared.get("/products/\(product.id)")
            guard let latest = fresh, let link = latest.url, let url = URL(string: link),
                  ["https", "http"].contains(url.scheme ?? "") else {
                Toast.shared.show(fresh == nil ? "商品已下架" : "商品购买地址无效"); return
            }
            let _: EmptyJSON? = try? await APIClient.shared.post("/products/\(product.id)/click", ["kind": "outbound"])
            openURL(url)
        }
    }
}
