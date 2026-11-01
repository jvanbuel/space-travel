//! A small PDP-7 CPU: enough to run the original Space Travel program.
//!
//! In "1970" mode the game is not the Rust translation in `game.rs` but the
//! original machine code, assembled from `original/*.s` by
//! `tools/assemble.py` into [`image`] and run here, floating-point package
//! (`fop.s`) and all. Its arithmetic is therefore the original's bit for bit,
//! quirks included.
//!
//! Instruction semantics follow SimH's PDP-7 simulator
//! (`PDP18B/pdp18b_cpu.c`, PDP-7 paths): memory reference instructions with
//! indirection and auto-increment registers 010-017, the operate group, and
//! the Type 177 Extended Arithmetic Element (multiply, divide, normalise and
//! shifts). There are no devices: `cal` (`sys`) stops the machine and the
//! host ([`original`]) plays the operating system.

#[rustfmt::skip]
pub mod image;
pub mod original;

/// 18-bit data word.
const DMASK: u32 = 0o777777;
/// The link, held as bit 18 of `lac` (as in SimH).
const LINK: u32 = 0o1000000;
const LACMASK: u32 = LINK | DMASK;
const SIGN: u32 = 0o400000;
/// 13-bit address within the 8K memory.
const AMASK: usize = 0o17777;

/// Sign-extend an 18-bit two's-complement word.
pub fn word_to_i32(w: u32) -> i32 {
    ((w << 14) as i32) >> 14
}

/// Why the machine stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// A `hlt` instruction: the normal end of a call stub.
    Halt,
    /// A system call (`sys n`): the host handles it and resumes at `pc + 1`.
    Sys { pc: usize, call: usize },
    /// Reached the breakpoint given to [`Machine::run`].
    Break,
    /// An instruction this CPU does not implement (I/O, XCT).
    Unimplemented { pc: usize, ir: u32 },
    /// Ran longer than the step limit.
    Runaway,
}

pub struct Machine {
    pub mem: Box<[u32; 8192]>,
    /// AC with the link in bit 18.
    pub lac: u32,
    pub mq: u32,
    sc: u32,
    pc: usize,
    eae_ac_sign: u32,
    /// Instructions executed so far (for curiosity and tests).
    pub steps: u64,
    /// When set, each instruction is recorded as (pc, ir, ac+link, mq).
    #[cfg(test)]
    pub trace: Option<Vec<(usize, u32, u32, u32)>>,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    /// A machine with the Space Travel image loaded.
    pub fn new() -> Self {
        let mut mem = Box::new([0u32; 8192]);
        mem[image::ORIGIN..image::ORIGIN + image::IMAGE.len()].copy_from_slice(&image::IMAGE);
        Machine {
            mem,
            lac: 0,
            mq: 0,
            sc: 0,
            pc: 0,
            eae_ac_sign: 0,
            steps: 0,
            #[cfg(test)]
            trace: None,
        }
    }

    pub fn ac(&self) -> u32 {
        self.lac & DMASK
    }

    pub fn set_ac(&mut self, v: u32) {
        self.lac = (self.lac & LINK) | (v & DMASK);
    }

    /// Run from `start` until a stop, or until about to execute the
    /// instruction at `breakpoint` (not counting the first instruction).
    pub fn run(&mut self, start: usize, breakpoint: Option<usize>, limit: u64) -> Stop {
        self.pc = start & AMASK;
        let first = self.steps;
        let end = self.steps + limit;
        while self.steps < end {
            if breakpoint == Some(self.pc) && self.steps != first {
                return Stop::Break;
            }
            self.steps += 1;
            if let Some(stop) = self.step() {
                return stop;
            }
        }
        Stop::Runaway
    }

    /// Indirect address: auto-increment locations 010-017 before use.
    fn ia(&mut self, ma: usize) -> usize {
        if ma & AMASK & !0o7 == 0o10 {
            self.mem[ma] = (self.mem[ma] + 1) & DMASK;
        }
        self.mem[ma] as usize & AMASK
    }

