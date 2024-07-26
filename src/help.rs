/// Responsible for simple help message:
pub(crate) struct Help;

impl Help {
    /// Show imple help message in console.
    ///
    /// # Parameters
    ///
    /// * `name: &str` - Name of app.
    /// * `version: &str` - Version of app.
    /// * `desc: &str` - Desciption of app.
    pub fn show(name: &str, version: &str, desc: &str) {
        let desc = desc.to_owned();
        let ver = format!("{} version: {}", name, version);
        let help = format!(
            "
    Usage: {} [start|stop|status|help] [-r <path to root folder>]
    
    Actions:
        start         : start server
        stop          : stop server
        status        : show server status
        help          : show this help
        
    ",
            name
        );
        println!("\n{}\n{}\n{}\n", desc, ver, help);
    }
}
