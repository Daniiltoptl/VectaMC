# Каталог для проверки ванильной совместимости

Эталон: Minecraft Java **26.3**. Базовый коммит: `c825001bb2e5a9585b8da529c52470ba1de4b9b3`.

Это инвентаризация регистрации, а не процент готовности и не доказательство отсутствия всей механики.
`yes` означает наличие сгенерированной регистрации; полноту реализации нужно проверять отдельно.
`no` означает необходимость проверить специальный класс, общие компоненты и отдельные пути исполнения.
Обычные `Item`, `Block`, воздух и создаваемый при входе `Player` не требуют отдельного поведения автоматически.
Некоторые классы имеют только клиентские отличия; сверяйте серверные методы с эталоном.

| Раздел | Записей в classes.json | Сгенерированных регистраций |
| --- | ---: | ---: |
| items | 1658 | 1278 |
| blocks | 1286 | 1028 |
| entities | 161 | 26 |

Полный поимённый каталог, включая уже зарегистрированные объекты: [objects.csv](objects.csv).

Блок-сущности и наличие фабрик: [block_entities.csv](block_entities.csv).
Незарегистрированные типы могут сохранять NBT через `UnimplementedBlockEntity`; это не игровая реализация.

Маркеры TODO/FIXME из исходников: [source_markers.csv](source_markers.csv), строк: 255.
Это текстовые указатели для разбора, включая потенциально устаревшие комментарии; один маркер не равен одной задаче.

## Классы без сгенерированного назначения

### items

