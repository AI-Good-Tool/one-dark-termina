#!/usr/bin/env python3
"""生成 1024x1024 应用图标：One Dark 底色圆角方块 + 终端提示符 >_"""
import zlib
import struct
import math
import sys

SIZE = 1024
BG = (40, 44, 52)       # #282c34
BLUE = (97, 175, 239)   # #61afef
FG = (171, 178, 191)    # #abb2bf

MARGIN = 96
RADIUS = 220

def dist_to_segment(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    if dx == 0 and dy == 0:
        return math.hypot(px - ax, py - ay)
    t = max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))

def pixel(x, y):
    # 圆角方形外框
    cx = min(max(x, MARGIN + RADIUS), SIZE - MARGIN - RADIUS)
    cy = min(max(y, MARGIN + RADIUS), SIZE - MARGIN - RADIUS)
    if math.hypot(x - cx, y - cy) > RADIUS:
        return (0, 0, 0, 0)
    # 提示符 ">"：两条线段，描边 64
    if dist_to_segment(x, y, 330, 330, 560, 512) <= 64:
        return (*BLUE, 255)
    if dist_to_segment(x, y, 560, 512, 330, 694) <= 64:
        return (*BLUE, 255)
    # 下划线 "_"
    if 610 <= x <= 850 and 636 <= y <= 724:
        return (*FG, 255)
    return (*BG, 255)

def main(out_path):
    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)  # filter: none
        for x in range(SIZE):
            raw.extend(pixel(x, y))

    def chunk(tag, data):
        c = tag + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 6))
    png += chunk(b"IEND", b"")

    with open(out_path, "wb") as f:
        f.write(png)
    print(f"written: {out_path}")

if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "app-icon.png")
