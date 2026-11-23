#!/usr/bin/env python3
"""Пересоздание карточки: Python 3, Pillow, шрифт DejaVu Sans."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

OUTPUT = Path(__file__).resolve().parents[1] / 'assets/ui'
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
SCALE = 2
WIDTH, HEIGHT = 960, 680
image = Image.new('RGB', (WIDTH * SCALE, HEIGHT * SCALE))
draw = ImageDraw.Draw(image)


def box(bounds, fill, radius=0, outline=None):
    draw.rounded_rectangle(tuple(int(v * SCALE) for v in bounds),
                           radius=radius * SCALE, fill=fill, outline=outline, width=SCALE)


def text(value, x, y, size, color):
    draw.text((x * SCALE, y * SCALE), value,
              font=ImageFont.truetype(FONT, size * SCALE), fill=color, anchor='mm')


for y in range(HEIGHT * SCALE):
    t = y / (HEIGHT * SCALE - 1)
    color = tuple(round(a + (b - a) * t) for a, b in zip((27, 36, 47), (16, 23, 32)))
    draw.line((0, y, WIDTH * SCALE, y), fill=color)
gold = (198, 151, 83)
box((18, 18, 942, 662), None, radius=14, outline=(76, 72, 63))
box((444, 0, 516, 4), gold)
text('Пауза', 480, 259, 78, (236, 230, 217))
text('Можно немного выдохнуть', 480, 347, 25, (137, 150, 161))
box((130, 424, 830, 425), (52, 62, 72))
label = '— продолжить'
label_width = ImageFont.truetype(FONT, 27 * SCALE).getlength(label) / SCALE
key_width, gap = 90, 18
left = (WIDTH - key_width - gap - label_width) / 2
box((left, 489, left + key_width, 539), (36, 46, 58), radius=9, outline=(77, 88, 99))
text('Esc', left + key_width / 2, 513, 23, (221, 225, 228))
text(label, left + key_width + gap + label_width / 2, 513, 27, (180, 191, 199))
image.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS).save(OUTPUT / 'pause-card.png')
# Скошенные углы не требуют прозрачности, которую BFGE не выводит из PNG.
points = [(12, 0), (948, 0), (960, 12), (960, 668),
          (948, 680), (12, 680), (0, 668), (0, 12)]
lines = ['mtllib pause-card.mtl', 'usemtl pause_card']
for x, y in points:
    lines.append(f'v {(x / WIDTH - 0.5) * 0.19:.7f} {(0.5 - y / HEIGHT) * 0.1345833:.7f} 0')
for x, y in points:
    lines.append(f'vt {x / WIDTH:.7f} {1 - y / HEIGHT:.7f}')
lines.append('f ' + ' '.join(f'{i}/{i}' for i in range(8, 0, -1)))
(OUTPUT / 'pause-card.obj').write_text('\n'.join(lines) + '\n')
(OUTPUT / 'pause-card.mtl').write_text('newmtl pause_card\nKd 1 1 1\nmap_Kd pause-card.png\n')
