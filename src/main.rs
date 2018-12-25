use filetime::FileTime;
use std::fs::{self, File};
use std::path::Path;

const USAGE: &str = "
tinge. change file access and modification times

Usage:
    tinge [-acm] [-r <file>] <file>>

Options:
    -a          Change access time
    -c          Do not create file if it exists
    -m          Change modification time
    -r <file>   Use access and modification times from this file
";

fn error(msg: &str) -> ! {
    eprintln!("Error! {}", msg);
    eprintln!("{}", USAGE);
    std::process::exit(1);
}

#[derive(Debug)]
struct Args {
    access: bool,           // -a
    no_create: bool,        // -c
    modify: bool,           // -m
    source: Option<String>, // -r file
    file: String,
}

impl Args {
    pub fn parse() -> Args {
        let mut access = None;
        let mut no_create = None;
        let mut modify = None;
        let mut replacement = None;
        let mut source = None;
        let mut file = None;

        macro_rules! check {
            ($flag:expr, $data:expr, $err:expr) => {{
                if $flag.is_some() {
                    error($err)
                }
                $flag.replace($data);
            }};
        }

        for arg in std::env::args().skip(1) {
            if arg.starts_with('-') {
                for ch in arg[1..].chars() {
                    match ch {
                        'a' => check!(access, true, "-a flag already specified"),
                        'c' => check!(no_create, true, "-c flag already specified"),
                        'm' => check!(modify, true, "-m flag already specified"),
                        'r' => check!(replacement, true, "-r flag already specified"),
                        _ => {}
                    };
                }
                continue;
            }

            let s = arg
                .chars()
                .skip_while(|c| c.is_whitespace())
                .take_while(|c| !c.is_whitespace());

            if replacement.is_some() && source.is_none() {
                source.replace(s.collect::<String>());
                continue;
            }

            if file.is_none() {
                file.replace(s.collect::<String>());
            }
        }

        if file.is_none() || file.as_ref().map(|d| d.len()) == Some(0) {
            error("a filename must be provided")
        }

        Self {
            access: access.unwrap_or_default(),
            no_create: no_create.unwrap_or_default(),
            modify: modify.unwrap_or_default(),
            source,
            file: file.unwrap(),
        }
    }
}

struct TempFile<'a>(&'a str);
impl<'a> TempFile<'a> {
    pub fn create(p: &'a str) -> Self {
        let _ = File::create(p).unwrap();
        TempFile(p)
    }
}

impl<'a> Drop for TempFile<'a> {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0);
    }
}

fn main() {
    let Args {
        access,
        no_create,
        modify,
        source,
        file,
    } = Args::parse();

    const TEMP: &str = "___touch";
    let _temp = TempFile::create(TEMP);

    let path = Path::new(&file);
    if path.exists() && no_create {
        return;
    }

    if !path.exists() {
        let _ = File::create(&file);
    }

    if !path.exists() {
        std::process::exit(1);
    }

    let df = fs::metadata(&file).unwrap();
    let dt = fs::metadata(TEMP).unwrap();

    let mut tatime = FileTime::from_last_access_time(&dt);
    let mut tmtime = FileTime::from_last_modification_time(&dt);

    if let Some(source) = source {
        let p = Path::new(&source);
        if p.exists() {
            let fi = fs::metadata(&source).unwrap();
            tatime = FileTime::from_last_access_time(&fi);
            tmtime = FileTime::from_last_modification_time(&fi);
        } else {
            error("cannot access reference file");
        }
    }

    let (fatime, fmtime) = (
        FileTime::from_last_access_time(&df),
        FileTime::from_last_modification_time(&df),
    );

    match (access, modify) {
        (true, false) => {
            let _ = filetime::set_file_times(file, tatime, fmtime);
        }
        (false, true) => {
            let _ = filetime::set_file_times(file, fatime, tmtime);
        }
        (true, true) => {
            let _ = filetime::set_file_times(file, tatime, tmtime);
        }
        _ => {}
    }
}
