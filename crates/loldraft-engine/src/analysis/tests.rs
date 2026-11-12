use super::*;
use crate::profile::{
    ArchetypeRating, ChampionProfile, ProfileRole, ProfileSet, Provenance, TraitRating,
};
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
    proficiency: f32,
    archetypes: &[(ArchetypeId, f32)],
    traits: &[(TraitId, f32)],
) -> ChampionProfile {
    ChampionProfile {
        key: ChampionKey(key),
        slug: format!("synth{key}"),
        name: format!("SYNTH_{key}"),
        roles: vec![ProfileRole {
            role,
            proficiency,
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

fn fixture_set() -> ProfileSet {
    set(vec![
        profile(
            1,
            Role::Top,
            1.0,
            &[
                (ArchetypeId::Engage, 0.90),
                (ArchetypeId::Frontline, 0.90),
                (ArchetypeId::Dps, 0.20),
            ],
            &[
                (TraitId::DamagePhysical, 1.00),
                (TraitId::DamageMagic, 0.00),
                (TraitId::DamageTrue, 0.00),
                (TraitId::Range, 0.10),
                (TraitId::Mobility, 0.20),
            ],
        ),
        profile(
            2,
            Role::Mid,
            1.0,
            &[
                (ArchetypeId::Engage, 0.20),
                (ArchetypeId::Frontline, 0.10),
                (ArchetypeId::Dps, 0.60),
            ],
            &[
                (TraitId::DamagePhysical, 0.05),
                (TraitId::DamageMagic, 0.95),
                (TraitId::DamageTrue, 0.00),
                (TraitId::Range, 0.75),
                (TraitId::Mobility, 0.35),
            ],
        ),
        profile(
            3,
            Role::Adc,
            1.0,
            &[
                (ArchetypeId::Engage, 0.00),
                (ArchetypeId::Frontline, 0.00),
                (ArchetypeId::Dps, 0.90),
            ],
            &[
                (TraitId::DamagePhysical, 0.95),
                (TraitId::DamageMagic, 0.05),
                (TraitId::DamageTrue, 0.00),
                (TraitId::Range, 0.85),
                (TraitId::Mobility, 0.30),
            ],
        ),
    ])
}

fn fixture_picks() -> Vec<PickedChampion> {
    vec![
        PickedChampion::new(ChampionKey(1), Role::Top, 1.0),
        PickedChampion::new(ChampionKey(2), Role::Mid, 1.0),
        PickedChampion::new(ChampionKey(3), Role::Adc, 1.0),
    ]
}

fn equal_gold_params() -> AnalysisParams {
    AnalysisParams {
        role_gold_weights: [1.0; 5],
        ..AnalysisParams::default()
    }
}

fn approx(left: f32, right: f32) -> bool {
    (left - right).abs() < 1e-4
}

#[test]
fn synthetic_fixture_matches_documented_aggregation() {
    let analysis = analyze_team(
        Side::Blue,
        &fixture_picks(),
        &fixture_set(),
        &equal_gold_params(),
    )
    .expect("análisis");

    assert!(approx(
        analysis.archetypes[&ArchetypeId::Engage].value_low,
        0.96
    ));
    assert!(approx(
        analysis.archetypes[&ArchetypeId::Frontline].value_low,
        0.93
    ));
    assert!(approx(
        analysis.archetypes[&ArchetypeId::Dps].value_low,
        1.0
    ));

    let damage = analysis.damage.expect("daño conocido");
    assert!(approx(damage.magic, 1.0 / 3.0));
    assert!(approx(damage.physical, 2.0 / 3.0));
    assert!(approx(damage.true_damage, 0.0));
    assert!(approx(damage.known_weight_ratio, 1.0));
    assert!(approx(
        damage.physical + damage.magic + damage.true_damage,
        1.0
    ));

    for dimension in [
        Dimension::Archetype(ArchetypeId::Engage),
        Dimension::Archetype(ArchetypeId::Frontline),
        Dimension::Archetype(ArchetypeId::Dps),
    ] {
        assert!(
            analysis
                .needs
                .iter()
                .all(|need| need.dimension != dimension),
            "{dimension:?} no debería ser una carencia"
        );
    }
    // El balance mágico queda justo por encima del 30 %.
    assert!(analysis
        .needs
        .iter()
        .all(|need| need.dimension != Dimension::Trait(TraitId::DamageMagic)));
}

#[test]
fn adding_a_pick_never_reduces_coverage() {
    let params = equal_gold_params();
    let set = fixture_set();
    let one = vec![PickedChampion::new(ChampionKey(1), Role::Top, 1.0)];
    let two = vec![
        PickedChampion::new(ChampionKey(1), Role::Top, 1.0),
        PickedChampion::new(ChampionKey(2), Role::Mid, 1.0),
    ];
    let first = analyze_team(Side::Blue, &one, &set, &params).unwrap();
    let second = analyze_team(Side::Blue, &two, &set, &params).unwrap();

    for id in ArchetypeId::ALL {
        assert!(
            second.archetypes[&id].value_low >= first.archetypes[&id].value_low - 1e-6,
            "el arquetipo {id} perdió cota inferior"
        );
        assert!(
            second.archetypes[&id].value_high >= first.archetypes[&id].value_high - 1e-6,
            "el arquetipo {id} perdió cota superior"
        );
    }
}

#[test]
fn increasing_intensity_never_reduces_coverage() {
    let params = equal_gold_params();
    let picks = fixture_picks();
    let base = fixture_set();
    let mut stronger = base.clone();
    stronger
        .get_mut(ChampionKey(2))
        .unwrap()
        .archetypes
        .get_mut(&ArchetypeId::Dps)
        .unwrap()
        .intensity = Some(1.0);

    let before = analyze_team(Side::Blue, &picks, &base, &params).unwrap();
    let after = analyze_team(Side::Blue, &picks, &stronger, &params).unwrap();
    assert!(
        after.archetypes[&ArchetypeId::Dps].value_low
            >= before.archetypes[&ArchetypeId::Dps].value_low
    );
    assert!(
        after.archetypes[&ArchetypeId::Dps].value_high
            >= before.archetypes[&ArchetypeId::Dps].value_high
    );
}

#[test]
fn an_unknown_cell_is_an_interval_not_a_zero() {
    let mut champion = profile(
        9,
        Role::Top,
        1.0,
        &[(ArchetypeId::Frontline, 0.8)],
        &[(TraitId::DamagePhysical, 1.0)],
    );
    champion
        .archetypes
        .insert(ArchetypeId::Engage, archetype(None));
    let analysis = analyze_team(
        Side::Blue,
        &[PickedChampion::new(ChampionKey(9), Role::Top, 1.0)],
        &set(vec![champion]),
        &equal_gold_params(),
    )
    .unwrap();

    let engage = &analysis.archetypes[&ArchetypeId::Engage];
    assert_eq!(engage.value_low, 0.0);
    assert_eq!(engage.value_high, 1.0);
    assert_eq!(engage.value_estimate, None);
    assert_eq!(engage.known, 0);
    assert_eq!(engage.total, 1);

    let need = analysis
        .needs
        .iter()
        .find(|need| need.dimension == Dimension::Archetype(ArchetypeId::Engage))
        .expect("carencia posible");
    assert_eq!(need.status, NeedStatus::Possible);
    assert!(approx(need.amount, 0.35));
    assert!(!analysis
        .weaknesses
        .iter()
        .any(|signal| signal.dimension == Dimension::Archetype(ArchetypeId::Engage)));
    assert!(!analysis
        .strengths
        .iter()
        .any(|signal| signal.dimension == Dimension::Archetype(ArchetypeId::Engage)));
}

#[test]
fn role_proficiency_scales_the_contribution() {
    let analysis = analyze_team(
        Side::Blue,
        &[PickedChampion::new(ChampionKey(4), Role::Top, 1.0)],
        &set(vec![profile(
            4,
            Role::Top,
            0.5,
            &[(ArchetypeId::Engage, 1.0)],
            &[],
        )]),
        &equal_gold_params(),
    )
    .unwrap();
    assert!(approx(
        analysis.archetypes[&ArchetypeId::Engage].value_low,
        0.5
    ));
}

#[test]
fn enemy_engage_creates_a_disengage_need() {
    let own = set(vec![profile(
        10,
        Role::Top,
        1.0,
        &[(ArchetypeId::Disengage, 0.10)],
        &[],
    )]);
    let enemy = set(vec![profile(
        11,
        Role::Top,
        1.0,
        &[(ArchetypeId::Engage, 0.90)],
        &[],
    )]);
    let analysis = analyze_draft(
        &[PickedChampion::new(ChampionKey(10), Role::Top, 1.0)],
        &[PickedChampion::new(ChampionKey(11), Role::Top, 1.0)],
        &set({
            let mut profiles = own.champions.into_values().collect::<Vec<_>>();
            profiles.extend(enemy.champions.into_values());
            profiles
        }),
        &equal_gold_params(),
    )
    .unwrap();

    let need = analysis
        .blue
        .needs
        .iter()
        .find(|need| need.dimension == Dimension::Archetype(ArchetypeId::Disengage))
        .expect("necesidad de disengage");
    assert_eq!(need.status, NeedStatus::Confirmed);
    assert!(need.sources.iter().any(|source| matches!(
        source,
        NeedSource::EnemyThreat {
            rule: ThreatRuleId::EnemyEngage,
            ..
        }
    )));
    assert!(need.amount > 0.0);
}

#[test]
fn dive_is_detected_from_confirmed_signals() {
    let analysis = analyze_team(
        Side::Blue,
        &[PickedChampion::new(ChampionKey(12), Role::Mid, 1.0)],
        &set(vec![profile(
            12,
            Role::Mid,
            1.0,
            &[
                (ArchetypeId::Engage, 0.90),
                (ArchetypeId::Burst, 0.80),
                (ArchetypeId::Assassin, 0.70),
            ],
            &[(TraitId::Mobility, 0.70)],
        )]),
        &equal_gold_params(),
    )
    .unwrap();

    let dive = analysis
        .win_conditions
        .iter()
        .find(|condition| condition.id == WinConditionId::Dive)
        .expect("Dive");
    assert!(dive.confirmed);
    assert_eq!(dive.missing.len(), 0);
}

#[test]
fn an_empty_team_is_partial_and_contains_no_invented_values() {
    let analysis = analyze_team(Side::Blue, &[], &fixture_set(), &equal_gold_params()).unwrap();
    assert_eq!(analysis.picks, 0);
    assert!(analysis.partial);
    assert_eq!(analysis.knowledge, 0.0);
    assert_eq!(analysis.confidence, 0.0);
    assert!(analysis.damage.is_none());
    assert!(analysis.needs.is_empty());
    assert!(analysis.win_conditions.is_empty());
    for coverage in analysis.archetypes.values() {
        assert_eq!(coverage.total, 0);
        assert_eq!(coverage.value_low, 0.0);
        assert_eq!(coverage.value_high, 0.0);
    }
}

#[test]
fn outputs_are_finite_and_inside_unit_range() {
    let analysis = analyze_team(
        Side::Blue,
        &fixture_picks(),
        &fixture_set(),
        &equal_gold_params(),
    )
    .unwrap();

    let checks = analysis
        .archetypes
        .values()
        .chain(analysis.traits.values())
        .chain([
            &analysis.curve.early,
            &analysis.curve.mid,
            &analysis.curve.late,
            &analysis.range.ranged_share,
            &analysis.cc.hard,
            &analysis.cc.soft,
            &analysis.mobility,
            &analysis.waveclear,
            &analysis.objective_dps,
        ]);
    for coverage in checks {
        assert!(coverage.value_low.is_finite());
        assert!(coverage.value_high.is_finite());
        assert!((0.0..=1.0).contains(&coverage.value_low));
        assert!((0.0..=1.0).contains(&coverage.value_high));
        assert!(coverage.value_low <= coverage.value_high);
        assert!(coverage.confidence.is_finite());
    }
    for need in &analysis.needs {
        assert!(need.amount.is_finite());
        assert!((0.0..=1.0).contains(&need.amount));
    }
    for condition in &analysis.win_conditions {
        assert!(condition.score.is_finite());
        assert!((0.0..=1.0).contains(&condition.score));
    }
}

#[test]
fn a_full_analysis_is_fast_enough() {
    let mut profiles = fixture_set().champions.into_values().collect::<Vec<_>>();
    profiles.push(profile(
        4,
        Role::Jungle,
        1.0,
        &[
            (ArchetypeId::Engage, 0.40),
            (ArchetypeId::Frontline, 0.55),
            (ArchetypeId::Sustain, 0.45),
        ],
        &[(TraitId::Mobility, 0.55), (TraitId::Waveclear, 0.50)],
    ));
    profiles.push(profile(
        5,
        Role::Support,
        1.0,
        &[
            (ArchetypeId::Peel, 0.75),
            (ArchetypeId::Engage, 0.45),
            (ArchetypeId::Disengage, 0.55),
        ],
        &[(TraitId::HardCc, 0.70), (TraitId::Range, 0.55)],
    ));
    let set = set(profiles);
    let picks = vec![
        PickedChampion::new(ChampionKey(1), Role::Top, 1.0),
        PickedChampion::new(ChampionKey(4), Role::Jungle, 1.0),
        PickedChampion::new(ChampionKey(2), Role::Mid, 1.0),
        PickedChampion::new(ChampionKey(3), Role::Adc, 1.0),
        PickedChampion::new(ChampionKey(5), Role::Support, 1.0),
    ];
    let params = equal_gold_params();

    for _ in 0..5 {
        analyze_draft(&picks, &[], &set, &params).unwrap();
    }
    let started = std::time::Instant::now();
    for _ in 0..100 {
        analyze_draft(&picks, &[], &set, &params).unwrap();
    }
    let average = started.elapsed() / 100;
    assert!(
        average < std::time::Duration::from_millis(10),
        "el análisis tardó {average:?} de media"
    );
}

#[test]
fn params_are_valid_and_roundtrip_through_json() {
    let params = AnalysisParams::default();
    params.validate().expect("parámetros por defecto");
    let json = serde_json::to_string(&params).expect("serializa");
    let parsed: AnalysisParams = serde_json::from_str(&json).expect("deserializa");
    assert_eq!(parsed, params);
}

#[test]
fn invalid_picks_and_params_are_rejected() {
    let params = equal_gold_params();
    let duplicate_roles = vec![
        PickedChampion::new(ChampionKey(1), Role::Top, 1.0),
        PickedChampion::new(ChampionKey(2), Role::Top, 1.0),
    ];
    assert!(analyze_team(Side::Blue, &duplicate_roles, &fixture_set(), &params).is_err());

    let duplicate_champions = vec![
        PickedChampion::new(ChampionKey(1), Role::Top, 1.0),
        PickedChampion::new(ChampionKey(1), Role::Mid, 1.0),
    ];
    assert!(analyze_team(Side::Blue, &duplicate_champions, &fixture_set(), &params).is_err());

    let too_many: Vec<_> = (1..=6)
        .map(|key| PickedChampion::new(ChampionKey(key), Role::ALL[key as usize % 5], 1.0))
        .collect();
    assert!(analyze_team(Side::Blue, &too_many, &fixture_set(), &params).is_err());

    let mut bad_params = params;
    bad_params.role_gold_weights[0] = 0.0;
    assert!(analyze_team(Side::Blue, &fixture_picks(), &fixture_set(), &bad_params).is_err());
}
