#[macro_export]
macro_rules! include_template {
    ($path:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/", $path))
    };
}
