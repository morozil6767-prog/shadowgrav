# WS6 files held out of the build

These files are the WS6 additions and the supplied bundle. They are not part of the Cargo workspace.

`xcheck_dump.rs` calls `Validation::graded()`. This tree has no graded quadrature. That API is WS3 and is not ported here. The file is stored in this directory instead of `shadowgrav-core/examples/`, so `cargo test` does not compile it. Nothing is `#[ignore]`d or `cfg`-gated to hide that.

`properties.rs` is the five std-only property tests. It is preserved here and is not registered as a Cargo test. A previous session ran it against this library; that result is not re-run by this commit.

`oracle/xcheck_random.py` drives `xcheck_dump`. It was not run in the commit that added this directory.

`bundle/` is the uploaded WS6 bundle, byte-for-byte. `sha256sum -c sg-ws6-sha256-manifest.txt` matched the six listed files when the files were copied. The bundle's own library sources include the graded tier. They were not copied over `shadowgrav-core`.
