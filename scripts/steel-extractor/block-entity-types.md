# Block entity custom-data authorization metadata — Minecraft 26.3

Target: Minecraft Java 26.3. SteelExtractor checkout:
`D:\CodexBuilds\SteelExtractor`, base `40e72f994d54edd619e3338fe431d7e938bf0db0`.

`block-entity-types.patch` extends `BlockEntities` with `minecraft_version`
and `only_op_can_set_nbt` from the live `BlockEntityType.onlyOpCanSetNbt()` method.
The dedicated `extractBlockEntityTypes` task bootstraps registries and writes
only `steel-registry/build_assets/block_entities.json`. It does not start
MinecraftServer, load a world, open a Minecraft port or accept an EULA.

The patch was generated against this checkout with the existing adjacent
`slot-displays.patch` already applied. It preserves that pre-existing work.
For a fresh checkout, apply the slot-display patch before this patch.

**Status: extraction succeeded in the owner's ordinary CMD on 2026-11-02.**
The exact task below completed with `BUILD SUCCESSFUL in 29s`; no MinecraftServer
or EULA acceptance was involved. Output contains version 26.3 and required boolean
flags for all 49 block entity types. The produced file was copied byte-for-byte
into `vecta-registry/build_assets/block_entities.json`.
SHA-256: `bfe9f72f37d43fc9297ccf39e5f2bf0cecdf053679e48c130e74e5e01abfc88b`.
The existing Kotlin non-null assertion warning and JOML Unsafe deprecation warning
were not suppressed. This was the owner's extraction run, validated locally by
reading/hash-comparing the produced file; the tool environment did not perform it.
The first two attempts on 2026-11-02 failed before Kotlin compilation/extraction:

```text
java.io.IOException: Unable to establish loopback connection
Caused by: java.net.SocketException: Invalid argument: connect
  at sun.nio.ch.UnixDomainSockets.connect0
```

The second invocation added only `--stacktrace`. The owner's ordinary terminal
then reached Gradle configuration but reported Java 17 instead of required 25.
Inspection confirmed `JAVA_HOME` pointed to installed JDK 17 although PATH's
`java` was JDK 25. Selecting the already installed JDK 25 through process-local
`JAVA_HOME` still failed in the tool environment with the loopback error.
No further JVM workaround flags, system settings or server run were attempted.
Logs: `D:\CodexBuilds\logs\codex-be-types-extraction*.log`, including
`codex-be-types-extraction-java25.log`.

Successful reproducible command from ordinary CMD:

```bat
cd /d D:\CodexBuilds\SteelExtractor
set "JAVA_HOME=C:\Program Files\Eclipse Adoptium\jdk-25.0.4.7-hotspot"
.\gradlew.bat extractBlockEntityTypes --no-daemon --max-workers=1
```

Produced file:
`D:\CodexBuilds\SteelExtractor\run\steel_extractor_output\steel-registry\build_assets\block_entities.json`.
Do not substitute an older output lacking the required flag or transcribe the
protected type set by hand. The Rust build generator validates the version and
requires the flag, then compiles it into typed entries. Placement uses the flag
with authorization tests; a missing flag is not defaulted to false.
