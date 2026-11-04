package com.steelextractor.extractors;

import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.Codec;
import com.mojang.serialization.DynamicOps;
import com.mojang.serialization.JsonOps;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import net.minecraft.SharedConstants;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.storage.loot.providers.number.floats.ContextFloatProviders;
import net.minecraft.world.level.storage.loot.providers.number.ints.ContextIntProviders;

/** Serializes actual configured providers with registry-aware Mojang codecs. */
public final class LootNumberProviderValues {
    public static void main(String[] args) throws Exception {
        if (args.length != 0) throw new IllegalArgumentException("Use STEEL_EXTRACTOR_OUTPUT_DIRECTORY");
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        HolderLookup.Provider registries = VanillaRegistries.createReloadableLookup(VanillaRegistries.createWorldLookup());
        DynamicOps<JsonElement> ops = registries.createSerializationContext(JsonOps.INSTANCE);
        JsonObject root = new JsonObject();
        root.addProperty("minecraft_version", SharedConstants.getCurrentVersion().id());
        JsonArray floats = entries(registries, Registries.CONTEXT_FLOAT_PROVIDER, ContextFloatProviders.DIRECT_CODEC, ops);
        JsonArray integers = entries(registries, Registries.CONTEXT_INT_PROVIDER, ContextIntProviders.DIRECT_CODEC, ops);
        root.add("float_values", floats);
        root.add("int_values", integers);
        Path directory = Path.of(System.getenv().getOrDefault("STEEL_EXTRACTOR_OUTPUT_DIRECTORY", "steel_extractor_output"));
        Path output = directory.resolve("steel-registry/build_assets/context_number_provider_values.json");
        Files.createDirectories(output.getParent());
        try (var writer = Files.newBufferedWriter(output, StandardCharsets.UTF_8)) {
            new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create().toJson(root, writer);
            writer.append('\n');
        }
        System.out.println("Wrote " + floats.size() + " float and " + integers.size() + " int provider values to " + output.toAbsolutePath());
    }

    private static <T> JsonArray entries(HolderLookup.Provider registries,
            ResourceKey<? extends Registry<T>> key, Codec<T> codec, DynamicOps<JsonElement> ops) {
        JsonArray entries = new JsonArray();
        registries.lookupOrThrow(key).listElements()
            .sorted(Comparator.comparing(holder -> holder.key().identifier().toString()))
            .forEach(holder -> {
                JsonObject entry = new JsonObject();
                entry.addProperty("key", holder.key().identifier().toString());
                JsonElement value = codec.encodeStart(ops, holder.value()).getOrThrow();
                T decoded = codec.parse(ops, value).getOrThrow();
                JsonElement roundTrip = codec.encodeStart(ops, decoded).getOrThrow();
                if (!value.equals(roundTrip)) {
                    throw new IllegalStateException("Provider codec round-trip changed " + holder.key().identifier());
                }
                entry.add("value", value);
                entries.add(entry);
            });
        return entries;
    }
}
