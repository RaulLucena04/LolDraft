use super::*;
use crate::analysis::{analyze_team, AnalysisParams, PickedChampion};
use crate::profile::{ArchetypeRating, ChampionProfile, ProfileRole, ProfileSet, TraitRating};
use crate::taxonomy::{ArchetypeId, TraitId, TAXONOMY_VERSION};
use loldraft_domain::{ChampionKey, Role, Side};

fn curated() -> Provenance {
    Provenance::Curated {
        editor: "test".into(),
        review: "test".into(),
    }
}

fn archetype(value: Option<f32>) -> ArchetypeRating {
    ArchetypeRating {
        intensity: value,
        confidence: if value.is_some() { 1.0 } else { 0.0 },
        source: curated(),
    }
}

fn trait_rating(value: Option<f32>) -> TraitRating {
    TraitRating {
        value,
        confidence: if value.is_some() { 1.0 } else { 0.0 },
        source: curated(),
    }
}

fn profile(
    key: u16,
    role: Role,
    archetypes: &[(ArchetypeId, f32)],
    traits: &[(TraitId, f32)],
) -> ChampionProfile {
    ChampionProfile {
        key: ChampionKey(key),
        slug: format!("synth{key}"),
        name: format!("SYNTH_{key}"),
        roles: vec![ProfileRole {
            role,
            proficiency: 1.0,
            confidence: 1.0,
            source: curated(),
        }],
        archetypes: archetypes
            .iter()
            .map(|(id, value)| (*id, archetype(Some(*value))))
            .collect(),
        traits: traits
            .iter()
            .map(|(id, value)| (*id, trait_rating(Some(*value))))
            .collect(),
    }
}

fn set(profiles: Vec<ChampionProfile>) -> ProfileSet {
    ProfileSet {
        id: "synth".into(),
        version: "1".into(),
        taxonomy_version: TAXONOMY_VERSION.into(),
        patch: Some("16.22".into()),
        champions: profiles
            .into_iter()
            .map(|profile| (profile.key, profile))
            .collect(),
    }
}

fn sustain_bruiser(sustain: f32) -> ChampionProfile {
    profile(
        1,
        Role::Top,
        &[
            (ArchetypeId::Sustain, sustain),
            (ArchetypeId::Frontline, 0.85),
            (ArchetypeId::Tank, 0.70),
            (ArchetypeId::Dps, 0.40),
        ],
        &[
            (TraitId::DamagePhysical, 0.48),
            (TraitId::DamageMagic, 0.48),
            (TraitId::DamageTrue, 0.04),
            (TraitId::Durability, 0.80),
            (TraitId::HardCc, 0.50),
            (TraitId::Mobility, 0.30),
        ],
    )
}

fn burst_mage() -> ChampionProfile {
    profile(
        2,
        Role::Mid,
        &[
            (ArchetypeId::Burst, 0.90),
            (ArchetypeId::Poke, 0.55),
            (ArchetypeId::Assassin, 0.30),
        ],
        &[
            (TraitId::DamagePhysical, 0.48),
            (TraitId::DamageMagic, 0.48),
            (TraitId::DamageTrue, 0.04),
            (TraitId::Durability, 0.20),
            (TraitId::HardCc, 0.45),
            (TraitId::Mobility, 0.70),
        ],
    )
}

fn tank() -> ChampionProfile {
    profile(
        3,
        Role::Jungle,
        &[
            (ArchetypeId::Tank, 0.90),
            (ArchetypeId::Frontline, 0.90),
            (ArchetypeId::Engage, 0.55),
        ],
        &[
            (TraitId::DamagePhysical, 0.48),
            (TraitId::DamageMagic, 0.48),
            (TraitId::DamageTrue, 0.04),
            (TraitId::Durability, 0.90),
            (TraitId::HardCc, 0.85),
            (TraitId::Mobility, 0.40),
        ],
    )
}

fn marksman() -> ChampionProfile {
    profile(
        4,
        Role::Adc,
        &[(ArchetypeId::Dps, 0.90), (ArchetypeId::Scaling, 0.55)],
        &[
            (TraitId::DamagePhysical, 0.48),
            (TraitId::DamageMagic, 0.48),
            (TraitId::DamageTrue, 0.04),
            (TraitId::Durability, 0.20),
            (TraitId::HardCc, 0.10),
            (TraitId::Mobility, 0.40),
        ],
    )
}

fn cc_support(sustain: f32) -> ChampionProfile {
    profile(
        5,
        Role::Support,
        &[
            (ArchetypeId::Engage, 0.80),
            (ArchetypeId::Peel, 0.60),
            (ArchetypeId::Sustain, sustain * 0.75),
        ],
        &[
            (TraitId::DamagePhysical, 0.48),
            (TraitId::DamageMagic, 0.48),
            (TraitId::DamageTrue, 0.04),
            (TraitId::Durability, 0.60),
            (TraitId::HardCc, 0.90),
            (TraitId::Mobility, 0.30),
        ],
    )
}

