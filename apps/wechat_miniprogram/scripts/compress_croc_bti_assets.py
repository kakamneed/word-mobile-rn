from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src" / "assets" / "croc_bti"
OUTPUT = ROOT / "src" / "assets" / "croc_bti_compressed"

ASSETS = [
    "alligator_jade.png",
    "armor_guard_croc.png",
    "bard_croc.jpg",
    "battle_mage_pencil_croc.png",
    "book_guest_bu_e_ke.png",
    "cavalry_croc.png",
    "chanter_croc.png",
    "classic_croc.png",
    "correction_officer_croc.jpg",
    "forgemaster_croc.png",
    "hermit_croc.png",
    "ranger_croc.jpg",
    "scroll_master_croc.png",
    "stargazer_croc.jpg",
    "stele_croc.png",
    "swordsman_croc.png",
]

OUTPUT.mkdir(parents=True, exist_ok=True)

total = 0
for name in ASSETS:
    src = SOURCE / name
    dst = OUTPUT / f"{Path(name).stem}.jpg"
    with Image.open(src) as image:
        image = image.convert("RGB")
        image.thumbnail((360, 360), Image.Resampling.LANCZOS)
        image.save(dst, "JPEG", quality=72, optimize=True, progressive=True)
    total += dst.stat().st_size

print({"files": len(ASSETS), "totalBytes": total})
