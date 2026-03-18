/// No-op metrics stubs — avoids pulling in solana-metrics which drags in
/// heavy networking deps. Replace with real metrics if desired later.
#[macro_export]
macro_rules! inc_new_counter_debug {
    ($label:expr, $val:expr) => {};
    ($label:expr, $val:expr, $($args:expr),*) => {};
}

#[macro_export]
macro_rules! inc_new_counter_info {
    ($label:expr, $val:expr) => {};
    ($label:expr, $val:expr, $($args:expr),*) => {};
}

#[macro_export]
macro_rules! datapoint_info {
    ($name:literal $(, ($key:literal, $val:expr, $type:ty))*) => {};
}
