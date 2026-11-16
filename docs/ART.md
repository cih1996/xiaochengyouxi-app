# 美术素材

金币到账音效位于 `assets/coin-arrival.wav`，由 `tools/make-coin-sound.py` 原创合成。1.85 秒立体声金属碰撞与落币声，44.1 kHz / PCM16；Android 和网页共用同一素材。源码可重复生成并复制到两端资源目录。

使用内置 imagegen 生成两张 2×2 图集，再通过 `tools/slice-assets.py` 按四象限切图。所有交付素材均已保存到本项目，原始图集保留。

| 图集 | 切图 |
| --- | --- |
| `assets/illustrations-sheet.png` | `packet.png`、`reward.png`、`mascot.png`、`shopping.png` |
| `assets/products-sheet.png` | `product-1.png` 至 `product-4.png` |

插画具有透明背景。Android 中商品切图文件名使用 `product_1.png` 形式，Vue 中使用 `product-1.png` 形式。

## 提示词：应用插画

Use case: stylized-concept. Create a production-ready square 1536x1536 transparent PNG sprite sheet for an elegant Chinese consumer rewards app. Exactly FOUR isolated 3D clay / premium soft plastic assets in a strict evenly spaced 2x2 grid, each entirely inside its quadrant with large transparent margins, no overlap: TOP LEFT a plump upright vermilion red lucky money envelope, folded rounded top flap, gold rim, round golden clasp with a tiny embossed four-petal flower, front three-quarter view. TOP RIGHT the SAME red envelope now opened, a fan of golden coins bursting upward and tiny warm gold sparkles, front three-quarter view. BOTTOM LEFT an adorable small tangerine mascot with green leaf on head, tiny hands, glossy warm orange skin, friendly simple eyes and small smile, holding one gold coin. BOTTOM RIGHT a beautiful small coral shopping tote with cream handles, pale yellow box and small orange gift box beside it. Cohesive high-end softly lit 3D app illustration style, warm coral and mandarin orange and pale gold, rich material depth with subtle highlights, clean silhouettes. True transparent background for all four assets, isolated shadows only. No text, no letters, no numbers, no watermarks, no outer panels or card backgrounds. Each asset fits within 80% of its grid cell for safe programmatic slicing.

## 提示词：商品图

Use case: product-mockup. A square 1536x1536 premium e-commerce product photography CONTACT SHEET with exactly FOUR equal separate square photos arranged precisely in a 2x2 grid, no margins, no gutters. Each quadrant is an independent tastefully styled studio product photo, item centered with generous breathing space for square crop. TOP LEFT: a soft ivory-white pack of thick cotton tissues with a simple pale orange botanical drawing, small loose tissues beside it, warm cream seamless backdrop. TOP RIGHT: a rounded sage green insulated stainless steel travel tumbler with cream lid and handle, warm pastel sage seamless background. BOTTOM LEFT: two peach-colored pump bottles of laundry detergent with a subtle botanical motif, one smaller than the other, on a soft light peach seamless background. BOTTOM RIGHT: a pair of comfortable soft pale beige closed toe house slippers, on a light warm oat seamless background. Beautiful natural soft studio light, soft shadows, very realistic textures, consumer home and daily essentials shopping app photography. No brands, no typography, no lettering, no price tags, no watermark, no collage overlap. Every product entirely within its own exact quadrant, leave 15% margins inside each quadrant.

## 切图

```powershell
python tools/slice-assets.py '<图集路径>' --kind illustrations
python tools/slice-assets.py '<商品图集路径>' --kind products
```

切图脚本将结果复制到 `assets/`、`web/public/assets/` 和 Android 的 `drawable-nodpi/`。
