// copy and modified from rtic_monotoics
use rtic_time::Monotonic;

use rtic_time::{TimeoutError, TimerQueue};
use core::future::Future;
use rp2040_hal::pac::{timer, Interrupt, NVIC, RESETS, TIMER};

/// Timer implementing [`Monotonic`] which runs at 1 MHz.
pub struct Timer;

impl Timer {
    /// Start a `Monotonic` based on RP2040's Timer.
    pub fn start(
        timer: TIMER,
        resets: &RESETS,
    ) {
        resets.reset().modify(|_, w| w.timer().clear_bit());
        while resets.reset_done().read().timer().bit_is_clear() {}
        timer.inte().modify(|_, w| w.alarm_0().bit(true));

        TIMER_QUEUE.initialize(Self {});

        unsafe {
        //     rtic_monotonics::set_monotonic_prio(rp2040_pac::NVIC_PRIO_BITS, Interrupt::TIMER_IRQ_0);
            let mut nvic: rp2040_hal::pac::NVIC = core::mem::transmute(());
            // nvic.set_priority(Interrupt::TIMER_IRQ_0, rp2040_hal::pac::NVIC_PRIO_BITS);
            nvic.set_priority(Interrupt::TIMER_IRQ_0, 0);
            NVIC::unmask(Interrupt::TIMER_IRQ_0);
        }
    }

    fn timer() -> &'static timer::RegisterBlock {
        unsafe { &*TIMER::ptr() }
    }
}

static TIMER_QUEUE: TimerQueue<Timer> = TimerQueue::new();

// Forward timerqueue interface
impl Timer {
    /// Used to access the underlying timer queue
    #[doc(hidden)]
    pub fn __tq() -> &'static TimerQueue<Timer> {
        &TIMER_QUEUE
    }

    /// Timeout at a specific time.
    #[inline]
    pub async fn timeout_at<F: Future>(
        instant: <Self as Monotonic>::Instant,
        future: F,
    ) -> Result<F::Output, TimeoutError> {
        TIMER_QUEUE.timeout_at(instant, future).await
    }

    /// Timeout after a specific duration.
    #[inline]
    pub async fn timeout_after<F: Future>(
        duration: <Self as Monotonic>::Duration,
        future: F,
    ) -> Result<F::Output, TimeoutError> {
        TIMER_QUEUE.timeout_after(duration, future).await
    }

    /// Delay for some duration of time.
    #[inline]
    pub async fn delay(duration: <Self as Monotonic>::Duration) {
        TIMER_QUEUE.delay(duration).await;
    }

    /// Delay to some specific time instant.
    #[inline]
    pub async fn delay_until(instant: <Self as Monotonic>::Instant) {
        TIMER_QUEUE.delay_until(instant).await;
    }
}

impl Monotonic for Timer {
    type Instant = fugit::TimerInstantU64<1_000_000>;
    type Duration = fugit::TimerDurationU64<1_000_000>;

    const ZERO: Self::Instant = Self::Instant::from_ticks(0);
    const TICK_PERIOD: Self::Duration = Self::Duration::from_ticks(1);

    fn now() -> Self::Instant {
        let timer = Self::timer();

        let mut hi0 = timer.timerawh().read().bits();
        loop {
            let low = timer.timerawl().read().bits();
            let hi1 = timer.timerawh().read().bits();
            if hi0 == hi1 {
                break Self::Instant::from_ticks((u64::from(hi0) << 32) | u64::from(low));
            }
            hi0 = hi1;
        }
    }

    fn set_compare(instant: Self::Instant) {
        let now = Self::now();

        let max = u32::MAX as u64;

        // Since the timer may or may not overflow based on the requested compare val, we check
        // how many ticks are left.
        let val = match instant.checked_duration_since(now) {
            Some(x) if x.ticks() <= max => instant.duration_since_epoch().ticks() & max, // Will not overflow
            _ => 0, // Will overflow or in the past, set the same value as after overflow to not get extra interrupts
        };

        Self::timer()
            .alarm0()
            .write(|w| unsafe { w.bits(val as u32) });
    }

    fn clear_compare_flag() {
        Self::timer().intr().modify(|_, w| w.alarm_0().bit(true));
    }

    fn pend_interrupt() {
        NVIC::pend(Interrupt::TIMER_IRQ_0);
    }

    fn on_interrupt() {}

    fn enable_timer() {}

    fn disable_timer() {}
}

// rtic_time::embedded_hal_delay_impl_fugit64!(Timer);

// rtic_time::embedded_hal_async_delay_impl_fugit64!(Timer);

#[macro_export]
macro_rules! create_rp2040_monotonic_token {
    () => {{
        #[no_mangle]
        #[allow(non_snake_case)]
        unsafe extern "C" fn TIMER_IRQ_0() {
            $crate::rp2040_monotonics::Timer::__tq().on_monotonic_interrupt();
        }
    }};
}