# Testing

`cargo test` runs the unit tests and `tests/manifest.rs`.

## Click to retry end to end

`view::tests::updater_api_test::clicking_retry_starts_one_more_install` drives the launcher through a failed install and a retry against a local server. It needs `Updater::with_api`, which egui_bench has from 0.10.1. The published 0.10.0 lacks it, so the test sits behind the `updater-api-test` feature, off by default.

Until egui_bench 0.10.1 is released, run it against a checkout of egui_bench on the `issue-1010-updater-api-base` branch:

```
cargo test --features updater-api-test --config 'patch.crates-io.egui_bench.path="<egui_bench checkout>"'
```

Once 0.10.1 is released, raise the egui_bench requirement in `Cargo.toml` to 0.10.1, drop the feature and remove the `cfg` gate on the module.
