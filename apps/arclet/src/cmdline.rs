//! The command line under the viewport: a short history, the "Command:" prompt, a live
//! suggestion list and typed letters that start a command. The pure state (the history ring
//! and the suggestion cursor) has no egui dependency and is tested directly.

use std::collections::VecDeque;

use egui::{RichText, Ui, pos2, vec2};
use solidcraft_ui_egui::i18n::{format as message, tr};
use solidcraft_ui_egui::keymap;
use solidcraft_ui_egui::scenario::publish_handle;
use solidcraft_ui_egui::theme::Tokens;
use solidcraft_ui_egui::ui_kit::{self, Container, Surface};
use solidcraft_ui_egui::{SolveApp, dialogs, tools};

use crate::commands::{self, Suggestion};

/// Lines kept in the history; the oldest are dropped first.
pub(crate) const HISTORY_CAP: usize = 200;
/// Lines visible at once; the rest scroll.
pub(crate) const HISTORY_ROWS: usize = 6;
/// Suggestions listed under the prompt.
const POPUP_ROWS: usize = 8;
/// Longest command line, as the input field enforces it.
const INPUT_LIMIT: usize = 256;
/// Id of the prompt's text field, so the keys can tell whether it has the focus.
const INPUT_ID: &str = "arclet_command_input";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineKind {
    /// Text the user typed and ran.
    Entered,
    /// An empty line that repeated the last command.
    Repeat,
    /// A command that started.
    Started,
    /// A command that could not be run, with the reason.
    Failed,
}

/// One history line: the text shown, and the text that clicking the line puts in the prompt
/// (empty when clicking should do nothing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryLine {
    pub(crate) kind: LineKind,
    pub(crate) text: String,
    pub(crate) fill: String,
}

/// A ring of the last `HISTORY_CAP` lines, oldest first.
#[derive(Default, Debug)]
pub(crate) struct CommandHistory {
    lines: VecDeque<HistoryLine>,
}

impl CommandHistory {
    pub(crate) fn push(&mut self, line: HistoryLine) {
        while self.lines.len() >= HISTORY_CAP {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &HistoryLine> {
        self.lines.iter()
    }

    /// Recall only what the user actually entered, not the second "started"
    /// or error message produced when it ran.
    fn entered_commands(&self) -> Vec<&str> {
        self.lines
            .iter()
            .filter(|line| matches!(line.kind, LineKind::Entered | LineKind::Repeat) && !line.fill.trim().is_empty())
            .map(|line| line.fill.as_str())
            .collect()
    }

    fn latest(&self) -> Option<&HistoryLine> {
        self.lines.back()
    }
}

/// Which suggestion the keys have highlighted. `None` until the first Up, Down or Tab, so an
/// Enter with no highlight runs the text as typed.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SuggestCursor {
    selected: Option<usize>,
}

impl SuggestCursor {
    pub(crate) fn selected(self) -> Option<usize> {
        self.selected
    }

    /// Down or Tab: the next suggestion, wrapping from the last to the first.
    pub(crate) fn next(&mut self, len: usize) {
        self.selected = match (self.selected, len) {
            (_, 0) => None,
            (None, _) => Some(0),
            (Some(i), _) => Some((i + 1) % len),
        };
    }

    /// Up or Shift+Tab: the previous suggestion, wrapping from the first to the last.
    pub(crate) fn prev(&mut self, len: usize) {
        self.selected = match (self.selected, len) {
            (_, 0) => None,
            (Some(i), _) if i > 0 && i < len => Some(i - 1),
            _ => Some(len.saturating_sub(1)),
        };
    }

    /// Forget a highlight that no longer names a suggestion.
    pub(crate) fn clamp(&mut self, len: usize) {
        if self.selected.is_some_and(|i| i >= len) {
            self.selected = None;
        }
    }

    pub(crate) fn reset(&mut self) {
        self.selected = None;
    }
}

/// Keeps the unfinished edit when browsing backward into command history, so a
/// Down arrow can restore it. Offsets count from newest (zero) to oldest.
#[derive(Default, Debug)]
struct HistoryRecall {
    offset: Option<usize>,
    draft: String,
}

impl HistoryRecall {
    fn reset(&mut self) {
        self.offset = None;
        self.draft.clear();
    }

