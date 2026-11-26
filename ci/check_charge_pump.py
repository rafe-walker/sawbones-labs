#!/usr/bin/env python3
"""The trust root's charge pump is fed only from main-loop toggles (#114, #144).

The relay in series with the rails is held closed by a charge pump that needs
a GPIO to keep toggling. If anything that runs on its own (a timer interrupt,
PWM, PIO, DMA, a second core) could toggle it, a hung firmware would keep the
relay closed. This grep-style lint checks the trust-root sources, with
comments stripped:

1. No interrupt, timer alarm, SysTick, PWM, PIO, DMA, second core, executor
   or async runtime is named anywhere in them.
2. `toggle_charge_pump(` is called from exactly one place, the loop in
   crates/sawbones-trust-root/src/lib.rs.
3. In the firmware, the GPIO XOR register (the pump's toggle) is written
   only inside `fn toggle_charge_pump`.

Usage: ci/check_charge_pump.py [--self-test] [ROOT]
"""
import re
import sys
import tempfile
from pathlib import Path

LOOP_FILE = "crates/sawbones-trust-root/src/lib.rs"
SOURCES = ["crates/sawbones-trust-root/src", "firmware/trust-root/src"]
FIRMWARE = "firmware/trust-root/src"

# Case-insensitive. Substrings where the word cannot hide inside an ordinary
# identifier, word matches where it could.
FORBIDDEN = [
    (re.compile(r"pwm", re.I), "PWM"),
    (re.compile(r"(?<!g)pio", re.I), "PIO"),
    (re.compile(r"dma", re.I), "DMA"),
    (re.compile(r"alarm", re.I), "a timer alarm"),
    (re.compile(r"systick|\bsyst\b", re.I), "SysTick"),
    (re.compile(r"nvic", re.I), "the interrupt controller (NVIC)"),
    (re.compile(r"#\s*\[\s*interrupt\b"), "an interrupt handler"),
    (re.compile(r"interrupt::enable|\.inte\s*\(|\binte\b", re.I), "an interrupt enable"),
    (re.compile(r"multicore|\bcore1\b|fifo", re.I), "the second core"),
    (re.compile(r"embassy|\brtic\b|executor|\basync\b|\.await\b", re.I), "an executor or async task"),
]

CALL = re.compile(r"\btoggle_charge_pump\s*\(")
DEFINITION = re.compile(r"\bfn\s+toggle_charge_pump\s*\(")
XOR = re.compile(r"gpio(_hi)?_out_xor")


def strip_comments(text):
    """Drop // comments (doc comments included). Strings with // in them are
    not used in these sources."""
    return "\n".join(line.split("//", 1)[0] for line in text.splitlines())


def function_spans(code, pattern):
    """(start, end) offsets of the bodies of every fn matching pattern."""
    spans = []
    for m in pattern.finditer(code):
        open_at = code.find("{", m.end())
        if open_at < 0:
            continue
        depth = 0
        for i in range(open_at, len(code)):
            if code[i] == "{":
                depth += 1
            elif code[i] == "}":
                depth -= 1
                if depth == 0:
                    spans.append((m.start(), i))
                    break
    return spans


def check(root):
    root = Path(root)
    errors = []
    calls = []
    for src in SOURCES:
        for path in sorted((root / src).rglob("*.rs")):
            rel = path.relative_to(root).as_posix()
            code = strip_comments(path.read_text())
            for lineno, line in enumerate(code.splitlines(), 1):
                for pattern, what in FORBIDDEN:
                    if pattern.search(line):
                        errors.append(f"{rel}:{lineno}: names {what}: {line.strip()}")
                for m in CALL.finditer(line):
                    if not DEFINITION.search(line):
                        calls.append(f"{rel}:{lineno}")
            if rel.startswith(FIRMWARE):
                inside = function_spans(code, DEFINITION)
                for m in XOR.finditer(code):
                    if not any(a <= m.start() <= b for a, b in inside):
                        lineno = code.count("\n", 0, m.start()) + 1
                        errors.append(
                            f"{rel}:{lineno}: the GPIO XOR register is written outside fn toggle_charge_pump"
                        )
    loop_calls = [c for c in calls if c.startswith(LOOP_FILE + ":")]
    if len(calls) != 1 or len(loop_calls) != 1:
        errors.append(
            "toggle_charge_pump must be called from exactly one place, the loop in "
            f"{LOOP_FILE}; found: {', '.join(calls) or 'none'}"
        )
    return errors