| Класс | Имена реестра |
| --- | --- |
| `ArmorStandItem` | `armor_stand` |
| `ArrowItem` | `arrow` |
| `BannerItem` | `black_banner`, `blue_banner`, `brown_banner`, `cyan_banner`, `gray_banner`, `green_banner`, `light_blue_banner`, `light_gray_banner`, `lime_banner`, `magenta_banner`, `orange_banner`, `pink_banner`, `purple_banner`, `red_banner`, `white_banner`, `yellow_banner` |
| `BoatItem` | `acacia_boat`, `acacia_chest_boat`, `bamboo_chest_raft`, `bamboo_raft`, `birch_boat`, `birch_chest_boat`, `cherry_boat`, `cherry_chest_boat`, `dark_oak_boat`, `dark_oak_chest_boat`, `jungle_boat`, `jungle_chest_boat`, `mangrove_boat`, `mangrove_chest_boat`, `oak_boat`, `oak_chest_boat`, `pale_oak_boat`, `pale_oak_chest_boat`, `poplar_boat`, `poplar_chest_boat`, `spruce_boat`, `spruce_chest_boat` |
| `BowItem` | `bow` |
| `CrossbowItem` | `crossbow` |
| `CushionItem` | `black_cushion`, `blue_cushion`, `brown_cushion`, `cyan_cushion`, `gray_cushion`, `green_cushion`, `light_blue_cushion`, `light_gray_cushion`, `lime_cushion`, `magenta_cushion`, `orange_cushion`, `pink_cushion`, `purple_cushion`, `red_cushion`, `white_cushion`, `yellow_cushion` |
| `DebugStickItem` | `debug_stick` |
| `DiscFragmentItem` | `disc_fragment_5` |
| `EmptyMapItem` | `map` |
| `EndCrystalItem` | `end_crystal` |
| `GameMasterBlockItem` | `chain_command_block`, `command_block`, `jigsaw`, `repeating_command_block`, `structure_block`, `test_block`, `test_instance_block` |
| `HangingEntityItem` | `painting` |
| `Item` | `amethyst_shard`, `angler_pottery_sherd`, `apple`, `archer_pottery_sherd`, `armadillo_scute`, `arms_up_pottery_sherd`, `baked_potato`, `beef`, `beetroot`, `beetroot_soup`, `black_harness`, `blade_pottery_sherd`, `blaze_powder`, `blaze_rod`, `blue_harness`, `bone`, `book`, `bordure_indented_banner_pattern`, `bowl`, `bread`, `breeze_rod`, `brewer_pottery_sherd`, `brick`, `brown_harness`, `burn_pottery_sherd`, `chainmail_boots`, `chainmail_chestplate`, `chainmail_helmet`, `chainmail_leggings`, `charcoal`, `chicken`, `chorus_fruit`, `clay_ball`, `clock`, `coal`, `cod`, `cooked_beef`, `cooked_chicken`, `cooked_cod`, `cooked_mutton`, `cooked_porkchop`, `cooked_rabbit`, `cooked_salmon`, `cookie`, `copper_axe`, `copper_boots`, `copper_chestplate`, `copper_helmet`, `copper_hoe`, `copper_horse_armor`, `copper_ingot`, `copper_leggings`, `copper_nautilus_armor`, `copper_nugget`, `copper_pickaxe`, `copper_shovel`, `copper_spear`, `copper_sword`, `creeper_banner_pattern`, `cyan_harness`, `danger_pottery_sherd`, `diamond`, `diamond_axe`, `diamond_boots`, `diamond_chestplate`, `diamond_helmet`, `diamond_hoe`, `diamond_horse_armor`, `diamond_leggings`, `diamond_nautilus_armor`, `diamond_pickaxe`, `diamond_shovel`, `diamond_spear`, `diamond_sword`, `dragon_breath`, `dried_kelp`, `echo_shard`, `elytra`, `emerald`, `enchanted_book`, `enchanted_golden_apple`, `explorer_pottery_sherd`, `feather`, `fermented_spider_eye`, `field_masoned_banner_pattern`, `firework_star`, `flint`, `flow_banner_pattern`, `flow_pottery_sherd`, `flower_banner_pattern`, `friend_pottery_sherd`, `ghast_tear`, `glistering_melon_slice`, `globe_banner_pattern`, `glowstone_dust`, `gold_ingot`, `gold_nugget`, `golden_apple`, `golden_axe`, `golden_boots`, `golden_carrot`, `golden_chestplate`, `golden_helmet`, `golden_hoe`, `golden_horse_armor`, `golden_leggings`, `golden_nautilus_armor`, `golden_pickaxe`, `golden_shovel`, `golden_spear`, `golden_sword`, `gray_harness`, `green_harness`, `gunpowder`, `guster_banner_pattern`, `guster_pottery_sherd`, `heart_of_the_sea`, `heart_pottery_sherd`, `heartbreak_pottery_sherd`, `honey_bottle`, `howl_pottery_sherd`, `iron_axe`, `iron_boots`, `iron_chestplate`, `iron_helmet`, `iron_hoe`, `iron_horse_armor`, `iron_ingot`, `iron_leggings`, `iron_nautilus_armor`, `iron_nugget`, `iron_pickaxe`, `iron_shovel`, `iron_spear`, `iron_sword`, `lapis_lazuli`, `leather`, `leather_boots`, `leather_chestplate`, `leather_helmet`, `leather_horse_armor`, `leather_leggings`, `light_blue_harness`, `light_gray_harness`, `lime_harness`, `magenta_harness`, `magma_cream`, `melon_slice`, `milk_bucket`, `miner_pottery_sherd`, `mojang_banner_pattern`, `mourner_pottery_sherd`, `mushroom_stew`, `music_disc_11`, `music_disc_13`, `music_disc_5`, `music_disc_blocks`, `music_disc_bounce`, `music_disc_cat`, `music_disc_chirp`, `music_disc_creator`, `music_disc_creator_music_box`, `music_disc_far`, `music_disc_lava_chicken`, `music_disc_mall`, `music_disc_mellohi`, `music_disc_otherside`, `music_disc_pigstep`, `music_disc_precipice`, `music_disc_relic`, `music_disc_stal`, `music_disc_strad`, `music_disc_tears`, `music_disc_wait`, `music_disc_ward`, `mutton`, `nautilus_shell`, `nether_brick`, `nether_star`, `netherite_axe`, `netherite_boots`, `netherite_chestplate`, `netherite_helmet`, `netherite_hoe`, `netherite_horse_armor`, `netherite_ingot`, `netherite_leggings`, `netherite_nautilus_armor`, `netherite_pickaxe`, `netherite_scrap`, `netherite_shovel`, `netherite_spear`, `netherite_sword`, `ominous_bottle`, `ominous_trial_key`, `orange_harness`, `paper`, `phantom_membrane`, `piglin_banner_pattern`, `pink_harness`, `plenty_pottery_sherd`, `poisonous_potato`, `popped_chorus_fruit`, `porkchop`, `prismarine_crystals`, `prismarine_shard`, `prize_pottery_sherd`, `pufferfish`, `pumpkin_pie`, `purple_harness`, `quartz`, `rabbit`, `rabbit_foot`, `rabbit_hide`, `rabbit_stew`, `raw_copper`, `raw_gold`, `raw_iron`, `recovery_compass`, `red_harness`, `resin_brick`, `rotten_flesh`, `saddle`, `salmon`, `scrape_pottery_sherd`, `sheaf_pottery_sherd`, `shelter_pottery_sherd`, `shulker_shell`, `skull_banner_pattern`, `skull_pottery_sherd`, `slime_ball`, `snort_pottery_sherd`, `spider_eye`, `stick`, `stone_axe`, `stone_hoe`, `stone_pickaxe`, `stone_shovel`, `stone_spear`, `stone_sword`, `sugar`, `suspicious_stew`, `totem_of_undying`, `trial_key`, `tropical_fish`, `turtle_helmet`, `turtle_scute`, `wheat`, `white_harness`, `wolf_armor`, `wooden_axe`, `wooden_hoe`, `wooden_pickaxe`, `wooden_shovel`, `wooden_spear`, `wooden_sword`, `yellow_harness` |
| `KnowledgeBookItem` | `knowledge_book` |
| `MapItem` | `abandoned_camp_map`, `buried_ancient_city_map`, `buried_mineshaft_map`, `buried_treasure_map`, `buried_trial_chambers_map`, `desert_pyramid_map`, `desert_village_map`, `filled_map`, `jungle_pyramid_map`, `ocean_monument_map`, `plains_village_map`, `savanna_village_map`, `snowy_village_map`, `swamp_hut_map`, `taiga_village_map`, `warm_ocean_ruins_map`, `woodland_mansion_map` |
| `MinecartItem` | `chest_minecart`, `command_block_minecart`, `furnace_minecart`, `hopper_minecart`, `minecart`, `tnt_minecart` |
| `MobBucketItem` | `axolotl_bucket`, `cod_bucket`, `pufferfish_bucket`, `salmon_bucket`, `sulfur_cube_bucket`, `tadpole_bucket`, `tropical_fish_bucket` |
| `SmithingTemplateItem` | `bolt_armor_trim_smithing_template`, `coast_armor_trim_smithing_template`, `dune_armor_trim_smithing_template`, `eye_armor_trim_smithing_template`, `flow_armor_trim_smithing_template`, `host_armor_trim_smithing_template`, `netherite_upgrade_smithing_template`, `raiser_armor_trim_smithing_template`, `rib_armor_trim_smithing_template`, `sentry_armor_trim_smithing_template`, `shaper_armor_trim_smithing_template`, `silence_armor_trim_smithing_template`, `snout_armor_trim_smithing_template`, `spire_armor_trim_smithing_template`, `tide_armor_trim_smithing_template`, `vex_armor_trim_smithing_template`, `ward_armor_trim_smithing_template`, `wayfinder_armor_trim_smithing_template`, `wild_armor_trim_smithing_template` |
| `SpectralArrowItem` | `spectral_arrow` |
| `TridentItem` | `trident` |
| `WindChargeItem` | `wind_charge` |

