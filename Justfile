fmt:
    cargo fmt --all -- --check

build:
    cargo build --workspace --all-features --tests --bins --benches --examples

test:
    # Doc tests run separately because Nextest does not support them.
    cargo nextest run --workspace --all-features --no-fail-fast
    cargo test --workspace --all-features --doc

clippy:
    cargo clippy --workspace --all-features --tests --bins --benches --examples -- -D warnings

doc:
    cargo doc --workspace --no-deps --document-private-items

examples: example-policies example-equivocations example-nakamoto

example-policies:
    cargo run -p consensus-ccp-examples -- policies

example-equivocations:
    cargo run -p consensus-ccp-examples -- equivocations

example-nakamoto:
    cargo run -p consensus-ccp-examples -- nakamoto

package:
    cargo publish --workspace --dry-run

release level:
    cargo release {{level}} --execute

ci: fmt build clippy doc test package

clean:
    cargo clean
