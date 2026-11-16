"""Resize the approved brand artwork into Android launcher resources."""

from pathlib import Path
from PIL import Image

android = Path(__file__).resolve().parents[1]
source = Image.open(android.parent / "assets/app-icon-mascot.png").convert("RGB")
resources = android / "app/src/main/res"

for density, size in {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}.items():
    directory = resources / f"mipmap-{density}"
    directory.mkdir(parents=True, exist_ok=True)
    icon = source.resize((size, size), Image.Resampling.LANCZOS)
    icon.save(directory / "ic_launcher.png", optimize=True)
    icon.save(directory / "ic_launcher_round.png", optimize=True)

# Adaptive layers are 108 dp; the approved square art occupies the 72 dp viewport.
# The background continues the source artwork's peach edge beyond that viewport.
foreground = resources / "drawable-nodpi/launcher_mascot.png"
source.resize((432, 432), Image.Resampling.LANCZOS).save(foreground, optimize=True)
adaptive = resources / "mipmap-anydpi-v26"
adaptive.mkdir(parents=True, exist_ok=True)
xml = """<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/launcher_background" />
    <foreground>
        <inset android:inset="16.6667%">
            <bitmap android:src="@drawable/launcher_mascot" android:gravity="fill" />
        </inset>
    </foreground>
</adaptive-icon>
"""
for name in ("ic_launcher.xml", "ic_launcher_round.xml"):
    (adaptive / name).write_text(xml, encoding="utf-8")
(resources / "values/launcher.xml").write_text(
    '<resources><color name="launcher_background">#FFDCB5</color></resources>\n', encoding="utf-8"
)
print("Launcher icons generated from assets/app-icon-mascot.png")
