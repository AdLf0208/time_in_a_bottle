# Licence provenance

- Mod: `tiab` (Time in a Bottle) by RealMangoRage —
  <https://github.com/RealMangorage/time-in-a-bottle>
- Licence: MIT, read from the pack's jar
  (`ports/_shared/licence-cache/tiab-neoforge-6.5.4.jar`, SHA-256
  `0beb4ff1d81facfe9039e8d657a75836d1e8c13c137c90b491cc07e763f26325`):
  `META-INF/neoforge.mods.toml` says `license = "MIT"` and the jar carries the
  MIT text as `LICENSE_Time In A Bottle`, `Copyright (c) <2024> RealMangoRage`.
- The carried assets (`assets/tiab/`, `data/tiab/`) are the jar's own MIT files;
  `tools/assemble_content.py` re-creates them from the jar and refuses a jar
  whose SHA-256 is not the audited one.
- This port's Rust source is a clean-room rewrite from the mod's behaviour
  (`FINDINGS.md`), not a translation of its Java. The `LICENSE` beside this
  file is the standard MIT text with both copyright lines.
