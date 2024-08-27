#[doc(hidden)]
pub use similar_asserts::assert_eq;


#[macro_export]
macro_rules! fixture_path {
    ($name:ident, $ext:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/", stringify!($name), $ext)
    };
}


#[macro_export]
macro_rules! auto_html_test {
    ($name:ident) => {
        auto_html_test!($name, HtmlEmitter::builder());
    };
    ($name:ident, $builder: expr) => {
        #[test]
        fn $name() {
            let input = include_str!(fixture_path!($name, ".kdl"));

            let builder = $builder;
            let result = emit_as_str(&builder, input);
            #[cfg(not(feature="test_gen"))]
            {
                let output = include_str!(fixture_path!($name, ".html"));
                $crate::assert_eq!(output, result);
            }

            #[cfg(feature="test_gen")]
            {
                let output = fixture_path!($name, ".html");
                std::fs::write(output, result).expect("Failed to save file");
            }
        }
    };
}

