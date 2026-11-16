"""Cut generated art sheets into app assets. Source artwork stays untouched."""
from pathlib import Path
import argparse
import shutil
from PIL import Image

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("sheet", type=Path)
parser.add_argument("--kind", choices=["illustrations", "products"], required=True)
args = parser.parse_args()
assets = root / "assets"
assets.mkdir(exist_ok=True)
im = Image.open(args.sheet).convert("RGBA")
shutil.copy2(args.sheet, assets / f"{args.kind}-sheet.png")
names = ["packet", "reward", "mascot", "shopping"] if args.kind == "illustrations" else [f"product-{i}" for i in range(1, 5)]
w, h = im.size
for i, name in enumerate(names):
    x, y = (i % 2) * w // 2, (i // 2) * h // 2
    tile = im.crop((x, y, x + w // 2, y + h // 2))
    tile.thumbnail((640, 640), Image.Resampling.LANCZOS)
    out = assets / f"{name}.png"
    tile.save(out, optimize=True)
    for target in [root / "web/public/assets", root / "android/app/src/main/res/drawable-nodpi"]:
        target.mkdir(parents=True, exist_ok=True)
        android_name = name.replace("-", "_") if "drawable-nodpi" in str(target) else name
        shutil.copy2(out, target / f"{android_name}.png")
    print(out)