### blocks

| Класс | Имена реестра |
| --- | --- |
| `AirBlock` | `air`, `cave_air`, `void_air` |
| `Block` | `acacia_planks`, `ancient_debris`, `andesite`, `bamboo_mosaic`, `bamboo_planks`, `bedrock`, `birch_planks`, `black_concrete`, `black_terracotta`, `black_wool`, `blackstone`, `blue_concrete`, `blue_terracotta`, `blue_wool`, `bookshelf`, `bricks`, `brown_concrete`, `brown_terracotta`, `brown_wool`, `calcite`, `cherry_planks`, `chiseled_cinnabar`, `chiseled_deepslate`, `chiseled_nether_bricks`, `chiseled_polished_blackstone`, `chiseled_quartz_block`, `chiseled_red_sandstone`, `chiseled_resin_bricks`, `chiseled_sandstone`, `chiseled_stone_bricks`, `chiseled_sulfur`, `chiseled_tuff`, `chiseled_tuff_bricks`, `cinnabar`, `cinnabar_bricks`, `clay`, `coal_block`, `coarse_dirt`, `cobbled_deepslate`, `cobblestone`, `cracked_deepslate_bricks`, `cracked_deepslate_tiles`, `cracked_nether_bricks`, `cracked_polished_blackstone_bricks`, `cracked_stone_bricks`, `crimson_planks`, `cut_red_sandstone`, `cut_sandstone`, `cyan_concrete`, `cyan_terracotta`, `cyan_wool`, `dark_oak_planks`, `dark_prismarine`, `dead_brain_coral_block`, `dead_bubble_coral_block`, `dead_fire_coral_block`, `dead_horn_coral_block`, `dead_tube_coral_block`, `deepslate_bricks`, `deepslate_tiles`, `diamond_block`, `diorite`, `dirt`, `dried_kelp_block`, `dripstone_block`, `emerald_block`, `end_stone`, `end_stone_bricks`, `fletching_table`, `gilded_blackstone`, `glowstone`, `gold_block`, `granite`, `gray_concrete`, `gray_terracotta`, `gray_wool`, `green_concrete`, `green_terracotta`, `green_wool`, `honeycomb_block`, `iron_block`, `jungle_planks`, `lapis_block`, `light_blue_concrete`, `light_blue_terracotta`, `light_blue_wool`, `light_gray_concrete`, `light_gray_terracotta`, `light_gray_wool`, `lime_concrete`, `lime_terracotta`, `lime_wool`, `lodestone`, `magenta_concrete`, `magenta_terracotta`, `magenta_wool`, `mangrove_planks`, `melon`, `mossy_cobblestone`, `mossy_stone_bricks`, `mud_bricks`, `nether_bricks`, `nether_wart_block`, `netherite_block`, `oak_planks`, `obsidian`, `orange_concrete`, `orange_terracotta`, `orange_wool`, `packed_ice`, `packed_mud`, `pale_oak_planks`, `pink_concrete`, `pink_terracotta`, `pink_wool`, `polished_andesite`, `polished_blackstone`, `polished_blackstone_bricks`, `polished_cinnabar`, `polished_deepslate`, `polished_diorite`, `polished_granite`, `polished_sulfur`, `polished_tuff`, `poplar_planks`, `prismarine`, `prismarine_bricks`, `purple_concrete`, `purple_terracotta`, `purple_wool`, `purpur_block`, `quartz_block`, `quartz_bricks`, `raw_copper_block`, `raw_gold_block`, `raw_iron_block`, `red_concrete`, `red_nether_bricks`, `red_sandstone`, `red_terracotta`, `red_wool`, `reinforced_deepslate`, `resin_block`, `resin_bricks`, `sandstone`, `sea_lantern`, `shroomlight`, `smooth_basalt`, `smooth_quartz`, `smooth_red_sandstone`, `smooth_sandstone`, `smooth_stone`, `snow_block`, `soul_soil`, `spruce_planks`, `stone`, `stone_bricks`, `sulfur`, `sulfur_bricks`, `terracotta`, `tuff`, `tuff_bricks`, `warped_planks`, `warped_wart_block`, `waxed_chiseled_copper`, `waxed_copper_block`, `waxed_cut_copper`, `waxed_exposed_chiseled_copper`, `waxed_exposed_copper`, `waxed_exposed_cut_copper`, `waxed_oxidized_chiseled_copper`, `waxed_oxidized_copper`, `waxed_oxidized_cut_copper`, `waxed_weathered_chiseled_copper`, `waxed_weathered_copper`, `waxed_weathered_cut_copper`, `white_concrete`, `white_terracotta`, `white_wool`, `yellow_concrete`, `yellow_terracotta`, `yellow_wool` |
| `BonemealableFeaturePlacerBlock` | `moss_block`, `pale_moss_block` |
| `BrewingStandBlock` | `brewing_stand` |
| `CalibratedSculkSensorBlock` | `calibrated_sculk_sensor` |
| `CartographyTableBlock` | `cartography_table` |
| `ChestBlock` | `chest` |
| `CommandBlock` | `chain_command_block`, `command_block`, `repeating_command_block` |
| `CopperGolemStatueBlock` | `waxed_copper_golem_statue`, `waxed_exposed_copper_golem_statue`, `waxed_oxidized_copper_golem_statue`, `waxed_weathered_copper_golem_statue` |
| `CrafterBlock` | `crafter` |
| `CreakingHeartBlock` | `creaking_heart` |
| `CryingObsidianBlock` | `crying_obsidian` |
| `DecoratedPotBlock` | `decorated_pot` |
| `DispenserBlock` | `dispenser` |
| `DriedGhastBlock` | `dried_ghast` |
| `DropperBlock` | `dropper` |
| `EnchantingTableBlock` | `enchanting_table` |
| `FrogspawnBlock` | `frogspawn` |
| `HalfTransparentBlock` | `blue_ice` |
| `HopperBlock` | `hopper` |
| `InfestedBlock` | `infested_chiseled_stone_bricks`, `infested_cobblestone`, `infested_cracked_stone_bricks`, `infested_mossy_stone_bricks`, `infested_stone`, `infested_stone_bricks` |
| `InfestedRotatedPillarBlock` | `infested_deepslate` |
| `JigsawBlock` | `jigsaw` |
| `LeavesBlock` | `spruce_leaves` |
| `LecternBlock` | `lectern` |
| `LightBlock` | `light` |
| `LoomBlock` | `loom` |
| `NyliumBlock` | `crimson_nylium`, `warped_nylium` |
| `SculkBlock` | `sculk` |
| `SculkCatalystBlock` | `sculk_catalyst` |
| `SculkSensorBlock` | `sculk_sensor` |
| `SculkShriekerBlock` | `sculk_shrieker` |
| `ShelfBlock` | `acacia_shelf`, `bamboo_shelf`, `birch_shelf`, `cherry_shelf`, `crimson_shelf`, `dark_oak_shelf`, `jungle_shelf`, `mangrove_shelf`, `oak_shelf`, `pale_oak_shelf`, `poplar_shelf`, `spruce_shelf`, `warped_shelf` |
| `SmithingTableBlock` | `smithing_table` |
| `SnifferEggBlock` | `sniffer_egg` |
| `SpawnerBlock` | `spawner` |
| `StonecutterBlock` | `stonecutter` |
| `StructureBlock` | `structure_block` |
| `StructureVoidBlock` | `structure_void` |
| `TestBlock` | `test_block` |
| `TestInstanceBlock` | `test_instance_block` |
| `TintedGlassBlock` | `tinted_glass` |
| `TntBlock` | `tnt` |
| `TransparentBlock` | `glass` |
| `TrappedChestBlock` | `trapped_chest` |
| `TrialSpawnerBlock` | `trial_spawner` |
| `VaultBlock` | `vault` |
| `WeatheringCopperGolemStatueBlock` | `copper_golem_statue`, `exposed_copper_golem_statue`, `oxidized_copper_golem_statue`, `weathered_copper_golem_statue` |

