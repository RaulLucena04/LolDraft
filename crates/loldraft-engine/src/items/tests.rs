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
    assert!(catalog.templates.len() >= 10);
    assert!(catalog.champion_builds.len() >= 150);
    let template = catalog
        .template("adc_crit")
        .expect("la plantilla de tirador debe existir");
    assert_eq!(
        template.core,
        vec![
            "yun_tal".to_string(),
            "infinity_edge".to_string(),
            "lord_dominiks_regards".to_string()
        ]
    );
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
        86,
        "Garen",
        Some(Role::Top),
        Some(&own),
        &[],
        Some(&with_sustain),
    );
    let without = recommendation(
        86,
        "Garen",
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
fn magic_threats_change_the_boots_choice() {
    let enemy = enemy_analysis(0.90);
    let own = own_tank();
    let recommendation =
        recommendation(86, "Garen", Some(Role::Top), Some(&own), &[], Some(&enemy));
    assert_eq!(
        recommendation.boots.as_ref().map(|boots| boots.id.as_str()),
        Some("mercurys_treads")
    );
    assert!(recommendation
        .boots
        .as_ref()
        .unwrap()
        .reasons
        .iter()
        .any(|reason| reason.code == "threat:magic"));
}

#[test]
fn runes_answer_hard_cc_with_tenacity() {
    let enemy = enemy_analysis(0.90);
    let own = own_tank();
    let recommendation =
        recommendation(86, "Garen", Some(Role::Top), Some(&own), &[], Some(&enemy));
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

fn damage_team(physical: f32, magic: f32) -> TeamAnalysis {
    let roles = [Role::Top, Role::Jungle, Role::Mid, Role::Adc, Role::Support];
    let profiles: Vec<ChampionProfile> = roles
        .iter()
        .enumerate()
        .map(|(index, role)| {
            profile(
                20 + index as u16,
                *role,
                &[(ArchetypeId::Frontline, 0.5), (ArchetypeId::Dps, 0.5)],
                &[
                    (TraitId::DamagePhysical, physical),
                    (TraitId::DamageMagic, magic),
                    (TraitId::DamageTrue, 0.0),
                    (TraitId::Durability, 0.5),
                ],
            )
        })
        .collect();
    let set = set(profiles);
    let picks: Vec<PickedChampion> = roles
        .iter()
        .enumerate()
        .map(|(index, role)| PickedChampion::new(ChampionKey(20 + index as u16), *role, 1.0))
        .collect();
    analyze_team(Side::Red, &picks, &set, &AnalysisParams::default()).unwrap()
}

fn all_items(recommendation: &LoadoutRecommendation) -> Vec<&ItemSuggestion> {
    recommendation
        .starter
        .iter()
        .chain(recommendation.boots.iter())
        .chain(recommendation.core.iter())
        .chain(recommendation.situational.iter())
        .collect()
}

#[test]
fn jinx_buys_a_crit_core_in_order() {
    let tags = vec!["Marksman".to_string()];
    let recommendation = recommendation(222, "Jinx", Some(Role::Adc), None, &tags, None);
    assert_eq!(recommendation.build_source, BuildSource::Template);
    assert_eq!(
        recommendation.build_name.as_deref(),
        Some("Tirador de crítico")
    );
    let core: Vec<&str> = recommendation
        .core
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(core, vec!["yun_tal", "infinity_edge", "runaans_hurricane"]);
    assert_eq!(
        recommendation.starter.as_ref().map(|item| item.id.as_str()),
        Some("dorans_bow")
    );
    assert_eq!(
        recommendation.boots.as_ref().map(|item| item.id.as_str()),
        Some("berserkers_greaves")
    );
    assert!(!recommendation.runes.is_empty());
}

#[test]
fn curated_templates_cover_common_champions_with_their_own_items() {
    struct Case<'a> {
        key: u16,
        name: &'a str,
        role: Role,
        tags: &'a [&'a str],
        core: &'a [&'a str],
        build_name: &'a str,
    }

    let cases = [
        Case {
            key: 103,
            name: "Ahri",
            role: Role::Mid,
            tags: &["Mage", "Assassin"],
            core: &["ludens_companion", "shadowflame", "rabadons_deathcap"],
            build_name: "Mago de burst",
        },
        Case {
            key: 64,
            name: "Lee Sin",
            role: Role::Jungle,
            tags: &["Fighter", "Assassin"],
            core: &["eclipse", "black_cleaver", "death_dance"],
            build_name: "Bruiser de daño físico",
        },
        Case {
            key: 412,
            name: "Thresh",
            role: Role::Support,
            tags: &["Support", "Tank"],
            core: &["locket", "knights_vow", "zekes_convergence"],
            build_name: "Support de engage",
        },
        Case {
            key: 99,
            name: "Lux",
            role: Role::Mid,
            tags: &["Mage", "Support"],
            core: &["ludens_companion", "horizon_focus", "rabadons_deathcap"],
            build_name: "Mago de poke",
        },
        Case {
            key: 157,
            name: "Yasuo",
            role: Role::Mid,
            tags: &["Fighter", "Assassin"],
            core: &["yun_tal", "infinity_edge", "phantom_dancer"],
            build_name: "Tirador de crítico",
        },
    ];

    for case in &cases {
        let tags: Vec<String> = case.tags.iter().map(|tag| tag.to_string()).collect();
        let result = recommendation(case.key, case.name, Some(case.role), None, &tags, None);
        assert_eq!(result.build_source, BuildSource::Template, "{}", case.name);
        assert_eq!(
            result.build_name.as_deref(),
            Some(case.build_name),
            "{}",
            case.name
        );
        let ids: Vec<&str> = result.core.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            &ids, case.core,
            "{} recibió un núcleo incoherente",
            case.name
        );
        assert!(
            result.starter.is_some() && result.boots.is_some(),
            "{} no recibió inicio o botas",
            case.name
        );
        assert!(!result.runes.is_empty(), "{} no recibió runas", case.name);
    }
}

