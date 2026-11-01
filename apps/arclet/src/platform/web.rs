use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use js_sys::{Array, Uint8Array};
use solidcraft_engine::io::vfs;
use solidcraft_ui_egui::{FilePickerFuture, Services};
use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{BeforeUnloadEvent, Event, HtmlInputElement};

use super::{MAX_BROWSER_FILE_BYTES, file_name, recovery::PendingDownloads, validate_upload};

const UPLOADS: &str = "/arclet/upload/";
const DOWNLOADS: &str = "/arclet/download/";
const MAX_PENDING_BYTES: usize = 128 * 1024 * 1024;

struct DownloadUrl {
    url: String,
    bytes: usize,
    until: f64,
}

#[derive(Default)]
struct BrowserState {
    errors: VecDeque<String>,
    urls: VecDeque<DownloadUrl>,
    pending: PendingDownloads,
    before_unload: Option<Closure<dyn FnMut(BeforeUnloadEvent)>>,
}

thread_local! {
    static STATE: RefCell<BrowserState> = RefCell::new(BrowserState::default());
    static NEXT_FILE: Cell<u32> = const { Cell::new(1) };
    static UNSAVED: Cell<bool> = const { Cell::new(false) };
}

fn state<T>(f: impl FnOnce(&mut BrowserState) -> T) -> Result<T, String> {
    STATE.with(|slot| slot.try_borrow_mut().map(|mut value| f(&mut value)).map_err(|_| "File services are busy".into()))
}

fn report(ctx: &egui::Context, error: String) {
    let _ = state(|state| {
        if state.errors.len() >= 32 {
            state.errors.pop_front();
        }
        state.errors.push_back(error);
    });
    ctx.request_repaint();
}

fn js_error(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

#[derive(Default)]
struct PickerState {
    answer: Option<Option<String>>,
    waker: Option<Waker>,
    reading: bool,
    active: bool,
}

fn finish(shared: &RefCell<PickerState>, ctx: &egui::Context, answer: Option<String>) {
    let waker = if let Ok(mut state) = shared.try_borrow_mut() {
        if !state.active || state.answer.is_some() {
            return;
        }
        state.answer = Some(answer);
        state.waker.take()
    } else {
        None
    };
    if let Some(waker) = waker {
        waker.wake();
    }
    ctx.request_repaint();
}

struct Picker {
    input: HtmlInputElement,
    changed: Closure<dyn FnMut(Event)>,
    cancelled: Closure<dyn FnMut(Event)>,
    state: Rc<RefCell<PickerState>>,
}

impl Future for Picker {
    type Output = Option<String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Ok(mut state) = self.state.try_borrow_mut() else { return Poll::Pending };
        state.waker = Some(cx.waker().clone());
        state.answer.take().map_or(Poll::Pending, Poll::Ready)
    }
}

impl Drop for Picker {
    fn drop(&mut self) {
        let _ = self.input.remove_event_listener_with_callback("change", self.changed.as_ref().unchecked_ref());
        let _ = self.input.remove_event_listener_with_callback("cancel", self.cancelled.as_ref().unchecked_ref());
        self.input.remove();
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.active = false;
        }
    }
}

