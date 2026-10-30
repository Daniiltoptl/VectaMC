# Slot display registry extraction (Minecraft Java 26.3)

The asset `vecta-registry/build_assets/slot_displays.json` comes from the live
`BuiltInRegistries.SLOT_DISPLAY`, not a transcription of Java registration order.

Reproducible source: [SteelExtractor](https://github.com/Steel-Foundation/SteelExtractor)
at `40e72f994d54edd619e3338fe431d7e938bf0db0`, with the adjacent
`slot-displays.patch` applied. The patch adds a dedicated `extractSlotDisplays`
JavaExec task and its Kotlin entry point. It bootstraps built-in registries,
extracts their actual IDs/keys and version, writes one file, and exits. It does
not start MinecraftServer, open a port, load a world, or accept an EULA.

On Windows with Java 25, from that extractor checkout:

```powershell
git apply --check 'C:\path\to\Vecta\scripts\steel-extractor\slot-displays.patch'
git apply 'C:\path\to\Vecta\scripts\steel-extractor\slot-displays.patch'
.\gradlew.bat extractSlotDisplays --no-daemon --max-workers=1
Copy-Item -LiteralPath 'run\steel_extractor_output\steel-registry\build_assets\slot_displays.json' `
  -Destination 'C:\path\to\Vecta\vecta-registry\build_assets\slot_displays.json'
```

`STEEL_EXTRACTOR_OUTPUT_DIRECTORY` overrides the output root, as in the other
extractor tasks. Copy only this produced file. Do not edit its JSON by hand.

The first extraction ran successfully on 2026-10-30 in
`D:\CodexBuilds\SteelExtractor` with Temurin 25.0.4. Gradle used Loom 1.18.4;
the unmodified extractor had Kotlin warnings and JOML emitted an Unsafe
deprecation warning. Neither was suppressed. Log:
`D:\CodexBuilds\logs\codex-slot-display-extraction.log`.

The Rust build validates the extracted Minecraft version against the crate's
`+mc` version suffix, checks contiguous IDs/unique keys, and generates typed
entries/registration order. Network dispatch then uses assigned registry IDs.
The payload enum no longer stores protocol IDs. Extension-owned codecs use
the same keyed downcasting pattern as position sources; plugin loading itself
is outside this change.