#[test]
fn a_champion_played_off_role_keeps_its_curated_build() {
    let tags = vec!["Marksman".to_string(), "Support".to_string()];
    let ashe = recommendation(22, "Ashe", Some(Role::Support), None, &tags, None);
    assert_eq!(ashe.build_source, BuildSource::Template);
    assert!(ashe.starter.is_some(), "Ashe support quedó sin inicio");
    assert!(!ashe.core.is_empty(), "Ashe support quedó sin núcleo");
    assert!(!ashe.runes.is_empty(), "Ashe support quedó sin runas");
}

#[test]
fn garen_and_janna_do_not_receive_ap_or_anti_heal_cores() {
    let garen = recommendation(
        86,
        "Garen",
        Some(Role::Top),
        None,
        &["Fighter".to_string(), "Tank".to_string()],
        None,
    );
    let garen_core: Vec<&str> = garen.core.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(
        garen_core,
        vec!["stridebreaker", "black_cleaver", "steraks_gage"]
    );

    let janna = recommendation(
        40,
        "Janna",
        Some(Role::Support),
        None,
        &["Support".to_string(), "Mage".to_string()],
        None,
    );
    let janna_core: Vec<&str> = janna.core.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(
        janna_core,
        vec![
            "moonstone_renewer",
            "staff_of_flowing_water",
            "ardent_censer"
        ]
    );

    for result in [&garen, &janna] {
        for item in all_items(result) {
            assert_ne!(
                item.id, "morellonomicon",
                "{} recibió Morellonomicón",
                result.champion_name
            );
            assert_ne!(item.id, "chempunk_chainsword");
        }
    }
}

#[test]
fn situational_items_require_their_threat() {
    let garen_tags = vec!["Fighter".to_string(), "Tank".to_string()];
    let calm = recommendation(86, "Garen", Some(Role::Top), None, &garen_tags, None);
    assert!(find_item(&calm, "thornmail").is_none());
    assert!(find_item(&calm, "spirit_visage").is_none());

    let enemy = enemy_analysis(0.95);
    let pressured = recommendation(
        86,
        "Garen",
        Some(Role::Top),
        None,
        &garen_tags,
        Some(&enemy),
    );
    assert!(find_item(&pressured, "thornmail").is_some());
}

