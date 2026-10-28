//! Type-erased data component values.

use std::fmt::{self, Debug, Formatter};

use vecta_utils::{Downcast as _, DowncastType, DowncastTypeKey, ErasedType};

use crate::block_transformer::BlockTransformerComponent;

use super::components::{
    ArmorTrim, AttackRange, BannerPatternLayers, Bees, BlockEntityData, BlockItemStateProperties,
    BlocksAttacks, BrewingFuel, BundleContents, ChargedProjectiles, Compostable, Consumable,
    CookingFuel, CustomData, CustomModelData, DamageResistant, DamageTypeComponent,
    DeathProtection, DebugStickState, DyedItemColor, Enchantable, EntityData, Equippable,
    FireworkExplosion, Fireworks, FoodProperties, InstrumentComponent, ItemAttributeModifiers,
    ItemContainerContents, ItemEnchantments, ItemLore, JukeboxPlayable, KineticWeapon,
    LodestoneTracker, MapDecorations, MapId, MapItemColor, MapPostProcessing, MobVisibility,
    OminousBottleAmplifier, PaintingVariantComponent, PiercingWeapon, PotDecorations,
    PotionContents, ProvidesBannerPatterns, ProvidesPotteryPattern, ProvidesTrimMaterial, Rarity,
    Recipes, Repairable, SeededContainerLoot, SignText, SulfurCubeContent, SuspiciousStewEffects,
    SwingAnimation, Tool, TooltipDisplay, UseCooldown, UseEffects, UseRemainder, VillagerFood,
    Weapon, WritableBookContent, WrittenBookContent,
};
use crate::cat_sound_variant::CatSoundVariant;
use crate::cat_variant::CatVariant;
use crate::chicken_sound_variant::ChickenSoundVariant;
use crate::chicken_variant::ChickenVariant;
use crate::cow_sound_variant::CowSoundVariant;
use crate::cow_variant::CowVariant;
use crate::frog_variant::FrogVariant;
use crate::item_predicate::{AdventureModePredicate, LockCode};
use crate::pig_sound_variant::PigSoundVariant;
use crate::pig_variant::PigVariant;
use crate::resolvable_profile::ResolvableProfile;
use crate::villager_type::VillagerType;
use crate::wolf_sound_variant::WolfSoundVariant;
use crate::wolf_variant::WolfVariant;
use crate::zombie_nautilus_variant::ZombieNautilusVariant;
use crate::{
    AxolotlVariant, DyeColor, FoxVariant, HorseVariant, LlamaVariant, MooshroomVariant,
    ParrotVariant, RabbitVariant, RegistryReference, SalmonVariant, TropicalFishPattern,
};

/// Behavior required from a value stored in a [`ComponentData`].
///
/// Concrete type recovery is provided by Vecta's deterministic keyed
/// downcasting foundation. A value is eligible for the blanket implementation
/// when it also supports cloning, comparison, debugging, and shared server
/// access. Persistent-codec hashing is registered separately so transient
/// values do not need a fake hash representation.
pub trait Component: ErasedType + Debug + Send + Sync + 'static {
    #[doc(hidden)]
    fn clone_component(&self) -> Box<dyn Component>;

    #[doc(hidden)]
    fn component_eq(&self, other: &dyn Component) -> bool;
}

impl<T> Component for T
where
    T: DowncastType + Clone + Debug + PartialEq + Send + Sync,
{
    fn clone_component(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn component_eq(&self, other: &dyn Component) -> bool {
        other.downcast_ref::<T>() == Some(self)
    }
}

/// A type-erased component value.
///
/// Component values retain their concrete Rust type and can be recovered with
/// [`Self::downcast_ref`].
pub struct ComponentData {
    value: Box<dyn Component>,
}

impl ComponentData {
    /// Erases a typed component value.
    #[must_use]
    pub fn new(value: impl Component) -> Self {
        Self {
            value: Box::new(value),
        }
    }

    /// Returns the concrete value when it has type `T`.
    #[must_use]
    pub fn downcast_ref<T: DowncastType>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }

    /// Returns the concrete type key.
    #[must_use]
    pub fn type_key(&self) -> DowncastTypeKey {
        self.value.downcast_type_key()
    }
}

impl Clone for ComponentData {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone_component(),
        }
    }
}

impl Debug for ComponentData {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ComponentData")
            .field(&self.value)
            .finish()
    }
}

impl PartialEq for ComponentData {
    fn eq(&self, other: &Self) -> bool {
        self.value.component_eq(other.value.as_ref())
    }
}

macro_rules! impl_component_downcast_type {
    ($type:ty, $key:literal) => {
        // SAFETY: This Vecta-owned key uniquely identifies the concrete
        // component implementation within the process.
        unsafe impl DowncastType for $type {
            const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new($key);
        }
    };
}

