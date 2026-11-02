//! Validated, portable index for local reusable CAD component templates.
//! The index is metadata only: loading a manifest never opens referenced designs.
//! Actual insertion must separately validate the referenced design and remap IDs.
use crate::{IoError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const FORMAT: &str = "arclet-component-library/1";
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
pub const MAX_COMPONENTS: usize = 256;

fn invalid(reason: &str) -> IoError { IoError::Invalid(reason.to_owned()) }
fn label(s: &str, max: usize) -> bool {
    !s.is_empty() && s == s.trim() && s.len() <= max && !s.chars().any(char::is_control)
}
fn slug(s: &str) -> bool {
    s.len() <= 64 && s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
}
fn relative_file(s: &str) -> bool {
    s.len() <= 240 && !s.is_empty() && !s.contains('\\') && !s.contains(':')
        && !s.starts_with('/') && !s.chars().any(char::is_control)
        && s.split('/').all(|part| slug(part.trim_end_matches(".sc")) || (
            part.ends_with(".sc") && slug(&part[..part.len()-3])))
        && s.ends_with(".sc")
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Category {
    Furniture, Electronics, Fasteners, Plumbing, Architecture, Lighting, Machinery, Primitives,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub default: f64,
    pub min: f64,
    pub max: f64,
    /// Informational only. The design evaluator remains the authority.
    pub unit: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    /// Bounds in the same units as the source design.
    pub bounds: [f64; 3],
    pub description: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentEntry {
    pub id: String,
    pub name: String,
    pub category: Category,
    #[serde(default)]
    pub tags: Vec<String>,
    /// A relative, in-library design path; never accessed while inspecting metadata.
    pub source: String,
    /// SHA-256 of the referenced source file, required at insertion time.
    pub sha256: String,
    /// Monotonically assigned content version, not an auto-upgrade instruction.
    pub version: u32,
    /// Source design's native length unit. Cross-unit insertions require explicit conversion.
    pub unit: String,
    pub preview: Preview,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentLibrary {
    pub format: String,
    pub name: String,
    pub entries: Vec<ComponentEntry>,
}
impl ComponentLibrary {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_MANIFEST_BYTES { return Err(invalid("component manifest exceeds 1 MiB")); }
        let manifest: Self = serde_json::from_slice(bytes).map_err(|_| invalid("malformed component library manifest"))?;
        manifest.validate()?;
        Ok(manifest)
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|_| invalid("cannot encode component manifest"))?;
        if bytes.len() > MAX_MANIFEST_BYTES { return Err(invalid("component manifest exceeds 1 MiB")); }
        Ok(bytes)
    }
    pub fn validate(&self) -> Result<()> {
        if self.format != FORMAT || !label(&self.name, 128) || self.entries.len() > MAX_COMPONENTS {
            return Err(invalid("invalid component library format, name, or entry count"));
        }
        let mut ids = BTreeSet::new();
        let mut sources = BTreeSet::new();
        for entry in &self.entries {
            if !slug(&entry.id) || !ids.insert(entry.id.clone()) || !label(&entry.name, 128)
                || !relative_file(&entry.source) || !sources.insert(entry.source.clone())
                || entry.sha256.len() != 64 || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                || entry.version == 0 || !matches!(entry.unit.as_str(), "mm" | "cm" | "m" | "in")
                || entry.tags.len() > 16 || entry.parameters.len() > 64
                || !label(&entry.preview.description, 512)
                || entry.preview.bounds.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1e9)
            { return Err(invalid("invalid or duplicate component entry")); }
            let mut tags = BTreeSet::new();
            for tag in &entry.tags {
                if !slug(tag) || !tags.insert(tag) { return Err(invalid("invalid or duplicate component tag")); }
            }
            let mut names = BTreeSet::new();
            for p in &entry.parameters {
                if !slug(&p.name) || !names.insert(&p.name) ||
                    !matches!(p.unit.as_str(), "mm" | "cm" | "m" | "in" | "deg" | "unitless") ||
                    !p.default.is_finite() || !p.min.is_finite() || !p.max.is_finite() ||
                    p.min > p.default || p.default > p.max || p.min < -1e9 || p.max > 1e9
                { return Err(invalid("invalid component parameter bounds")); }
            }
        }
        Ok(())
    }
    /// Stable, case-insensitive search over validated metadata; no filesystem access.
    pub fn search<'a>(&'a self, query: &str, category: Option<&Category>, tags: &[String]) -> Result<Vec<&'a ComponentEntry>> {
        self.validate()?;
        if query.len() > 256 || query.chars().any(char::is_control) || tags.len() > 16 ||
            tags.iter().any(|tag| !slug(tag)) { return Err(invalid("invalid component search filter")); }
        let q = query.to_lowercase();
        Ok(self.entries.iter().filter(|e| {
            category.is_none_or(|c| c == &e.category)
                && tags.iter().all(|t| e.tags.contains(t))
                && (q.is_empty() || e.id.contains(&q) || e.name.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.contains(&q)))
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> ComponentLibrary {
        ComponentLibrary {format: FORMAT.into(), name: "Local parts".into(), entries: vec![
            ComponentEntry {
                id:"chair".into(), name:"Adjustable chair".into(), category:Category::Furniture,
                tags:vec!["seating".into()], source:"furniture/chair.sc".into(),
                sha256:"a".repeat(64), version:1, unit:"mm".into(),
                preview:Preview {bounds:[450.,450.,900.],description:"Parametric chair".into()},
                parameters:vec![Parameter{name:"seat_width".into(),default:450.,min:300.,max:650.,unit:"mm".into()}],
            }
        ]}
    }
    #[test] fn roundtrip_and_search() {
        let x=fixture(); let bytes=x.encode().unwrap(); let y=ComponentLibrary::decode(&bytes).unwrap();
        assert_eq!(x,y);
        assert_eq!(y.search("CHAIR",Some(&Category::Furniture), &["seating".into()]).unwrap().len(),1);
        assert!(y.search("chair",Some(&Category::Lighting), &[]).unwrap().is_empty());
    }
    #[test] fn hostile_manifest_rejected() {
        let x=fixture();
        for source in ["../escape.sc", "/tmp/file.sc", "C:/file.sc", "nested/../file.sc", "other.json", "a\\b.sc"] {
            let mut y=x.clone(); y.entries[0].source=source.into();
            assert!(y.encode().is_err(), "{source}");
        }
        let mut y=x.clone(); y.entries[0].parameters[0].default=f64::NAN; assert!(y.encode().is_err());
        let mut y=x.clone(); y.entries.push(y.entries[0].clone()); assert!(y.encode().is_err());
        let mut y=x.clone(); y.entries[0].sha256="bad".into(); assert!(y.encode().is_err());
        let mut y=x.clone(); y.entries[0].tags.push("seating".into()); assert!(y.encode().is_err());
        assert!(ComponentLibrary::decode(&vec![b'x';MAX_MANIFEST_BYTES+1]).is_err());
        let mut unknown=serde_json::to_value(x).unwrap();
        unknown["entries"][0]["unexpected"]=serde_json::json!(1);
        assert!(ComponentLibrary::decode(&serde_json::to_vec(&unknown).unwrap()).is_err());
    }
}
