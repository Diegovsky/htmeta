
[no-cd]
test *args:
    cargo test -- --nocapture {{ args }}

doc *args:
    RUSTDOCFLAGS="--html-in-header $PWD/docs/meta/highlight.html" cargo doc {{ args }}

[no-cd]
update-tests:
    cargo test --features test_gen
