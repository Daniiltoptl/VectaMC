# Loot context extraction — Minecraft Java 26.3

The adjacent `LootContexts.java` and `LootNumberProviderTypes.java` are the exact
source entry points added to the local SteelExtractor checkout at
`D:\CodexBuilds\SteelExtractor\src\main\java\com\steelextractor\extractors`.
Existing slot-display and block-entity extractor changes were preserved.

Both entry points bootstrap built-in registries and exit without starting
MinecraftServer, loading a world, opening a server port or accepting an EULA.
They use actual registry IDs/keys; none are transcribed from registration order.

## Verified source-launch workflow

On 2026-11-03 both entry points ran successfully with the installed Temurin
25.0.4 and Vecta's prepared 26.3 server/libraries. Java source-file launch avoids
requiring Gradle for these self-contained entry points. Previous Gradle loopback
failures were not retried or hidden with JVM/system workaround flags. The new
Gradle task registrations in the local checkout were not exercised this session.

Copy the adjacent Java sources into the path above. From an ordinary PowerShell:

```powershell
Set-Location 'D:\CodexBuilds\SteelExtractor\run'
$java25 = 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.7-hotspot\bin\java.exe'
$vanilla26 = 'C:\Users\My comp\Desktop\Vecta\.vecta-cache\vanilla\26.3'
$extractorClasspath = "$vanilla26\minecraft-server.jar;$vanilla26\libraries\*"
& $java25 -cp $extractorClasspath '..\src\main\java\com\steelextractor\extractors\LootContexts.java'
& $java25 -cp $extractorClasspath '..\src\main\java\com\steelextractor\extractors\LootNumberProviderTypes.java'
```

Keep the prepared jar and libraries together and verify the target through
`minecraft-src/.vecta-reference.json` and `SharedConstants`. Do not substitute
another target's assets. `STEEL_EXTRACTOR_OUTPUT_DIRECTORY` overrides the output
root; otherwise it is `run\steel_extractor_output`.

JOML printed its existing deprecated Unsafe warning. It was not suppressed.
Logs: `D:\CodexBuilds\logs\codex-session07-loot-extractor.log` and
`codex-session07-provider-types-extractor.log`.

## Produced files and scope

- `steel-registry/build_assets/loot_contexts.json`:16keys with reflected Java
  value types and31registered parameter sets with actual IDs, required and
  allowed key names. SHA256
  `e191f8eab770640ce2618ebaf198a75d7c551255706ae754f2fbc03cd00af437`.
  This file was copied byte-for-byte to `vecta-registry/build_assets/loot_contexts.json`.
  The Rust build validates target, contiguous set IDs, unique keys, referenced
  parameters and required-subset-of-allowed, then generates typed keys/sets.
- `steel-registry/build_assets/context_number_provider_types.json`:28float and
  23int codec types. SHA256
  `99fec44ffff3a60b536eb4a51f133e0b3915db842e090f0cf7edaf9d383488a6`.
  It remains in the extractor output for the next implementation slice; it is
  not an active provider factory or a claim that51providers work. The older
  27+22 catalog omitted `number_dispatcher` in both registries.

The provider-type extractor does not extract configured provider values from
`Registries.CONTEXT_FLOAT_PROVIDER` / `CONTEXT_INT_PROVIDER`; these require their
actual bootstrap/lookup when the corresponding consumer is implemented.

Copy only produced files needed by the change. Do not edit extracted JSON by
hand or manually recreate missing registry data.