### entities

| Класс | Имена реестра |
| --- | --- |
| `Allay` | `allay` |
| `AreaEffectCloud` | `area_effect_cloud` |
| `Armadillo` | `armadillo` |
| `ArmorStand` | `armor_stand` |
| `Arrow` | `arrow` |
| `Axolotl` | `axolotl` |
| `Bee` | `bee` |
| `Blaze` | `blaze` |
| `Boat` | `acacia_boat`, `birch_boat`, `cherry_boat`, `dark_oak_boat`, `jungle_boat`, `mangrove_boat`, `oak_boat`, `pale_oak_boat`, `poplar_boat`, `spruce_boat` |
| `Bogged` | `bogged` |
| `Breeze` | `breeze` |
| `BreezeWindCharge` | `breeze_wind_charge` |
| `Camel` | `camel` |
| `CamelHusk` | `camel_husk` |
| `Cat` | `cat` |
| `CaveSpider` | `cave_spider` |
| `ChestBoat` | `acacia_chest_boat`, `birch_chest_boat`, `cherry_chest_boat`, `dark_oak_chest_boat`, `jungle_chest_boat`, `mangrove_chest_boat`, `oak_chest_boat`, `pale_oak_chest_boat`, `poplar_chest_boat`, `spruce_chest_boat` |
| `ChestRaft` | `bamboo_chest_raft` |
| `Cod` | `cod` |
| `CopperGolem` | `copper_golem` |
| `Creaking` | `creaking` |
| `Creeper` | `creeper` |
| `Cushion` | `cushion` |
| `Dolphin` | `dolphin` |
| `Donkey` | `donkey` |
| `DragonFireball` | `dragon_fireball` |
| `Drowned` | `drowned` |
| `ElderGuardian` | `elder_guardian` |
| `EnderDragon` | `ender_dragon` |
| `Enderman` | `enderman` |
| `Evoker` | `evoker` |
| `EvokerFangs` | `evoker_fangs` |
| `Fox` | `fox` |
| `Frog` | `frog` |
| `Ghast` | `ghast` |
| `Giant` | `giant` |
| `GlowSquid` | `glow_squid` |
| `Goat` | `goat` |
| `Guardian` | `guardian` |
| `HappyGhast` | `happy_ghast` |
| `Hoglin` | `hoglin` |
| `Horse` | `horse` |
| `Husk` | `husk` |
| `Illusioner` | `illusioner` |
| `IronGolem` | `iron_golem` |
| `ItemDisplay` | `item_display` |
| `LargeFireball` | `fireball` |
| `LightningBolt` | `lightning_bolt` |
| `Llama` | `llama` |
| `LlamaSpit` | `llama_spit` |
| `MagmaCube` | `magma_cube` |
| `Mannequin` | `mannequin` |
| `Minecart` | `minecart` |
| `MinecartCommandBlock` | `command_block_minecart` |
| `MinecartFurnace` | `furnace_minecart` |
| `MinecartHopper` | `hopper_minecart` |
| `MinecartSpawner` | `spawner_minecart` |
| `MinecartTNT` | `tnt_minecart` |
| `Mule` | `mule` |
| `Nautilus` | `nautilus` |
| `Ocelot` | `ocelot` |
| `OminousItemSpawner` | `ominous_item_spawner` |
| `Painting` | `painting` |
| `Panda` | `panda` |
| `Parched` | `parched` |
| `Parrot` | `parrot` |
| `Phantom` | `phantom` |
| `Piglin` | `piglin` |
| `PiglinBrute` | `piglin_brute` |
| `Pillager` | `pillager` |
| `Player` | `player` |
| `PolarBear` | `polar_bear` |
| `PrimedTnt` | `tnt` |
| `Pufferfish` | `pufferfish` |
| `Rabbit` | `rabbit` |
| `Raft` | `bamboo_raft` |
| `Ravager` | `ravager` |
| `Salmon` | `salmon` |
| `Shulker` | `shulker` |
| `ShulkerBullet` | `shulker_bullet` |
| `Silverfish` | `silverfish` |
| `Skeleton` | `skeleton` |
| `SkeletonHorse` | `skeleton_horse` |
| `Slime` | `slime` |
| `SmallFireball` | `small_fireball` |
| `Sniffer` | `sniffer` |
| `SnowGolem` | `snow_golem` |
| `SpectralArrow` | `spectral_arrow` |
| `Spider` | `spider` |
| `Squid` | `squid` |
| `Stray` | `stray` |
| `Strider` | `strider` |
| `SulfurCube` | `sulfur_cube` |
| `Tadpole` | `tadpole` |
| `TextDisplay` | `text_display` |
| `ThrownLingeringPotion` | `lingering_potion` |
| `ThrownTrident` | `trident` |
| `TraderLlama` | `trader_llama` |
| `TropicalFish` | `tropical_fish` |
| `Turtle` | `turtle` |
| `Vex` | `vex` |
| `Villager` | `villager` |
| `Vindicator` | `vindicator` |
| `WanderingTrader` | `wandering_trader` |
| `Warden` | `warden` |
| `WindCharge` | `wind_charge` |
| `Witch` | `witch` |
| `WitherBoss` | `wither` |
| `WitherSkeleton` | `wither_skeleton` |
| `WitherSkull` | `wither_skull` |
| `Wolf` | `wolf` |
| `Zoglin` | `zoglin` |
| `Zombie` | `zombie` |
| `ZombieHorse` | `zombie_horse` |
| `ZombieNautilus` | `zombie_nautilus` |
| `ZombieVillager` | `zombie_villager` |
| `ZombifiedPiglin` | `zombified_piglin` |

## Обновление

Из корня репозитория:

```sh
cargo check --locked -p vecta-core
python scripts/audit_vanilla.py
```

Скрипт читает результаты генераторов. После изменения регистраций сначала пересоберите проект.
Исходные JSON Minecraft и сгенерированные Rust-файлы скрипт не изменяет.
Хеши входных каталогов и результатов генерации сохранены в [provenance.json](provenance.json).
