#!/usr/bin/env python3
"""Real keyboard regression: Undo, start Line, Escape, then Redo."""
import argparse
import json
import os
from pathlib import Path
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='serpentine-history-') as folder:
        root = Path(folder)
        os.environ['SERP3D_CONFIG'] = str(root / 'settings.json')
        os.environ['SERP3D_JOURNAL_DIR'] = str(root / 'journals')
        os.environ['SERP3D_AUTOSAVE_DIR'] = str(root / 'autosave')
        os.environ['SERP3D_NO_RPC'] = '1'
        (root / 'settings.json').write_text('{"autosave_interval_sec": 0}')
        from serpentine3d.utils.glsetup import set_default_gl_format, match_pyopengl_to_qt
        set_default_gl_format()
        from PySide6.QtWidgets import QApplication
        from PySide6.QtCore import Qt
        from PySide6.QtTest import QTest
        app = QApplication([])
        match_pyopengl_to_qt()
        from serpentine3d.app import MainWindow
        window = MainWindow()
        window.show()
        checks = []
        def settle():
            for _ in range(10):
                app.processEvents()
                QTest.qWait(20)
        def check(name, result):
            checks.append({'name': name, 'passed': bool(result)})
            if not result:
                raise AssertionError(name)
        def submit(text):
            prompt = window.command_line.input
            prompt.setFocus()
            prompt.clear()
            QTest.keyClicks(prompt, text)
            QTest.keyClick(prompt, Qt.Key.Key_Return)
            settle()
        try:
            settle()
            submit('line')
            submit('0,0,0')
            submit('10,0,0')
            check('keyboard creates one line', len(window.scene.all()) == 1)
            submit('undo')
            check('keyboard Undo removes line', len(window.scene.all()) == 0)
            check('Redo available after Undo', window.history.can_redo)
            submit('line')
            check('Line awaits point', window.processor.busy)
            QTest.keyClick(window.command_line.input, Qt.Key.Key_Escape)
            settle()
            check('Escape cancels Line', not window.processor.busy)
            check('Escape preserves Redo', window.history.can_redo)
            submit('redo')
            check('keyboard Redo restores line', len(window.scene.all()) == 1)
            report = {'passed': len(checks), 'total': len(checks), 'checks': checks,
                      'input': 'QTest keyboard events through the unchanged command prompt'}
            (args.output / 'keyboard-history.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
            print(json.dumps(report, indent=2))
        finally:
            window.processor.cancel()
            window.mark_saved()
            window.close()
            app.processEvents()


if __name__ == '__main__':
    main()
