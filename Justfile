
[no-cd]
test *args:
    cargo test -- --nocapture {{ args }}

[no-cd]
update-tests:
    cargo test --features test_gen
