//! The RP2350 side: boot block, clocks, pins, timer, watchdog, random number
//! generator and record store, as a `Board`, and the main loop.
//!
//! Every register access here is a plain read or write from the main loop or
//! from boot. Nothing configures an interrupt.

#![allow(unsafe_code)]

use core::num::NonZeroU64;
use core::panic::PanicInfo;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use rp235x_pac as pac;
use sawbones_interlock::{HeartbeatReply, Port, Tick, RAIL_B_PORTS};
use sawbones_trust_root::{
    Board, LinkRequest, Pins, RecordWords, RecordWriteFailed, TickReport, TrustRoot, RECORD_WORDS,
};

/// The IMAGE_DEF block the boot ROM needs in the first 4 KiB of flash: one
/// item, a secure Arm executable for the RP2350, then the end of the block
/// loop (a loop of one block). The words match `rp235x-hal`'s
/// `ImageDef::secure_exe()`; `ci/build_trust_root_fw.sh` checks that the
/// built image carries them.
#[link_section = ".start_block"]
#[used]
static IMAGE_DEF: [u32; 5] = [
    0xffff_ded3, // block start marker
    0x1021_0142, // IMAGE_TYPE: EXE | SECURE | ARM | RP2350
    0x0000_01ff, // LAST item, one word of items before it
    0x0000_0000, // offset to the next block: this one
    0xab12_3579, // block end marker
];

/// Microseconds per interlock tick. A placeholder: 10 ms gives a heartbeat
/// period of 100 ms with the interlock's placeholder constants.
pub const TICK_US: u64 = 10_000;

/// The watchdog's timeout. Five ticks: a pass that stalls this long resets
/// the chip with every output off.
pub const WATCHDOG_US: u32 = 50_000;

/// The crystal on the board, in MHz. A placeholder: the Pico 2 carries 12.
const XOSC_MHZ: u32 = 12;

// Pin map. Placeholders until the board is drawn (#144). Inputs are pulled
// so that a broken wire reads as the safe position: the emergency stop is a
// normally-closed contact to ground (open or broken reads pressed); the
// switches, button and jumper close to ground; rail sense is high when live,
// pulled low, so an open sense line reads dead.
const ESTOP: u32 = 2;
const ARM_SWITCH: u32 = 3;
const KEY_SWITCH: u32 = 4;
const PANEL_BUTTON: u32 = 5;
const FIRMWARE_JUMPER: u32 = 6;
const RAIL_A_SENSE: u32 = 7;
const RAIL_B_SENSE: [u32; RAIL_B_PORTS] = [8, 9, 10, 11];
const RAIL_A_ENABLE: u32 = 12;
const RAIL_B_ENABLE: [u32; RAIL_B_PORTS] = [13, 14, 15, 16];
/// The charge-pump pin. Toggled only in `toggle_charge_pump`.
const CHARGE_PUMP: u32 = 17;

const PULLED_UP: [u32; 5] = [ESTOP, ARM_SWITCH, KEY_SWITCH, PANEL_BUTTON, FIRMWARE_JUMPER];

const fn bit(pin: u32) -> u32 {
    1 << pin
}

const fn rail_b_enable_mask() -> u32 {
    let mut mask = 0;
    let mut p = 0;
    while p < RAIL_B_PORTS {
        mask |= bit(RAIL_B_ENABLE[p]);
        p += 1;
    }
    mask
}

/// Every pin this image drives. All of them are low whenever outputs are off.
const OUTPUT_MASK: u32 = bit(RAIL_A_ENABLE) | rail_b_enable_mask() | bit(CHARGE_PUMP);

/// The RP2350 as the trust root's board.
pub struct Rp2350 {
    sio: pac::SIO,
    timer: pac::TIMER0,
    watchdog: pac::WATCHDOG,
    trng: pac::TRNG,
    /// The crystal started and the reference clock runs from it. Without it
    /// the tick is not trusted, so no nonce is given and nothing can arm.
    clock_ok: bool,
    last_tick: Tick,
}

