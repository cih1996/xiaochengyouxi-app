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
    @Environment(\.openURL) private var openURL

    var body: some View {
        Button(action: open) {
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
    }

    private func open() {
        // 与 Android 一致：跳转前重新读取有效详情，记录点击
        Task {
            let fresh: Product? = try? await APIClient.shared.get("/products/\(product.id)")
            guard let link = (fresh ?? product).url, let url = URL(string: link) else {
                Toast.shared.show("商品已下架"); return
            }
            let _: OK? = try? await APIClient.shared.post("/products/\(product.id)/click", ["kind": "open"])
            openURL(url)
        }
    }
}