fn open(ctx: &egui::Context, extensions: &[&str]) -> Result<Picker, String> {
    let document = web_sys::window().and_then(|window| window.document()).ok_or("The browser document is unavailable")?;
    let input: HtmlInputElement = document.create_element("input").map_err(js_error)?.dyn_into().map_err(|_| "Could not create a file picker")?;
    input.set_type("file");
    input.set_multiple(false);
    input.set_attribute("style", "display:none").map_err(js_error)?;
    if !extensions.is_empty() {
        let accept = extensions
            .iter()
            .take(64)
            .filter(|extension| !extension.is_empty() && extension.len() <= 32 && extension.chars().all(|c| c.is_ascii_alphanumeric()))
            .map(|extension| format!(".{extension}"))
            .collect::<Vec<_>>()
            .join(",");
        input.set_accept(&accept);
    }
    let shared = Rc::new(RefCell::new(PickerState { active: true, ..Default::default() }));
    let (changed_state, cancelled_state) = (shared.clone(), shared.clone());
    let (changed_ctx, cancelled_ctx) = (ctx.clone(), ctx.clone());
    let chosen_input = input.clone();
    let changed = Closure::new(move |_: Event| {
        if let Ok(mut state) = changed_state.try_borrow_mut() {
            if state.reading || !state.active {
                return;
            }
            state.reading = true;
        } else {
            return;
        }
        let Some(file) = chosen_input.files().and_then(|files| files.get(0)) else {
            finish(&changed_state, &changed_ctx, None);
            return;
        };
        let shared = changed_state.clone();
        let ctx = changed_ctx.clone();
        spawn_local(async move {
            match read_file(file).await {
                Ok((path, bytes)) => {
                    if shared.try_borrow().is_ok_and(|state| state.active && state.answer.is_none()) {
                        vfs::insert(&path, bytes);
                        finish(&shared, &ctx, Some(path));
                    }
                }
                Err(error) => {
                    report(&ctx, error);
                    finish(&shared, &ctx, None);
                }
            }
        });
    });
    let cancelled = Closure::new(move |_: Event| finish(&cancelled_state, &cancelled_ctx, None));
    let picker = Picker { input, changed, cancelled, state: shared };
    picker.input.add_event_listener_with_callback("change", picker.changed.as_ref().unchecked_ref()).map_err(js_error)?;
    picker.input.add_event_listener_with_callback("cancel", picker.cancelled.as_ref().unchecked_ref()).map_err(js_error)?;
    document.body().ok_or("The browser page is unavailable")?.append_child(&picker.input).map_err(js_error)?;
    // Must happen inside the user's original click, before returning the future.
    picker.input.click();
    Ok(picker)
}

async fn read_file(file: web_sys::File) -> Result<(String, Vec<u8>), String> {
    let existing = vfs::list("/arclet/");
    validate_upload(file.size(), &existing)?;
    let data = JsFuture::from(file.array_buffer()).await.map_err(js_error)?;
    let array = Uint8Array::new(&data);
    if array.length() as usize > MAX_BROWSER_FILE_BYTES {
        return Err("The selected file exceeds the browser upload limit".into());
    }
    // Another picker may have completed while the browser read this file. Recheck the
    // actual byte count immediately before allocating the Rust copy and accepting it.
    validate_upload(f64::from(array.length()), &vfs::list("/arclet/"))?;
    let id = NEXT_FILE.with(|next| {
        let id = next.get();
        next.set(id.checked_add(1).ok_or("Browser file identifiers are exhausted")?);
        Ok::<u32, String>(id)
    })?;
    Ok((format!("{UPLOADS}{id}/{}", file_name(&file.name())), array.to_vec()))
}

fn picker(ctx: &egui::Context, extensions: &[&str]) -> FilePickerFuture {
    match open(ctx, extensions) {
        Ok(picker) => Box::pin(picker),
        Err(error) => {
            report(ctx, error);
            Box::pin(std::future::ready(None))
        }
    }
}

fn download(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_BROWSER_FILE_BYTES {
        return Err("Browser downloads are limited to 64 MiB per file".into());
    }
    let full = state(|state| {
        expire_urls(state);
        state.urls.len() >= 16 || state.urls.iter().map(|url| url.bytes).sum::<usize>() + bytes.len() > MAX_PENDING_BYTES
    })?;
    if full {
        return Err("Several downloads are still active; wait a minute and try again".into());
    }
    let document = web_sys::window().and_then(|window| window.document()).ok_or("The browser document is unavailable")?;
    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/octet-stream");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(js_error)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_error)?;
    let start = || -> Result<(), String> {
        let anchor: web_sys::HtmlAnchorElement =
            document.create_element("a").map_err(js_error)?.dyn_into().map_err(|_| "Could not create a download")?;
        anchor.set_href(&url);
        anchor.set_download(&file_name(name));
        document.body().ok_or("The browser page is unavailable")?.append_child(&anchor).map_err(js_error)?;
        anchor.click();
        anchor.remove();
        Ok(())
    };
    if let Err(error) = start() {
        let _ = web_sys::Url::revoke_object_url(&url);
        return Err(error);
    }
    state(|state| state.urls.push_back(DownloadUrl { url, bytes: bytes.len(), until: js_sys::Date::now() + 60_000.0 }))?;
    ctx.request_repaint_after(std::time::Duration::from_secs(60));
    Ok(())
}

fn expire_urls(state: &mut BrowserState) {
    let now = js_sys::Date::now();
    while state.urls.front().is_some_and(|url| url.until <= now) {
        if let Some(url) = state.urls.pop_front() {
            let _ = web_sys::Url::revoke_object_url(&url.url);
        }
    }
}

