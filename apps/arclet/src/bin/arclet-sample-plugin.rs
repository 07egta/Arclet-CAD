//! Minimal third-party-style Arclet plugin executable.
//! Build with: cargo build -p arclet --bin arclet-sample-plugin
//! Protocol: one UTF-8 JSON request on stdin, one JSON response on stdout.
#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::io::Read as _;
    use serde_json::{Value, json};
    let mut input = Vec::new();
    let result = std::io::stdin().take(64 * 1024 + 1).read_to_end(&mut input);
    let parsed = if result.is_ok() && input.len() <= 64 * 1024 {
        serde_json::from_slice::<Value>(&input).ok()
    } else {
        None
    };
    let response = match parsed {
        Some(request)
            if request["protocol"] == "arclet.plugins/1"
                && request["api_version"] == "1.0"
                && request["plugin_id"] == "org.arclet.sample" =>
        {
            match request["command"].as_str() {
                Some("report") if request["params"]["fail"] == true =>
                    json!({"protocol":"arclet.plugins/1","ok":false,"error":"Deliberate sample plugin error"}),
                Some("report") => {
                    let body_count = request["model"]["body_count"].as_u64().unwrap_or(0);
                    let volume = request["model"]["total"]["volume_mm3"].as_f64().unwrap_or(0.0);
                    json!({"protocol":"arclet.plugins/1","ok":true,"output":{
                        "body_count":body_count,"volume_mm3":volume
                    }})
                },
                Some("box") => {
                    let size = request["params"]["size_mm"].as_f64().unwrap_or(0.0);
                    if !size.is_finite() || !(1.0..=100.0).contains(&size) {
                        json!({"protocol":"arclet.plugins/1","ok":false,"error":"size_mm must be 1..100"})
                    } else {
                        json!({"protocol":"arclet.plugins/1","ok":true,
                            "output":{"size_mm":size},
                            "actions":[{"command":"solid.box",
                                "params":{"length":size,"width":size,"height":size,"body_name":"Plugin box"}}]})
                    }
                },
                _ => json!({"protocol":"arclet.plugins/1","ok":false,"error":"Unknown sample command"})
            }
        }
        _ => json!({"protocol":"arclet.plugins/1","ok":false,"error":"Invalid sample plugin request"})
    };
    println!("{response}");
}

#[cfg(target_arch = "wasm32")]
fn main() {}
