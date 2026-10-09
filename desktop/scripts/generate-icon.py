from pathlib import Path
import struct
import zlib

root = Path(__file__).resolve().parent.parent
size = 32
rows = bytearray()
for y in range(size):
    rows.append(0)
    for x in range(size):
        arrow = (12 <= x <= 19 and 11 <= y <= 23) or (5 <= y <= 13 and abs(x - 15.5) <= y - 4)
        rows.extend((245, 247, 250, 255) if arrow else (30, 75, 140, 255))

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))

png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0))
png += chunk(b'IDAT', zlib.compress(bytes(rows))) + chunk(b'IEND', b'')
(root / 'src-tauri/icons/icon.png').write_bytes(png)
