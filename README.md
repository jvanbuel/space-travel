# space-travel

A Rust port of Ken Thompson's *Space Travel* (1969), playable in the terminal
([ratatui](https://ratatui.rs) + crossterm) and in the browser
([ratzilla](https://github.com/ratatui/ratzilla) + WebAssembly).

> Status: skeleton only. The game loop, input and rendering plumbing work on
> both targets; the game itself still has to be ported.

## Running it

Prerequisites (once):

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
```

Terminal:

```sh
cargo run --release
```

Browser:

```sh
trunk serve --release     # then open http://127.0.0.1:8080
trunk build --release     # static site in dist/, deployable anywhere
```

`q` or `Esc` quits the terminal version.

## Layout

| File | Role |
|------|------|
| `src/app.rs` | **All game code goes here.** `App::update(dt)`, `App::on_key(key)`, `App::draw(frame)`. Platform-independent. |
| `src/native.rs` | Terminal front end: ~30 fps loop, maps crossterm keys to `app::Key`. |
| `src/web.rs` | Browser front end: runs on `requestAnimationFrame`, maps ratzilla keys to `app::Key`. |
| `src/main.rs` | Picks the front end with `cfg(target_arch = "wasm32")`. |
| `index.html` | Trunk entry point; keeps the terminal grid focused so it receives keys. |

As long as `app.rs` uses only `ratatui` (and no `std::time`, threads, or file
I/O), it compiles for both targets unchanged.

### Things worth knowing about the platforms

- **Input is key presses only.** The browser sends `keydown` (with OS
  key-repeat while held), and most terminals report presses, not releases. So
  model controls as "each press does X", not "while held".
- **Drawing:** `ratatui::widgets::canvas::Canvas` with `Marker::Braille` gives
  2×4 dots per character cell, the closest thing to a vector display you get
  in a terminal. Use `ctx.draw(&Line { .. })`, `Points`, `Circle`, and
  `ctx.print(x, y, "text")` for labels.
- **Aspect ratio:** cells are about twice as tall as wide. If the canvas is
  `w × h` cells, give it world bounds in a ratio of about `w : 2h` or circles
  come out squashed.
- **Time:** both front ends pass real elapsed seconds to `update`. If you want
  to reproduce the original's fixed-tick behaviour, accumulate `dt` and step
  the simulation in fixed increments.
- **wasm:** `Cargo.toml` pulls in `critical-section` with the `std` feature for
  wasm32. ratatui needs it to link there; don't remove it.

## Getting the original source

The recovered PDP-7 source lives in Warren Toomey's
[pdp7-unix](https://github.com/DoctorWkt/pdp7-unix) restoration (the repository itself is GPLv3; see the licence note below):

```sh
git clone https://github.com/DoctorWkt/pdp7-unix.git
```

- `src/cmd/st1.s` … `st7.s`: Space Travel (~1,400 lines).
- `src/cmd/fop.s`: the floating-point routines the game calls (`fld`, `fmp`, …).
- `misc/asm_syntax.txt`: **read this first.** It explains the assembler's syntax.
- `tools/as7`: the assembler. The README explains running the whole system
  (including the GRAPHIC-2 display) under SimH, which is the best way to
  check your port against the real thing.
- `scans/`: unaltered OCR transcriptions of the original listings (page
  images at <https://www.tuhs.org/Archive/Distributions/Research/McIlroy_v0/>).
  The files in `src/cmd/` carry `"** <pdf> page N` markers pointing back to
  them.

**Licence:** this is not straightforward. The pdp7-unix repo is GPLv3, but
its `scans/README.md` says the original Unix code is probably © 1970 Micro
Focus and "certainly not under the GPLv3 license". A Rust translation would be
derived from that code. Look into its status before you publish the port or
pick a licence; `Cargo.toml` currently says MIT as a placeholder.

## Reading PDP-7 assembly: a starter kit

From `misc/asm_syntax.txt` and the PDP-7 manual:

- **Machine:** 18-bit words, one accumulator (`AC`), a one-bit link (`L`, the
  carry), and with the EAE option an `MQ` register for multiply/divide.
- **Comments** start with `"`. **Numbers** with a leading `0` are octal;
  otherwise decimal.
- **Labels:** `name:`. Single digits are local labels. `1f` / `1b` mean the
  next / previous label `1`.
- **No immediate mode:** constants live in memory. A bare `-32` on its own line
  is a `law` (load-AC-with-word) instruction that loads -32 into AC.
- **Core instructions:** `lac x` loads AC from x, `dac x` stores AC to x,
  `tad x` does a two's-complement add, `add x` a one's-complement add,
  `and x`, `xor x`, `isz x` (increment, skip next if zero: the loop counter
  idiom), `jmp x`, `jms x` (call: stores the return address at `x`, starts
  at `x+1`, returns with `jmp i x`). `i` means indirect.
- **Skips** like `sza`, `sna`, `spa`, `sma`, `szl`, `snl` skip the next
  instruction on a condition. That's how every `if` is built.
- **Floating point** (`fld`, `fmp`, `fad`, `fst`, …) is software, provided
  by `fop.s`. In Rust these become plain `f64` arithmetic.
- **`sys ...`** is a Unix system call (I/O, display, and so on).

## Suggested porting workflow

1. **Read before you translate.** Go through `st1.s`–`st7.s` once and write
   down, in your own notes, what each routine does and which variables it
   touches. The original author left comments like `" while (cplan != 0) {`
   that help a lot.
2. **Port the data first:** the tables of bodies and ship state become Rust
   structs in `app.rs`.
3. **Then the simulation step.** Get it running headless and print state, or
   write a unit test, before you touch drawing. Compare against SimH if you
   can.
4. **Then drawing.** Map the display-list code to `Canvas` calls.
5. **Then controls**, mapped to `app::Key`. Remember press-only input.
6. Translate by *meaning*, not instruction by instruction: `AC`/`MQ`
   juggling, `isz` loops and the floating-point interpreter calls all collapse
   into ordinary Rust expressions.
