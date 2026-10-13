"""Generate the README cover using the supplied component-library screenshot."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent
FONT_DIR = Path('C:/Windows/Fonts')


def font(size, bold=False):
    name = 'msyhbd.ttc' if bold else 'msyh.ttc'
    return ImageFont.truetype(str(FONT_DIR / name), size)


canvas = Image.new('RGBA', (1600, 900), '#eef3fa')
glow = Image.new('RGBA', canvas.size)
draw = ImageDraw.Draw(glow)
draw.ellipse((1050, -300, 1950, 600), fill=(112, 162, 255, 55))
draw.ellipse((-400, 500, 650, 1400), fill=(123, 199, 190, 40))
canvas = Image.alpha_composite(canvas, glow.filter(ImageFilter.GaussianBlur(110)))
draw = ImageDraw.Draw(canvas)
draw.text((90, 150), 'Lark', font=font(108, True), fill='#22344b')
draw.rounded_rectangle((94, 310, 156, 316), 3, fill='#4f7df3')
draw.text((90, 352), '用一个快捷键，', font=font(48, True), fill='#22344b')
draw.text((90, 426), '连接你的桌面工作。', font=font(48, True), fill='#22344b')
draw.text((94, 532), '应用启动 · 文件搜索', font=font(29), fill='#60718a')
draw.text((94, 582), '剪贴板历史 · 插件工具', font=font(29), fill='#60718a')
draw.text((94, 756), 'TAURI  /  RUST  /  REACT', font=font(20), fill='#74849a')

screenshot = Image.open(ROOT / '组件库.png').convert('RGBA')
# Remove the black capture corners, preserving the visible product content.
mask = Image.new('L', screenshot.size)
ImageDraw.Draw(mask).rounded_rectangle(
    (0, 0, screenshot.width - 1, screenshot.height - 1), 17, fill=255
)
screenshot.putalpha(mask)
x, y = 792, 150
shadow = Image.new('RGBA', canvas.size)
ImageDraw.Draw(shadow).rounded_rectangle(
    (x + 4, y + 18, x + screenshot.width - 4, y + screenshot.height + 18),
    24, fill=(40, 65, 100, 45),
)
canvas = Image.alpha_composite(canvas, shadow.filter(ImageFilter.GaussianBlur(24)))
canvas.alpha_composite(screenshot, (x, y))
canvas.convert('RGB').save(ROOT / 'cover.png', optimize=True)
