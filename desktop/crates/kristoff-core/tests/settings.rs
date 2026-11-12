//! Get config and get usage: ported from `get-config` and `get-usage` tests.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::model::Usage;
use kristoff_core::ports::UsageMeter;
use kristoff_core::usecases::settings::{ConfigSource, ModelOption, get_config, get_usage};
use kristoff_testkit::world::World;

#[test]
fn shows_the_first_model_as_default_and_labels_each_id() {
    let source = ConfigSource {
        model_ids: &["claude-sonnet-5-5", "custom-model"],
        labels: &[("claude-sonnet-5-5", "Sonnet 5.5")],
        max_attached_docs: 5,
        daily_question_cap: 100,
    };

    let config = get_config(&source);

    assert_eq!(config.model, "claude-sonnet-5-5");
    assert_eq!(
        config.models,
        [
            ModelOption {
                id: "claude-sonnet-5-5".into(),
                label: "Sonnet 5.5".into()
            },
            ModelOption {
                id: "custom-model".into(),
                label: "custom-model".into()
            },
        ]
    );
    assert_eq!(
        (config.max_attached_docs, config.daily_question_cap),
        (5, 100)
    );
}

#[test]
fn reads_today_by_the_utc_day_of_the_clock() {
    let w = World::new();
    let call = Usage {
        input: 1,
        output: 2,
        cache_read: 3,
        cache_write: 4,
    };
    w.store.add_tokens("2026-11-11", &call).unwrap();
    w.store.add_tokens("2026-11-12", &call).unwrap();

    let report = get_usage(&w.store, &w.clock).unwrap();

    assert_eq!(report.today, call);
    assert_eq!(report.total.input, 2);
}
