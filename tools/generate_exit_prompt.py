#!/usr/bin/env python3
"""Пересоздание подсказки выхода: Python 3, Pillow и DejaVu Sans."""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

output = Path(__file__).resolve().parents[1] / 'assets/ui/exit-prompt.png'
font_path = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
image = Image.new('RGB', (640, 112), (19, 25, 34))
draw = ImageDraw.Draw(image)
draw.rectangle((1, 1, 638, 110), outline=(155, 113, 52), width=2)
draw.rectangle((39, 24, 104, 87), fill=(52, 42, 31), outline=(214, 163, 84), width=2)
draw.text((72, 55), 'E', font=ImageFont.truetype(font_path, 39),
          fill=(255, 218, 150), anchor='mm')
draw.text((133, 55), 'Перейти в следующую сцену',
          font=ImageFont.truetype(font_path, 30), fill=(245, 239, 227), anchor='lm')
image.save(output)
