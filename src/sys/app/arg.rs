use std::{env, io::Error, path::PathBuf, sync::Arc};

#[derive(Debug)]
pub(crate) enum Mode {
    Help,
    Start,
    Stop,
    Status,
    Run,
}

#[derive(Debug)]
pub(crate) struct Arg {
    pub mode: Mode,
    pub exe: PathBuf,
    pub root: Arc<PathBuf>,
}

impl Arg {
    pub(crate) fn get() -> Result<Arg, Error> {
        let exe = env::current_exe()?;

        #[cfg(not(debug_assertions))]
        let mut root = {
            let mut root = exe.clone();
            root.pop();
            root
        };
        #[cfg(debug_assertions)]
        let mut root = env::current_dir()?;

        let mut mode = Mode::Help;
        let mut args = env::args();
        while let Some(a) = args.next() {
            match a.as_ref() {
                "start" => mode = Mode::Start,
                "stop" => mode = Mode::Stop,
                "status" => mode = Mode::Status,
                "run" => mode = Mode::Run,
                "-r" => match args.next() {
                    Some(path) => root = path.into(),
                    None => break,
                },
                _ => {}
            }
        }

        Ok(Arg { mode, exe, root: Arc::new(root) })
    }
}
