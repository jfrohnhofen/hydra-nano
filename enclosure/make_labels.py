#!/usr/bin/env python3
"""Generate printable hydra nano label sheets (PDF) from a list of units.

Input is a list of (mac, hostname, ip), one unit per line, comma / semicolon /
tab / whitespace separated. Blank lines, `#` comments and a header row are
ignored:

    mac,hostname,ip
    aa:bb:cc:1a:2b:3c,hydra-1a2b3c,192.168.1.51
    aa:bb:cc:4d:5e:6f,hydra-4d5e6f,192.168.1.52

Every unit is printed COPIES times (default 3, next to each other) on A4/Letter
sheets, each label as large as the template (49.69 x 50 mm), laid out on a centred
grid with crop marks (short ticks outside each corner) to cut along, plus a tick on
each side at the height of the BOOTSEL hole to pencil-mark the punch position.

    ./make_labels.py units.csv                 # -> labels.pdf
    ./make_labels.py units.csv -o out.pdf --copies 3 --paper letter
    ./make_labels.py units.csv --cutlines      # also draw the magenta cut layer
    ./make_labels.py units.csv --no-cropmarks  # sheet without crop marks
    ./make_labels.py - < units.csv             # read stdin

PDF conversion uses the first of rsvg-convert, inkscape or ImageMagick found on
PATH (rsvg-convert / inkscape give a vector PDF with embedded fonts; ImageMagick
falls back to a 600 dpi raster). With --svg-only (or no converter) the sheet
SVGs are written instead.

Only the Python standard library is needed. The label design lives in
hydra-nano-label.svg; the elements with ids `mac`, `host`, `ip` (and
optionally `version`) are replaced per unit.
"""
from __future__ import annotations

import argparse
import csv
import ipaddress
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from xml.sax.saxutils import escape

PAPERS = {"a4": (210.0, 297.0), "letter": (215.9, 279.4)}
HERE = Path(__file__).resolve().parent


@dataclass(frozen=True)
class Unit:
    mac: str
    hostname: str
    ip: str


# --------------------------------------------------------------------------- input


def split_line(line: str) -> list[str]:
    for sep in (",", ";", "\t"):
        if sep in line:
            return [f.strip() for f in next(csv.reader([line], delimiter=sep))]
    return line.split()


def parse_units(text: str, add_local: bool) -> list[Unit]:
    units: list[Unit] = []
    for n, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = split_line(line)
        if len(fields) != 3:
            sys.exit(f"line {n}: expected 3 fields (mac, hostname, ip), got {len(fields)}: {raw!r}")
        mac, host, ip = fields
        if not units and {f.lower() for f in fields} & {"ip", "hostname", "host", "mac"}:
            continue  # header row
        hexdigits = re.sub(r"[^0-9a-fA-F]", "", mac)
        if len(hexdigits) == 12 and re.fullmatch(r"[0-9a-fA-F]{2}([-:.]?[0-9a-fA-F]{2}){5}", mac):
            mac = ":".join(hexdigits[i : i + 2] for i in range(0, 12, 2)).upper()
        else:
            print(f"warning: line {n}: {mac!r} is not a MAC address", file=sys.stderr)
        try:
            ipaddress.IPv4Address(ip)
        except ValueError:
            print(f"warning: line {n}: {ip!r} is not an IPv4 address", file=sys.stderr)
        if add_local and "." not in host:
            host += ".local"
        units.append(Unit(mac, host, ip))
    return units


# --------------------------------------------------------------------------- label


def split_template(svg: str) -> tuple[str, str]:
    """Returns (attributes of the root <svg>, inner markup)."""
    m = re.search(r"<svg\b([^>]*)>(.*)</svg>", svg, re.S)
    if not m:
        sys.exit("template is not an SVG document")
    return m.group(1), m.group(2)


def set_text(markup: str, elem_id: str, value: str) -> str:
    pat = re.compile(r'(<text\b[^>]*\bid="%s"[^>]*>)(.*?)(</text>)' % re.escape(elem_id), re.S)
    if not pat.search(markup):
        return markup
    return pat.sub(lambda m: m.group(1) + escape(value) + m.group(3), markup, count=1)