GOOD_LOOP = """
pub trait Board { fn toggle_charge_pump(&mut self); }
// A comment may say PWM, PIO, DMA or alarm.
fn tick<B: Board>(b: &mut B, on: bool) { if on { b.toggle_charge_pump(); } }
"""
GOOD_FW = """
impl Board for Chip {
    fn toggle_charge_pump(&mut self) {
        self.sio.gpio_out_xor().write(|w| unsafe { w.bits(1 << 17) });
    }
}
"""


def self_test():
    cases = {
        "clean tree": ({}, True),
        "PWM in the firmware": ({FIRMWARE + "/bad.rs": "fn f(p: Pin) { p.into_function::<FunctionPwm>(); }"}, False),
        "PIO block": ({FIRMWARE + "/bad.rs": "fn f(p: pac::Peripherals) { let _ = p.PIO0; }"}, False),
        "PIO pin function": ({FIRMWARE + "/bad.rs": "fn f(p: Pin) { p.into_function::<FunctionPio1>(); }"}, False),
        "plain GPIO is fine": ({FIRMWARE + "/ok.rs": "fn f(p: &P) { p.gpio(3).gpio_ctrl().write(|w| w.funcsel().sio()); }"}, True),
        "DMA channel": ({FIRMWARE + "/bad.rs": "fn f() { start_dma(); }"}, False),
        "timer alarm": ({FIRMWARE + "/bad.rs": "fn f(t: &T) { t.alarm0().write(|w| w); }"}, False),
        "interrupt handler": ({FIRMWARE + "/bad.rs": "#[interrupt]\nfn TIMER0_IRQ_0() {}"}, False),
        "NVIC unmask": ({FIRMWARE + "/bad.rs": "fn f() { unsafe { NVIC::unmask(I) } }"}, False),
        "interrupt enable register": ({FIRMWARE + "/bad.rs": "fn f(t: &T) { t.inte().write(|w| w); }"}, False),
        "second core": ({FIRMWARE + "/bad.rs": "fn f(s: &S) { s.fifo_wr().write(|w| w); }"}, False),
        "async task": ({FIRMWARE + "/bad.rs": "async fn pump() {}"}, False),
        "a second call site": ({FIRMWARE + "/bad.rs": "fn f(b: &mut B) { b.toggle_charge_pump(); }"}, False),
        "XOR outside the toggle": ({FIRMWARE + "/bad.rs": "fn f(s: &S) { s.gpio_out_xor().write(|w| w); }"}, False),
        "forbidden words only in comments": ({FIRMWARE + "/ok.rs": "// never PWM, PIO, DMA, an alarm or SysTick\nfn f() {}"}, True),
    }
    failed = []
    for name, (extra, should_pass) in cases.items():
        with tempfile.TemporaryDirectory() as tmp:
            files = {LOOP_FILE: GOOD_LOOP, FIRMWARE + "/main.rs": GOOD_FW, **extra}
            for rel, text in files.items():
                path = Path(tmp) / rel
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text)
            passed = not check(tmp)
            if passed != should_pass:
                failed.append(name)
    if failed:
        print("self-test failed: " + ", ".join(failed))
        return 1
    print(f"self-test: {len(cases)} cases ok")
    return 0


def main(argv):
    if argv and argv[0] == "--self-test":
        return self_test()
    root = argv[0] if argv else Path(__file__).resolve().parent.parent
    errors = check(root)
    for e in errors:
        print(f"error: {e}")
    if errors:
        print("\nThe charge pump may be fed only by the main loop's one call per pass (#114, #144). See the sawbones-interlock crate docs.")
        return 1
    print("ok: the charge pump is fed only from the main loop")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
