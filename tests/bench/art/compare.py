"""Issue #13 prototype: the Rust images against the committed ones (made by tools/make_art.py), as seen on white and
on a dark background. Needs Pillow. Run from run.sh."""
import os
import sys

from PIL import Image, ImageChops

ROOT, OUT = sys.argv[1], sys.argv[2]
PAIRS = [('app/res/drawable-nodpi/icon.png', 'icon.png'), ('app/res/drawable-nodpi/banner.png', 'banner.png'),
         ('docs/assets/icon-512.png', 'icon-512.png'), ('docs/assets/apple-touch-icon.png', 'apple-touch-icon.png'),
         ('docs/assets/favicon-64.png', 'favicon-64.png')]
for committed, made in PAIRS:
    a = Image.open(os.path.join(ROOT, committed)).convert('RGBA')
    b = Image.open(os.path.join(OUT, made)).convert('RGBA')
    row = []
    for bg in ((255, 255, 255, 255), (30, 30, 30, 255)):
        x, y = Image.new('RGBA', a.size, bg), Image.new('RGBA', a.size, bg)
        x.alpha_composite(a)
        y.alpha_composite(b)
        px = list(ImageChops.difference(x.convert('RGB'), y.convert('RGB')).getdata())
        row.append('mean %.2f, max %d, %.2f%% within 8' % (sum(map(sum, px)) / (len(px) * 3), max(map(max, px)),
                                                         sum(1 for p in px if max(p) <= 8) / len(px) * 100))
    print('%-22s white: %s | dark: %s' % (made, row[0], row[1]))
