#[doc(hidden)]
pub use similar_asserts::assert_eq;

#[macro_export]
macro_rules! fixture_path {
    ($name:ident, $ext:expr) => {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/",
            stringify!($name),
            $ext
        )
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
            let result = match emit_as_str(&builder, input) {
                Ok(v) => v,
                Err(e) => panic!("Failed to emit str: {}", e),
            };
            #[cfg(not(feature = "test_gen"))]
            {
                let output = include_str!(fixture_path!($name, ".html"));
                $crate::assert_eq!(output, result);
            }

            #[cfg(feature = "test_gen")]
            {
                let output = fixture_path!($name, ".html");
                std::fs::write(output, result).expect("Failed to save file");
            }
        }
    };
}

#[doc(hidden)]
pub use ron;

#[macro_export]
macro_rules! auto_html_test_fail {
    ($name:ident) => {
        auto_html_test_fail!($name, HtmlEmitter::builder());
    };
    ($name:ident, $builder: expr) => {
        #[test]
        fn $name() {
            let input = include_str!(fixture_path!($name, ".kdl"));

            let builder = $builder;
            let result = emit_as_str(&builder, input).unwrap_err();
            #[cfg(not(feature = "test_gen"))]
            {
                let output: Error =
                    ron::from_str(include_str!(fixture_path!($name, ".ron"))).unwrap();
                $crate::assert_eq!(output, result);
            }

            #[cfg(feature = "test_gen")]
            {
                let output = fixture_path!($name, ".ron");
                std::fs::write(
                    output,
                    $crate::ron::ser::to_string_pretty(&result, Default::default()).unwrap(),
                )
                .expect("Failed to save file");
            }
        }
    };
}
