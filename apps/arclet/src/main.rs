#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod classic_look;
mod cmdline;
#[cfg(not(target_arch = "wasm32"))]
mod diagnostics;
mod commands;
mod panels;
mod platform;
mod statusbar;
mod toolbars;

#[cfg(not(target_arch = "wasm32"))]
fn screenshot_window_size(value: &str) -> Option<[f32; 2]> {
    // Developer-only capture aid. Reject malformed, non-finite and tiny windows;
    // the usual Arclet default and its minimum size are unchanged.
    let (width, height) = value.split_once('x')?;
    let width = width.parse::<f32>().ok()?;
    let height = height.parse::<f32>().ok()?;
    if !(960.0..=3840.0).contains(&width) || !(600.0..=2160.0).contains(&height) {
        return None;
    }
    Some([width, height])
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    diagnostics::init();
    log::info!("Starting Arclet CAD {}", env!("CARGO_PKG_VERSION"));
    let initial_size = std::env::var("ARCLET_WINDOW_SIZE")
        .ok()
        .and_then(|value| screenshot_window_size(&value))
        .unwrap_or([1440.0, 900.0]);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Arclet CAD")
            .with_app_id("arclet-cad")
            .with_inner_size(initial_size)
            .with_min_inner_size([960.0, 600.0])
            .with_decorations(true),
        renderer: eframe::Renderer::Wgpu,
        // CAD renders into its own offscreen depth target. The outer UI pass stays cheap.
        depth_buffer: 0,
        stencil_buffer: 0,
        multisampling: 1,
        // eframe enables Vulkan, DX12, Metal and GL where supported; its ordinary adapter
        // selection and WGPU_BACKEND override remain available without a second renderer.
        ..Default::default()
    };
    let result = eframe::run_native("Arclet CAD", options, Box::new(|cc| Ok(Box::new(app::ArcletApp::new(cc)))));
    if let Err(error) = &result {
        log::error!("Arclet GUI / GPU initialization failed: {error:?}. Try updating the graphics driver or setting WGPU_BACKEND=gl.");
    }
    log::logger().flush();
    result
}

#[cfg(target_arch = "wasm32")]
fn main() {
    let _ = eframe::WebLogger::init(log::LevelFilter::Info);
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = start_web().await {
            log::error!("{error}");
            if let Some(element) =
                web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id("arclet-loading"))
            {
                element.set_text_content(Some(&error));
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
async fn start_web() -> Result<(), String> {
    use wasm_bindgen::JsCast as _;

    let window = web_sys::window().ok_or("The browser window is unavailable")?;
    let document = window.document().ok_or("The browser document is unavailable")?;
    let canvas = document
        .get_element_by_id("arclet-canvas")
        .ok_or("The Arclet canvas is missing")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "The Arclet canvas is invalid")?;
    let mut options = eframe::WebOptions { depth_buffer: 0, ..Default::default() };
    // WebGPU is preferred and WebGL2 is available as a fallback. The explicit switch helps
    // older drivers without requiring a different build or storing a hidden global setting.
    if window.location().search().is_ok_and(|query| query.split(['?', '&']).any(|part| part == "webgl"))
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup
    {
        setup.instance_descriptor.backends = eframe::wgpu::Backends::GL;
    }
    eframe::WebRunner::new()
        .start(canvas, options, Box::new(|cc| Ok(Box::new(app::ArcletApp::new(cc)))))
        .await
        .map_err(|error| format!("Arclet could not start: {error:?}. A browser with WebGPU or WebGL2 is required."))?;
    if let Some(element) = document.get_element_by_id("arclet-loading") {
        element.remove();
    }
    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod screenshot_size_tests {
    use super::screenshot_window_size;

    #[test]
    fn capture_size_accepts_supported_resolutions_only() {
        assert_eq!(screenshot_window_size("960x600"), Some([960.0, 600.0]));
        assert_eq!(screenshot_window_size("1440x900"), Some([1440.0, 900.0]));
        for invalid in ["0x0", "959x600", "960x599", "NaNx900", "960x600x24", "-1x600", "abc"] {
            assert_eq!(screenshot_window_size(invalid), None, "{invalid}");
        }
    }
}