impl Rp2350 {
    /// Brings the chip up with every output low, then starts the watchdog.
    fn take() -> Self {
        // `take` runs once, at boot, before anything else.
        let Some(p) = pac::Peripherals::take() else {
            fail_safe_reset();
        };

        // Out of reset: the GPIO banks, the timer and the generator.
        p.RESETS.reset().modify(|_, w| {
            w.io_bank0()
                .clear_bit()
                .pads_bank0()
                .clear_bit()
                .timer0()
                .clear_bit()
                .trng()
                .clear_bit()
        });
        loop {
            let done = p.RESETS.reset_done().read();
            if done.io_bank0().bit_is_set()
                && done.pads_bank0().bit_is_set()
                && done.timer0().bit_is_set()
                && done.trng().bit_is_set()
            {
                break;
            }
        }

        // Outputs first: low, then enabled, then routed to SIO, and only then
        // the pad released from isolation, so no output ever glitches high.
        // SAFETY: GPIO_OUT_CLR and GPIO_OE_SET take any 32-bit mask; bits
        // outside the output pins are not set.
        p.SIO
            .gpio_out_clr()
            .write(|w| unsafe { w.bits(OUTPUT_MASK) });
        // SAFETY: as above.
        p.SIO
            .gpio_oe_set()
            .write(|w| unsafe { w.bits(OUTPUT_MASK) });
        for pin in 0..32 {
            if OUTPUT_MASK & bit(pin) == 0 {
                continue;
            }
            let n = pin as usize;
            p.IO_BANK0.gpio(n).gpio_ctrl().write(|w| w.funcsel().sio());
            p.PADS_BANK0.gpio(n).write(|w| {
                w.ie()
                    .clear_bit()
                    .od()
                    .clear_bit()
                    .pue()
                    .clear_bit()
                    .pde()
                    .set_bit()
                    .iso()
                    .clear_bit()
            });
        }

        // Inputs.
        let inputs = PULLED_UP
            .iter()
            .map(|&pin| (pin, true))
            .chain(core::iter::once((RAIL_A_SENSE, false)))
            .chain(RAIL_B_SENSE.iter().map(|&pin| (pin, false)));
        for (pin, pull_up) in inputs {
            let n = pin as usize;
            p.IO_BANK0.gpio(n).gpio_ctrl().write(|w| w.funcsel().sio());
            p.PADS_BANK0.gpio(n).write(|w| {
                w.ie()
                    .set_bit()
                    .od()
                    .set_bit()
                    .schmitt()
                    .set_bit()
                    .pue()
                    .bit(pull_up)
                    .pde()
                    .bit(!pull_up)
                    .iso()
                    .clear_bit()
            });
        }

        let clock_ok = start_crystal(&p.XOSC, &p.CLOCKS);

        // One tick per microsecond for the timer and the watchdog.
        for tick in [p.TICKS.ticktimer0(), p.TICKS.tickwatchdog()] {
            tick.ctrl().write(|w| w.enable().clear_bit());
            // SAFETY: CYCLES is a 9-bit divider; 12 is in range.
            tick.cycles()
                .write(|w| unsafe { w.proc0_cycles().bits(XOSC_MHZ as u16) });
            tick.ctrl().write(|w| w.enable().set_bit());
        }

        p.WATCHDOG.ctrl().modify(|_, w| w.enable().clear_bit());
        // The watchdog resets every block but the two oscillators.
        p.PSM.wdsel().write(|w| {
            // SAFETY: WDSEL takes any mask of the PSM's blocks; this is all
            // of them, and the oscillators are cleared next.
            unsafe { w.bits(0x0001_ffff) };
            w.xosc().clear_bit().rosc().clear_bit()
        });
        // SAFETY: LOAD takes up to 24 bits; WATCHDOG_US is far below.
        p.WATCHDOG.load().write(|w| unsafe { w.bits(WATCHDOG_US) });
        p.WATCHDOG.ctrl().modify(|_, w| w.enable().set_bit());

        configure_trng(&p.TRNG);

        let mut board = Rp2350 {
            sio: p.SIO,
            timer: p.TIMER0,
            watchdog: p.WATCHDOG,
            trng: p.TRNG,
            clock_ok,
            last_tick: 0,
        };
        board.last_tick = board.tick_now();
        board
    }

    /// The free-running microsecond counter, read without latching.
    fn micros(&self) -> u64 {
        loop {
            let hi = self.timer.timerawh().read().bits();
            let lo = self.timer.timerawl().read().bits();
            if self.timer.timerawh().read().bits() == hi {
                return (u64::from(hi) << 32) | u64::from(lo);
            }
        }
    }

    fn tick_now(&self) -> Tick {
        self.micros() / TICK_US
    }