fn enemy_analysis(sustain: f32) -> TeamAnalysis {
    let profiles = vec![
        sustain_bruiser(sustain),
        burst_mage(),
        tank(),
        marksman(),
        cc_support(sustain),
    ];
    let roles = [
        (1, Role::Top),
        (2, Role::Mid),
        (3, Role::Jungle),
        (4, Role::Adc),
        (5, Role::Support),
    ];
    let set = set(profiles);
    let picks: Vec<PickedChampion> = roles
        .iter()
        .map(|(key, role)| PickedChampion::new(ChampionKey(*key), *role, 1.0))
        .collect();
    analyze_team(Side::Red, &picks, &set, &AnalysisParams::default()).unwrap()
}

fn own_tank() -> ChampionProfile {
    profile(
        64,
        Role::Top,
        &[
            (ArchetypeId::Tank, 0.95),
            (ArchetypeId::Frontline, 0.90),
            (ArchetypeId::Engage, 0.60),
        ],
        &[
            (TraitId::DamagePhysical, 0.60),
            (TraitId::DamageMagic, 0.30),
            (TraitId::DamageTrue, 0.10),
            (TraitId::Durability, 0.95),
            (TraitId::HardCc, 0.75),
            (TraitId::Mobility, 0.35),
        ],
    )
}

fn own_mage() -> ChampionProfile {
    profile(
        103,
        Role::Mid,
        &[
            (ArchetypeId::Burst, 0.80),
            (ArchetypeId::Poke, 0.70),
            (ArchetypeId::Dps, 0.40),
        ],
        &[
            (TraitId::DamagePhysical, 0.05),
            (TraitId::DamageMagic, 0.95),
            (TraitId::DamageTrue, 0.00),
            (TraitId::Durability, 0.25),
            (TraitId::HardCc, 0.40),
            (TraitId::Mobility, 0.65),
        ],
    )
}

fn recommendation(
    champion: u16,
    name: &str,
    role: Option<Role>,
    profile: Option<&ChampionProfile>,
    tags: &[String],
    enemy: Option<&TeamAnalysis>,
) -> LoadoutRecommendation {
    let params = LoadoutParams::default();
    recommend_loadout(&LoadoutRequest {
        champion: ChampionKey(champion),
        champion_name: name,
        role,
        riot_tags: tags,
        profile,
        enemy,
        enemy_picks: if enemy.is_some() { 5 } else { 0 },
        catalog: item_catalog(),
        params: &params,
    })
    .unwrap()
}

fn find_item<'a>(
    recommendation: &'a LoadoutRecommendation,
    id: &str,
) -> Option<&'a ItemSuggestion> {
    recommendation
        .starter
        .iter()
        .chain(recommendation.boots.iter())
        .chain(recommendation.core.iter())
        .chain(recommendation.situational.iter())
        .find(|item| item.id == id)
}

#[test]
fn the_seed_catalog_is_valid_and_has_key_functions() {
    let catalog = item_catalog();
    assert_eq!(catalog.schema, ITEM_CATALOG_SCHEMA);
    assert!(catalog.items.len() >= 40);
    assert!(catalog.rune_pages.len() >= 10);
    for id in [
        "thornmail",
        "morellonomicon",
        "black_cleaver",
        "mercurys_treads",
    ] {
        assert!(catalog.item(id).is_some(), "falta el objeto {id}");
    }
    for id in ["conqueror_bruiser", "grasp_tank", "aery_enchanter"] {
        assert!(catalog.rune_page(id).is_some(), "falta la página {id}");
    }
}

#[test]
fn detects_physical_magic_cc_and_sustain() {
    let enemy = enemy_analysis(0.90);
    let threats = detect_threats(&enemy);
    let ids: Vec<ThreatId> = threats.iter().map(|threat| threat.id).collect();
    assert!(ids.contains(&ThreatId::Physical));
    assert!(ids.contains(&ThreatId::Magic));
    assert!(ids.contains(&ThreatId::HardCc));
    assert!(ids.contains(&ThreatId::Sustain));
    assert!(ids.contains(&ThreatId::Tanks));
    assert!(ids.contains(&ThreatId::Burst));
    assert!(ids.contains(&ThreatId::Dps));
    assert!(threats
        .windows(2)
        .all(|pair| pair[0].severity >= pair[1].severity));
}

#[test]
fn sustain_increases_the_score_of_anti_heal_items() {
    let with_sustain = enemy_analysis(0.95);
    let without_sustain = enemy_analysis(0.0);
    let own = own_tank();

    let with = recommendation(
        64,
        "SYNTH_64",
        Some(Role::Top),
        Some(&own),
        &[],
        Some(&with_sustain),
    );
    let without = recommendation(
        64,
        "SYNTH_64",
        Some(Role::Top),
        Some(&own),
        &[],
        Some(&without_sustain),
    );

    let thornmail_with = find_item(&with, "thornmail").expect("Thornmail con sustain");
    assert!(thornmail_with
        .reasons
        .iter()
        .any(|reason| reason.code == "threat:sustain"));
    if let Some(thornmail_without) = find_item(&without, "thornmail") {
        assert!(thornmail_with.score > thornmail_without.score);
    }
}

