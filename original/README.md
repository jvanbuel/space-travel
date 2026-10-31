# Original Space Travel source (PDP-7 assembly)

Ken Thompson's *Space Travel* for the PDP-7, kept here as the reference for
the Rust port.

**Provenance:** copied unmodified from `src/cmd/` of
[DoctorWkt/pdp7-unix](https://github.com/DoctorWkt/pdp7-unix) at commit
`555eb30fc76b8fa29095d32eca9a43e9b1638288` (2023-09-08). That project
transcribed the 1970 listings
(<https://www.tuhs.org/Archive/Distributions/Research/McIlroy_v0/>) and
fixed them so they assemble and run under SimH. Each file's `"** <pdf> page N`
markers point back to the scanned pages.

| File | Contents |
|------|----------|
| `st1.s` … `st7.s` | The game |
| `fop.s` | Software floating-point routines (`fld`, `fst`, `fad`, `fmp`, …) used by the game |

**Build order** (from pdp7-unix's `build/Makefile`):
`st1.s st2.s st3.s st4.s fop.s st5.s st6.s st7.s`

**Licence:** the pdp7-unix repository is distributed under GPLv3
(`LICENSE-pdp7-unix`). Its `scans/README.md` also states that the original
Unix code is probably © 1970 Micro Focus and not itself under GPLv3. Check
that before publishing this repository or the port.

For the assembler syntax, see pdp7-unix's `misc/asm_syntax.txt`. The top-level
README here has a short primer.
