import sys; sys.path.insert(0,'temp/pylib')
from PIL import Image, ImageDraw
VOID=(0x15,0x0A,0x2B); PANEL=(0x22,0x10,0x46)
fg=Image.open('android/app/src/main/res/drawable-nodpi/ic_launcher_foreground.png').convert('RGBA')
bee=fg.crop(fg.getchannel('A').getbbox())
bee=bee.resize((59,68),Image.NEAREST)  # recovered native pixel grid
def up(im,k): return im.resize((im.width*k,im.height*k),Image.NEAREST)
HEX=[(27,20),(54,20),(81,20),(40.5,44),(67.5,44),(27,68),(54,68),(81,68)]
def comb(w,h,s,ox=0,oy=0):
    im=Image.new('RGBA',(w,h),VOID+(255,)); d=ImageDraw.Draw(im)
    for cx,cy in HEX:
        p=[(cx,cy+0),(cx+9,cy+5),(cx+9,cy+15),(cx,cy+20),(cx-9,cy+15),(cx-9,cy+5)]
        d.polygon([(ox+x*s,oy+y*s) for x,y in p],fill=PANEL)
    return im
# icon: same composition as adaptive pair (108dp -> 512px)
ic=comb(512,512,512/108); b=up(bee,6)
ic.alpha_composite(b,((512-b.width)//2,(512-b.height)//2)); ic.save('docs/store/icon-512.png')
# feature graphic
fgi=comb(1024,500,500/108*1.0,ox=262,oy=0)
logo=Image.open('docs/logo.png').convert('RGBA')
print(logo.getpixel((0,0)))
gb=logo.crop((50,480,275,550)); em=logo.crop((315,480,740,550))
gb=up(gb,2); em=em.crop(em.getbbox())
fgi.alpha_composite(up(bee,5),(90,80))
tx=490
fgi.alpha_composite(gb.crop(gb.getbbox()) if gb.getbbox() else gb,(tx,120))
fgi.alpha_composite(em,(tx,120+gb.getbbox()[3]-gb.getbbox()[1]+28))
fgi.convert('RGB').save('docs/store/feature-graphic.png')
