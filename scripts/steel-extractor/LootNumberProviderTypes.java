package com.steelextractor.extractors;

import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import net.minecraft.SharedConstants;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;

/** Extracts codec-type identities, not placeholder provider implementations. */
public final class LootNumberProviderTypes {
    public static void main(String[] args) throws Exception {
        if (args.length != 0) throw new IllegalArgumentException("Use STEEL_EXTRACTOR_OUTPUT_DIRECTORY");
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        JsonObject root = new JsonObject();
        root.addProperty("minecraft_version", SharedConstants.getCurrentVersion().id());
        JsonArray floats = entries(BuiltInRegistries.CONTEXT_FLOAT_PROVIDER_TYPE);
        JsonArray integers = entries(BuiltInRegistries.CONTEXT_INT_PROVIDER_TYPE);
        root.add("float_types", floats);
        root.add("int_types", integers);
        Path directory = Path.of(System.getenv().getOrDefault("STEEL_EXTRACTOR_OUTPUT_DIRECTORY", "steel_extractor_output"));
        Path output = directory.resolve("steel-registry/build_assets/context_number_provider_types.json");
        Files.createDirectories(output.getParent());
        try (var writer = Files.newBufferedWriter(output, StandardCharsets.UTF_8)) {
            new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create().toJson(root, writer);
            writer.append('\n');
        }
        System.out.println("Wrote " + floats.size() + " float and " + integers.size() + " int provider types to " + output.toAbsolutePath());
    }

    private static <T> JsonArray entries(Registry<T> registry) {
        JsonArray entries = new JsonArray();
        for (T type : registry) {
            JsonObject entry = new JsonObject();
            entry.addProperty("id", registry.getId(type));
            entry.addProperty("key", registry.getKey(type).toString());
            entries.add(entry);
        }
        return entries;
    }
}
