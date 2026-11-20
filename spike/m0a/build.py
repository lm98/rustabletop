import sys, typst
from pathlib import Path

here = Path(__file__).parent
out = here / "out"
out.mkdir(exist_ok=True)
inputs = dict(a.split("=", 1) for a in sys.argv[1:])

typst.compile(str(here / "pnp.typ"), output=str(out / "pnp.pdf"), sys_inputs=inputs)
pages = typst.compile(str(here / "cards.typ"), format="png", ppi=300, sys_inputs=inputs)
for i, png in enumerate(pages if isinstance(pages, list) else [pages], 1):
    (out / f"card-{i:02}.png").write_bytes(png)