#[test]
fn boots_respond_to_the_enemy_damage_type() {
    let tags = vec!["Fighter".to_string(), "Tank".to_string()];
    let physical = recommendation(
        86,
        "Garen",
        Some(Role::Top),
        None,
        &tags,
        Some(&damage_team(0.97, 0.02)),
    );
    assert_eq!(
        physical.boots.as_ref().map(|boots| boots.id.as_str()),
        Some("plated_steelcaps")
    );

    let magic = recommendation(
        86,
        "Garen",
        Some(Role::Top),
        None,
        &tags,
        Some(&damage_team(0.02, 0.97)),
    );
    assert_eq!(
        magic.boots.as_ref().map(|boots| boots.id.as_str()),
        Some("mercurys_treads")
    );
}

#[test]
fn rules_fallback_keeps_damage_types_coherent() {
    let tags = vec!["Mage".to_string()];
    let recommendation = recommendation(9999, "SYNTH_MAGE", Some(Role::Mid), None, &tags, None);
    assert_eq!(recommendation.build_source, BuildSource::Rules);
    let physical_items = [
        "youmuus_ghostblade",
        "black_cleaver",
        "infinity_edge",
        "yun_tal",
        "collector",
        "trinity_force",
        "death_dance",
        "steraks_gage",
        "profane_hydra",
        "ravenous_hydra",
        "manamune",
        "kraken_slayer",
        "blade_of_the_ruined_king",
        "guardian_angel",
        "mercurial_scimitar",
        "edge_of_night",
        "seryldas_grudge",
    ];
    for item in all_items(&recommendation) {
        assert!(
            !physical_items.contains(&item.id.as_str()),
            "un mago recibió el objeto físico {}",
            item.id
        );
    }
    assert!(!recommendation.core.is_empty());
}

#[test]
fn a_champion_without_profile_tags_or_template_gets_no_invented_build() {
    let recommendation = recommendation(9999, "SIN_DATOS", Some(Role::Mid), None, &[], None);
    assert_eq!(recommendation.build_source, BuildSource::Rules);
    assert!(recommendation.core.is_empty());
    assert!(recommendation.situational.is_empty());
    assert!(recommendation.runes.is_empty());
    assert!(recommendation
        .caveats
        .iter()
        .any(|caveat| caveat.contains("reglas por función")));
}

#[test]
fn refreshed_catalog_offers_current_items_by_role() {
    // Tanque de frontline: el refresco 2.1.0 incorpora los objetos de tanque
    // actuales y el inicio mixto de Doran.
    let garen_tags = vec!["Fighter".to_string(), "Tank".to_string()];
    let garen = recommendation(86, "Garen", Some(Role::Top), None, &garen_tags, None);
    assert_eq!(
        garen.starter.as_ref().map(|item| item.id.as_str()),
        Some("dorans_helm")
    );
    assert!(find_item(&garen, "hollow_radiance").is_some());
    assert!(find_item(&garen, "unending_despair").is_some());

    let pressured = recommendation(
        86,
        "Garen",
        Some(Role::Top),
        None,
        &garen_tags,
        Some(&enemy_analysis(0.95)),
    );
    assert!(find_item(&pressured, "thornmail").is_some());
    assert!(find_item(&pressured, "protoplasm_harness").is_some());

    // Enchanter: el núcleo albar escala con la regeneración de maná.
    let janna_tags = vec!["Support".to_string(), "Mage".to_string()];
    let janna = recommendation(40, "Janna", Some(Role::Support), None, &janna_tags, None);
    assert!(find_item(&janna, "dawncore").is_some());

    // Tirador contra burst: el arcoescudo aparece solo con esa amenaza.
    let jinx_tags = vec!["Marksman".to_string()];
    let jinx = recommendation(
        222,
        "Jinx",
        Some(Role::Adc),
        None,
        &jinx_tags,
        Some(&enemy_analysis(0.95)),
    );
    assert!(find_item(&jinx, "immortal_shieldbow").is_some());
    assert!(find_item(&jinx, "stormrazor").is_none());

    // Mago de burst: el actualizador entra como opción de maná y daño.
    let ahri_tags = vec!["Mage".to_string(), "Assassin".to_string()];
    let ahri = recommendation(103, "Ahri", Some(Role::Mid), None, &ahri_tags, None);
    assert!(find_item(&ahri, "actualizer").is_some());
}
