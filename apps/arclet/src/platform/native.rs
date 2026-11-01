use solidcraft_ui_egui::i18n::tr;
use solidcraft_ui_egui::{FilePickerFuture, HostPaths, Services};

use super::file_name;

fn open(ctx: &egui::Context, label: &str, extensions: &[&str]) -> FilePickerFuture {
    let ctx = ctx.clone();
    let mut dialog = rfd::AsyncFileDialog::new().set_title(tr("arclet.files.open"));
    if extensions.is_empty() {
        dialog = dialog.add_filter(tr("arclet.files.cad_mesh"), &["sc", "json", "step", "stp", "stl", "obj", "3mf", "3dm", "dxf", "svg"]);
    } else {
        dialog = dialog.add_filter(label, extensions);
    }
    let future = dialog.add_filter(tr("arclet.files.all"), &["*"]).pick_file();
    Box::pin(async move {
        let answer = future.await.map(|file| file.path().to_string_lossy().into_owned());
        ctx.request_repaint();
        answer
    })
}

pub(super) fn services(ctx: &egui::Context) -> Services {
    let (open_ctx, file_ctx, save_ctx) = (ctx.clone(), ctx.clone(), ctx.clone());
    Services {
        paths: Some(HostPaths {
            recovery: super::recovery_dir(),
            crashes: super::crash_dir(),
            plugins: super::data_dir().map(|dir| dir.join("plugins")),
        }),
        pick_open: Some(Box::new(move |extensions| open(&open_ctx, tr("arclet.files.files"), extensions))),
        pick_file: Some(Box::new(move |label, extensions| open(&file_ctx, label, extensions))),
        pick_save: Some(Box::new(move |name, extensions| {
            let ctx = save_ctx.clone();
            let mut dialog = rfd::AsyncFileDialog::new().set_title(tr("arclet.files.save")).set_file_name(file_name(name));
            if !extensions.is_empty() {
                dialog = dialog.add_filter(tr("arclet.files.files"), extensions);
            }
            let future = dialog.save_file();
            Box::pin(async move {
                let answer = future.await.map(|file| file.path().to_string_lossy().into_owned());
                ctx.request_repaint();
                answer
            })
        })),
        ..Default::default()
    }
}
