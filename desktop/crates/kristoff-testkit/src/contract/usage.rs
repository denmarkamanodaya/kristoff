//! `UsageMeter` contract.

use super::assert_code;
use kristoff_core::error::ErrorCode;
use kristoff_core::model::{Usage, UsageReport};
use kristoff_core::ports::UsageMeter;

const DAY: &str = "2026-11-12";
const NEXT: &str = "2026-11-13";

/// Runs every `UsageMeter` promise against fresh meters from `make`.
pub fn usage_meter_contract<S: UsageMeter>(make: impl Fn() -> S) {
    reads_zeros_when_nothing_was_counted(&make());
    admits_questions_up_to_the_cap_per_day(&make());
    adds_tokens_to_the_day_and_the_total(&make());
}

fn reads_zeros_when_nothing_was_counted(meter: &impl UsageMeter) {
    assert_eq!(meter.read_usage(DAY).unwrap(), UsageReport::default());
}

fn admits_questions_up_to_the_cap_per_day(meter: &impl UsageMeter) {
    meter.admit_question(DAY, 2).unwrap();
    meter.admit_question(DAY, 2).unwrap();

    assert_code(meter.admit_question(DAY, 2), ErrorCode::DailyCapReached);
    meter.admit_question(NEXT, 2).unwrap();
}

fn adds_tokens_to_the_day_and_the_total(meter: &impl UsageMeter) {
    let call = Usage {
        input: 10,
        output: 20,
        cache_read: 3,
        cache_write: 4,
    };
    meter.add_tokens(DAY, &call).unwrap();
    meter.add_tokens(NEXT, &call).unwrap();

    let report = meter.read_usage(NEXT).unwrap();

    assert_eq!(report.today, call);
    assert_eq!(
        report.total,
        Usage {
            input: 20,
            output: 40,
            cache_read: 6,
            cache_write: 8
        }
    );
}