impl_component_downcast_type!(DamageTypeComponent, "vecta:item_component/damage_type");
impl_component_downcast_type!(
    AdventureModePredicate,
    "vecta:item_component/adventure_mode_predicate"
);
impl_component_downcast_type!(LockCode, "vecta:item_component/lock");
impl_component_downcast_type!(CustomData, "vecta:item_component/custom_data");
impl_component_downcast_type!(CustomModelData, "vecta:item_component/custom_model_data");
impl_component_downcast_type!(DyeColor, "vecta:dye_color");
impl_component_downcast_type!(FoxVariant, "vecta:fox_variant");
impl_component_downcast_type!(SalmonVariant, "vecta:salmon_variant");
impl_component_downcast_type!(ParrotVariant, "vecta:parrot_variant");
impl_component_downcast_type!(TropicalFishPattern, "vecta:tropical_fish_pattern");
impl_component_downcast_type!(MooshroomVariant, "vecta:mooshroom_variant");
impl_component_downcast_type!(RabbitVariant, "vecta:rabbit_variant");
impl_component_downcast_type!(HorseVariant, "vecta:horse_variant");
impl_component_downcast_type!(LlamaVariant, "vecta:llama_variant");
impl_component_downcast_type!(AxolotlVariant, "vecta:axolotl_variant");
impl_component_downcast_type!(DyedItemColor, "vecta:item_component/dyed_item_color");
impl_component_downcast_type!(MapItemColor, "vecta:item_component/map_item_color");
impl_component_downcast_type!(MapId, "vecta:item_component/map_id");
impl_component_downcast_type!(FoodProperties, "vecta:item_component/food");
impl_component_downcast_type!(
    SuspiciousStewEffects,
    "vecta:item_component/suspicious_stew_effects"
);
impl_component_downcast_type!(
    WritableBookContent,
    "vecta:item_component/writable_book_content"
);
impl_component_downcast_type!(
    WrittenBookContent,
    "vecta:item_component/written_book_content"
);
impl_component_downcast_type!(DebugStickState, "vecta:item_component/debug_stick_state");
impl_component_downcast_type!(Bees, "vecta:item_component/bees");
impl_component_downcast_type!(EntityData, "vecta:item_component/entity_data");
impl_component_downcast_type!(BlockEntityData, "vecta:item_component/block_entity_data");
impl_component_downcast_type!(KineticWeapon, "vecta:item_component/kinetic_weapon");
impl_component_downcast_type!(LodestoneTracker, "vecta:item_component/lodestone_tracker");
impl_component_downcast_type!(MapDecorations, "vecta:item_component/map_decorations");
impl_component_downcast_type!(FireworkExplosion, "vecta:item_component/firework_explosion");
impl_component_downcast_type!(Fireworks, "vecta:item_component/fireworks");
impl_component_downcast_type!(BlockItemStateProperties, "vecta:item_component/block_state");
impl_component_downcast_type!(BlocksAttacks, "vecta:item_component/blocks_attacks");
impl_component_downcast_type!(Consumable, "vecta:item_component/consumable");
impl_component_downcast_type!(DeathProtection, "vecta:item_component/death_protection");
impl_component_downcast_type!(ResolvableProfile, "vecta:item_component/profile");
impl_component_downcast_type!(SeededContainerLoot, "vecta:item_component/container_loot");
impl_component_downcast_type!(
    OminousBottleAmplifier,
    "vecta:item_component/ominous_bottle_amplifier"
);
impl_component_downcast_type!(Enchantable, "vecta:item_component/enchantable");
impl_component_downcast_type!(InstrumentComponent, "vecta:item_component/instrument");
impl_component_downcast_type!(ArmorTrim, "vecta:item_component/trim");
impl_component_downcast_type!(Recipes, "vecta:item_component/recipes");
impl_component_downcast_type!(PotDecorations, "vecta:item_component/pot_decorations");
impl_component_downcast_type!(PotionContents, "vecta:item_component/potion_contents");
impl_component_downcast_type!(UseRemainder, "vecta:item_component/use_remainder");
impl_component_downcast_type!(
    ChargedProjectiles,
    "vecta:item_component/charged_projectiles"
);
impl_component_downcast_type!(BundleContents, "vecta:item_component/bundle_contents");
impl_component_downcast_type!(ItemContainerContents, "vecta:item_component/container");
impl_component_downcast_type!(
    SulfurCubeContent,
    "vecta:item_component/sulfur_cube_content"
);
impl_component_downcast_type!(BannerPatternLayers, "vecta:item_component/banner_patterns");
impl_component_downcast_type!(
    PaintingVariantComponent,
    "vecta:item_component/painting_variant"
);
impl_component_downcast_type!(
    ProvidesTrimMaterial,
    "vecta:item_component/provides_trim_material"
);
impl_component_downcast_type!(
    ProvidesPotteryPattern,
    "vecta:item_component/provides_pottery_pattern"
);
impl_component_downcast_type!(JukeboxPlayable, "vecta:item_component/jukebox_playable");
impl_component_downcast_type!(
    ProvidesBannerPatterns,
    "vecta:item_component/provides_banner_patterns"
);
impl_component_downcast_type!(DamageResistant, "vecta:item_component/damage_resistant");
impl_component_downcast_type!(Repairable, "vecta:item_component/repairable");
impl_component_downcast_type!(Tool, "vecta:item_component/tool");
impl_component_downcast_type!(Weapon, "vecta:item_component/weapon");
impl_component_downcast_type!(AttackRange, "vecta:item_component/attack_range");
impl_component_downcast_type!(UseCooldown, "vecta:item_component/use_cooldown");
impl_component_downcast_type!(UseEffects, "vecta:item_component/use_effects");
impl_component_downcast_type!(ItemLore, "vecta:item_component/lore");
impl_component_downcast_type!(Rarity, "vecta:item_component/rarity");
impl_component_downcast_type!(TooltipDisplay, "vecta:item_component/tooltip_display");
impl_component_downcast_type!(SwingAnimation, "vecta:item_component/swing_animation");
impl_component_downcast_type!(CookingFuel, "vecta:item_component/cooking_fuel");
impl_component_downcast_type!(BrewingFuel, "vecta:item_component/brewing_fuel");
impl_component_downcast_type!(MobVisibility, "vecta:item_component/mob_visibility");
impl_component_downcast_type!(
    BlockTransformerComponent,
    "vecta:item_component/block_transformer"
);
impl_component_downcast_type!(VillagerFood, "vecta:item_component/villager_food");
impl_component_downcast_type!(SignText, "vecta:item_component/sign_text");
impl_component_downcast_type!(Compostable, "vecta:item_component/compostable");
impl_component_downcast_type!(
    MapPostProcessing,
    "vecta:item_component/map_post_processing"
);
impl_component_downcast_type!(PiercingWeapon, "vecta:item_component/piercing_weapon");
impl_component_downcast_type!(Equippable, "vecta:item_component/equippable");
impl_component_downcast_type!(
    ItemAttributeModifiers,
    "vecta:item_component/attribute_modifiers"
);
impl_component_downcast_type!(ItemEnchantments, "vecta:item_component/enchantments");
impl_component_downcast_type!(
    RegistryReference<VillagerType>,
    "vecta:item_component/villager_variant"
);
impl_component_downcast_type!(
    RegistryReference<WolfVariant>,
    "vecta:item_component/wolf_variant"
);
impl_component_downcast_type!(
    RegistryReference<WolfSoundVariant>,
    "vecta:item_component/wolf_sound_variant"
);
impl_component_downcast_type!(
    RegistryReference<PigVariant>,
    "vecta:item_component/pig_variant"
);
impl_component_downcast_type!(
    RegistryReference<PigSoundVariant>,
    "vecta:item_component/pig_sound_variant"
);
impl_component_downcast_type!(
    RegistryReference<CowVariant>,
    "vecta:item_component/cow_variant"
);
impl_component_downcast_type!(
    RegistryReference<CowSoundVariant>,
    "vecta:item_component/cow_sound_variant"
);
impl_component_downcast_type!(
    RegistryReference<ChickenVariant>,
    "vecta:item_component/chicken_variant"
);
impl_component_downcast_type!(
    RegistryReference<ChickenSoundVariant>,
    "vecta:item_component/chicken_sound_variant"
);
impl_component_downcast_type!(
    RegistryReference<ZombieNautilusVariant>,
    "vecta:item_component/zombie_nautilus_variant"
);
impl_component_downcast_type!(
    RegistryReference<FrogVariant>,
    "vecta:item_component/frog_variant"
);
impl_component_downcast_type!(
    RegistryReference<CatVariant>,
    "vecta:item_component/cat_variant"
);
impl_component_downcast_type!(
    RegistryReference<CatSoundVariant>,
    "vecta:item_component/cat_sound_variant"
);

#[cfg(test)]
mod tests {
    use super::ComponentData;

    #[test]
    fn typed_values_downcast_by_deterministic_key() {
        let value = ComponentData::new(17_i32);

        assert_eq!(value.downcast_ref::<i32>(), Some(&17));
        assert_eq!(value.downcast_ref::<bool>(), None);
    }

    #[test]
    fn equality_requires_the_same_concrete_type() {
        assert_eq!(ComponentData::new(17_i32), ComponentData::new(17_i32));
        assert_ne!(ComponentData::new(17_i32), ComponentData::new(17.0_f32));
    }
}
