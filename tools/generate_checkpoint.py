#!/usr/bin/env python3
"""Создать низкий световой маркер: Python 3 и Pillow только при подготовке."""
import gzip
import math
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets'
SEGMENTS = 48
RADIUS = 1.05
HEIGHT = 0.78

# Отдельная часть на сегмент нужна для сортировки прозрачных поверхностей BFGE.
lines = ['mtllib checkpoint-light.mtl']
for index in range(SEGMENTS):
    lines += [f'o segment{index}', 'usemtl checkpoint_light']
    a, b = [math.tau * n / SEGMENTS for n in [index, index + 1]]
    for angle, y in [(a, 0), (b, 0), (b, HEIGHT), (a, HEIGHT)]:
        lines.append(f'v {RADIUS * math.cos(angle):.7f} {y} {RADIUS * math.sin(angle):.7f}')
    # Половина текселя от края исключает шов повторяющейся текстуры.
    lines += ['vt 0 0.00390625', 'vt 1 0.00390625', 'vt 1 0.99609375', 'vt 0 0.99609375']
    n = index * 4 + 1
    lines.append('f ' + ' '.join(f'{v}/{v}' for v in range(n, n + 4)))
(ASSETS / 'checkpoint-light.obj.gz').write_bytes(
    gzip.compress(('\n'.join(lines) + '\n').encode(), mtime=0))
(ASSETS / 'checkpoint-light.mtl').write_text(
    'newmtl checkpoint_light\nKd 1 1 1\nd 0.65\nmap_Kd checkpoint-fade.png\n')
image = Image.new('RGBA', (1, 128))
for y in range(128):
    image.putpixel((0, y), (255, 255, 255, round(210 * (y / 127) ** 1.6)))
image.save(ASSETS / 'checkpoint-fade.png')
