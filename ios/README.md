# 小橙有喜 iOS

SwiftUI 原生工程，最低 iOS 15。工程文件由 [XcodeGen](https://github.com/yonaskolb/XcodeGen) 从 `project.yml` 生成，`.xcodeproj` 不入库。

## 首次准备（Mac）

```sh
brew install xcodegen
cd ios
cp Config/Local.xcconfig.example Config/Local.xcconfig   # 填 ORANGE_TEAM_ID
xcodegen generate
```

## 编译与安装真机

```sh
xcodebuild -project XiaoCheng.xcodeproj -scheme XiaoCheng -configuration Debug \
  -destination 'generic/platform=iOS' -derivedDataPath build -allowProvisioningUpdates build
xcrun devicectl device install app --device <设备ID> build/Build/Products/Debug-iphoneos/XiaoCheng.app
```

## 约定

- 个人开发者团队阶段包名 `com.lingzidev.xiaochengyouxi.dev`；正式包名留给公司账号。
- 服务端地址、活动域名在 `Config/*.xcconfig`，与 Android `BuildConfig` 对应。
- 网页活动沿用 `web/public/activity/`，`window.OrangeApp` 形状与 Android 保持一致（`platform()` 返回 `ios`）。
