/// Responsible for simple help message:
pub struct Help;

impl Help {
    /// Show imple help message in console.
    ///
    /// # Parameters
    ///
    /// * `version: &str` - Version of app.
    pub fn show(version: &str) {
        let desc = "Tiny is a high-speed FastCGI server for WEB applications.";
        let ver = format!("tiny version: {}", version);
        let help = format!(
            "
    Usage: {} [start|stop|status|help] [-r <path to root folder>]
    
    Actions:
        start         : start server
        stop          : stop server
        status        : show server status
        help          : show this help
        
    ",
            env!("CARGO_PKG_NAME")
        );
        println!("\n{}\n{}\n{}\n", desc, ver, help);
    }
}
