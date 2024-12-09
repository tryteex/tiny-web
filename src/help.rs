use crate::sys::app::init::Init;

/// Responsible for simple help message:
pub(crate) struct Help;

impl Help {
    /// Show imple help message in console.
    pub(crate) fn show(init: Init) {
        println!(
            r#"
{}
{} version: {}

Usage: {} [start|stop|status|help] [-r <path to root folder>]

Actions:
    start         : start server in the background mode
    stop          : stop server
    status        : show server status
    run           : start server in interactive mode
    help          : show this help
    
Options:
    -r            : path to root folder, where located the config file "config.toml"
"#,
            init.desc, init.name, init.version, init.name
        );
    }
}