def render_label(inner: str, unit: Unit, version: str | None, cutlines: bool) -> str:
    out = re.sub(r"<title>.*?</title>", "", inner, flags=re.S)
    if not cutlines:
        out = re.sub(r'<g\b[^>]*\bid="cut"[^>]*>.*?</g>', "", out, flags=re.S)
    out = set_text(out, "mac", unit.mac)
    out = set_text(out, "host", unit.hostname)
    out = set_text(out, "ip", unit.ip)
    if version:
        out = set_text(out, "version", version)
    # ids repeat once per copy; drop them in the sheet.
    return re.sub(r'\s+id="[^"]*"', "", out)


# --------------------------------------------------------------------------- sheet


def label_size_mm(root_attrs: str) -> tuple[float, float]:
    sizes = []
    for attr in ("width", "height"):
        m = re.search(r'\b%s="([\d.]+)mm"' % attr, root_attrs)
        if not m:
            sys.exit(f'template root needs {attr}="<n>mm"')
        sizes.append(float(m.group(1)))
    return sizes[0], sizes[1]


def user_unit_mm(root_attrs: str) -> float:
    """Millimetres per user unit of the template (width in mm / viewBox width)."""
    m = re.search(r'\bviewBox="[\d.\-]+[ ,]+[\d.\-]+[ ,]+([\d.]+)[ ,]+[\d.]+"', root_attrs)
    if not m:
        sys.exit("template root needs a viewBox")
    return label_size_mm(root_attrs)[0] / float(m.group(1))


def punch_height_mm(template: str) -> float | None:
    """Height of the BOOTSEL hole centre (from the label's top edge), read from the cut layer."""
    m = re.search(r'<g\b[^>]*\bid="cut"[^>]*>.*?<circle\b[^>]*\bcy="([\d.]+)"', template, re.S)
    return float(m.group(1)) * user_unit_mm(split_template(template)[0]) if m else None


def crop_marks(
    x: float,
    y: float,
    w: float,
    h: float,
    punch_y: float | None = None,
    offset: float = 0.4,
    length: float = 1.0,
) -> str:
    """Short ticks just outside a label's cut edge: 8 crop marks (4 corners x 2) plus,
    if `punch_y` is given, a tick on each side at the height of the punch hole, so
    the hole position can be transferred to the cut label with a pencil."""
    d = []
    if punch_y is not None:
        d.append(f"M{x - offset:.3f} {y + punch_y:.3f}h{-length:.3f}")
        d.append(f"M{x + w + offset:.3f} {y + punch_y:.3f}h{length:.3f}")
    for cx, sx in ((x, -1), (x + w, 1)):
        for cy, sy in ((y, -1), (y + h, 1)):
            d.append(f"M{cx + sx * offset:.3f} {cy:.3f}h{sx * length:.3f}")
            d.append(f"M{cx:.3f} {cy + sy * offset:.3f}v{sy * length:.3f}")
    return f'<path d="{"".join(d)}" fill="none" stroke="#000" stroke-width="0.15"/>'