    fn step(&mut self, history: &CommandHistory, edit: &mut String, older: bool) {
        let commands = history.entered_commands();
        if commands.is_empty() {
            return;
        }
        let next = if older {
            Some(self.offset.map_or(0, |offset| offset.saturating_add(1).min(commands.len() - 1)))
        } else {
            match self.offset {
                Some(0) => None,
                Some(offset) => Some(offset.min(commands.len() - 1) - 1),
                None => return,
            }
        };
        if self.offset.is_none() && next.is_some() {
            self.draft.clone_from(edit);
        }
        self.offset = next;
        match next {
            Some(offset) => *edit = commands[commands.len() - 1 - offset].to_owned(),
            None => *edit = self.draft.clone(),
        }
    }
}

/// Size and position of a history/suggestion popup in **logical points**.
/// Popups rise above the prompt rather than consuming viewport height.
fn popup_rect(viewport: egui::Rect, editor: egui::Rect, desired_height: f32) -> egui::Rect {
    const MARGIN: f32 = 8.0;
    let width = editor.width().max(220.0).min((viewport.width() - MARGIN * 2.0).max(1.0));
    let height = desired_height.min((viewport.height() - MARGIN * 2.0).max(1.0)).max(1.0);
    let left = editor.left().clamp(viewport.left() + MARGIN, (viewport.right() - width - MARGIN).max(viewport.left() + MARGIN));
    let top = (editor.top() - height - 4.0).clamp(viewport.top() + MARGIN, (viewport.bottom() - height - MARGIN).max(viewport.top() + MARGIN));
    egui::Rect::from_min_size(pos2(left, top), vec2(width, height))
}

/// What the command line asks the app to do after this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Nothing,
    /// Run the text in the prompt (an empty prompt repeats the last command).
    Run,
    /// Run a suggestion the user picked, by its id and its name.
    Suggested {
        id: &'static str,
        label: &'static str,
    },
    /// Escape in the prompt: the prompt is already cleared; cancel the active command.
    Cancel,
    /// The Finish button.
    Finish,
}

/// The command line's state, kept between frames by the app.
#[derive(Default)]
pub(crate) struct CommandLine {
    pub(crate) history: CommandHistory,
    cursor: SuggestCursor,
    /// The text the suggestions were made for, so they are only recomputed when it changes.
    query: String,
    suggestions: Vec<Suggestion>,
    history_open: bool,
    recall: HistoryRecall,
}

impl CommandLine {
    /// A line the user entered and ran.
    pub(crate) fn entered(&mut self, text: &str) {
        self.push(LineKind::Entered, message("arclet.cmd.entered", &[("text", text)]), text);
    }

    /// An empty line that repeats `id`.
    pub(crate) fn repeated(&mut self, id: &str) {
        self.push(LineKind::Repeat, message("arclet.cmd.repeated", &[("id", id)]), id);
    }

    /// `id` started.
    pub(crate) fn started(&mut self, id: &str) {
        self.push(LineKind::Started, message("arclet.cmd.started", &[("id", id)]), id);
    }

    /// The last line did not run; `fill` is what clicking the line puts back in the prompt.
    pub(crate) fn failed(&mut self, fill: &str, error: &str) {
        self.push(LineKind::Failed, message("arclet.cmd.failed", &[("error", error)]), fill);
    }

    fn push(&mut self, kind: LineKind, text: String, fill: &str) {
        self.history.push(HistoryLine { kind, text, fill: fill.to_string() });
    }

    /// Recompute the suggestions when the text changed; a new text drops the highlight.
    fn refresh(&mut self, command: &str) {
        if self.query != command {
            self.query.clear();
            self.query.push_str(command);
            self.suggestions =
                if command.trim().is_empty() { Vec::new() } else { commands::suggestions(command).into_iter().take(POPUP_ROWS).collect() };
            self.cursor.reset();
        }
        self.cursor.clamp(self.suggestions.len());
    }

