# 真实商品库 · Android 0.3.7

捡漏页读取 `GET /api/products`，按返回的真实分类筛选；空库显示暂无在售好物。商品列表、大厅商品卡及详情均读取 `reward_coins`，显示“预计返 N 金币”，金额由服务端计算。

图片使用 `HttpURLConnection` 和 `BitmapFactory`。`/media/products/…` 相对路径按 API origin 解析，完整图片地址使用 HTTPS。加载时按尺寸采样，离开视图后取消请求；失败在图片区域显示原因和重试入口，并记录带地址与异常的错误日志。

打开详情和点击购买时分别调用 `GET /api/products/{id}`。仅最新有效商品返回后以其当前 `url` 调用系统 `ACTION_VIEW`，由 HTTP/HTTPS 通用链接打开购买平台。失效或下架错误直接显示。此页面展示服务端商品信息，金币到账由后续订单业务实现。

Launcher 使用正式素材 `assets/app-icon-mascot.png`，提供 mdpi 至 xxxhdpi 图标和 Android 8+ adaptive icon。可用 `python android/tools/generate-launcher-icons.py` 从项目根目录重新生成，需本机已有 Pillow。

详情展示前向 `/api/products/{id}/click` 上报 `kind: "detail"`，外链跳转前上报 `kind: "outbound"`。上报失败只写客户端错误日志，不影响浏览和跳转；点击去重与统计由服务端处理。

捡漏页顶部图片读取商品配置的 `promo_image_url`（兼容 `image` 字段），空值时隐藏图片区域，不使用客户端固定宣传图。

发布版本 `0.3.9` / `versionCode 12`，固定包名与原发布证书。更新说明：新增真实商品库、金币返佣展示、商品点击统计与正式服务文案。
