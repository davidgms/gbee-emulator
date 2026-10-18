# Run from repo root: prlimit --as=4294967296 python3 -I docs/store/shots.py
import sys, math, random
sys.path.insert(0, 'temp/pylib')
from PIL import Image, ImageDraw, ImageFilter, ImageFont

L_ = 'temp/listing/'
OUT = 'docs/store/screenshots/'
S = 6; W, H = 1080, 1920; LW, LH = W // S, H // S
def hx(s): return tuple(int(s[i:i+2], 16) for i in (1, 3, 5))
VOID, PANEL, EDGE = hx('#150A2B'), hx('#221046'), hx('#45268A')
GOLD, MIST, WING, INK = hx('#FACC15'), hx('#FFF9C2'), hx('#FDEEB7'), hx('#1A0F00')
CYAN, PINK, GREEN, MAGENTA, VIOLET = hx('#55F6FD'), hx('#FCA8CE'), hx('#7CE04A'), hx('#650EBA'), hx('#7C30BC')
def mix(a, b, t): return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))

def pad(im, p):
    o = Image.new('RGBA', (im.width + 2 * p, im.height + 2 * p), (0, 0, 0, 0)); o.alpha_composite(im, (p, p)); return o
def glow(im, col, rad, strength):
    a = im.split()[3].filter(ImageFilter.MaxFilter(5)).filter(ImageFilter.GaussianBlur(rad))
    a = a.point(lambda v: int(v * strength)); g = Image.new('RGBA', im.size, col + (0,)); g.putalpha(a); return g
def put(base, im, cx, cy): base.alpha_composite(im, (int(cx - im.width / 2), int(cy - im.height / 2)))

