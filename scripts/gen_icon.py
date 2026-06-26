"""生成 DevLauncher 应用图标 (1024x1024 PNG，透明背景)。

图标设计：品牌蓝绿渐变圆角方底 + 白色火箭剪影，呼应 logo-dot 渐变与
"Launcher（发射器）" 寓意。仅依赖 Pillow，无需外部图片资源。
"""

from PIL import Image, ImageDraw
import math

SIZE = 1024
# 品牌色（与 TitleBar .logo-dot 渐变一致）
BLUE = (24, 144, 255, 255)    # #1890ff
GREEN = (82, 196, 26, 255)    # #52c41a

img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
px = img.load()

RADIUS = int(SIZE * 0.22)


def lerp(a, b, t):
    return a + (b - a) * t


def lerp_color(c1, c2, t):
    return tuple(int(round(lerp(c1[i], c2[i], t))) for i in range(len(c1)))


# ===== 1. 对角渐变圆角方底 =====
for y in range(SIZE):
    for x in range(SIZE):
        t = (x + y) / (2 * (SIZE - 1))
        px[x, y] = lerp_color(BLUE, GREEN, t)

mask = Image.new("L", (SIZE, SIZE), 0)
md = ImageDraw.Draw(mask)
md.rounded_rectangle([0, 0, SIZE - 1, SIZE - 1], radius=RADIUS, fill=255)
bg = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
bg.paste(img, (0, 0), mask)
img = bg

draw = ImageDraw.Draw(img)
cx = SIZE // 2
WHITE = (255, 255, 255, 255)

# ===== 2. 火箭几何（修正比例：修长挺拔） =====
# 整体纵向贯穿画布 ~0.14 → 0.90
nose_tip = int(SIZE * 0.135)      # 鼻尖
nose_base = int(SIZE * 0.345)     # 鼻锥底（= 机身顶）
body_bottom = int(SIZE * 0.715)   # 机身底
fin_bottom = int(SIZE * 0.80)     # 尾翼底
flame_top = fin_bottom + int(SIZE * 0.005)
flame_bottom = int(SIZE * 0.905)

hw = int(SIZE * 0.115)            # 机身半宽（更窄 → 修长）
nose_base_hw = hw                 # 鼻锥底与机身等宽，无肩部

# --- 鼻锥（细长三角，挺拔） ---
draw.polygon(
    [(cx, nose_tip), (cx - nose_base_hw, nose_base), (cx + nose_base_hw, nose_base)],
    fill=WHITE,
)

# --- 机身（圆角矩形，偏圆头，修长） ---
draw.rounded_rectangle(
    [cx - hw, nose_base, cx + hw, body_bottom],
    radius=hw,
    fill=WHITE,
)

# --- 尾翼（加大，向斜下张开，对称） ---
fin_top_y = body_bottom - int(SIZE * 0.135)
fin_out_hw = int(SIZE * 0.30)
# 左翼（四边形：机身腰→外下角→内下角→机身底）
draw.polygon(
    [
        (cx - hw, fin_top_y),          # 机身上侧
        (cx - fin_out_hw, fin_bottom), # 外下尖
        (cx - hw, fin_bottom),         # 内下角
    ],
    fill=WHITE,
)
# 右翼
draw.polygon(
    [
        (cx + hw, fin_top_y),
        (cx + fin_out_hw, fin_bottom),
        (cx + hw, fin_bottom),
    ],
    fill=WHITE,
)

# ===== 3. 舷窗（位置上移、加白描边、用深蓝增强对比） =====
win_cy = int(SIZE * 0.405)
win_r = int(SIZE * 0.078)
# 白色描边圈（比窗大一圈）
ring_pad = int(SIZE * 0.012)
draw.ellipse(
    [cx - win_r - ring_pad, win_cy - win_r - ring_pad,
     cx + win_r + ring_pad, win_cy + win_r + ring_pad],
    fill=WHITE,
)
# 窗：深蓝（比渐变更深更稳，玻璃质感不与背景混淆）
WIN = (13, 92, 170, 255)  # 较深的品牌蓝
draw.ellipse(
    [cx - win_r, win_cy - win_r, cx + win_r, win_cy + win_r],
    fill=WIN,
)
# 高光：左上小白点，增加立体感
hi_r = int(win_r * 0.32)
hi_cx = cx - int(win_r * 0.35)
hi_cy = win_cy - int(win_r * 0.35)
draw.ellipse(
    [hi_cx - hi_r, hi_cy - hi_r, hi_cx + hi_r, hi_cy + hi_r],
    fill=(200, 225, 255, 255),
)

# ===== 4. 底部喷射火焰（加大、两段，有张力） =====
# 外焰（橙）
flame_hw = int(SIZE * 0.085)
draw.polygon(
    [(cx - flame_hw, flame_top), (cx, flame_bottom), (cx + flame_hw, flame_top)],
    fill=(255, 170, 24, 255),
)
# 内焰（亮黄）
flame_hw2 = int(flame_hw * 0.52)
flame_bottom2 = flame_top + int((flame_bottom - flame_top) * 0.68)
draw.polygon(
    [(cx - flame_hw2, flame_top), (cx, flame_bottom2), (cx + flame_hw2, flame_top)],
    fill=(255, 228, 41, 255),
)

out = "D:/工作地带/开发机项目管理器/app-icon.png"
img.save(out)
print("saved:", out)
