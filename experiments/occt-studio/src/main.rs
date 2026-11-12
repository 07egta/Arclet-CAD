use std::{io::{self,BufRead},path::Path};
use arclet_occt_studio::{command,model::Session,Error,Result};
fn main(){if let Err(e)=run(){eprintln!("ERROR: {e}");std::process::exit(1);}}
fn run()->Result<()>{
    let args=std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|s|s=="--desktop") || (args.is_empty() && cfg!(feature="desktop")) {
        #[cfg(feature="desktop")]
        {return arclet_occt_studio::ui::run(args.iter().any(|s|s=="--demo"));}
        #[cfg(not(feature="desktop"))]
        {return Err(Error::new("Build with --features desktop to enable the native window"));}
    }
    let mut session=Session::default();
    match args.first().map(String::as_str){
        Some("--demo")=>{if args.len()!=2{return Err(Error::new("Usage: --demo OUTPUT_DIRECTORY"));}let dir=Path::new(&args[1]);std::fs::create_dir_all(dir)?;command::demo(&mut session)?;
            session.save(&dir.join("showcase.arcocct"))?;session.export(&dir.join("showcase.step"),false)?;session.export(&dir.join("showcase.stl"),true)?;
            let json=serde_json::to_string_pretty(&session.summary()?)?;std::fs::write(dir.join("measurements.json"),&json)?;println!("{json}");},
        Some("--batch")|Some("--atomic")=>{if args.len()!=2{return Err(Error::new("Usage: --batch FILE or --atomic FILE"));}let data=std::fs::read_to_string(&args[1])?;
            if data.len()>1024*1024{return Err(Error::new("Command script exceeds 1 MiB"));}let lines=data.lines().filter(|l|!l.trim().is_empty()&&!l.trim_start().starts_with('#')).map(str::to_owned).collect::<Vec<_>>();
            if args[0]=="--atomic"{println!("{}",session.batch(&lines)?.message);}else{for (i,line)in lines.iter().enumerate(){let result=session.execute(line).map_err(|e|Error::new(format!("Line {}: {e}",i+1)))?;println!("{}",result.message);}}
            println!("{}",serde_json::to_string_pretty(&session.summary()?)?);},
        Some("--headless")=>{for line in io::stdin().lock().lines(){let line=line?;if line.trim().is_empty(){continue;}match session.execute(&line){Ok(r)=>println!("{}",serde_json::to_string(&r)?),Err(e)=>println!("{}",serde_json::json!({"error":e.to_string()}))}}},
        Some("--version")=>println!("Arclet OCCT Studio 0.1.0 / OCCT {}",arclet_occt_studio::kernel::version()),
        Some("--help")|None=>{println!("Arclet OCCT Studio - experimental Rust application, real Open CASCADE geometry\n\n--desktop [--demo]        Native desktop (build with --features desktop)\n--demo DIR                Create and export the exact-geometry showcase\n--batch FILE              Run commands in order, fail on first error\n--atomic FILE             Apply modeling commands as one transaction\n--headless                Read commands from standard input\n--version                 Print application and kernel versions\n\nCommands:\n{}",command::HELP.iter().map(|h|h.1).collect::<Vec<_>>().join("\n"));},
        _=>return Err(Error::new("Unknown option. Use --help.")),
    }Ok(())
}