    fn recall(&mut self, command: &mut String, older: bool) {
        self.recall.step(&self.history, command, older);
        self.history_open = false;
        self.refresh(command);
    }
}

/// Draw the command line and report what the user asked for. `focus` is taken (cleared) when
/// set, so the prompt takes the keyboard for the next key.
pub(crate) fn show(ui: &mut Ui, line: &mut CommandLine, command: &mut String, focus: &mut bool, invalid: bool) -> Action {
    let tokens = Tokens::get();
    line.refresh(command);
    let input_id = egui::Id::new(INPUT_ID);
    let mut action = Action::Nothing;
    let mut recalled_by_keyboard = false;

    if ui.ctx().memory(|m| m.has_focus(input_id)) {
        // Consume keys before the text field, including Tab (which would otherwise
        // leave the command prompt). Ctrl+Up/Down always recalls history; ordinary
        // arrows browse suggestions when present, then fall back to history.
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if line.history_open {
                line.history_open = false;
            } else {
                command.clear();
                line.cursor.reset();
                line.recall.reset();
                action = Action::Cancel;
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::ArrowUp)) {
            line.recall(command, true);
            recalled_by_keyboard = true;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::ArrowDown)) {
            line.recall(command, false);
            recalled_by_keyboard = true;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab)) {
            line.cursor.prev(line.suggestions.len());
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
            line.cursor.next(line.suggestions.len());
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
            if line.suggestions.is_empty() {
                line.recall(command, false);
                recalled_by_keyboard = true;
            } else {
                line.cursor.next(line.suggestions.len());
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
            if line.suggestions.is_empty() {
                line.recall(command, true);
                recalled_by_keyboard = true;
            } else {
                line.cursor.prev(line.suggestions.len());
            }
        }
    }

    // A single truncated feedback line is much less intrusive than a permanent
    // six-line history dock. Full history is accessible by keyboard or button.
    if !invalid && let Some(last) = line.history.latest() {
        let color = if last.kind == LineKind::Failed { tokens.error } else { tokens.text_dim };
        ui.add(egui::Label::new(RichText::new(&last.text).monospace().small().color(color)).truncate())
            .on_hover_text(&last.text);
    }

    let initial = command.clone();
    let mut run_requested = false;
    let mut picked = Action::Nothing;
    let mut editor = egui::Rect::NOTHING;

    ui.horizontal_centered(|ui| {
        ui.label(RichText::new(tr("arclet.command")).strong().color(tokens.text));
        let available = ui.available_width();
        let compact = available < 530.0;
        if !line.history.is_empty() {
            let history_label = if line.history_open { tr("arclet.cmd.hide_history") } else { tr("arclet.cmd.show_history") };
            if ui_kit::button(ui, history_label, ui_kit::Button::Ghost).clicked() {
                line.history_open = !line.history_open;
            }
        }
        let reserve = if compact { 128.0 } else { 182.0 };
        let width = (ui.available_width() - reserve).max(110.0);
        let response = ui
            .allocate_ui(vec2(width, 30.0), |ui| {
                ui_kit::bounded_cell_editor(ui, input_id, &mut *command, tr("arclet.command_hint"), invalid, INPUT_LIMIT)
            })
            .inner;
        editor = response.rect;
        publish_handle("arclet:command", editor.center());
        if std::mem::take(focus) {
            response.request_focus();
        }
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            run_requested = true;
        }
        if ui_kit::button(ui, tr("arclet.run"), ui_kit::Button::Primary).clicked() {
            run_requested = true;
        }
        if !compact && ui_kit::button(ui, tr("arclet.finish"), ui_kit::Button::Normal).clicked() {
            picked = Action::Finish;
        }
    });

    if command != &initial {
        // Text may have changed inside the egui editor in this same frame.
        // Refresh *before* Enter chooses the highlighted suggestion, otherwise
        // a stale highlight could execute a different command.
        line.refresh(command);
        if !recalled_by_keyboard {
            line.recall.reset();
        }
    }
    if run_requested {
        action = submit(line);
    }
    if picked != Action::Nothing {
        action = picked;
    }
    // Compact chrome never removes Finish: put it on a second line only on
    // very small logical screens, while the editor keeps a usable hit target.
    if ui.available_width() < 530.0 && ui_kit::button(ui, tr("arclet.finish"), ui_kit::Button::Normal).clicked() {
        action = Action::Finish;
    }

    let history = line.history_open && !line.history.is_empty();
    if editor != egui::Rect::NOTHING && (history || !line.suggestions.is_empty()) {
        let entries = if history { line.history.len().min(HISTORY_ROWS) } else { line.suggestions.len().min(POPUP_ROWS) };
        let height = (entries as f32 * 29.0 + 22.0).min(ui.ctx().viewport_rect().height() * 0.55);
        let rect = popup_rect(ui.ctx().viewport_rect(), editor, height);
        let area = Container::new(ui.ctx(), Surface::Popup)
            .area(egui::Id::new("arclet_command_popup"))
            .fixed_pos(rect.min);
        let shown = area.show(ui.ctx(), |ui| {
            Container::popup_frame(ui.style()).show(ui, |ui| {
                ui.set_min_width((rect.width() - 20.0).max(40.0));
                egui::ScrollArea::vertical()
                    .id_salt("arclet_command_popup_scroll")
                    .max_height((rect.height() - 12.0).max(1.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        if history {
                            let start = line.history.len().saturating_sub(HISTORY_ROWS);
                            for entry in line.history.iter().skip(start) {
                                let color = if entry.kind == LineKind::Failed { tokens.error } else { tokens.text };
                                let label = egui::Label::new(RichText::new(&entry.text).monospace().color(color))
                                    .sense(egui::Sense::click())
                                    .truncate();
                                if ui.add(label).on_hover_text(&entry.text).clicked() && !entry.fill.is_empty() {
                                    *command = entry.fill.clone();
                                    *focus = true;
                                    line.history_open = false;
                                    line.recall.reset();
                                }
                            }
                        } else {
                            for (index, suggestion) in line.suggestions.iter().enumerate() {
                                let selected = line.cursor.selected() == Some(index);
                                let label = format!("{}    {}", suggestion.label, suggestion.id);
                                if ui.selectable_label(selected, label).clicked() {
                                    picked = Action::Suggested { id: suggestion.id, label: suggestion.label };
                                }
                            }
                        }
                    });
            });
        });
        // Give a pointer click on a suggestion precedence over Enter from the
        // editor. No file operation or CAD modification occurs in popup painting.
        if picked != Action::Nothing {
            action = picked;
        }
        let _ = shown;
    }
    action
}

