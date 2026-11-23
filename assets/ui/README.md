# Карточка паузы

`pause-card.obj` — плоская карточка со скошенными углами, `pause-card.mtl`
связывает её с `pause-card.png`. Фон непрозрачный: текущий шейдер BFGE
не использует прозрачность PNG. Мир остаётся видимым вокруг карточки.

Текст и оформление создаёт `tools/generate_pause_card.py`. Для повторного
создания на Ubuntu нужны Python 3, Pillow и DejaVu Sans:

```sh
/usr/bin/python3 tools/generate_pause_card.py
```

Скрипт использует системный файл
`/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf`; для другой системы путь
можно изменить в константе `FONT`. Для сборки и запуска Amber Python,
Pillow и установленный шрифт не требуются: готовые ассеты хранятся в репозитории.

Шрифт DejaVu Sans основан на Bitstream Vera; изменения DejaVu находятся
в общественном достоянии. Сам файл шрифта не поставляется. Условия
использования шрифта сохранены в `DejaVu-LICENSE.txt`.
