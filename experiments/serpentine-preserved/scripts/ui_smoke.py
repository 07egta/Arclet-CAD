#!/usr/bin/env python3
"""Exercise the real unchanged Qt window under Xvfb and capture evidence."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import tempfile
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='serpentine-ui-check-') as config:
        os.environ['SERP3D_CONFIG'] = str(Path(config) / 'settings.json')
        os.environ['SERP3D_JOURNAL_DIR'] = str(Path(config) / 'journals')
        os.environ['SERP3D_AUTOSAVE_DIR'] = str(Path(config) / 'autosave')
        os.environ['SERP3D_NO_RPC'] = '1'
        Path(os.environ['SERP3D_CONFIG']).write_text(json.dumps({'autosave_interval_sec': 0}))
        from serpentine3d.utils.glsetup import set_default_gl_format, match_pyopengl_to_qt
        set_default_gl_format()
        from PySide6.QtWidgets import QApplication
        from PySide6.QtCore import Qt, QPoint
        from PySide6.QtTest import QTest
        app = QApplication([])
        match_pyopengl_to_qt()
        from serpentine3d.ui import theme
        app.setStyleSheet(theme.QSS)
        theme.apply_saved_interface_font(app)
        from serpentine3d.app import MainWindow
        from serpentine3d.core import geometry as g
        from serpentine3d.core.scene import Scene
        from serpentine3d import fileio
        import numpy as np
        window = MainWindow()
        window.resize(1440, 900)
        checks = []
        def check(name, value):
            checks.append({'name': name, 'passed': bool(value)})
            if not value:
                raise AssertionError(name)
        def settle():
            for _ in range(35):
                app.processEvents()
                QTest.qWait(20)
        try:
            plate = g.make_box((-30, -18, 0), 60, 36, 8)
            for x in (-18., 18.):
                plate = g.boolean_difference(plate, g.make_cylinder((x, 0, -1), 4, 10))
            part = window.scene.add(plate, name='Precision plate')
            ring = window.scene.add(g.make_torus((55, 0, 10), 14, 3), name='Torus')
            profiles = [g.make_circle((0, 55, z), radius) for z, radius in ((0, 12), (15, 19), (35, 8))]
            loft = window.scene.add(g.loft(profiles, solid=True), name='Loft')
            window.scene.update(ring.id, color=(0.8, 0.55, 0.28))
            window.scene.update(loft.id, color=(0.45, 0.6, 0.82))
            window.set_view_layout('quad')
            window.show()
            settle()
            for vp in window.all_viewports():
                vp.zoom_extents()
            settle()
            check('four original viewports', len(window.all_viewports()) == 4)
            check('all viewports have OpenGL contexts', all(v.isValid() for v in window.all_viewports()))
            check('three exact solids', all(g.volume(o.shape) > 0 for o in window.scene.all()))
            window.command_line.input.clearFocus()
            settle()
            check('screenshot saved', window.grab().save(str(output / 'original-ui-quad.png')))
            # Real pointer selection in the original top viewport, not a mock.
            top = next(vp for vp in window.all_viewports() if vp.view_key == 'top')
            top.zoom_extents()
            settle()
            # Project a point well inside the plate and away from both holes.
            xy = top.camera.project(np.array([[0., 10., 8.]]), top.width(), top.height())[0]
            QTest.mouseClick(top, Qt.MouseButton.LeftButton, Qt.KeyboardModifier.NoModifier, QPoint(int(xy[0]), int(xy[1])))
            settle()
            check('pointer selects plate', part.id in window.selection.ids)
            before = window.scene.get(part.id).bbox()
            window.history.checkpoint('translate evidence')
            window.scene.replace_shape(part.id, g.translate(plate, (5., 0., 0.)))
            settle()
            check('translated bounds', np.allclose(np.array(window.scene.get(part.id).bbox()) - np.array(before), [5, 0, 0], atol=1e-5))
            window.history.undo()
            settle()
            check('undo restores bounds', np.allclose(window.scene.get(part.id).bbox(), before))
            window.history.redo()
            settle()
            check('redo restores moved bounds', np.allclose(np.array(window.scene.get(part.id).bbox()) - np.array(before), [5, 0, 0], atol=1e-5))
            for ext in ('serp', 'step', 'stl', 'glb'):
                path = output / ('showcase.' + ext)
                fileio.export_file(window.scene, str(path))
                check('export ' + ext, path.stat().st_size > 0)
            restored = Scene()
            fileio.import_file(restored, str(output / 'showcase.serp'), replace=True)
            check('native save round trip', len(restored.all()) == 3)
            check('round trip exact volumes', all(abs(g.volume(a.shape) - g.volume(b.shape)) < 1e-6 for a, b in zip(window.scene.all(), restored.all())))
            window.mark_saved()
            report = {'checks': checks, 'passed': sum(c['passed'] for c in checks), 'total': len(checks),
                      'renderer': 'Xvfb/CI software OpenGL; not Intel GPU certification',
                      'ui_source': 'Unmodified pinned Serpentine3D Qt implementation'}
            (output / 'ui-checks.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
            print(json.dumps(report, indent=2))
        finally:
            window.mark_saved()
            window.close()
            app.processEvents()


if __name__ == '__main__':
    main()
