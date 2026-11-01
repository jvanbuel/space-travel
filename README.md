# space-travel

A Rust port of Ken Thompson's *Space Travel* (1969), playable in the terminal
([ratatui](https://ratatui.rs) + crossterm) and in the browser (WebAssembly,
drawn on an HTML canvas like the original's vector display).

It has two engines, switched with `m`:

- **1970 machine code** (the default): the original program, assembled from
  [`original/`](original/) and run on a small PDP-7 emulator, with its own
  software floating point. Its arithmetic is the original's bit for bit,
  quirks included (see [The 1970 engine](#the-1970-engine)).
- **Rust, 64-bit floats**: `src/game.rs`, a translation that follows the
  original routine by routine; see [Notes on the port](#notes-on-the-port)
  for the places where the listing had to be interpreted.

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

## Playing

You start landed on Earth (`L` at the bottom right), at scale `+00`. The view
is centred on the ship and rotates with its heading. The bottom readout shows
the body you are flying relative to (whichever pulls on you hardest), the
scale, and `L` (landed) or `CL` (crash landing). The short bar under the view
is the original's horizontal-velocity indicator.

| Key | Original push button | Action |
|-----|----------------------|--------|
| `↓` / `s` | 2 | Thrust "forward" (along the heading, toward the bottom of the view) |
| `↑` / `w` | 3 | Thrust backward (toward the top of the view) |
| `←` / `a`, `→` / `d` | 5, 4 | Rotate 1.2° per control poll (5 polls per tick) |
| `+` / `=` / `x` | 7 | Zoom in (`scale--`) |
| `-` / `z` | 6 | Zoom out (`scale++`) |
| `r` / `Enter` | 1 | Clear a crash (does nothing otherwise) |
| `n` | (none) | New game, parked on Earth (not in the original) |
| `m` | (none) | Switch engine (1970 machine code / Rust) and start a new game |
| `q` / `Esc` | 0 | Quit (terminal) |

One screen unit is 2^scale Earth radii, and **thrust is half a screen unit per
tick², so it scales with the zoom**. At `+00` one burst sends you off at
thousands of km/s. Zoom in to around `-18` (thrust just over 1 g) to take off
gently, and land below about 1 km/s or it counts as a crash. After a crash the
simulation freezes until you press `r`.

On touch screens the browser version shows on-screen buttons: turn, reset and new game
at the bottom left, zoom and thrust at the bottom right (both on the right in
landscape). Hold a turn or thrust button for continuous input.

Held keys: terminals and browsers only report key presses (with key repeat),
so each press holds the button for 0.12 s and key repeat keeps it held.

## Layout

| File | Role |
|------|------|
| `src/game.rs` | The simulation, translated from `st1.s`–`st5.s`. Builds a display list per tick. |
| `src/pdp7/mod.rs` | A PDP-7 CPU (memory reference, operate group, EAE), following SimH's PDP-7 simulator. |
| `src/pdp7/original.rs` | Runs the original program: plays PDP-7 Unix (system calls, push buttons) and decodes its Graphic-II display list. |
| `src/pdp7/image.rs` | The assembled 1970 program (generated; see below). |
| `src/sim.rs` | The `Sim` trait both engines implement for the front ends. |
| `tools/assemble.py` | Assembles `original/*.s` in the original build order into `src/pdp7/image.rs`. Run it from the repo root after changing `original/`; `--check` verifies the file is current. |
| `src/tables.rs` | The planet tables from `st6.s` and constants from `st7.s`, kept as raw PDP-7 words and decoded at start-up. |
| `src/app.rs` | Fixed-rate ticking (20 Hz), key mapping, and the engine switch. Platform-independent. |
| `src/native.rs` | Terminal front end: ~30 fps loop, maps crossterm keys to `app::Key`. |
| `src/tui.rs` | Terminal renderer: the display list on ratatui's Braille canvas. |
| `src/web.rs` | Browser front end: keys from `window`, a `requestAnimationFrame` loop, and the canvas renderer. |
| `src/main.rs` | Picks the front end with `cfg(target_arch = "wasm32")`. |
| `index.html` | Trunk entry point: the canvas, touch controls and the how-to-play panel. |

`app.rs`, `game.rs` and `tables.rs` use no platform APIs (no `std::time`,
threads or file I/O), so they compile for both targets unchanged.

### Things worth knowing about the platforms

- **Input is key presses only.** The browser sends `keydown` (with OS
  key-repeat while held), and most terminals report presses, not releases. So
  model controls as "each press does X", not "while held".
- **Drawing in the terminal:** `ratatui::widgets::canvas::Canvas` with
  `Marker::Braille` gives 2×4 dots per character cell, the closest thing to a
  vector display a terminal can do. Cells are about twice as tall as wide, so
  the view is given twice as many columns as rows.
- **Drawing in the browser:** the original drew on a Graphic-II vector
  display: a beam tracing lines at four intensities, on a phosphor that glowed
  and faded. `web.rs` draws the same display list on a canvas in the display's
  own coordinates (0–1023, y up): each line is stroked twice (a faint halo,
  then the line), body centres are glowing dots, each frame fades the last one
  instead of erasing it, and the `L`/`CL` flag blinks as in the listing
  (`blink on`).
- **Time:** both front ends pass real elapsed seconds to `update`. If you want
  to reproduce the original's fixed-tick behaviour, accumulate `dt` and step
  the simulation in fixed increments.

## The 1970 engine

The original program runs as machine code: `tools/assemble.py` (written for
this port from the pdp7-unix project's description of the Unix v0 assembler)
assembles `st1.s`–`st4.s`, `fop.s`, `st5.s`–`st7.s` into 3,014 words loaded at
octal 10000, the same layout as the 1970 build. `src/pdp7/mod.rs` executes it:
the instruction semantics follow SimH's PDP-7 code, including the Extended
Arithmetic Element (hardware multiply, divide, normalise) that `fop.s` uses.

The host plays the operating system and the display. The program asks
`sysloc` for the push-button word and reads its buttons from there; `capt`
and `time` do nothing (the frame rate replaces the delay); each pass of
`loop:` ends at the top of the loop, and the display list it built in memory
is decoded into lines and dots: `dsetx`/`dsety`, long vectors (`vecx`,
`vecy`), short vectors (outlines), `dsplanet`'s incremental steps (a dot) and
intensity words. The texts are read from the list's header.

One pass is about 60,000–100,000 instructions, 0.2–0.3 ms on a laptop. On a
real PDP-7 (1.75 µs memory cycle, most instructions 2 cycles, EAE operations
longer) that is very roughly a third to half a second, so the original
probably ran at a few frames per second; the port runs 20.

Tests check the emulator through the original routines (hardware multiply
and divide exact over random operands; `fmp`, `fad` and `sqrt` good to the
format's 35 bits) and compare a take-off in both engines. The comparison
turned up two quirks of the 1969 code, both in `fop.s` as listed (the
restored file matches the scan's OCR there):

- **`fdv` is only good to about 1 part in 30,000.** It adds 1 to the high
  quotient word before its final shift whenever its correction count is not
  negative, which it never is (`6: lac 2b; sma; tad d1`), and it shifts the
  divisor with `ral` without clearing the link, so the caller's carry bit can
  slip in (`sfdv` uses `ral cll`). The scan has a box around that `ral` and
  notes and arrows around the correction code.
- **`fcp` misjudges close numbers.** When exponents and high words are equal
  it returns `ans - hls`, the difference of the low words, as an 18-bit
  word; a difference of 2¹⁷ or more has the sign bit set and reads as
  "less". The game compares its distance from the planet with the planet's
  radius this way, so a ship climbing from Earth gets "landed" again at about
  60 m, and the landing code (which rescales the position by `rpar / dpar`
  through the biased `fdv`) puts it down 146 m up. The Rust engine, comparing
  f64s, doesn't do this.

A third shows on screen: zoomed far in, the 1970 engine draws a planet's
surface as a shallow V instead of a flat arc. `surf` steps round the outline
with `sfmp`, which keeps 17 bits, so `vv = 1 - v²/2` (just under 1) truncates
to 1 - 2⁻¹⁷ and every step pulls the outline in by r·2⁻¹⁷: 2 units per step
for Earth at scale -18.

## Notes on the port

- **Float format.** `fop.s` floats are three words: exponent, sign + high
  mantissa, low mantissa, with the mantissa in [0.5, 1). `tables::pdp7`
  decodes them; unit tests check known values (1.0, 2π, sin 1.2°, Earth's
  radius and distance from the Sun).
- **Units.** Lengths are Earth radii and one tick is about one second: Earth's
  `accl` is its real GM in R⊕³/s², and surface gravity comes out at 9.8 m/s².
  The port runs 20 ticks per real second.
- **Planets barely move.** `pww` is used as the per-tick rotation angle, but
  its values equal GM/r³ (ω², not ω). With one-second ticks the bodies are
  effectively fixed. The port keeps the original numbers.
- **`ascale`** is the single word `-1`, so `fld` reads its mantissa from
  `sdphi` that follows it, giving about -0.5. Reproduced as such.
- **`crash: -028`** is not valid octal. The port reads it as -(2·8+8) = -24,
  i.e. a crash above about 1.1 km/s.
- **`f2pi` / `pid10`** have low words missing their octal prefix; read as
  octal (it only affects the last bits).
- **Screen orientation.** GRAPHIC-2 y is taken to point up, as the listing's
  comments say. Then the view is a pure rotation with the heading pointing
  down the screen, so "forward" thrust moves you down the screen, and the
  arrow keys are mapped so ↓ is forward.
- **Never-set features.** `accflg` and the heading lock (`locflg`) are marked
  "how to set?" in the listing and have no button; their code is ported but
  stays off.
- **Display list.** `dsplanet`'s tiny incremental-vector square becomes a
  dot, `surf`'s short vectors become canvas lines at intensity 0, and the
  `"CL"`/`"L"`, name and scale text become the readout line.
- `sys time` (the delay) becomes the fixed tick rate; `absv`'s
  step-back-and-forward through `invert` is computed without mutating the
  planets (the original's round trip is the identity up to rounding).

## Getting the original source

A copy of the game's assembly is in [`original/`](original/) (see its README
for provenance and build order). The full restoration is below.

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