    fn step(&mut self) -> Option<Stop> {
        let pc = self.pc;
        let ir = self.mem[pc];
        #[cfg(test)]
        if let Some(t) = &mut self.trace {
            t.push((pc, ir, self.lac, self.mq));
        }

        self.pc = (pc + 1) & AMASK;
        let mut ma = ir as usize & AMASK;
        let op = (ir >> 13) & 0o37;
        // Memory reference instructions: odd codes are indirect.
        if op < 0o32 && op & 1 == 1 && op > 1 {
            ma = self.ia(ma);
        }
        match op {
            // cal / sys: a system call; the number is in the address field
            0o00 | 0o01 => {
                return Some(Stop::Sys {
                    pc,
                    call: ir as usize & AMASK,
                });
            }
            0o02 | 0o03 => self.mem[ma] = self.lac & DMASK, // dac
            0o04 | 0o05 => {
                // jms: save link and return address, continue after it
                self.mem[ma] = ((self.lac & LINK) >> 1) | self.pc as u32;
                self.pc = (ma + 1) & AMASK;
            }
            0o06 | 0o07 => self.mem[ma] = 0, // dzm
            0o10 | 0o11 => self.lac = (self.lac & LINK) | self.mem[ma], // lac
            0o12 | 0o13 => self.lac ^= self.mem[ma], // xor
            0o14 | 0o15 => {
                // add: one's complement, end-around carry, link on overflow
                let mb = self.mem[ma];
                let mut t = (self.lac & DMASK) + mb;
                if t > DMASK {
                    t = (t + 1) & DMASK;
                }
                if ((!self.lac ^ mb) & (self.lac ^ t)) & SIGN != 0 {
                    self.lac = LINK | t;
                } else {
                    self.lac = (self.lac & LINK) | t;
                }
            }
            0o16 | 0o17 => self.lac = (self.lac + self.mem[ma]) & LACMASK, // tad
            0o22 | 0o23 => {
                // isz
                let v = (self.mem[ma] + 1) & DMASK;
                self.mem[ma] = v;
                if v == 0 {
                    self.pc = (self.pc + 1) & AMASK;
                }
            }
            0o24 | 0o25 => self.lac &= self.mem[ma] | LINK, // and
            0o26 | 0o27 => {
                // sad
                if self.lac & DMASK != self.mem[ma] {
                    self.pc = (self.pc + 1) & AMASK;
                }
            }
            0o30 | 0o31 => self.pc = ma, // jmp (indirect already resolved)
            0o32 | 0o33 => self.eae(ir),
            0o36 => return self.opr(ir),
            0o37 => self.lac = (self.lac & LINK) | ir, // law
            _ => return Some(Stop::Unimplemented { pc, ir }),
        }
        None
    }

    /// The operate group: skips, clear/complement, rotates, halt.
    fn opr(&mut self, ir: u32) -> Option<Stop> {
        let lac = self.lac;
        let skip = match (ir >> 6) & 0o17 {
            0 => false,
            1 => lac & SIGN != 0,                        // sma
            2 => lac & DMASK == 0,                       // sza
            3 => lac & DMASK == 0 || lac & SIGN != 0,    // sza sma
            4 => lac >= LINK,                            // snl
            5 => lac >= SIGN,                            // snl sma
            6 => lac >= LINK || lac == 0,                // snl sza
            7 => lac >= SIGN || lac == 0,                // snl sza sma
            0o10 => true,                                // skp
            0o11 => lac & SIGN == 0,                     // spa
            0o12 => lac & DMASK != 0,                    // sna
            0o13 => lac & DMASK != 0 && lac & SIGN == 0, // sna spa
            0o14 => lac < LINK,                          // szl
            0o15 => lac < SIGN,                          // szl spa
            0o16 => lac < LINK && lac != 0,              // szl sna
            _ => lac < SIGN && lac != 0,                 // szl sna spa
        };
        self.lac = match ((ir >> 9) & 0o14) | (ir & 0o3) {
            0 => self.lac,
            1 => self.lac ^ DMASK,             // cma
            2 => self.lac ^ LINK,              // cml
            3 => self.lac ^ LACMASK,           // cml cma
            4 => self.lac & DMASK,             // cll
            5 => (self.lac & DMASK) ^ DMASK,   // cll cma
            6 => self.lac | LINK,              // stl
            7 => (self.lac | LINK) ^ DMASK,    // stl cma
            0o10 => self.lac & LINK,           // cla
            0o11 => self.lac | DMASK,          // cla cma
            0o12 => (self.lac & LINK) ^ LINK,  // cla cml
            0o13 => (self.lac | DMASK) ^ LINK, // cla cml cma
            0o14 => 0,                         // cla cll
            0o15 => DMASK,                     // cla cll cma
            0o16 => LINK,                      // cla cll cml
            _ => LACMASK,                      // cla cll cml cma
        };
        // (oas would OR in the console switches, which are all zero.)
        match ((ir >> 8) & 0o4) | ((ir >> 3) & 0o3) {
            1 => self.lac = ((self.lac << 1) | (self.lac >> 18)) & LACMASK, // ral
            2 => self.lac = ((self.lac >> 1) | (self.lac << 18)) & LACMASK, // rar
            5 => self.lac = ((self.lac << 2) | (self.lac >> 17)) & LACMASK, // rtl
            6 => self.lac = ((self.lac >> 2) | (self.lac << 17)) & LACMASK, // rtr
            0 | 4 => {}
            _ => {
                return Some(Stop::Unimplemented {
                    pc: self.pc - 1,
                    ir,
                });
            }
        }
        if ir & 0o40 != 0 {
            return Some(Stop::Halt);
        }
        if skip {
            self.pc = (self.pc + 1) & AMASK;
        }
        None
    }

