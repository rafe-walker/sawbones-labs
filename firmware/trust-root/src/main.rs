//! # sawbones-trust-root-fw: PROVISIONAL
//!
//! The firmware image of the trust root, the interlock controller on an
//! RP2350 (Cortex-M33, `thumbv8m.main-none-eabihf`). It builds in CI (#114)
//! and has not been flashed: the board comes in P1 (#144).
//!
//! All decisions live in `sawbones-trust-root` and `sawbones-interlock`,
//! which are tested on the host. This crate only connects them to the chip:
//! it implements `sawbones_trust_root::Board` for the RP2350's registers and
//! runs `TrustRoot::tick` in a plain loop, paced by polling the timer.
//!
//! ## Why no HAL
//!
//! The trust root needs a handful of peripherals: GPIO in and out, a free
//! running timer it reads, the watchdog, the true random number generator,
//! and somewhere to keep the fault record. Both maintained HALs for the chip,
//! `rp235x-hal` 0.4 and `embassy-rp` 0.10, pull in the PIO assembler
//! (`pio-proc`, which runs the `lalrpop` parser generator at build time) and
//! about a hundred crates in all, with some twenty that run code at build
//! time. They also bring PWM, PIO, DMA, timer alarms and an interrupt-driven
//! executor, which are exactly what the charge pump must never be fed from.
//! This image uses the peripheral access crate `rp235x-pac` with `cortex-m`
//! and `cortex-m-rt`: about twenty crates, six of them new build-time code.
//! The cost is a few dozen lines of register code in `rp2350.rs`, each
//! `unsafe` block with its reason.
//!
//! ## What runs
//!
//! - **No interrupts.** Nothing unmasks an interrupt in the NVIC, and there
//!   is no executor, timer alarm, SysTick or second core. The one loop in
//!   `main` is the only thing that runs, so a hung loop stops everything,
//!   the charge pump included. `ci/check_charge_pump.py` enforces it.
//! - **Outputs off from reset.** Output pins are set low before their pads are
//!   released, so they never glitch high. Panics and hard faults drive every
//!   output low and reset the chip through the watchdog.
//! - **Watchdog.** Fed once at the end of every pass. A pass that takes
//!   longer than `WATCHDOG_US` resets the chip, outputs off.
//!
//! ## Not done yet
//!
//! - **The link.** There is no supervisor link driver, so no heartbeat reply
//!   ever arrives and this image can never arm. That is the safe default
//!   until the link protocol is written.
//! - **A durable record.** The fault record lives in the watchdog's scratch
//!   registers, which survive a watchdog or soft reset but not a power cut or
//!   brown-out. After one, the record fails its check and the interlock comes
//!   up with the fault state unknown: safe, but it needs a panel clear after
//!   every cold start. Flash or ferroelectric RAM is a P1 choice (#144).
//! - **Pin map, crystal and tick period** are placeholders until the board
//!   exists. The pump's electrical side is #123.

#![cfg_attr(all(target_arch = "arm", target_os = "none"), no_std, no_main)]

#[cfg(all(target_arch = "arm", target_os = "none"))]
mod rp2350;

/// On the host this crate builds as a stub, so the workspace still builds and
/// tests everywhere. The logic it would run is tested in
/// `sawbones-trust-root`.
#[cfg(not(all(target_arch = "arm", target_os = "none")))]
fn main() {
    eprintln!(
        "sawbones-trust-root-fw runs only on the RP2350: build it with ci/build_trust_root_fw.sh"
    );
}