/// Enter runs the highlighted suggestion when there is one, else the text as typed.
fn submit(line: &CommandLine) -> Action {
    match line.cursor.selected().and_then(|i| line.suggestions.get(i)) {
        Some(suggestion) => Action::Suggested { id: suggestion.id, label: suggestion.label },
        None => Action::Run,
    }
}

/// Escape's cancel, as the viewport does it: the tool or the dialog that is out.
pub(crate) fn cancel_active(app: &mut SolveApp) {
    if app.tool.is_some() {
        tools::finish(app);
    } else if app.dialog.is_some() {
        dialogs::cancel(app);
    }
}

/// Letters typed while the viewport has the keyboard start the command line: the letter is
/// added to the prompt, which then takes the focus. A letter the keymap uses (or that
/// SolidCraft reserves) is left to the keymap, and nothing routes while a tool, a dialog, a
/// menu or a text field is taking input. Returns whether a letter was added.
pub(crate) fn route_letters(ctx: &egui::Context, app: &SolveApp, command: &mut String) -> bool {
    if app.nodes.open
        || app.home.open
        || app.tool.is_some()
        || app.dialog.is_some()
        || ctx.egui_wants_keyboard_input()
        || egui::Popup::is_any_open(ctx)
        || ctx.memory(|m| m.focused().is_some())
        || command.chars().count() >= INPUT_LIMIT
    {
        return false;
    }
    let (free, typed): (Vec<char>, Vec<char>) = ctx.input(|input| {
        let mut free = Vec::new();
        let mut typed = Vec::new();
        for event in &input.events {
            match event {
                egui::Event::Key { key, pressed: true, modifiers, .. }
                    if !(modifiers.command || modifiers.ctrl || modifiers.alt || modifiers.mac_cmd) && is_free(app, *key, *modifiers) =>
                {
                    if let Some(letter) = letter_of(*key) {
                        free.push(letter);
                    }
                }
                egui::Event::Text(text) => {
                    let mut chars = text.chars();
                    if let (Some(c), None) = (chars.next(), chars.next())
                        && c.is_ascii_alphabetic()
                    {
                        typed.push(c);
                    }
                }
                _ => {}
            }
        }
        (free, typed)
    });
    let room = INPUT_LIMIT.saturating_sub(command.chars().count());
    let routed: Vec<char> = typed.into_iter().filter(|c| free.contains(&c.to_ascii_uppercase())).take(room).collect();
    command.extend(routed.iter());
    !routed.is_empty()
}

