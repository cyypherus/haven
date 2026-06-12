#[cfg(feature = "test-api")]
#[test]
fn public_api() {
    rustup_toolchain::install(public_api::MINIMUM_NIGHTLY_RUST_VERSION).unwrap();

    let rustdoc_json = rustdoc_json::Builder::default()
        .toolchain(public_api::MINIMUM_NIGHTLY_RUST_VERSION)
        .build()
        .unwrap();

    let public_api = public_api::Builder::from_rustdoc_json(rustdoc_json.clone())
        .build()
        .unwrap();

    let public_api_simplified = public_api::Builder::from_rustdoc_json(rustdoc_json)
        .omit_blanket_impls(true)
        .omit_auto_trait_impls(true)
        .omit_auto_derived_impls(true)
        .build()
        .unwrap();

    insta::assert_snapshot!("api_full", public_api);
    insta::assert_snapshot!("api_simplified", public_api_simplified);
}