    /// Spins until the next tick starts. A pass that overran is not made up:
    /// the interlock sees the gap and judges the heartbeat on it.
    fn wait_for_next_tick(&mut self) {
        while self.tick_now() <= self.last_tick {}
        self.last_tick = self.tick_now();
    }

    fn pin_high(&self, pin: u32) -> bool {
        self.sio.gpio_in().read().bits() & bit(pin) != 0
    }
}

fn start_crystal(xosc: &pac::XOSC, clocks: &pac::CLOCKS) -> bool {
    xosc.ctrl().write(|w| w.freq_range()._1_15mhz());
    // About 1 ms at 12 MHz in units of 256 cycles, times 64, as the HAL does.
    let delay = (XOSC_MHZ * 1_000_000 / 1_000 / 256) * 64;
    // SAFETY: DELAY is 14 bits; 2944 is in range.
    xosc.startup()
        .write(|w| unsafe { w.delay().bits(delay as u16) });
    xosc.ctrl().modify(|_, w| w.enable().enable());
    let mut spins = 0u32;
    while xosc.status().read().stable().bit_is_clear() {
        spins += 1;
        if spins > 5_000_000 {
            return false;
        }
    }
    clocks.clk_ref_ctrl().write(|w| w.src().xosc_clksrc());
    let mut spins = 0u32;
    // The selected register is one-hot by source; the crystal is source 2.
    while clocks.clk_ref_selected().read().bits() != 1 << 2 {
        spins += 1;
        if spins > 1_000_000 {
            return false;
        }
    }
    true
}

fn configure_trng(trng: &pac::TRNG) {
    trng.rng_imr().write(|w| w.ehr_valid_int_mask().set_bit());
    // SAFETY: RND_SRC_SEL picks one of four inverter chains; 0 is the
    // shortest, the default the other drivers use.
    trng.trng_config()
        .write(|w| unsafe { w.rnd_src_sel().bits(0) });
    // SAFETY: SAMPLE_CNT1 is any 32-bit sample period; 25 is the vendor
    // default.
    trng.sample_cnt1()
        .write(|w| unsafe { w.sample_cntr1().bits(25) });
    // Every health test stays on.
    trng.trng_debug_control().write(|w| {
        w.auto_correlate_bypass()
            .clear_bit()
            .trng_crngt_bypass()
            .clear_bit()
            .vnc_bypass()
            .clear_bit()
    });
}

impl Board for Rp2350 {
    fn drive_rails(&mut self, rail_a: bool, rail_b: Option<Port>) {
        let mut on = 0;
        if rail_a {
            on |= bit(RAIL_A_ENABLE);
        }
        if let Some(Port(port)) = rail_b {
            if let Some(&pin) = RAIL_B_ENABLE.get(usize::from(port)) {
                on |= bit(pin);
            }
        }
        let enables = bit(RAIL_A_ENABLE) | rail_b_enable_mask();
        // Off first, then on, so two enables are never on together by
        // accident of ordering.
        // SAFETY: any mask; only enable pins are set.
        self.sio
            .gpio_out_clr()
            .write(|w| unsafe { w.bits(enables & !on) });
        // SAFETY: as above.
        self.sio.gpio_out_set().write(|w| unsafe { w.bits(on) });
    }

    fn toggle_charge_pump(&mut self) {
        // SAFETY: any mask; only the pump pin is set.
        self.sio
            .gpio_out_xor()
            .write(|w| unsafe { w.bits(bit(CHARGE_PUMP)) });
    }

    fn read_record(&mut self) -> Option<RecordWords> {
        let wd = &self.watchdog;
        let words: RecordWords = [
            wd.scratch0().read().bits(),
            wd.scratch1().read().bits(),
            wd.scratch2().read().bits(),
            wd.scratch3().read().bits(),
        ];
        Some(words)
    }

    fn write_record(&mut self, words: &RecordWords) -> Result<(), RecordWriteFailed> {
        const _: () = assert!(RECORD_WORDS == 4);
        let wd = &self.watchdog;
        // The scratch registers take the write at once; the loop reads it
        // back before trusting it.
        // SAFETY: the scratch registers hold any 32-bit value.
        wd.scratch0().write(|w| unsafe { w.bits(words[0]) });
        // SAFETY: as above.
        wd.scratch1().write(|w| unsafe { w.bits(words[1]) });
        // SAFETY: as above.
        wd.scratch2().write(|w| unsafe { w.bits(words[2]) });
        // SAFETY: as above.
        wd.scratch3().write(|w| unsafe { w.bits(words[3]) });
        Ok(())
    }