bee = Image.open('android/app/src/main/res/drawable-nodpi/loading_bee.png').convert('RGBA')
face = Image.open('docs/design/gb-face.png').convert('RGBA'); face = face.crop(face.getbbox())
face = face.resize((face.width // 8, face.height // 8), Image.NEAREST)
face = face.resize((face.width * 3, face.height * 3), Image.NEAREST)   # 8px grid -> 3px, integer

def background(acc, seed):
    random.seed(seed)
    L = Image.new('RGB', (LW, LH), VOID); px = L.load()
    cx, cy = LW / 2, LH * 0.55
    for y in range(LH):
        for x in range(LW):
            g = max(0, 1 - math.hypot((x - cx) / 1.0, (y - cy) / 1.3) / 120); g = math.floor(g * 5) / 5
            px[x, y] = mix(VOID, acc, g * 0.22)
    d = ImageDraw.Draw(L)
    R = 9; hw = R * math.sqrt(3)
    for row in range(-1, int(LH / (R * 1.5)) + 2):
        for col in range(-1, int(LW / hw) + 2):
            x = col * hw + (hw / 2 if row % 2 else 0); y = row * R * 1.5
            pts = [(x + (R - 1) * math.cos(math.radians(60 * i + 30)), y + (R - 1) * math.sin(math.radians(60 * i + 30))) for i in range(6)]
            lit = (row * 7 + col * 3) % 6 == 0
            d.polygon(pts, outline=mix(VOID, EDGE, 0.55 if not lit else 0.9), fill=PANEL if lit else None)
    def star(x, y, r, c):
        for i in range(-r, r + 1): d.point((x + i, y), fill=c); d.point((x, y + i), fill=c)
        d.point((x, y), fill=WING)
    spots = [(8, 8, 3, acc), (165, 12, 2, GOLD), (20, 150, 2, GOLD), (168, 160, 3, acc), (90, 300, 2, GOLD),
             (10, 250, 3, acc), (170, 270, 2, PINK), (150, 60, 2, acc), (12, 60, 2, PINK), (172, 310, 2, GOLD)]
    for s in spots: star(*s)
    for _ in range(80):
        d.point((random.randrange(LW), random.randrange(LH)), fill=random.choice([EDGE, VIOLET, mix(VOID, acc, .5)]))
    return L.resize((W, H), Image.NEAREST).convert('RGBA')

def bee_at(img, w, cx, cy, flip=False):
    # The bee art is already pixel art: scale it once and keep its own pixels.
    b = bee.resize((w * S, round(w * S * 220 / 240)), Image.LANCZOS)
    if flip: b = b.transpose(Image.FLIP_LEFT_RIGHT)
    put(img, glow(pad(b, 20), GOLD, 10, 0.5), cx, cy); put(img, b, cx, cy)

def fit_font(path, text, sizes, maxw):
    for s in sizes:
        f = ImageFont.truetype(path, s)
        if f.getlength(text) <= maxw: return f
    return f

def trophy(img, cx, cy, k=6):
    P = ['0111111111110', '1111111111111', '1011111111101', '1011111111101', '1011111111101', '0111111111110',
         '0011111111100', '0001111111000', '0000011100000', '0000011100000', '0000111110000', '0001111111000', '0001111111000']
    for r, row in enumerate(P):
        for c, v in enumerate(row):
            if v == '1':
                col = GOLD if (c + r) % 5 else WING
                ImageDraw.Draw(img).rectangle((cx + (c - 6.5) * k, cy + (r - 6.5) * k, cx + (c - 5.5) * k - 1, cy + (r - 5.5) * k - 1), fill=col)

def header(img, head, sub):
    d = ImageDraw.Draw(img)
    f = fit_font('android/app/src/main/res/font/press_start_2p_regular.ttf', head, (56, 48, 40, 32), 900)
    w = f.getlength(head); y = 150
    sh = Image.new('RGBA', (W, 200), (0, 0, 0, 0)); ImageDraw.Draw(sh).text((W / 2, 100), head, font=f, fill=GOLD + (255,), anchor='mm')
    img.alpha_composite(glow(sh, GOLD, 18, .6), (0, y - 100))
    d.text((W / 2 + 5, y + 5), head, font=f, fill=INK, anchor='mm')
    d.text((W / 2, y), head, font=f, fill=GOLD, anchor='mm')
    fs = fit_font('android/app/src/main/res/font/pixelify_sans_bold.ttf', sub, (44, 40, 36, 32, 30), 980)
    d.text((W / 2, 250), sub, font=fs, fill=MIST, anchor='mm')

def card(img, shot, acc, top=340, bottom=1850, maxw=900):
    sc = min(maxw / shot.width, (bottom - top) / shot.height)
    sh = shot.resize((int(shot.width * sc), int(shot.height * sc)), Image.LANCZOS).convert('RGBA')
    m = Image.new('L', sh.size, 0); ImageDraw.Draw(m).rounded_rectangle((0, 0, sh.width - 1, sh.height - 1), 40, fill=255)
    sh.putalpha(m)
    bd = 5; fr = Image.new('RGBA', (sh.width + 2 * bd, sh.height + 2 * bd), (0, 0, 0, 0))
    ImageDraw.Draw(fr).rounded_rectangle((0, 0, fr.width - 1, fr.height - 1), 44, fill=acc + (255,))
    cx, cy = W / 2, (top + bottom) / 2
    gl = pad(fr, 60)
    put(img, glow(gl, acc, 28, 0.9), cx, cy)
    put(img, fr, cx, cy); put(img, sh, cx, cy)
    return cx - sh.width / 2, cy - sh.height / 2, sh.width, sh.height

def decor(img, extra):
    for e in extra: e()
    put(img, glow(pad(face, 30), MAGENTA, 16, 1.0), 0, 0) if False else None

def open_(n): return Image.open(L_ + n + '.png').convert('RGB')
def finish(img, name): img.convert('RGB').save(OUT + name)

def corner_decor(img, acc, pos):
    # small GB face + bees in the margins beside the header, away from the screenshot
    gx, gy = pos
    f = face.resize((face.width * 2 // 3 // 3 * 3 or face.width, face.height * 2 // 3 // 3 * 3 or face.height), Image.NEAREST) if False else face
    put(img, glow(pad(f, 30), acc, 14, 0.9), gx, gy); put(img, f, gx, gy)

# 1 gameplay: bar+game and controls joined (black between them), cropped for a bigger game
g = open_('s-b6'); top = g.crop((0, 60, 1080, 900)); bot = g.crop((0, 1500, 1080, 2400))
s1 = Image.new('RGB', (1080, top.height + bot.height)); s1.paste(top, (0, 0)); s1.paste(bot, (0, top.height))
# 5 filters
def labelled(name, label, acc):
    c = open_(name).crop((160, 430, 640, 580)).resize((960, 300), Image.NEAREST)
    f = ImageFont.truetype('android/app/src/main/res/font/press_start_2p_regular.ttf', 28)
    p = Image.new('RGB', (960, 300 + 70), PANEL); p.paste(c, (0, 70))
    ImageDraw.Draw(p).text((24, 35), label, font=f, fill=GOLD, anchor='lm'); return p
fl = [labelled('s-f-none', 'PIXEL PERFECT', GOLD), labelled('s-f-crt', 'CRT SCANLINES', GOLD), labelled('s-f-sai', '2xSaI SMOOTH', GOLD)]
s5 = Image.new('RGB', (960, 370 * 3 + 40 * 2), VOID)
for i, p in enumerate(fl): s5.paste(p, (0, i * 410))
# 6 settings
s6 = open_('s-set-ra').crop((0, 60, 1080, 1790))

specs = [
 ('1-play.png', 'PLAY YOUR GBA GAMES', 'Fast, accurate core, low-latency sound', s1, CYAN, 1),
 ('2-layout.png', 'MOVE ANY BUTTON', 'Drag and resize each control, portrait and landscape', open_('s-ed2').crop((0, 60, 1080, 2400)), PINK, 2),
 ('3-custom.png', 'BUILD YOUR OWN BUTTONS', 'Combo, sequence and toggle-hold', open_('s-cb3').crop((0, 60, 1080, 2400)), GREEN, 3),
 ('5-save.png', 'SAVE ANYWHERE', 'Quick save, 7 slots, and rewind', open_('s-ss4'), CYAN, 4),
 ('6-filters.png', 'YOUR LOOK', 'Filters, integer scaling and frame blending', s5, PINK, 5),
 ('7-achievements.png', 'EARN ACHIEVEMENTS', 'RetroAchievements built in', s6, GREEN, 6),
 ('4-themes.png', 'PICK YOUR COLOURS', 'Button colour themes, from honey to classic grey', None, GOLD, 7),
]
def themes(img, acc):
    lf = ImageFont.truetype('android/app/src/main/res/font/press_start_2p_regular.ttf', 30)
    def cardimg(nm, sc, ang):
        g = open_(nm); sh = Image.new('RGB', (1080, 840 + 900)); sh.paste(g.crop((0, 60, 1080, 900)), (0, 0)); sh.paste(g.crop((0, 1500, 1080, 2400)), (0, 840))
        sh = sh.resize((int(1080 * sc), int(1740 * sc)), Image.LANCZOS).convert('RGBA')
        m = Image.new('L', sh.size, 0); ImageDraw.Draw(m).rounded_rectangle((0, 0, sh.width - 1, sh.height - 1), 34, fill=255); sh.putalpha(m)
        fr = Image.new('RGBA', (sh.width + 8, sh.height + 8), (0, 0, 0, 0))
        ImageDraw.Draw(fr).rounded_rectangle((0, 0, fr.width - 1, fr.height - 1), 38, fill=acc + (255,))
        fr.alpha_composite(sh, (4, 4))
        if ang: fr = fr.rotate(ang, Image.BICUBIC, expand=True)
        return fr
    for nm, lab, sc, ang, cx, cy in (('s-tint-honey', 'HONEY', .37, 5, 225, 1160), ('s-tint-classic', 'CLASSIC', .37, -5, 855, 1160),
                                     ('s-tint-violet', 'VIOLET', .50, 0, 540, 1090)):
        fr = cardimg(nm, sc, ang)
        put(img, glow(pad(fr, 50), acc, 24, .9), cx, cy); put(img, fr, cx, cy)
        ImageDraw.Draw(img).text((cx if lab == 'VIOLET' else (165 if lab == 'HONEY' else 915), 1640), lab, font=lf, fill=GOLD, anchor='mm')

for name, head, sub, shot, acc, seed in specs:
    img = background(acc, seed)
    header(img, head, sub)
    top = 340
    if seed == 7: themes(img, acc); finish(img, name); continue
    x, y, w, h = card(img, shot, acc, top=top, bottom=1860, maxw=920 if seed != 5 else 960)
    # decoration in the margins around the header, clear of the UI
    right = seed % 2 == 0
    bee_at(img, 14, 80 if right else 1000, 40, right)
    bee_at(img, 11, 1000 if right else 80, 310, not right)
    if seed == 6: trophy(img, 80, 150, 5); trophy(img, 1000, 150, 5)
    else:
        put(img, glow(pad(face, 20), acc, 10, .8), 540, 1880) if False else None
    finish(img, name)