    /// The Type 177 Extended Arithmetic Element.
    fn eae(&mut self, ir: u32) {
        if ir & 0o020000 != 0 {
            // AC0 to link
            self.lac = ((self.lac << 1) & LINK) | (self.lac & DMASK);
        }
        if ir & 0o010000 != 0 {
            self.mq = 0;
        }
        self.eae_ac_sign = if ir & 0o004000 != 0 && self.lac & SIGN != 0 {
            self.lac & LINK
        } else {
            0
        };
        if ir & 0o002000 != 0 {
            self.mq = (self.mq | self.lac) & DMASK;
        } else if self.eae_ac_sign != 0 {
            self.lac ^= DMASK; // |AC|
        }
        if ir & 0o001000 != 0 {
            self.lac &= LINK; // clear AC
        }
        let link_init = self.lac & LINK;
        let fill = if link_init != 0 { DMASK } else { 0 };
        let esc = ir & 0o77;
        match (ir >> 6) & 0o7 {
            0 => {
                // setup: cmq, omq, osc (lacs)
                if ir & 0o4 != 0 {
                    self.mq ^= DMASK;
                }
                if ir & 0o2 != 0 {
                    self.lac |= self.mq;
                }
                if ir & 0o1 != 0 {
                    self.lac |= self.sc.wrapping_neg() & 0o77;
                }
            }
            1 => {
                // multiply MQ by the next word, shift-and-add
                let mb = self.mem[self.pc];
                self.pc = (self.pc + 1) & AMASK;
                if self.eae_ac_sign != 0 {
                    self.mq ^= DMASK;
                }
                self.lac &= DMASK;
                self.sc = esc;
                loop {
                    if self.mq & 1 != 0 {
                        self.lac += mb;
                    }
                    self.mq = (self.mq >> 1) | ((self.lac & 1) << 17);
                    self.lac >>= 1;
                    self.sc = (self.sc.wrapping_sub(1)) & 0o77;
                    if self.sc == 0 {
                        break;
                    }
                }
                if self.eae_ac_sign ^ link_init != 0 {
                    self.lac ^= DMASK;
                    self.mq ^= DMASK;
                }
            }
            3 => {
                // divide AC'MQ by the next word, non-restoring
                let mb = self.mem[self.pc];
                self.pc = (self.pc + 1) & AMASK;
                if self.eae_ac_sign != 0 {
                    self.mq ^= DMASK;
                }
                if self.lac & DMASK >= mb {
                    // overflow: set link
                    self.lac = (self.lac.wrapping_sub(mb) & LACMASK) | LINK;
                    return;
                }
                self.lac &= DMASK;
                let mut t = 0;
                self.sc = esc;
                loop {
                    self.lac = if t != 0 {
                        (self.lac + mb) & LACMASK
                    } else {
                        self.lac.wrapping_sub(mb) & LACMASK
                    };
                    t = (self.lac >> 18) & 1;
                    if self.sc > 1 {
                        self.lac = ((self.lac << 1) | (self.mq >> 17)) & LACMASK;
                    }
                    self.mq = ((self.mq << 1) | (t ^ 1)) & DMASK;
                    self.sc = (self.sc.wrapping_sub(1)) & 0o77;
                    if self.sc == 0 {
                        break;
                    }
                }
                if t != 0 {
                    self.lac = (self.lac + mb) & LACMASK;
                }
                if self.eae_ac_sign != 0 {
                    self.lac ^= DMASK;
                }
                if self.eae_ac_sign ^ link_init != 0 {
                    self.mq ^= DMASK;
                }
            }
            4 => {
                // normalise: shift AC'MQ left until AC0 != AC1
                self.sc = esc;
                while (self.lac & SIGN) == ((self.lac << 1) & SIGN) {
                    self.lac = (self.lac << 1) | ((self.mq >> 17) & 1);
                    self.mq = (self.mq << 1) | (link_init >> 18);
                    self.sc = (self.sc.wrapping_sub(1)) & 0o77;
                    if self.sc == 0 {
                        break;
                    }
                }
                self.lac = link_init | (self.lac & DMASK);
                self.mq &= DMASK;
            }
            5 => {
                // long right shift, filling from the link
                if esc < 18 {
                    self.mq = ((self.lac << (18 - esc)) | (self.mq >> esc)) & DMASK;
                    self.lac = ((fill << (18 - esc)) | (self.lac >> esc)) & LACMASK;
                } else {
                    self.mq = if esc < 36 {
                        ((fill << (36 - esc)) | (self.lac >> (esc - 18))) & DMASK
                    } else {
                        fill
                    };
                    self.lac = link_init | fill;
                }
                self.sc = 0;
            }
            6 => {
                // long left shift, filling from the link
                if esc < 18 {
                    self.lac = link_init | (((self.lac << esc) | (self.mq >> (18 - esc))) & DMASK);
                    self.mq = ((self.mq << esc) | (fill >> (18 - esc))) & DMASK;
                } else {
                    self.lac = if esc < 36 {
                        link_init | (((self.mq << (esc - 18)) | (fill >> (36 - esc))) & DMASK)
                    } else {
                        link_init | fill
                    };
                    self.mq = fill;
                }
                self.sc = 0;
            }
            7 => {
                // AC left shift
                self.lac = if esc < 18 {
                    link_init | (((self.lac << esc) | (fill >> (18 - esc))) & DMASK)
                } else {
                    link_init | fill
                };
                self.sc = 0;
            }
            _ => {} // 2: not a PDP-7 EAE command
        }
    }
}

