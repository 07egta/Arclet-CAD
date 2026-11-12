//! Transactional document state. Failed commands never replace live geometry.
use std::{collections::{BTreeMap,VecDeque}, fs::{self,File,OpenOptions}, io::{Read,Write}, path::{Path,PathBuf}, sync::atomic::{AtomicU64,Ordering}};
use serde::{Deserialize,Serialize};
use crate::{kernel::{Shape,Stats}, Error,Result};
pub const MAX_OBJECTS:usize=2048;
pub const MAX_HISTORY:usize=128;
pub const MAX_PROJECT_BYTES:usize=64*1024*1024;
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {pub id:u64,pub name:String,pub color:[u8;3],pub visible:bool,pub locked:bool}
#[derive(Clone,Debug)]
pub struct Object {pub id:u64,pub name:String,pub layer:u64,pub visible:bool,pub locked:bool,pub shape:Shape}
#[derive(Clone,Debug)]
pub struct State {pub objects:BTreeMap<u64,Object>,pub layers:BTreeMap<u64,Layer>,pub current_layer:u64,token:u64}
impl Default for State {fn default()->Self{Self{objects:BTreeMap::new(),layers:BTreeMap::from([(0,Layer{id:0,name:"Default".into(),color:[101,170,190],visible:true,locked:false})]),current_layer:0,token:0}}}
pub fn name_ok(name:&str)->Result<()> {if name.trim().is_empty()||name.chars().count()>128||name.chars().any(char::is_control){Err(Error::new("Names must contain 1..128 printable characters"))}else{Ok(())}}
pub(crate) fn allocate(next:&mut u64)->Result<u64>{let id=*next;*next=next.checked_add(1).ok_or_else(||Error::new("Object identifier exhausted"))?;Ok(id)}
impl State {
    pub fn object(&self,id:u64)->Result<&Object>{self.objects.get(&id).ok_or_else(||Error::new(format!("Object #{id} does not exist")))}
    pub fn editable(&self,id:u64)->Result<&Object>{let o=self.object(id)?;let l=self.layers.get(&o.layer).ok_or_else(||Error::new("Object layer is missing"))?;if o.locked||l.locked{Err(Error::new(format!("Object #{id} or its layer is locked")))}else{Ok(o)}}
    pub fn visible(&self,o:&Object)->bool{o.visible&&self.layers.get(&o.layer).is_some_and(|l|l.visible)}
    pub fn locked(&self,o:&Object)->bool{o.locked||self.layers.get(&o.layer).is_none_or(|l|l.locked)}
    pub(crate) fn add(&mut self,next:&mut u64,name:&str,shape:Shape)->Result<u64>{
        name_ok(name)?;if self.objects.len()>=MAX_OBJECTS{return Err(Error::new("Document object limit reached"));}
        if self.layers.get(&self.current_layer).is_none_or(|l|l.locked){return Err(Error::new("Current layer is missing or locked"));}
        shape.stats()?;let id=allocate(next)?;self.objects.insert(id,Object{id,name:name.into(),layer:self.current_layer,visible:true,locked:false,shape});Ok(id)
    }
    pub(crate) fn replace(&mut self,id:u64,shape:Shape)->Result<()> {self.editable(id)?;shape.stats()?;let o=self.objects.get_mut(&id).ok_or_else(||Error::new("Object disappeared"))?;o.shape=shape;Ok(())}
    fn validate(&self)->Result<()> {
        if self.objects.len()>MAX_OBJECTS||self.layers.len()>256||!self.layers.contains_key(&0)||!self.layers.contains_key(&self.current_layer){return Err(Error::new("Invalid document structure"));}
        for (id,l) in &self.layers {if *id!=l.id{return Err(Error::new("Layer ID mismatch"));}name_ok(&l.name)?;}
        for (id,o) in &self.objects {if *id!=o.id||*id==0||!self.layers.contains_key(&o.layer){return Err(Error::new("Invalid object reference"));}name_ok(&o.name)?;}Ok(())
    }
}
#[derive(Clone)]
pub struct Session {state:State,undo:VecDeque<State>,redo:VecDeque<State>,next_id:u64,revision:u64,saved_token:u64}
impl Default for Session {fn default()->Self{Self{state:State::default(),undo:VecDeque::new(),redo:VecDeque::new(),next_id:1,revision:0,saved_token:0}}}
#[derive(Clone,Debug,Default,Serialize)]
pub struct Outcome {pub message:String,pub created:Vec<u64>}
impl Outcome {pub fn message(s:impl Into<String>)->Self{Self{message:s.into(),created:vec![]}}pub fn created(id:u64,name:&str)->Self{Self{message:format!("Created {name} #{id}"),created:vec![id]}}}
#[derive(Debug,Serialize)]
pub struct ObjectSummary {pub id:u64,pub name:String,pub layer:u64,pub visible:bool,pub locked:bool,pub stats:Stats}
#[derive(Debug,Serialize)]
pub struct Summary {pub revision:u64,pub dirty:bool,pub undo:usize,pub redo:usize,pub current_layer:u64,pub layers:Vec<Layer>,pub objects:Vec<ObjectSummary>}
impl Session {
    pub fn state(&self)->&State{&self.state}
    pub fn revision(&self)->u64{self.revision}
    pub fn dirty(&self)->bool{self.state.token!=self.saved_token}
    pub fn undo_len(&self)->usize{self.undo.len()}
    pub fn redo_len(&self)->usize{self.redo.len()}
    pub fn execute(&mut self,line:&str)->Result<Outcome>{crate::command::execute(self,line)}
    pub(crate) fn edit<F>(&mut self,f:F)->Result<Outcome> where F:FnOnce(&mut State,&mut u64)->Result<Outcome>{
        let mut candidate=self.state.clone();let mut next=self.next_id;let result=f(&mut candidate,&mut next)?;candidate.validate()?;
        let revision=self.revision.checked_add(1).ok_or_else(||Error::new("Revision exhausted"))?;candidate.token=revision;
        self.undo.push_back(self.state.clone());if self.undo.len()>MAX_HISTORY{self.undo.pop_front();}
        self.state=candidate;self.next_id=next;self.revision=revision;self.redo.clear();Ok(result)
    }
    pub fn insert(&mut self,name:&str,shape:Shape)->Result<u64>{let result=self.edit(|s,next|{let id=s.add(next,name,shape)?;Ok(Outcome::created(id,name))})?;result.created.first().copied().ok_or_else(||Error::new("Missing new object ID"))}
    pub fn undo(&mut self)->Result<Outcome>{let revision=self.revision.checked_add(1).ok_or_else(||Error::new("Revision exhausted"))?;let state=self.undo.pop_back().ok_or_else(||Error::new("Nothing to undo"))?;self.redo.push_back(self.state.clone());self.state=state;self.revision=revision;Ok(Outcome::message("Undo"))}
    pub fn redo(&mut self)->Result<Outcome>{let revision=self.revision.checked_add(1).ok_or_else(||Error::new("Revision exhausted"))?;let state=self.redo.pop_back().ok_or_else(||Error::new("Nothing to redo"))?;self.undo.push_back(self.state.clone());if self.undo.len()>MAX_HISTORY{self.undo.pop_front();}self.state=state;self.revision=revision;Ok(Outcome::message("Redo"))}
    /// Atomic macro. Geometry and metadata changes commit as one undo step.
    /// I/O, nested batches, undo and redo are intentionally excluded.
    pub fn batch(&mut self,lines:&[String])->Result<Outcome>{
        if lines.is_empty()||lines.len()>256{return Err(Error::new("A batch needs 1..256 commands"));}let mut candidate=self.clone();let mut created=Vec::new();
        for (index,line) in lines.iter().enumerate(){let tokens=crate::command::tokenize(line)?;let verb=tokens.first().map(|s|s.trim_start_matches('_').to_ascii_lowercase()).unwrap_or_default();
            if matches!(verb.as_str(),"save"|"open"|"importstep"|"exportstep"|"exportstl"|"undo"|"redo"|"demo"|"help"|"list") {return Err(Error::new(format!("{verb} is not allowed inside an atomic batch")));}
            let result=candidate.execute(line).map_err(|e|Error::new(format!("Batch command {}: {e}",index+1)))?;created.extend(result.created);
        }
        self.undo.push_back(self.state.clone());if self.undo.len()>MAX_HISTORY{self.undo.pop_front();}self.state=candidate.state;self.next_id=candidate.next_id;self.revision=candidate.revision;self.redo.clear();Ok(Outcome{message:format!("Committed {} commands as one undo step",lines.len()),created})
    }
    pub fn summary(&self)->Result<Summary>{Ok(Summary{revision:self.revision,dirty:self.dirty(),undo:self.undo.len(),redo:self.redo.len(),current_layer:self.state.current_layer,layers:self.state.layers.values().cloned().collect(),objects:self.state.objects.values().map(|o|Ok(ObjectSummary{id:o.id,name:o.name.clone(),layer:o.layer,visible:self.state.visible(o),locked:self.state.locked(o),stats:o.shape.stats()?})).collect::<Result<_>>()?})}
    pub fn project_json(&self)->Result<Vec<u8>>{let objects=self.state.objects.values().map(|o|Ok(SavedObject{id:o.id,name:o.name.clone(),layer:o.layer,visible:o.visible,locked:o.locked,brep:o.shape.to_brep()?})).collect::<Result<Vec<_>>>()?;
        let file=Project{format:"arclet-occt".into(),version:1,units:"mm".into(),current_layer:self.state.current_layer,layers:self.state.layers.values().cloned().collect(),objects};let data=serde_json::to_vec(&file)?;if data.len()>MAX_PROJECT_BYTES{return Err(Error::new("Project exceeds 64 MiB limit"));}Ok(data)}
    pub fn save(&mut self,path:&Path)->Result<Outcome>{let data=self.project_json()?;atomic_file(path,|tmp|{let mut f=OpenOptions::new().write(true).truncate(true).open(tmp)?;f.write_all(&data)?;f.sync_all()?;Ok(())})?;self.saved_token=self.state.token;Ok(Outcome::message(format!("Saved {}",path.display())))}
    pub fn open(&mut self,path:&Path)->Result<Outcome>{let mut data=Vec::new();File::open(path)?.take((MAX_PROJECT_BYTES+1)as u64).read_to_end(&mut data)?;self.load_json(&data)?;Ok(Outcome::message(format!("Opened {}",path.display())))}
    pub fn load_json(&mut self,data:&[u8])->Result<()> {
        if data.len()>MAX_PROJECT_BYTES{return Err(Error::new("Project exceeds 64 MiB limit"));}let file:Project=serde_json::from_slice(data)?;
        if file.format!="arclet-occt"||file.version!=1||file.units!="mm"{return Err(Error::new("Unsupported project format, version or units"));}if file.objects.len()>MAX_OBJECTS||file.layers.len()>256{return Err(Error::new("Project exceeds object or layer limits"));}
        let mut state=State{objects:BTreeMap::new(),layers:BTreeMap::new(),current_layer:file.current_layer,token:0};let mut highest=0;
        for l in file.layers {highest=highest.max(l.id);if state.layers.insert(l.id,l).is_some(){return Err(Error::new("Duplicate layer ID"));}}
        for o in file.objects {highest=highest.max(o.id);name_ok(&o.name)?;if o.brep.len()>MAX_PROJECT_BYTES{return Err(Error::new("BREP exceeds limit"));}let shape=Shape::from_brep(&o.brep)?;shape.stats()?;let object=Object{id:o.id,name:o.name,layer:o.layer,visible:o.visible,locked:o.locked,shape};if state.objects.insert(o.id,object).is_some(){return Err(Error::new("Duplicate object ID"));}}
        state.validate()?;let next=self.next_id.max(highest.checked_add(1).ok_or_else(||Error::new("Invalid maximum ID"))?);self.edit(|s,n|{*s=state;*n=next;Ok(Outcome::message("Loaded project"))})?;self.saved_token=self.state.token;Ok(())
    }
    pub fn export(&self,path:&Path,stl:bool)->Result<Outcome>{
        let shapes=self.state.objects.values().filter(|o|self.state.visible(o)).map(|o|o.shape.clone()).collect::<Vec<_>>();if shapes.is_empty(){return Err(Error::new("No visible objects to export"));}
        if stl && shapes.iter().any(|s|s.stats().map_or(true,|m|m.faces==0)){return Err(Error::new("STL requires faces; hide wire-only objects first"));}
        let all=Shape::compound(&shapes)?;atomic_file(path,|tmp|{let p=tmp.to_str().ok_or_else(||Error::new("Export path is not Unicode"))?;if stl{all.write_stl(p)}else{all.write_step(p)}})?;Ok(Outcome::message(format!("Exported {}",path.display())))
    }
}
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedObject {id:u64,name:String,layer:u64,visible:bool,locked:bool,brep:String}
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {format:String,version:u32,units:String,current_layer:u64,layers:Vec<Layer>,objects:Vec<SavedObject>}
static TEMP_ID:AtomicU64=AtomicU64::new(1);
struct TempPath(PathBuf);
impl Drop for TempPath {fn drop(&mut self){let _=fs::remove_file(&self.0);}}
fn atomic_file<F>(path:&Path,write:F)->Result<()> where F:FnOnce(&Path)->Result<()> {
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or_else(||Path::new("."));let name=path.file_name().ok_or_else(||Error::new("A filename is required"))?.to_string_lossy();
    let tmp_path=parent.join(format!(".{name}.{}.{}.tmp",std::process::id(),TEMP_ID.fetch_add(1,Ordering::Relaxed)));
    // Only construct the cleanup guard AFTER exclusive reservation succeeds.
    OpenOptions::new().write(true).create_new(true).open(&tmp_path)?;let tmp=TempPath(tmp_path);
    write(&tmp.0)?;File::open(&tmp.0)?.sync_all()?;fs::rename(&tmp.0,path)?;
    #[cfg(unix)]
    if let Ok(dir)=File::open(parent){let _=dir.sync_all();}Ok(())
}
