"""Cancelling a command is not a new history branch or lost undo capacity."""
import gc
import weakref
from unittest import mock

import pytest
from serpentine3d.core import history as history_mod
from serpentine3d.core.history import History


class Snapshot(dict):
    pass


class Model:
    def __init__(self):
        self.value = 0
        self.fail_snapshot = False
    def snapshot(self):
        if self.fail_snapshot:
            raise MemoryError('snapshot refused')
        return Snapshot(value=self.value)
    def restore(self, snapshot):
        self.value = snapshot['value']


def edit(history, value):
    history.checkpoint(str(value))
    history.scene.value = value


def seeded():
    model = Model()
    history = History(model)
    edit(history, 1)
    edit(history, 2)
    history.undo()
    return model, history


def test_cancelled_checkpoint_restores_the_redo_branch():
    model, history = seeded()
    history.checkpoint('cancelled')
    assert not history.can_redo
    history.discard_checkpoint()
    assert history.can_redo
    assert history.redo() == '2'
    assert model.value == 2


def test_cancel_at_history_capacity_does_not_drop_the_oldest_edit(monkeypatch):
    monkeypatch.setattr(history_mod, 'MAX_UNDO', 3)
    model = Model()
    history = History(model)
    for value in (1, 2, 3):
        edit(history, value)
    history.checkpoint('cancelled')
    history.discard_checkpoint()
    assert [history.undo() for _ in range(3)] == ['3', '2', '1']
    assert model.value == 0
    assert not history.can_undo


def test_repeated_cancel_keeps_redo_and_never_grows_history():
    model, history = seeded()
    for _ in range(200):
        history.checkpoint('cancel')
        history.discard_checkpoint()
        assert len(history._undo) == 1
        assert len(history._redo) == 1
    history.redo()
    assert model.value == 2


def test_pending_checkpoint_does_not_allow_invalid_redo():
    model, history = seeded()
    history.checkpoint('new command')
    model.value = 9
    assert not history.can_redo
    assert history.redo() is None
    assert model.value == 9


def test_undo_redo_after_a_new_edit_follows_the_new_branch():
    model, history = seeded()
    edit(history, 9)
    history.undo()
    assert model.value == 1
    assert history.redo() == '9'
    assert model.value == 9
    assert not history.can_redo


def test_old_future_is_released_when_the_next_checkpoint_starts():
    model, history = seeded()
    old_future = weakref.ref(history._redo[0][1])
    edit(history, 9)
    history.checkpoint('next')
    gc.collect()
    assert old_future() is None


def test_checkpoint_snapshot_failure_has_no_journal_or_history_side_effects():
    model, history = seeded()
    undo, redo = list(history._undo), list(history._redo)
    calls = []
    history.on_checkpoint = calls.append
    model.fail_snapshot = True
    with pytest.raises(MemoryError):
        history.checkpoint('failed')
    assert history._undo == undo and history._redo == redo
    assert calls == []
    assert history.can_redo


@pytest.mark.parametrize('operation', ['undo', 'redo'])
def test_failed_live_snapshot_does_not_remove_a_history_entry(operation):
    model, history = seeded()
    undo, redo = list(history._undo), list(history._redo)
    model.fail_snapshot = True
    with pytest.raises(MemoryError):
        getattr(history, operation)()
    assert history._undo == undo and history._redo == redo
    assert model.value == 1


def test_journal_callbacks_keep_their_labels_and_count():
    model, history = seeded()
    calls = []
    history.on_checkpoint = lambda label: calls.append(('checkpoint', label))
    history.on_discard = lambda: calls.append(('discard', None))
    history.checkpoint('cancel')
    history.discard_checkpoint()
    assert calls == [('checkpoint', 'cancel'), ('discard', None)]


def test_suspended_future_remains_visible_to_script_identity_migration():
    model, history = seeded()
    history.checkpoint('pending')
    # The unchanged script editor walks these two lists to migrate owners.
    for _, snapshot in (*history._undo, *history._redo):
        snapshot['value'] += 100
    history.discard_checkpoint()
    history.redo()
    assert model.value == 102


def test_empty_history_and_default_label_behave_as_before():
    model = Model()
    history = History(model)
    calls = []
    history.on_discard = lambda: calls.append(1)
    assert history.undo() is None and history.redo() is None
    history.discard_checkpoint()
    assert not calls
    history.checkpoint()
    model.value = 2
    assert history.undo() == 'operation'
    assert history.redo() == 'operation'


def test_real_command_escape_keeps_redo_available():
    import serpentine3d.commands
    from serpentine3d.commands.base import CommandContext, CommandProcessor
    from serpentine3d.core.scene import Scene
    from serpentine3d.core.selection import SelectionManager
    scene = Scene()
    selection = SelectionManager(scene)
    history = History(scene)
    processor = CommandProcessor(CommandContext(scene, selection, history))
    assert processor.run('line')
    processor.provide_text('0,0,0')
    processor.provide_text('10,0,0')
    assert len(scene.all()) == 1
    history.undo()
    assert not scene.all() and history.can_redo
    assert processor.run('line')
    processor.cancel()
    assert history.can_redo
    history.redo()
    assert len(scene.all()) == 1