/// Tests of the CPU through the original floating-point routines: values
/// worked out by hand from the format, and agreement with f64.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::{Word3, pdp7};

    type W = [u32; 3];

    fn enc(x: f64) -> W {
        if x == 0.0 {
            return [0, 0, 0];
        }
        let mut exp = x.abs().log2().floor() as i32 + 1;
        let mut mant = x.abs() / 2f64.powi(exp);
        if mant >= 1.0 {
            mant /= 2.0;
            exp += 1;
        } else if mant < 0.5 {
            mant *= 2.0;
            exp -= 1;
        }
        let bits = (mant * (1u64 << 35) as f64) as u64;
        let sign = if x < 0.0 { 0o400000 } else { 0 };
        [
            exp as u32 & DMASK,
            sign | ((bits >> 18) as u32 & 0o377777),
            bits as u32 & DMASK,
        ]
    }

    fn dec(w: W) -> f64 {
        pdp7((word_to_i32(w[0]), w[1], w[2]) as Word3)
    }

    /// `jms routine; operand; hlt` with the FAC loaded and the link set.
    fn call(
        m: &mut Machine,
        entry: usize,
        fac: W,
        arg: Option<W>,
        ac: u32,
        link: bool,
    ) -> (W, u32) {
        const STUB: usize = 0o17700;
        const ARG: usize = 0o17710;
        m.mem[image::AEXP..image::AEXP + 3].copy_from_slice(&fac);
        m.mem[STUB] = 0o100000 | entry as u32;
        let mut p = STUB + 1;
        if let Some(a) = arg {
            m.mem[ARG..ARG + 3].copy_from_slice(&a);
            m.mem[p] = ARG as u32;
            p += 1;
        }
        m.mem[p] = 0o740040;
        m.lac = (ac & DMASK) | if link { LINK } else { 0 };
        assert_eq!(m.run(STUB, None, 100_000), Stop::Halt);
        let mut out = [0; 3];
        out.copy_from_slice(&m.mem[image::AEXP..image::AEXP + 3]);
        (out, m.ac())
    }

    fn op(entry: usize, a: f64, b: f64) -> f64 {
        let mut m = Machine::new();
        dec(call(&mut m, entry, enc(a), Some(enc(b)), 0, true).0)
    }

    #[test]
    fn encodes_like_the_listing() {
        assert_eq!(enc(1.0), [1, 0o200000, 0]); // fpone
        assert_eq!(enc(2.0), [2, 0o200000, 0]); // thrs
        assert_eq!(enc(400.0), [0o11, 0o310000, 0]); // f400
        assert_eq!(dec(enc(-1.0)), -1.0);
    }

    #[test]
    fn exact_arithmetic() {
        assert_eq!(op(image::FAD, 1.0, 1.0), 2.0);
        assert_eq!(op(image::FAD, 5.0, -3.0), 2.0);
        assert_eq!(op(image::FMP, 3.0, 4.0), 12.0);
        assert_eq!(op(image::SFMP, 3.0, 4.0), 12.0);
        assert_eq!(op(image::SFDV, 12.0, 4.0), 3.0);
        let mut m = Machine::new();
        assert_eq!(
            dec(call(&mut m, image::FNG, enc(2.0), None, 0, false).0),
            -2.0
        );
        assert_eq!(
            dec(call(&mut m, image::SQRT, enc(16.0), None, 0, false).0),
            4.0
        );
        assert_eq!(call(&mut m, image::FIX, enc(400.0), None, 0, false).1, 400);
        let fix = call(&mut m, image::FIX, enc(-2.5), None, 0, false).1;
        assert_eq!(word_to_i32(fix), -3, "fix rounds down");
        let flt = call(
            &mut m,
            image::FLT,
            [0; 3],
            None,
            (-20i32) as u32 & DMASK,
            false,
        )
        .0;
        assert_eq!(dec(flt), -20.0);
    }

    /// Within the format's 35-bit precision of f64, on awkward values.
    #[test]
    fn close_to_f64() {
        let xs = [
            0.1,
            -0.3,
            1.0 / 3.0,
            6371.0,
            2.5e-7,
            -9.81,
            23510.123,
            1.0e-12,
            7.0e5,
        ];
        for &a in &xs {
            for &b in &xs {
                for (entry, want, tol) in [
                    (image::FMP, a * b, 4e-10),
                    (image::FAD, a + b, 4e-10),
                    // fdv: see fdv_is_biased
                    (image::FDV, a / b, 5e-5),
                ] {
                    let got = op(entry, a, b);
                    assert!(
                        (got - want).abs() <= want.abs() * tol,
                        "{entry:#o}: {a}, {b}: {got} vs {want}"
                    );
                }
                let mut m = Machine::new();
                let ac = call(&mut m, image::FCP, enc(a), Some(enc(b)), 0, false).1;
                if (a < 0.0) == (b < 0.0) {
                    assert_eq!(ac & SIGN != 0, a < b, "fcp {a} {b}");
                }
            }
            let mut m = Machine::new();
            let r = dec(call(&mut m, image::SQRT, enc(a.abs()), None, 0, false).0);
            assert!(
                (r - a.abs().sqrt()).abs() <= a.abs().sqrt() * 4e-10,
                "sqrt {a}: {r}"
            );
        }
    }

    /// `sfmp` multiplies only the high 17-bit words.
    #[test]
    fn quick_multiply_is_short() {
        let g = op(image::SFMP, 1.0 / 3.0, 3.0);
        assert!((g - 1.0).abs() < 2e-5 && g != 1.0, "{g}");
    }

    /// `fdv` is only good to about 1 part in 30,000, where `fmp` and `fad`
    /// use all 35 bits. Two things in the listing:
    ///
    /// - It shifts the divisor with `ral`, which rotates in the link without
    ///   clearing it (`sfdv` uses `ral cll`). The link there is the caller's,
    ///   flipped by the carry from `garg`'s `tad dm1`, so the divisor can
    ///   gain a stray low bit. The scan has a box drawn around that `ral`.
    /// - Before the final shift (`6: lac 2b; sma; tad d1; tad 5b`) it adds 1
    ///   to the high quotient word whenever the correction count is not
    ///   negative, which it never is here. The scan has notes and arrows
    ///   around exactly these lines.
    ///
    /// The EAE divide itself is exact (`eae_multiply_and_divide_are_exact`),
    /// and the listing matches the scan's OCR, so this is how the restored
    /// 1969 code behaves.
    #[test]
    fn fdv_is_biased() {
        let mut m = Machine::new();
        let with = dec(call(&mut m, image::FDV, enc(12.0), Some(enc(4.0)), 0, true).0);
        let without = dec(call(&mut m, image::FDV, enc(12.0), Some(enc(4.0)), 0, false).0);
        assert_eq!(with, 3.0 + 2f64.powi(-16), "{with}");
        assert!(without < 3.0, "{without}");
    }

    /// `fcp` compares equal exponents and high words by returning the
    /// difference of the low words, `ans - hls`, as an 18-bit word. Low words
    /// are unsigned 18-bit fractions, so a difference of 2^17 or more has the
    /// sign bit set and reads as "less". The game hits this: a ship climbing
    /// through about 50-100 m above Earth compares its distance `dpar`
    /// (1.0000099 Earth radii) with Earth's radius `rpar` (1.0) and is judged
    /// to be below the surface.
    #[test]
    fn fcp_low_word_quirk() {
        let mut m = Machine::new();
        let dpar = [1, 0o200000, 0o515637]; // 1.0000099
        let ac = call(&mut m, image::FCP, dpar, Some(enc(1.0)), 0, false).1;
        assert_eq!(ac, 0o515637);
        assert_ne!(ac & SIGN, 0, "reads as dpar < rpar");
    }

    /// `fcp` with operands of different signs returns a positive AC
    /// whichever is larger: the game only compares like-signed values.
    #[test]
    fn fcp_mixed_signs() {
        let mut m = Machine::new();
        let ac = call(&mut m, image::FCP, enc(-2.0), Some(enc(3.0)), 0, false).1;
        assert_eq!(ac & SIGN, 0);
    }

    /// Run one EAE instruction with an operand word on AC/MQ.
    fn eae(ir: u32, ac: u32, mq: u32, operand: u32) -> (u32, u32, bool) {
        let mut m = Machine::new();
        m.mem[0o17700] = ir;
        m.mem[0o17701] = operand;
        m.mem[0o17702] = 0o740040;
        m.lac = ac;
        m.mq = mq;
        assert_eq!(m.run(0o17700, None, 100), Stop::Halt);
        (m.ac(), m.mq, m.lac & LINK != 0)
    }

    /// The hardware multiply and divide (SimH's algorithms) are exact.
    #[test]
    fn eae_multiply_and_divide_are_exact() {
        let mut seed = 12345u64;
        let mut rnd = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as u32 & DMASK
        };
        for _ in 0..2000 {
            let (a, b) = (rnd(), rnd() | 1);
            // mul (0o653122 = clear MQ, AC to MQ, clear AC, multiply):
            // AC'MQ = AC * operand
            let (hi, lo, _) = eae(0o653122, a, 0, b);
            assert_eq!(
                (u64::from(hi) << 18) | u64::from(lo),
                u64::from(a) * u64::from(b)
            );
            // div: AC'MQ / operand, quotient in MQ, remainder in AC
            let hi = rnd() % b; // no overflow
            let lo = rnd();
            let n = (u64::from(hi) << 18) | u64::from(lo);
            let (rem, quo, link) = eae(0o640323, hi, lo, b);
            assert!(!link);
            assert_eq!(u64::from(quo), n / u64::from(b), "{n} / {b}");
            assert_eq!(u64::from(rem), n % u64::from(b), "{n} % {b}");
        }
    }
}