    fn hardware_nonce(&mut self) -> Option<NonZeroU64> {
        if !self.clock_ok {
            return None;
        }
        let trng = &self.trng;
        for _attempt in 0..4 {
            trng.rnd_source_enable().write(|w| w.rnd_src_en().set_bit());
            let mut spins = 0u32;
            let valid = loop {
                if trng.trng_valid().read().ehr_valid().bit_is_set() {
                    break true;
                }
                if trng.trng_busy().read().trng_busy().bit_is_clear() {
                    // Idle without a result: a health test failed.
                    break false;
                }
                spins += 1;
                if spins > 1_000_000 {
                    break false;
                }
            };
            let isr = trng.rng_isr().read();
            let healthy = valid
                && isr.autocorr_err().bit_is_clear()
                && isr.crngt_err().bit_is_clear()
                && isr.vn_err().bit_is_clear();
            let lo = u64::from(trng.ehr_data4().read().bits());
            // Reading the last result word clears the result registers.
            let hi = u64::from(trng.ehr_data5().read().bits());
            trng.rnd_source_enable()
                .write(|w| w.rnd_src_en().clear_bit());
            trng.rst_bits_counter()
                .write(|w| w.rst_bits_counter().set_bit());
            trng.rng_icr().write(|w| {
                w.autocorr_err()
                    .set_bit()
                    .crngt_err()
                    .set_bit()
                    .vn_err()
                    .set_bit()
                    .ehr_valid()
                    .set_bit()
            });
            if healthy {
                if let Some(nonce) = NonZeroU64::new((hi << 32) | lo) {
                    return Some(nonce);
                }
            } else {
                trng.trng_sw_reset().write(|w| w.trng_sw_reset().set_bit());
                // A read after the soft reset gives it the delay it needs.
                let _ = trng.trng_sw_reset().read();
                configure_trng(trng);
            }
        }
        None
    }

    fn now(&mut self) -> Tick {
        self.tick_now()
    }

    fn sample(&mut self) -> Pins {
        let mut rail_b_sensed = [false; RAIL_B_PORTS];
        for (sensed, &pin) in rail_b_sensed.iter_mut().zip(RAIL_B_SENSE.iter()) {
            *sensed = self.pin_high(pin);
        }
        Pins {
            estop_pressed: self.pin_high(ESTOP),
            arm_switch_on: !self.pin_high(ARM_SWITCH),
            key_switch_on: !self.pin_high(KEY_SWITCH),
            panel_button_pressed: !self.pin_high(PANEL_BUTTON),
            rail_a_sensed: self.pin_high(RAIL_A_SENSE),
            rail_b_sensed,
            firmware_jumper_fitted: !self.pin_high(FIRMWARE_JUMPER),
        }
    }

    fn heartbeat_reply(&mut self) -> Option<HeartbeatReply> {
        // No link driver yet: no reply ever arrives, so nothing can arm.
        None
    }

    fn link_request(&mut self) -> Option<LinkRequest> {
        None
    }

    fn report(&mut self, _report: &TickReport) {
        // No link or panel display yet.
    }

    fn feed_watchdog(&mut self) {
        // SAFETY: LOAD takes up to 24 bits; WATCHDOG_US is far below.
        self.watchdog
            .load()
            .write(|w| unsafe { w.bits(WATCHDOG_US) });
    }
}

#[entry]
fn main() -> ! {
    let mut root = TrustRoot::new(Rp2350::take());
    loop {
        root.tick();
        root.board_mut().wait_for_next_tick();
    }
}

/// Drives every output low, then resets the chip through the watchdog, which
/// keeps the scratch registers (and so the fault record). If the watchdog
/// cannot, the core's own reset request follows.
fn fail_safe_reset() -> ! {
    // SAFETY: called only on the way to a reset. The register blocks are
    // fixed addresses; the writes clear the output pins and trigger the
    // watchdog, and nothing runs after them.
    unsafe {
        let sio = &*pac::SIO::ptr();
        sio.gpio_out_clr().write(|w| w.bits(OUTPUT_MASK));
        let watchdog = &*pac::WATCHDOG::ptr();
        watchdog.ctrl().modify(|_, w| w.trigger().set_bit());
    }
    cortex_m::peripheral::SCB::sys_reset()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    fail_safe_reset()
}

#[exception]
unsafe fn HardFault(_frame: &ExceptionFrame) -> ! {
    fail_safe_reset()
}
