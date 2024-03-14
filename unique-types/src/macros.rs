/// Create a custom counter type
#[macro_export]
macro_rules! custom_counter {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident($value:ty);
    ) => {
        $(#[$meta])*
        $v struct $name;

        /// SAFETY: with is only ever passed the GLOBAL_COUNTER
        unsafe impl $crate::unique_indices::CounterRef for $name {
            type Counter = <$value as $crate::unique_indices::CounterValue>::AtomicCounter;
            type Value = $value;

            fn with<T>(f: impl FnOnce(&Self::Counter) -> T) -> T {
                static GLOBAL_COUNTER: <$value as $crate::unique_indices::CounterValue>::AtomicCounter = $crate::unique_indices::Counter::NEW;
                f(&GLOBAL_COUNTER)
            }
        }
    };
}
