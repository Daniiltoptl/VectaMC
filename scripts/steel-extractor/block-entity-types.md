# Block entity custom-data authorization metadata — pending extraction

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

**Status: prepared, not validated. No new extractor output has been copied.**
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

Requested action: run these exact commands from ordinary CMD:

```bat
cd /d D:\CodexBuilds\SteelExtractor
set "JAVA_HOME=C:\Program Files\Eclipse Adoptium\jdk-25.0.4.7-hotspot"
.\gradlew.bat extractBlockEntityTypes --no-daemon --max-workers=1
```

Provide its result or the produced file:
`D:\CodexBuilds\SteelExtractor\run\steel_extractor_output\steel-registry\build_assets\block_entities.json`.
Do not substitute an older output lacking the required flag or transcribe the
protected type set by hand. After a successful run, verify version/flags and copy
only the produced file to `vecta-registry/build_assets/block_entities.json`.
Update the build generator and typed registry field with a real placement consumer
and authorization tests in the same slice; do not default a missing flag to false.
