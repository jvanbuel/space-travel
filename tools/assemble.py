#!/usr/bin/env python3
"""Assemble the original Space Travel listing into a PDP-7 memory image.

The port does its 1969-style arithmetic by running the original
floating-point package (original/fop.s) on a small PDP-7 emulator
(src/pdp7/mod.rs). This script assembles the whole game, in the original
build order, so the emulator gets the exact memory image and constants the
1970 program used, and writes src/pdp7/image.rs.

It follows the rules of the Unix v0 assembler as documented by the
pdp7-unix project (misc/asm_syntax.txt and tools/as7):

  * `"` starts a comment; `;` separates words on a line.
  * `name:` defines a label; single-digit labels are local and referenced
    as `1f` / `1b` (next / previous). Global labels may be redefined (the
    game relies on this for `q:` in fop.s); a use sees the latest value.
  * `name = expr` sets a symbol; `.` is the location counter.
  * An expression is a list of syllables combined with OR (space), `+` or
    `-`. Numbers with a leading 0 are octal, read digit by digit (so the
    listing's `028` is 2*8+8), others decimal; values are 18 bits.
  * `<x` is a character in the high half of a word, `x>` in the low half.
  * Two passes. The program is relocated to start at 4096 (`..`).

Usage: tools/assemble.py [--check]   (run from the repository root)
"""

import os
import re
import sys

FILES = ["st1.s", "st2.s", "st3.s", "st4.s", "fop.s", "st5.s", "st6.s", "st7.s"]
BASE = 4096
MASK = 0o777777

# Predefined symbols: system calls and instruction codes (as in as7/sop.s).
PREDEF = {
    "save": 1, "getuid": 2, "open": 3, "read": 4, "write": 5, "creat": 6,
    "seek": 7, "tell": 8, "close": 9, "link": 10, "unlink": 11,
    "setuid": 12, "rename": 13, "exit": 14, "time": 15, "intrp": 16,
    "chdir": 17, "chmod": 18, "chown": 19, "sysloc": 21, "capt": 23,
    "rele": 24, "status": 25, "smes": 27, "rmes": 28, "fork": 29,
    "sys": 0o020000, "i": 0o020000,
    "dac": 0o040000, "jms": 0o100000, "dzm": 0o140000, "lac": 0o200000,
    "xor": 0o240000, "add": 0o300000, "tad": 0o340000, "xct": 0o400000,
    "isz": 0o440000, "and": 0o500000, "sad": 0o540000, "jmp": 0o600000,
    "eae": 0o640000, "osc": 0o640001, "omq": 0o640002, "cmq": 0o640004,
    "div": 0o640323, "norm": 0o640444, "lls": 0o640600, "clls": 0o641600,
    "als": 0o640700, "lrs": 0o640500, "ecla": 0o641000, "lacs": 0o641001,
    "lacq": 0o641002, "abs": 0o644000, "divs": 0o644323, "clq": 0o650000,
    "frdiv": 0o650323, "lmq": 0o652000, "mul": 0o653122, "idiv": 0o653323,
    "idivs": 0o657323, "frdivs": 0o654323, "muls": 0o657122,
    "norms": 0o660444, "gsm": 0o664000, "lrss": 0o660500,
    "llss": 0o660600, "alss": 0o660700,
    "opr": 0o740000, "nop": 0o740000, "cma": 0o740001, "cml": 0o740002,
    "oas": 0o740004, "ral": 0o740010, "rar": 0o740020, "hlt": 0o740040,
    "xx": 0o740040, "sma": 0o740100, "sza": 0o740200, "snl": 0o740400,
    "skp": 0o741000, "spa": 0o741100, "sna": 0o741200, "szl": 0o741400,
    "rtl": 0o742010, "rtr": 0o742020, "cll": 0o744000, "stl": 0o744002,
    "rcl": 0o744010, "rcr": 0o744020, "cla": 0o750000, "clc": 0o750001,
    "las": 0o750004, "glk": 0o750010, "law": 0o760000,
}

# Symbols the Rust side needs: floating-point routines (called through
# `jms q`, so the address of q), and labels in the game.
ROUTINES = ["fld", "fst", "fad", "fmp", "fdv", "fng", "fcp", "fix", "flt",
            "sqrt", "sfmp", "sfdv"]
LABELS = {
    "aexp": "floating accumulator: exponent",
    "ams": "floating accumulator: sign and high mantissa",
    "ans": "floating accumulator: low mantissa",
    "start": "program entry",
    "loop": "top of the main loop (one pass per frame)",
    "displist": "display list: fixed header (frame, text)",
    "dispcl": "display list: the L / CL text word",
    "namedsp": "display list: 10 words of body name",
    "dssca": "display list: 3 words of scale",
    "dspl": "display list: per-frame part, built by the loop",
    "par": "index of the body the ship is relative to",
    "scale": "view scale",
    "lanflg": "landed flag",
    "crflg": "crash flag",
    "x": "ship x (float)",
    "y": "ship y (float)",
    "ox": "ship previous x (float)",
    "oy": "ship previous y (float)",
    "rpar": "radius of par (float)",
    "dpar": "distance to the body being processed (float)",
    "dtmp1": "scratch float (distance squared in updacc)",
}


