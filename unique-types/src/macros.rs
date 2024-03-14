/// Create a custom counter type
#[macro_export]
macro_rules! custom_counter {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident;
    ) => {
        $crate::custom_counter! {
            $($meta)*
            $v struct $name(());
        }
    };
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
            type TypeTraits = ();

            fn with<T>(f: impl FnOnce(&Self::Counter) -> T) -> T {
                static GLOBAL_COUNTER: <$name as $crate::unique_indices::CounterRef>::Counter = $crate::unique_indices::Counter::NEW;
                f(&GLOBAL_COUNTER)
            }
        }
    };
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident($value:ty);
        with_reuse $reuse:ty
    ) => {
        $(#[$meta])*
        $v struct $name;

        /// SAFETY: with is only ever passed the GLOBAL_COUNTER
        unsafe impl $crate::unique_indices::CounterRef for $name {
            type Counter =
                $crate::reuse::ReuseCounter<
                    <$value as $crate::unique_indices::CounterValue>::AtomicCounter,
                    $reuse,
            >;
            type Value = $value;
            type TypeTraits = ();

            fn with<T>(f: impl FnOnce(&Self::Counter) -> T) -> T {
                static GLOBAL_COUNTER: <$name as $crate::unique_indices::CounterRef>::Counter = $crate::unique_indices::Counter::NEW;
                f(&GLOBAL_COUNTER)
            }
        }
    };
}

/// Create a custom thread local counter type
#[macro_export]
#[cfg(feature = "std")]
macro_rules! custom_thread_local_counter {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident;
    ) => {
        $crate::thread_local_custom_counter! {
            $($meta)*
            $v struct $name(());
        }
    };
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident($value:ty);
    ) => {
        $(#[$meta])*
        $v struct $name;

        /// SAFETY: with is only ever passed the GLOBAL_COUNTER
        unsafe impl $crate::unique_indices::CounterRef for $name {
            type Counter = <$value as $crate::unique_indices::CounterValue>::CellCounter;
            type Value = $value;
            type TypeTraits = *mut ();

            fn with<T>(f: impl FnOnce(&Self::Counter) -> T) -> T {
                $crate::std::thread_local! {
                    static GLOBAL_COUNTER: <$name as $crate::unique_indices::CounterRef>::Counter = const { $crate::unique_indices::Counter::NEW };
                }
                GLOBAL_COUNTER.with(f)
            }
        }
    };
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident($value:ty);
        with_reuse $reuse:ty
    ) => {
        $(#[$meta])*
        $v struct $name;

        /// SAFETY: with is only ever passed the GLOBAL_COUNTER
        unsafe impl $crate::unique_indices::CounterRef for $name {
            type Counter =
                $crate::reuse::ReuseCounter<
                    <$value as $crate::unique_indices::CounterValue>::CellCounter,
                    $reuse,
            >;
            type Value = $value;
            type TypeTraits = *mut ();

            fn with<T>(f: impl FnOnce(&Self::Counter) -> T) -> T {
                $crate::std::thread_local! {
                    static GLOBAL_COUNTER: <$name as $crate::unique_indices::CounterRef>::Counter = const { $crate::unique_indices::Counter::NEW };
                }
                GLOBAL_COUNTER.with(f)
            }
        }
    };
}