def build_sheets(
    labels: list[str],
    root_attrs: str,
    paper: tuple[float, float],
    margin: float,
    gap: float,
    cropmarks: bool = True,
    punch_y: float | None = None,
) -> list[str]:
    pw, ph = paper
    lw, lh = label_size_mm(root_attrs)
    cols = int((pw - 2 * margin + gap) // (lw + gap))
    rows = int((ph - 2 * margin + gap) // (lh + gap))
    if cols < 1 or rows < 1:
        sys.exit("labels do not fit on the page; reduce --margin / --gap")
    per_page = cols * rows
    x0 = (pw - (cols * lw + (cols - 1) * gap)) / 2
    y0 = (ph - (rows * lh + (rows - 1) * gap)) / 2

    # keep the template's namespace declarations (inkscape layer attributes etc.)
    ns = " ".join(re.findall(r'xmlns(?::\w+)?="[^"]*"', root_attrs))
    if "xmlns=" not in ns:
        ns = 'xmlns="http://www.w3.org/2000/svg" ' + ns

    k = user_unit_mm(root_attrs)
    sheets = []
    for start in range(0, len(labels), per_page):
        parts = [
            f'<svg {ns} width="{pw}mm" height="{ph}mm" viewBox="0 0 {pw} {ph}">',
            f'<rect width="{pw}" height="{ph}" fill="#fff"/>',
        ]
        for i, label in enumerate(labels[start : start + per_page]):
            r, c = divmod(i, cols)
            x = x0 + c * (lw + gap)
            y = y0 + r * (lh + gap)
            parts.append(f'<g transform="translate({x:.3f} {y:.3f}) scale({k:.6f})">{label}</g>')
            if cropmarks:
                parts.append(crop_marks(x, y, lw, lh, punch_y))
        parts.append("</svg>")
        sheets.append("\n".join(parts))
    print(f"{len(labels)} labels, {cols} x {rows} per page, {len(sheets)} page(s)", file=sys.stderr)
    return sheets


# --------------------------------------------------------------------------- pdf


def to_pdf(svg_files: list[Path], pdf: Path) -> str:
    if exe := shutil.which("rsvg-convert"):
        subprocess.run([exe, "-f", "pdf", "-o", str(pdf), *map(str, svg_files)], check=True)
        return "rsvg-convert (vector)"
    if exe := shutil.which("inkscape"):
        if len(svg_files) == 1:
            subprocess.run([exe, str(svg_files[0]), "--export-type=pdf", f"--export-filename={pdf}"], check=True)
            return "inkscape (vector)"
        pdfs = []
        for f in svg_files:
            p = f.with_suffix(".pdf")
            subprocess.run([exe, str(f), "--export-type=pdf", f"--export-filename={p}"], check=True)
            pdfs.append(str(p))
        if gs := shutil.which("gs"):
            subprocess.run(
                [gs, "-q", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite", f"-sOutputFile={pdf}", *pdfs], check=True
            )
            return "inkscape + ghostscript (vector)"
    if exe := shutil.which("magick") or shutil.which("convert"):
        subprocess.run(
            [exe, "-density", "600", "-background", "white", *map(str, svg_files),
             "-alpha", "remove", "-units", "PixelsPerInch", str(pdf)],
            check=True,
        )
        return "ImageMagick (600 dpi raster)"
    raise FileNotFoundError("no SVG -> PDF converter found (install librsvg / inkscape / imagemagick)")


# --------------------------------------------------------------------------- main


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("input", help="units file (mac,hostname,ip per line) or - for stdin")
    ap.add_argument("-o", "--output", default="labels.pdf", help="output PDF (default: labels.pdf)")
    ap.add_argument("-t", "--template", type=Path, default=HERE / "hydra-nano-label.svg")
    ap.add_argument("-n", "--copies", type=int, default=3, help="copies per unit (default: 3)")
    ap.add_argument("--paper", choices=sorted(PAPERS), default="a4")
    ap.add_argument("--margin", type=float, default=8.0, help="page margin in mm (default: 8)")
    ap.add_argument("--gap", type=float, default=3.0, help="gap between labels in mm (default: 3)")
    ap.add_argument("--version", help="override the VERSION text on all labels")
    ap.add_argument("--no-local", action="store_true", help="do not append .local to bare hostnames")
    ap.add_argument("--cutlines", action="store_true", help="include the magenta cut layer")
    ap.add_argument("--no-cropmarks", action="store_true", help="omit the crop marks around each label")
    ap.add_argument("--svg-only", action="store_true", help="write sheet SVGs, skip the PDF")
    args = ap.parse_args()

    text = sys.stdin.read() if args.input == "-" else Path(args.input).read_text()
    units = parse_units(text, add_local=not args.no_local)
    if not units:
        sys.exit("no units in input")

    root_attrs, inner = split_template(args.template.read_text())
    labels = [
        render_label(inner, u, args.version, args.cutlines) for u in units for _ in range(args.copies)
    ]
    template_text = args.template.read_text()
    sheets = build_sheets(
        labels, root_attrs, PAPERS[args.paper], args.margin, args.gap, not args.no_cropmarks,
        punch_height_mm(template_text),
    )

    out = Path(args.output)
    if args.svg_only:
        for i, svg in enumerate(sheets, 1):
            p = out.with_name(f"{out.stem}-{i}.svg")
            p.write_text(svg)
            print(p)
        return
    with tempfile.TemporaryDirectory() as tmp:
        files = []
        for i, svg in enumerate(sheets, 1):
            f = Path(tmp) / f"sheet-{i:03}.svg"
            f.write_text(svg)
            files.append(f)
        how = to_pdf(files, out)
    print(f"wrote {out} via {how}")


if __name__ == "__main__":
    main()