class Asm:
    def __init__(self):
        self.vars = dict(PREDEF)
        self.labels = {}
        self.local = {}   # numeric label -> sorted list of locations
        self.mem = {}
        self.errors = []

    def num(self, s):
        if s.startswith("0"):
            v = 0
            for d in s:
                v = v * 8 + int(d)
            return v & MASK
        return int(s) & MASK

    def rel(self, n, direction):
        locs = self.local.get(n, [])
        here = self.loc
        if direction == "f":
            for l in locs:
                if l > here:
                    return l
        else:
            for l in reversed(locs):
                if l < here:
                    return l
        if self.stage == 2:
            self.err(f"undefined {n}{direction}")
        return 0

    def err(self, msg):
        self.errors.append(f"{self.file}:{self.line}: {msg}")

    def expr(self, text):
        """Parse an expression from the start of text; return (value, rest)."""
        word = 0
        while True:
            text = text.lstrip(" \t")
            if text == "" or text[0] in '";':
                return word & MASK, text
            op = "|"
            if text[0] in "+-":
                op, text = text[0], text[1:]
            if m := re.match(r"<(.)", text):
                syl = ord(m.group(1)) << 9
            elif m := re.match(r"(.)>", text):
                syl = ord(m.group(1))
            elif m := re.match(r">(.)", text):
                syl = ord(m.group(1))
            elif m := re.match(r"[A-Za-z_.][A-Za-z0-9_.]*", text):
                sym = m.group(0)
                if sym == ".":
                    syl = self.loc
                elif sym == "..":
                    syl = BASE
                elif sym in self.vars:
                    syl = self.vars[sym]
                elif sym[:8] in self.labels:
                    syl = self.labels[sym[:8]]
                else:
                    if self.stage == 2:
                        self.err(f"{sym} not defined")
                    syl = 0
            elif m := re.match(r"(\d+)([fb])", text):
                syl = self.rel(m.group(1), m.group(2))
            elif m := re.match(r"\d+", text):
                syl = self.num(m.group(0))
            else:
                self.err(f"cannot parse {text!r}")
                return word & MASK, ""
            text = text[m.end():]
            syl &= MASK
            if op == "+":
                word += syl
            elif op == "-":
                word -= syl
            else:
                word |= syl
            word &= MASK

    def parse_line(self, text):
        while True:
            text = text.lstrip(" \t")
            if text == "" or text.startswith('"'):
                return
            while m := re.match(r"([A-Za-z0-9_.]+):\s*", text):
                name = m.group(1)
                if name.isdigit():
                    if self.stage == 1:
                        self.local.setdefault(name, []).append(self.loc)
                else:
                    self.labels[name[:8]] = self.loc
                text = text[m.end():]
            if text == "" or text.startswith('"'):
                return
            if m := re.match(r"([^;= \t]+)\s*=", text):
                value, text = self.expr(text[m.end():])
                if m.group(1) == ".":
                    self.loc = value
                else:
                    self.vars[m.group(1)] = value
            else:
                value, text = self.expr(text)
                if self.stage == 2:
                    self.mem[self.loc] = value
                self.loc += 1
            text = re.sub(r"^\s*;?", "", text)

    def run(self, srcdir):
        for self.stage in (1, 2):
            self.loc = BASE
            for self.file in FILES:
                with open(os.path.join(srcdir, self.file), encoding="ascii") as f:
                    for self.line, text in enumerate(f, 1):
                        self.parse_line(text.rstrip("\n"))
        for k in self.local:
            self.local[k].sort()


def call_target(asm, name):
    """A routine's symbol is `jms q`; return the address of q."""
    word = asm.vars[name]
    assert word & 0o760000 == 0o100000, (name, oct(word))
    return word & 0o017777


def main():
    root = os.getcwd()
    asm = Asm()
    asm.run(os.path.join(root, "original"))
    if asm.errors:
        sys.exit("\n".join(asm.errors))
    lo, hi = min(asm.mem), max(asm.mem)
    words = [asm.mem.get(a, 0) for a in range(lo, hi + 1)]
    out = []
    out.append("//! Generated by tools/assemble.py from original/*.s; do not edit.\n")
    out.append("//! The 1970 Space Travel memory image, assembled in the original build")
    out.append("//! order (st1-st4, fop, st5-st7), and the addresses the port uses.\n")
    out.append("#![allow(dead_code)]\n")
    out.append(f"/// Address of the first word of [`IMAGE`].\npub const ORIGIN: usize = {lo:#o};")
    out.append(f"/// Memory image from [`ORIGIN`]: {len(words)} words.")
    out.append("pub const IMAGE: [u32; %d] = [" % len(words))
    for i in range(0, len(words), 8):
        out.append("    " + ", ".join(f"{w:#08o}" for w in words[i:i + 8]) + ",")
    out.append("];\n")
    for name in ROUTINES:
        addr = call_target(asm, name)
        doc = f"Entry of `{name}` (the word `jms` stores the return address in)."
        out.append(f"/// {doc}\npub const {name.upper()}: usize = {addr:#o};")
    for name, doc in LABELS.items():
        out.append(f"/// `{name}`: {doc}.\npub const {name.upper()}: usize = {asm.labels[name]:#o};")
    text = "\n".join(out) + "\n"
    path = os.path.join(root, "src", "pdp7", "image.rs")
    if "--check" in sys.argv:
        with open(path, encoding="ascii") as f:
            sys.exit(0 if f.read() == text else "src/pdp7/image.rs is out of date")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="ascii") as f:
        f.write(text)
    print(f"wrote {path}: {len(words)} words at {lo:#o}..{hi:#o}")


if __name__ == "__main__":
    main()