#[test]
fn magic_threats_surface_magic_defenses() {
    let enemy = enemy_analysis(0.90);
    let own = own_tank();
    let recommendation = recommendation(
        64,
        "SYNTH_64",
        Some(Role::Top),
        Some(&own),
        &[],
        Some(&enemy),
    );
    let has_magic_reason = recommendation
        .starter
        .iter()
        .chain(recommendation.boots.iter())
        .chain(recommendation.core.iter())
        .chain(recommendation.situational.iter())
        .flat_map(|item| item.reasons.iter())
        .any(|reason| reason.code == "threat:magic");
    assert!(has_magic_reason);
}

#[test]
fn runes_answer_hard_cc_with_tenacity() {
    let enemy = enemy_analysis(0.90);
    let own = own_tank();
    let recommendation = recommendation(
        64,
        "SYNTH_64",
        Some(Role::Top),
        Some(&own),
        &[],
        Some(&enemy),
    );
    assert!(!recommendation.runes.is_empty());
    let has_tenacity = recommendation
        .runes
        .iter()
        .flat_map(|page| page.reasons.iter())
        .any(|reason| reason.code == "threat:hard_cc");
    assert!(has_tenacity);
}

#[test]
fn a_missing_profile_falls_back_to_riot_tags_with_a_caveat() {
    let enemy = enemy_analysis(0.90);
    let tags = vec!["Mage".to_string()];
    let recommendation = recommendation(
        103,
        "SYNTH_MAGE",
        Some(Role::Mid),
        None,
        &tags,
        Some(&enemy),
    );
    assert_eq!(recommendation.profile_source, ProfileSource::RiotTags);
    assert!(recommendation
        .caveats
        .iter()
        .any(|caveat| caveat.contains("tags Riot")));
    assert!(!recommendation.core.is_empty());
    let ap_items = [
        "ludens_companion",
        "shadowflame",
        "rabadons_deathcap",
        "void_staff",
        "liandrys_torment",
        "morellonomicon",
    ];
    assert!(recommendation
        .core
        .iter()
        .chain(recommendation.situational.iter())
        .any(|item| ap_items.contains(&item.id.as_str())));
}

#[test]
fn no_enemy_picks_still_returns_champion_fit_without_threats() {
    let own = own_mage();
    let recommendation = recommendation(103, "SYNTH_103", Some(Role::Mid), Some(&own), &[], None);
    assert!(recommendation.threats.is_empty());
    assert!(!recommendation.core.is_empty());
    assert!(recommendation
        .caveats
        .iter()
        .any(|caveat| caveat.contains("No hay rivales")));
}

#[test]
fn support_role_does_not_receive_carry_items() {
    let tags = vec!["Support".to_string()];
    let recommendation =
        recommendation(117, "SYNTH_SUPPORT", Some(Role::Support), None, &tags, None);
    for forbidden in [
        "infinity_edge",
        "rabadons_deathcap",
        "black_cleaver",
        "kraken_slayer",
    ] {
        assert!(find_item(&recommendation, forbidden).is_none());
    }
    assert!(find_item(&recommendation, "world_atlas").is_some());
}

#[test]
fn an_inferred_profile_is_flagged_and_caveated() {
    let mut own = own_mage();
    let inferred = Provenance::Inferred {
        method: "piloto".into(),
        version: "1".into(),
    };
    for rating in own.archetypes.values_mut() {
        rating.source = inferred.clone();
    }
    for rating in own.traits.values_mut() {
        rating.source = inferred.clone();
    }

    let inferred_result = recommendation(103, "SYNTH_103", Some(Role::Mid), Some(&own), &[], None);
    assert!(inferred_result.profile_inferred);
    assert!(inferred_result
        .caveats
        .iter()
        .any(|caveat| caveat.contains("inferido")));

    let curated = recommendation(
        103,
        "SYNTH_103",
        Some(Role::Mid),
        Some(&own_mage()),
        &[],
        None,
    );
    assert!(!curated.profile_inferred);
}

#[test]
fn the_result_is_deterministic_and_has_no_duplicates() {
    let enemy = enemy_analysis(0.90);
    let own = own_tank();
    let first = recommendation(
        64,
        "SYNTH_64",
        Some(Role::Top),
        Some(&own),
        &[],
        Some(&enemy),
    );
    let second = recommendation(
        64,
        "SYNTH_64",
        Some(Role::Top),
        Some(&own),
        &[],
        Some(&enemy),
    );
    assert_eq!(first, second);

    let mut ids = BTreeSet::new();
    for item in first
        .starter
        .iter()
        .chain(first.boots.iter())
        .chain(first.core.iter())
        .chain(first.situational.iter())
    {
        assert!(ids.insert(item.id.clone()), "objeto repetido: {}", item.id);
    }
    let mut rune_ids = BTreeSet::new();
    for page in &first.runes {
        assert!(
            rune_ids.insert(page.id.clone()),
            "página repetida: {}",
            page.id
        );
    }
}