fn retain_failed(ctx: &egui::Context, path: String, length: usize, error: String) {
    let _ = state(|state| state.pending.failed(path, length, js_sys::Date::now()));
    report(ctx, format!("Not downloaded yet: {error}. Your work remains unsaved."));
    ctx.request_repaint_after(std::time::Duration::from_secs(60));
}

fn completed(path: &str) {
    let _ = state(|state| state.pending.completed(path));
    if path.starts_with(DOWNLOADS) {
        let _ = vfs::remove(path);
    }
}

pub(super) fn has_unsaved_downloads() -> bool {
    state(|state| state.pending.is_unsaved()).unwrap_or(true)
}

pub(super) fn set_unsaved(unsaved: bool) -> Result<(), String> {
    UNSAVED.with(|value| value.set(unsaved));
    state(|state| -> Result<(), String> {
        if unsaved == state.before_unload.is_some() {
            return Ok(());
        }
        let window = web_sys::window().ok_or("The browser window is unavailable")?;
        if unsaved {
            let handler = Closure::new(|event: BeforeUnloadEvent| {
                if UNSAVED.with(Cell::get) {
                    event.prevent_default();
                    // A nonempty value supports legacy browsers; current browsers display
                    // their own translated message, not application-supplied text.
                    event.set_return_value("true");
                }
            });
            window.add_event_listener_with_callback("beforeunload", handler.as_ref().unchecked_ref()).map_err(js_error)?;
            state.before_unload = Some(handler);
        } else if let Some(handler) = &state.before_unload {
            // Keep the closure alive if removal fails: the browser still owns its callback.
            window.remove_event_listener_with_callback("beforeunload", handler.as_ref().unchecked_ref()).map_err(js_error)?;
            state.before_unload = None;
        }
        Ok(())
    })?
}

pub(super) fn services(ctx: &egui::Context) -> Services {
    let (open_ctx, file_ctx, download_ctx) = (ctx.clone(), ctx.clone(), ctx.clone());
    Services {
        paths: Some(solidcraft_ui_egui::HostPaths::default()),
        pick_open: Some(Box::new(move |extensions| picker(&open_ctx, extensions))),
        pick_file: Some(Box::new(move |_, extensions| picker(&file_ctx, extensions))),
        pick_save: Some(Box::new(move |name, _| Box::pin(std::future::ready(Some(format!("{DOWNLOADS}{}", file_name(name))))))),
        download_design: Some(Box::new(move |name, bytes| download(&download_ctx, name, bytes))),
        ..Default::default()
    }
}

pub(super) fn poll(ctx: &egui::Context) -> Vec<String> {
    // Fresh writes take priority. A save to a path awaiting retry replaces its old bytes,
    // so retrying first could download the same new snapshot twice.
    for (path, bytes) in vfs::take_writes() {
        if path.starts_with(DOWNLOADS) || path.starts_with(UPLOADS) {
            match download(ctx, &file_name(&path), &bytes) {
                Ok(()) => completed(&path),
                Err(error) => retain_failed(ctx, path, bytes.len(), error),
            }
        }
    }
    let retry = state(|state| {
        expire_urls(state);
        state.pending.due(js_sys::Date::now()).map(str::to_string)
    })
    .ok()
    .flatten();
    if let Some(path) = retry {
        match vfs::read(&path, MAX_BROWSER_FILE_BYTES) {
            Ok(bytes) => match download(ctx, &file_name(&path), &bytes) {
                Ok(()) => completed(&path),
                Err(error) => retain_failed(ctx, path, bytes.len(), error),
            },
            Err(error) => retain_failed(ctx, path, MAX_BROWSER_FILE_BYTES + 1, error.to_string()),
        }
    }
    match state(|state| {
        expire_urls(state);
        let next = state.urls.front().map(|url| url.until).into_iter().chain(state.pending.next_retry()).reduce(f64::min);
        if let Some(next) = next {
            let delay = ((next - js_sys::Date::now()) / 1000.0).clamp(0.01, 60.0);
            let delay = std::time::Duration::try_from_secs_f64(delay).unwrap_or(std::time::Duration::from_secs(60));
            ctx.request_repaint_after(delay);
        }
        state.errors.drain(..).collect()
    }) {
        Ok(errors) => errors,
        Err(error) => vec![error],
    }
}
