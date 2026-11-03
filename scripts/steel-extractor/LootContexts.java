package com.steelextractor.extractors;

import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.lang.reflect.Modifier;
import java.lang.reflect.ParameterizedType;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import java.util.Set;
import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.util.context.ContextKey;
import net.minecraft.world.level.storage.loot.parameters.LootContextParams;

/** Extracts the actual loot keys and registered parameter sets without starting a server. */
public final class LootContexts {
    public static void main(String[] args) throws Exception {
        if (args.length != 0) throw new IllegalArgumentException("Use STEEL_EXTRACTOR_OUTPUT_DIRECTORY");
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        JsonArray parameters = new JsonArray();
        for (var field : LootContextParams.class.getDeclaredFields()) {
            if (!Modifier.isStatic(field.getModifiers()) || field.getType() != ContextKey.class) continue;
            ContextKey<?> key = (ContextKey<?>) field.get(null);
            JsonObject parameter = new JsonObject();
            parameter.addProperty("key", key.name().toString());
            var generic = (ParameterizedType) field.getGenericType();
            parameter.addProperty("java_type", generic.getActualTypeArguments()[0].getTypeName());
            parameters.add(parameter);
        }
        JsonArray sets = new JsonArray();
        var registry = BuiltInRegistries.CONTEXT_KEY_SET;
        for (var set : registry) {
            JsonObject entry = new JsonObject();
            entry.addProperty("id", registry.getId(set));
            entry.addProperty("key", registry.getKey(set).toString());
            entry.add("required", names(set.required()));
            entry.add("allowed", names(set.allowed()));
            sets.add(entry);
        }
        JsonObject root = new JsonObject();
        root.addProperty("minecraft_version", SharedConstants.getCurrentVersion().id());
        root.add("parameters", parameters);
        root.add("sets", sets);
        Path directory = Path.of(System.getenv().getOrDefault("STEEL_EXTRACTOR_OUTPUT_DIRECTORY", "steel_extractor_output"));
        Path output = directory.resolve("steel-registry/build_assets/loot_contexts.json");
        Files.createDirectories(output.getParent());
        try (var writer = Files.newBufferedWriter(output, StandardCharsets.UTF_8)) {
            new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create().toJson(root, writer);
            writer.append('\n');
        }
        System.out.println("Wrote " + parameters.size() + " parameters, " + sets.size() + " sets to " + output.toAbsolutePath());
    }

    private static JsonArray names(Set<ContextKey<?>> keys) {
        JsonArray result = new JsonArray();
        keys.stream().map(key -> key.name().toString()).sorted(Comparator.naturalOrder()).forEach(result::add);
        return result;
    }
}
