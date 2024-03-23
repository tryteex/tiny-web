/// Actions (web controllers)
pub mod app;

fn main() {
    tiny_web::run(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
        || {
            tiny_web_macro::addfn!();
        },
    );
}
