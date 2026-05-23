set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

release-local:
    cargo build --release --locked
    rm -rf dist/package
    mkdir -p dist/package/bin dist/package/corpus/sqlite_parity
    cp target/release/redline-testing dist/package/bin/redline-testing
    cp corpus/sqlite_parity/generated_manifest.json dist/package/corpus/sqlite_parity/generated_manifest.json
    printf '{\n  "name": "redline-testing",\n  "version": "0.1.1",\n  "target": "linux-x86_64",\n  "binary": "bin/redline-testing",\n  "corpus": ["corpus/sqlite_parity/generated_manifest.json"],\n  "generated_by": "just release-local"\n}\n' > dist/package/release-manifest.json
    tar -C dist/package --sort=name --owner=0 --group=0 --numeric-owner -czf dist/redline-testing-0.1.1-linux-x86_64.tar.gz bin release-manifest.json corpus
    sha256sum dist/redline-testing-0.1.1-linux-x86_64.tar.gz > dist/redline-testing-0.1.1-linux-x86_64.tar.gz.sha256
