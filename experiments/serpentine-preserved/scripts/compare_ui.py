"""Compare real baseline/patched screenshots captured on the same CI host."""
import argparse
import json
from pathlib import Path
import numpy as np
from PySide6.QtGui import QImage


def pixels(path):
    image = QImage(str(path)).convertToFormat(QImage.Format.Format_RGBA8888)
    if image.isNull():
        raise ValueError(f'Cannot read screenshot: {path}')
    data = np.frombuffer(image.constBits(), np.uint8).reshape(image.height(), image.bytesPerLine())
    return data[:, :image.width() * 4].reshape(image.height(), image.width(), 4).copy()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    a, b = pixels(args.before), pixels(args.after)
    if a.shape != b.shape:
        raise AssertionError(f'UI dimensions changed: {a.shape} -> {b.shape}')
    delta = np.abs(a[:, :, :3].astype(np.int16) - b[:, :, :3].astype(np.int16))
    fraction = float(np.mean(np.any(delta != 0, axis=2)))
    report = {'shape': list(a.shape), 'exact_pixel_match': bool(np.array_equal(a, b)),
              'changed_pixel_fraction': fraction, 'mean_channel_delta': float(delta.mean()),
              'max_channel_delta': int(delta.max()),
              'acceptable': fraction <= .001 and float(delta.mean()) <= .05,
              'note': 'Same-host software OpenGL. Tolerance allows at most 0.1% transient UI pixels; source UI identity is checked separately.'}
    args.output.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report, indent=2))
    if not report['acceptable']:
        raise AssertionError('Original UI screenshot comparison failed')


if __name__ == '__main__':
    main()
