//! The command line under the viewport: a short history, the "Command:" prompt, a live
//! suggestion list and typed letters that start a command. The pure state (the history ring
//! and the suggestion cursor) has no egui dependency and is tested directly.

use std::collections::VecDeque;

use egui::{RichText, Ui, vec2};
use solidcraft_ui_egui::i18n::{format as message, tr};
use solidcraft_ui_egui::keymap;
use solidcraft_ui_egui::scenario::publish_handle;
use solidcraft_ui_egui::theme::Tokens;
use solidcraft_ui_egui::ui_kit;
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
}

/// Draw the command line and report what the user asked for. `focus` is taken (cleared) when
/// set, so the prompt takes the keyboard for the next key.
pub(crate) fn show(ui: &mut Ui, line: &mut CommandLine, command: &mut String, focus: &mut bool, invalid: bool, guided: bool) -> Action {
    let tokens = Tokens::get();
    if guided {
        line.query.clear();
        line.suggestions.clear();
        line.cursor.reset();
    } else {
        line.refresh(command);
    }
    let input_id = egui::Id::new(INPUT_ID);
    let mut action = Action::Nothing;
    if ui.ctx().memory(|m| m.has_focus(input_id)) {
        // These keys belong to the suggestion list while the prompt has the focus. They are
        // taken before the text field sees them (Tab would move the focus away).
        let len = line.suggestions.len();
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            command.clear();
            line.cursor.reset();
            action = Action::Cancel;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab)) {
            line.cursor.prev(len);
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
            line.cursor.next(len);
        }
        if !guided && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
            line.cursor.next(len);
        }
        if !guided && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
            line.cursor.prev(len);
        }
    }

    if !line.history.is_empty() {
        let row = ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y;
        let mut recalled: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt(input_id.with("history"))
            .max_height(row * HISTORY_ROWS as f32)
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show_rows(ui, row, line.history.len(), |ui, range| {
                for entry in line.history.iter().skip(range.start).take(range.len()) {
                    let color = match entry.kind {
                        LineKind::Failed => tokens.error,
                        LineKind::Entered => tokens.text,
                        LineKind::Repeat | LineKind::Started => tokens.text_dim,
                    };
                    let label = egui::Label::new(RichText::new(entry.text.as_str()).monospace().color(color)).sense(egui::Sense::click());
                    if ui.add(label).clicked() && !entry.fill.is_empty() {
                        recalled = Some(entry.fill.clone());
                    }
                }
            });
        if let Some(text) = recalled {
            *command = text;
            line.cursor.reset();
            *focus = true;
        }
    }

    if !line.suggestions.is_empty() {
        for (index, suggestion) in line.suggestions.iter().enumerate() {
            let selected = line.cursor.selected() == Some(index);
            let text = format!("{}  {}", suggestion.label, suggestion.id);
            if ui.selectable_label(selected, text).clicked() {
                action = Action::Suggested { id: suggestion.id, label: suggestion.label };
            }
        }
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("arclet.command")).strong());
        let response = ui
            .allocate_ui(vec2((ui.available_width() - 170.0).max(100.0), 28.0), |ui| {
                ui_kit::bounded_cell_editor(ui, input_id, &mut *command, tr("arclet.command_hint"), invalid, INPUT_LIMIT)
            })
            .inner;
        publish_handle("arclet:command", response.rect.center());
        if std::mem::take(focus) {
            response.request_focus();
        }
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            action = submit(line);
        }
        if ui_kit::button(ui, tr("arclet.run"), ui_kit::Button::Primary).clicked() {
            action = submit(line);
        }
        if ui_kit::button(ui, tr("arclet.finish"), ui_kit::Button::Normal).clicked() {
            action = Action::Finish;
        }
        ui.label(RichText::new("mm").color(tokens.text_dim));
    });
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
