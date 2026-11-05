#!/usr/bin/env python3
"""Generate the gallery's original, offline PNG texture using only the standard library."""
import math
from pathlib import Path
import struct
import zlib

size = 256
pixels = bytearray()
for y in range(size):
    pixels.append(0)
    for x in range(size):
        checker = (x // 32 + y // 32) % 2
        wave = .5 + .5 * math.sin(x / 22 + y / 35)
        pixels.extend((int(35 + 65 * wave), 110 + checker * 65, 170 + checker * 55, 255))

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))

image = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0))
image += chunk(b'IDAT', zlib.compress(bytes(pixels), 9)) + chunk(b'IEND', b'')
(Path(__file__).resolve().parents[1] / 'demo/assets/texture.png').write_bytes(image)
