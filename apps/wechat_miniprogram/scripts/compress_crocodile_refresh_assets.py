from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src" / "assets" / "crocodile" / "crawl"
OUTPUT = ROOT / "src" / "assets" / "crocodile_refresh"

OUTPUT.mkdir(parents=True, exist_ok=True)

total = 0
for src in sorted(SOURCE.glob("crocodile_crawl_frame_*.png")):
    dst = OUTPUT / src.name
    with Image.open(src) as image:
        image.thumbnail((184, 92), Image.Resampling.LANCZOS)
        image.save(dst, "PNG", optimize=True)
    total += dst.stat().st_size

print({"files": len(list(OUTPUT.glob("*.png"))), "totalBytes": total})