/// The letter a key names (`A` to `Z`).
fn letter_of(key: egui::Key) -> Option<char> {
    let name = key.name();
    match name.as_bytes() {
        [byte] if byte.is_ascii_uppercase() => Some(char::from(*byte)),
        _ => None,
    }
}

/// Whether a letter pressed with these modifiers is free: no command is bound to it (even one
/// that is off right now, so a letter never runs twice), and SolidCraft does not reserve it.
fn is_free(app: &SolveApp, key: egui::Key, mods: egui::Modifiers) -> bool {
    [egui::Modifiers::NONE, mods].into_iter().all(|m| {
        let name = keymap::key_name(key, m);
        !keymap::RESERVED.contains(&name.as_str()) && keymap::commands_for(app, &name).is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> HistoryLine {
        HistoryLine { kind: LineKind::Entered, text: text.to_string(), fill: String::new() }
    }

    #[test]
    fn history_keeps_the_newest_lines_in_order_up_to_the_cap() {
        let mut history = CommandHistory::default();
        assert!(history.is_empty());
        for n in 0..HISTORY_CAP + 5 {
            history.push(line(&n.to_string()));
        }
        let texts: Vec<&str> = history.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts.len(), HISTORY_CAP);
        assert_eq!(texts.first().copied(), Some("5"));
        assert_eq!(texts.last().copied(), Some("204"));
    }

    #[test]
    fn suggestion_cursor_wraps_and_forgets_a_highlight_that_left_the_list() {
        let mut cursor = SuggestCursor::default();
        assert_eq!(cursor.selected(), None);
        cursor.next(3);
        assert_eq!(cursor.selected(), Some(0));
        cursor.prev(3);
        assert_eq!(cursor.selected(), Some(2), "Up from the first wraps to the last");
        cursor.next(3);
        assert_eq!(cursor.selected(), Some(0), "Down from the last wraps to the first");
        cursor.next(0);
        assert_eq!(cursor.selected(), None, "no suggestions, no highlight");
        cursor.next(4);
        cursor.next(4);
        cursor.next(4);
        cursor.clamp(2);
        assert_eq!(cursor.selected(), None);
    }

    #[test]
    fn entered_started_and_failed_lines_keep_what_clicking_them_should_fill() {
        let mut line = CommandLine::default();
        line.entered("Line");
        line.started("sketch.line");
        line.entered("bogus");
        line.failed("bogus", "Unknown command.");
        let kinds: Vec<LineKind> = line.history.iter().map(|l| l.kind).collect();
        assert_eq!(kinds, vec![LineKind::Entered, LineKind::Started, LineKind::Entered, LineKind::Failed]);
        let fills: Vec<&str> = line.history.iter().map(|l| l.fill.as_str()).collect();
        assert_eq!(fills, vec!["Line", "sketch.line", "bogus", "bogus"]);
    }

    #[test]
    fn changing_the_text_drops_the_highlight() {
        let mut line = CommandLine::default();
        line.refresh("li");
        line.cursor.next(9);
        assert!(line.cursor.selected().is_some());
        line.refresh("lin");
        assert_eq!(line.cursor.selected(), None);
    }
}
