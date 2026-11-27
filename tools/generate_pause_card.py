#!/usr/bin/env python3
"""Пересоздание карточки: Python 3, Pillow, шрифт DejaVu Sans."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

OUTPUT = Path(__file__).resolve().parents[1] / 'assets/ui'
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
SCALE = 2
WIDTH, HEIGHT = 960, 540
EXIT_BOUNDS = (180, 375, 780, 445)
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
box((18, 18, 942, 522), None, radius=14, outline=(76, 72, 63))
box((444, 0, 516, 4), gold)
text('Пауза', 480, 125, 72, (236, 230, 217))
text('Можно немного выдохнуть', 480, 195, 25, (137, 150, 161))
box((100, 250, 860, 251), (52, 62, 72))
label = '— продолжить'
label_width = ImageFont.truetype(FONT, 27 * SCALE).getlength(label) / SCALE
key_width, gap = 90, 18
left = (WIDTH - key_width - gap - label_width) / 2
box((left, 290, left + key_width, 340), (36, 46, 58), radius=9, outline=(77, 88, 99))
text('Esc', left + key_width / 2, 314, 23, (221, 225, 228))
text(label, left + key_width + gap + label_width / 2, 314, 27, (180, 191, 199))
for hovered in [False, True]:
    box(EXIT_BOUNDS, (65, 52, 38) if hovered else (35, 42, 51), radius=12,
        outline=gold if hovered else (78, 85, 93))
    text('Выйти из игры', 480, 409, 28, (238, 220, 191) if hovered else (199, 205, 209))
    suffix = '-hover' if hovered else ''
    image.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS).save(OUTPUT / f'pause-card{suffix}.png')
# Контур карточки задаётся геометрией, без прозрачных полей текстуры.
points = [(12, 0), (WIDTH-12, 0), (WIDTH, 12), (WIDTH, HEIGHT-12),
          (WIDTH-12, HEIGHT), (12, HEIGHT), (0, HEIGHT-12), (0, 12)]
for suffix in ['', '-hover']:
    lines = [f'mtllib pause-card{suffix}.mtl', 'usemtl pause_card']
    for x, y in points:
        lines.append(f'v {(x / WIDTH - 0.5) * 0.19:.7f} {(0.5 - y / HEIGHT) * 0.19 * HEIGHT / WIDTH:.7f} 0')
    for x, y in points:
        lines.append(f'vt {x / WIDTH:.7f} {1 - y / HEIGHT:.7f}')
    lines.append('f ' + ' '.join(f'{i}/{i}' for i in range(8, 0, -1)))
    (OUTPUT / f'pause-card{suffix}.obj').write_text('\n'.join(lines) + '\n')
    (OUTPUT / f'pause-card{suffix}.mtl').write_text(f'newmtl pause_card\nKd 1 1 1\nmap_Kd pause-card{suffix}.png\n')
# Область клика и размеры берутся из того же макета, что и текстура.
OUTPUT.parents[1].joinpath('src/pause_layout.rs').write_text(
    '// Создано tools/generate_pause_card.py; изменять исходный макет в скрипте.\n'
    'pub const CARD_WIDTH: f32 = 0.19;\n'
    f'pub const CARD_HEIGHT: f32 = 0.19 * {HEIGHT}.0 / {WIDTH}.0;\n'
    f'pub const EXIT_BOUNDS: [f32; 4] = [{EXIT_BOUNDS[0]}.0 / {WIDTH}.0, {EXIT_BOUNDS[1]}.0 / {HEIGHT}.0, {EXIT_BOUNDS[2]}.0 / {WIDTH}.0, {EXIT_BOUNDS[3]}.0 / {HEIGHT}.0];\n')
