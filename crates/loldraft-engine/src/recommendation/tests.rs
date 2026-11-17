use super::score::{score, FeatureValue};
use super::*;
use crate::analysis::{AnalysisParams, PickedChampion};
use crate::profile::{
    ArchetypeRating, ChampionProfile, ProfileRole, ProfileSet, Provenance, TraitRating,
};
use crate::taxonomy::{ArchetypeId, TraitId, TAXONOMY_VERSION};
use loldraft_domain::{ChampionKey, DraftFormat, Role, Side};

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
    roles: &[(Role, f32)],
    archetypes: &[(ArchetypeId, f32)],
    traits: &[(TraitId, f32)],
) -> ChampionProfile {
    ChampionProfile {
        key: ChampionKey(key),
        slug: format!("synth{key}"),
        name: format!("SYNTH_{key}"),
        roles: roles
            .iter()
            .map(|(role, proficiency)| ProfileRole {
                role: *role,
                proficiency: *proficiency,
                confidence: 1.0,
                source: curated(),
            })
            .collect(),
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

/// Equipo sintético con perfiles completos: cada campeón declara las 10
/// dimensiones que el análisis usa para necesidades, daño y seguridad.
fn fighter(key: u16, roles: &[(Role, f32)]) -> ChampionProfile {
    profile(
        key,
        roles,
        &[
            (ArchetypeId::Engage, 0.90),
            (ArchetypeId::Frontline, 0.90),
            (ArchetypeId::Tank, 0.60),
            (ArchetypeId::Dps, 0.20),
        ],
        &[
            (TraitId::DamagePhysical, 1.00),
            (TraitId::DamageMagic, 0.00),
            (TraitId::DamageTrue, 0.00),
            (TraitId::Durability, 0.90),
            (TraitId::LaneSafety, 0.80),
            (TraitId::Mobility, 0.40),
            (TraitId::Range, 0.10),
            (TraitId::HardCc, 0.80),
            (TraitId::GoldDependency, 0.10),
        ],
    )
}

fn mage(key: u16, roles: &[(Role, f32)]) -> ChampionProfile {
    profile(
        key,
        roles,
        &[
            (ArchetypeId::Dps, 0.90),
            (ArchetypeId::Poke, 0.50),
            (ArchetypeId::Burst, 0.30),
        ],
        &[
            (TraitId::DamagePhysical, 0.10),
            (TraitId::DamageMagic, 0.90),
            (TraitId::DamageTrue, 0.00),
            (TraitId::Durability, 0.20),
            (TraitId::LaneSafety, 0.60),
            (TraitId::Mobility, 0.50),
            (TraitId::Range, 0.80),
            (TraitId::HardCc, 0.40),
            (TraitId::GoldDependency, 0.70),
        ],
    )
}

fn marksman(key: u16, roles: &[(Role, f32)]) -> ChampionProfile {
    profile(
        key,
        roles,
        &[(ArchetypeId::Dps, 0.90), (ArchetypeId::Scaling, 0.60)],
        &[
            (TraitId::DamagePhysical, 1.00),
            (TraitId::DamageMagic, 0.00),
            (TraitId::DamageTrue, 0.00),
            (TraitId::Durability, 0.15),
            (TraitId::LaneSafety, 0.70),
            (TraitId::Mobility, 0.30),
            (TraitId::Range, 0.90),
            (TraitId::HardCc, 0.10),
            (TraitId::GoldDependency, 0.90),
        ],
    )
}

fn enchanter(key: u16, roles: &[(Role, f32)]) -> ChampionProfile {
    profile(
        key,
        roles,
        &[
            (ArchetypeId::Engage, 0.80),
            (ArchetypeId::Peel, 0.70),
            (ArchetypeId::Frontline, 0.40),
        ],
        &[
            (TraitId::DamagePhysical, 0.10),
            (TraitId::DamageMagic, 0.90),
            (TraitId::DamageTrue, 0.00),
            (TraitId::Durability, 0.60),
            (TraitId::LaneSafety, 0.60),
            (TraitId::Mobility, 0.30),
            (TraitId::Range, 0.40),
            (TraitId::HardCc, 0.90),
            (TraitId::GoldDependency, 0.10),
        ],
    )
}

fn assassin(key: u16, roles: &[(Role, f32)]) -> ChampionProfile {
    profile(
        key,
        roles,
        &[
            (ArchetypeId::Assassin, 0.90),
            (ArchetypeId::Burst, 0.80),
            (ArchetypeId::Catch, 0.40),
        ],
        &[
            (TraitId::DamagePhysical, 1.00),
            (TraitId::DamageMagic, 0.00),
            (TraitId::DamageTrue, 0.00),
            (TraitId::Durability, 0.20),
            (TraitId::LaneSafety, 0.40),
            (TraitId::Mobility, 0.90),
            (TraitId::Range, 0.20),
            (TraitId::HardCc, 0.20),
            (TraitId::GoldDependency, 0.80),
        ],
    )
}

fn fixture_set() -> ProfileSet {
    set(vec![
        fighter(1, &[(Role::Top, 1.0)]),
        mage(2, &[(Role::Mid, 1.0)]),
        marksman(3, &[(Role::Adc, 1.0)]),
        enchanter(4, &[(Role::Support, 1.0)]),
        fighter(5, &[(Role::Jungle, 1.0), (Role::Top, 0.60)]),
        assassin(6, &[(Role::Mid, 1.0), (Role::Jungle, 0.50)]),
        profile(
            7,
            &[(Role::Top, 1.0)],
            &[(ArchetypeId::Splitpush, 0.90), (ArchetypeId::Scaling, 0.80)],
            &[
                (TraitId::DamagePhysical, 1.00),
                (TraitId::DamageMagic, 0.00),
                (TraitId::DamageTrue, 0.00),
                (TraitId::Durability, 0.30),
                (TraitId::LaneSafety, 0.50),
                (TraitId::Mobility, 0.70),
                (TraitId::Range, 0.10),
                (TraitId::HardCc, 0.20),
                (TraitId::GoldDependency, 0.90),
                (TraitId::LatePower, 0.90),
                (TraitId::EarlyPower, 0.20),
            ],
        ),
    ])
}

#[allow(clippy::too_many_arguments)]
fn request<'a>(
    format: &'a DraftFormat,
    action_index: usize,
    blue: &'a [PickedChampion],
    red: &'a [PickedChampion],
    used: &'a [ChampionKey],
    profiles: &'a ProfileSet,
    analysis_params: &'a AnalysisParams,
    params: &'a RecommendationParams,
) -> RecommendationRequest<'a> {
    RecommendationRequest {
        format,
        action_index,
        blue_picks: blue,
        red_picks: red,
        used,
        profiles,
        analysis_params,
        params,
    }
}

fn pick(key: u16, role: Role) -> PickedChampion {
    PickedChampion::new(ChampionKey(key), role, 1.0)
}

#[test]
fn deterministic_output_for_same_state() {
    let format = DraftFormat::standard();
    let profiles = fixture_set();
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();
    let blue = [pick(1, Role::Top), pick(2, Role::Mid)];
    let red = [pick(3, Role::Adc), pick(4, Role::Support)];
    let used = [
        ChampionKey(1),
        ChampionKey(2),
        ChampionKey(3),
        ChampionKey(4),
    ];
    let req = request(
        &format, 9, &blue, &red, &used, &profiles, &analysis, &params,
    );

    let first = recommend(&req).expect("recommendation");
    let second = recommend(&req).expect("recommendation");
    assert_eq!(first, second);
}

#[test]
fn contributions_and_risks_sum_to_score() {
    let format = DraftFormat::standard();
    let profiles = fixture_set();
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();
    let blue = [pick(1, Role::Top), pick(2, Role::Mid)];
    let red = [pick(3, Role::Adc), pick(4, Role::Support)];
    let used = [
        ChampionKey(1),
        ChampionKey(2),
        ChampionKey(3),
        ChampionKey(4),
    ];
    let output = recommend(&request(
        &format, 9, &blue, &red, &used, &profiles, &analysis, &params,
    ))
    .expect("recommendation");

    assert!(!output.picks.is_empty());
    for pick in &output.picks {
        let contributions: f32 = pick.contributions.iter().map(|c| c.contribution).sum();
        let risks: f32 = pick.risks.iter().map(|r| r.contribution).sum();
        assert!(
            (contributions + risks - pick.score_fraction).abs() <= 1e-5,
            "{}: {} + {} != {}",
            pick.champion,
            contributions,
            risks,
            pick.score_fraction
        );
        assert_eq!(pick.score, (pick.score_fraction * 100.0).round() as u8);
    }
}

#[test]
fn documented_no_winrate_fixture_prefers_strategic_fit() {
    let params = RecommendationParams::default();
    let weights = params.weights_late_pick;
    let features_a = [
        FeatureValue::new(FeatureId::Need, 0.85, 1.0),
        FeatureValue::new(FeatureId::Answer, 0.70, 1.0),
        FeatureValue::new(FeatureId::Synergy, 0.65, 1.0),
        FeatureValue::new(FeatureId::Counter, 0.40, 1.0),
        FeatureValue::new(FeatureId::Power, 0.45, 1.0),
        FeatureValue::new(FeatureId::Flex, 0.50, 1.0),
        FeatureValue::new(FeatureId::Balance, 0.80, 1.0),
        FeatureValue::new(FeatureId::Safety, 0.40, 1.0),
    ];
    let features_b = [
        FeatureValue::new(FeatureId::Need, 0.20, 1.0),
        FeatureValue::new(FeatureId::Answer, 0.25, 1.0),
        FeatureValue::new(FeatureId::Synergy, 0.35, 1.0),
        FeatureValue::new(FeatureId::Counter, 0.55, 1.0),
        FeatureValue::new(FeatureId::Power, 0.90, 1.0),
        FeatureValue::new(FeatureId::Flex, 0.20, 1.0),
        FeatureValue::new(FeatureId::Balance, 0.20, 1.0),
        FeatureValue::new(FeatureId::Safety, 0.30, 1.0),
    ];

    let a = score(
        &features_a,
        &weights,
        &[(RiskId::UnknownProfile, 0.05)],
        1.0,
        &params,
    )
    .expect("candidate a");
    let b = score(
        &features_b,
        &weights,
        &[(RiskId::UnknownProfile, 0.10)],
        1.0,
        &params,
    )
    .expect("candidate b");

    assert!(a.score > b.score, "A debe ganar pese a peor `power`");
    assert!((a.score - 0.6145).abs() < 1e-3, "score A = {}", a.score);
    assert!((b.score - 0.3665).abs() < 1e-3, "score B = {}", b.score);
    assert_eq!(super::score::display(a.score), 61);
    assert_eq!(super::score::display(b.score), 37);
}

#[test]
fn dominance_holds_when_one_candidate_is_better_everywhere() {
    let params = RecommendationParams::default();
    let weights = params.weights_mid_pick;
    let better = [
        FeatureValue::new(FeatureId::Need, 0.80, 1.0),
        FeatureValue::new(FeatureId::Synergy, 0.70, 1.0),
        FeatureValue::new(FeatureId::Safety, 0.60, 1.0),
    ];
    let worse = [
        FeatureValue::new(FeatureId::Need, 0.80, 1.0),
        FeatureValue::new(FeatureId::Synergy, 0.40, 1.0),
        FeatureValue::new(FeatureId::Safety, 0.60, 1.0),
    ];
    let a = score(&better, &weights, &[], 1.0, &params).expect("better");
    let b = score(&worse, &weights, &[], 1.0, &params).expect("worse");
    assert!(a.score > b.score);
}

#[test]
fn feature_weights_are_normalized_per_phase() {
    let params = RecommendationParams::default();
    for weights in [
        params.weights_early_pick,
        params.weights_mid_pick,
        params.weights_late_pick,
    ] {
        assert!((weights.sum() - 1.0).abs() < 1e-5);
        assert_eq!(weights.normalized(), weights);
    }
    assert_eq!(params.weights_early_pick.flex, 0.22);
    assert_eq!(params.weights_late_pick.flex, 0.00);
    assert!(params.weights_late_pick.counter > params.weights_early_pick.counter);
    assert!(params.weights_early_pick.safety > params.weights_late_pick.safety);
}

#[test]
fn flex_matters_early_and_not_late() {
    let params = RecommendationParams::default();
    let flexible = [
        FeatureValue::new(FeatureId::Flex, 0.90, 1.0),
        FeatureValue::new(FeatureId::Need, 0.50, 1.0),
    ];
    let rigid = [
        FeatureValue::new(FeatureId::Flex, 0.10, 1.0),
        FeatureValue::new(FeatureId::Need, 0.50, 1.0),
    ];

    let early_flex =
        score(&flexible, &params.weights_early_pick, &[], 1.0, &params).expect("early flex");
    let early_rigid =
        score(&rigid, &params.weights_early_pick, &[], 1.0, &params).expect("early rigid");
    assert!(early_flex.score > early_rigid.score);

    let late_flex =
        score(&flexible, &params.weights_late_pick, &[], 1.0, &params).expect("late flex");
    let late_rigid =
        score(&rigid, &params.weights_late_pick, &[], 1.0, &params).expect("late rigid");
    assert!((late_flex.score - late_rigid.score).abs() < 1e-6);
}

#[test]
fn power_never_invented_and_reported_missing() {
    let format = DraftFormat::standard();
    let profiles = fixture_set();
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();
    let blue = [pick(1, Role::Top), pick(2, Role::Mid)];
    let red = [pick(3, Role::Adc), pick(4, Role::Support)];
    let used = [
        ChampionKey(1),
        ChampionKey(2),
        ChampionKey(3),
        ChampionKey(4),
    ];
    let output = recommend(&request(
        &format, 9, &blue, &red, &used, &profiles, &analysis, &params,
    ))
    .expect("recommendation");

    assert!(output.missing_features.contains(&FeatureId::Power));
    for pick in &output.picks {
        assert!(pick.missing.contains(&FeatureId::Power));
        assert!(
            pick.contributions
                .iter()
                .all(|contribution| contribution.feature != FeatureId::Power),
            "no debe haber una feature `power` sin estadísticas observadas"
        );
    }
}

#[test]
fn empty_profile_set_returns_no_data() {
    let format = DraftFormat::standard();
    let profiles = set(Vec::new());
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();
    let output = recommend(&request(
        &format,
        6,
        &[],
        &[],
        &[],
        &profiles,
        &analysis,
        &params,
    ))
    .expect("recommendation");

    assert_eq!(output.no_data, Some(NoDataReason::NoProfiles));
    assert!(output.picks.is_empty());
    assert!(output.bans.is_empty());
}

#[test]
fn all_champions_used_returns_no_available() {
    let format = DraftFormat::standard();
    let profiles = fixture_set();
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();
    let used: Vec<ChampionKey> = profiles.champions.keys().copied().collect();
    let output = recommend(&request(
        &format,
        6,
        &[],
        &[],
        &used,
        &profiles,
        &analysis,
        &params,
    ))
    .expect("recommendation");

    assert_eq!(output.no_data, Some(NoDataReason::NoAvailableChampions));
    assert!(output.picks.is_empty());
}

#[test]
fn outputs_are_finite_and_bounded() {
    let format = DraftFormat::standard();
    let profiles = fixture_set();
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();
    let blue = [pick(1, Role::Top), pick(2, Role::Mid)];
    let red = [pick(3, Role::Adc), pick(4, Role::Support)];
    let used = [
        ChampionKey(1),
        ChampionKey(2),
        ChampionKey(3),
        ChampionKey(4),
    ];
    let output = recommend(&request(
        &format, 9, &blue, &red, &used, &profiles, &analysis, &params,
    ))
    .expect("recommendation");

    for pick in &output.picks {
        assert!(pick.score_fraction.is_finite());
        assert!((0.0..=1.0).contains(&pick.score_fraction));
        assert!(pick.ranking_score.is_finite());
        assert!(pick.confidence.is_finite());
        assert!((0.0..=1.0).contains(&pick.confidence));
        for contribution in &pick.contributions {
            assert!(contribution.value.is_finite());
            assert!(contribution.weight.is_finite());
            assert!(contribution.contribution.is_finite());
        }
        for risk in &pick.risks {
            assert!(risk.amount.is_finite());
            assert!(risk.contribution.is_finite());
            assert!(risk.contribution <= 0.0);
        }
    }
}

#[test]
fn earlier_need_increases_frontline_candidate_need_feature() {
    let format = DraftFormat::standard();
    let profiles = fixture_set();
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();

    let strong_frontline = [pick(1, Role::Top), pick(2, Role::Mid)];
    let weak_frontline = [pick(7, Role::Top), pick(2, Role::Mid)];
    let red = [pick(3, Role::Adc), pick(4, Role::Support)];
    let used_a = [
        ChampionKey(1),
        ChampionKey(2),
        ChampionKey(3),
        ChampionKey(4),
    ];
    let used_b = [
        ChampionKey(7),
        ChampionKey(2),
        ChampionKey(3),
        ChampionKey(4),
    ];

    let covered = recommend(&request(
        &format,
        9,
        &strong_frontline,
        &red,
        &used_a,
        &profiles,
        &analysis,
        &params,
    ))
    .expect("covered");
    let lacking = recommend(&request(
        &format,
        9,
        &weak_frontline,
        &red,
        &used_b,
        &profiles,
        &analysis,
        &params,
    ))
    .expect("lacking");

    let covered_fighter = covered
        .picks
        .iter()
        .find(|pick| pick.champion == ChampionKey(5))
        .expect("fighter candidate");
    let lacking_fighter = lacking
        .picks
        .iter()
        .find(|pick| pick.champion == ChampionKey(5))
        .expect("fighter candidate");

    let covered_need = covered_fighter
        .contributions
        .iter()
        .find(|contribution| contribution.feature == FeatureId::Need)
        .map(|contribution| contribution.contribution)
        .unwrap_or(0.0);
    let lacking_need = lacking_fighter
        .contributions
        .iter()
        .find(|contribution| contribution.feature == FeatureId::Need)
        .map(|contribution| contribution.contribution)
        .expect("need feature present when frontline is missing");

    assert!(lacking_need > covered_need);
    assert!(lacking_need > 0.0);
}

#[test]
fn ban_list_is_scored_from_opponent_perspective() {
    let format = DraftFormat::standard();
    let profiles = fixture_set();
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();
    let blue = [pick(3, Role::Adc)];
    let red = [pick(1, Role::Top), pick(2, Role::Mid)];
    let used = [ChampionKey(1), ChampionKey(2), ChampionKey(3)];
    let output = recommend(&request(
        &format, 15, &blue, &red, &used, &profiles, &analysis, &params,
    ))
    .expect("recommendation");

    assert_eq!(output.action_kind, loldraft_domain::ActionKind::Ban);
    assert_eq!(output.side, Side::Red);
    assert!(output.picks.is_empty());
    assert!(!output.bans.is_empty());
    assert!(output
        .missing_ban_features
        .contains(&BanFeatureId::OppPower));

    for ban in &output.bans {
        let contributions: f32 = ban.contributions.iter().map(|c| c.contribution).sum();
        let risks: f32 = ban.risks.iter().map(|r| r.contribution).sum();
        assert!((contributions + risks - ban.score_fraction).abs() <= 1e-5);
        assert!(ban
            .contributions
            .iter()
            .all(|c| c.feature != BanFeatureId::OppPower));
    }

    let fighter_ban = output
        .bans
        .iter()
        .find(|ban| ban.champion == ChampionKey(5))
        .expect("fighter ban candidate");
    let assassin_ban = output
        .bans
        .iter()
        .find(|ban| ban.champion == ChampionKey(6))
        .expect("assassin ban candidate");
    assert!(
        fighter_ban.score_fraction > assassin_ban.score_fraction,
        "el rival necesita frontline/engage: {} vs {}",
        fighter_ban.score_fraction,
        assassin_ban.score_fraction
    );
}

#[test]
fn diversity_caps_repeated_roles_when_alternatives_exist() {
    let format = DraftFormat::standard();
    let mut champions: Vec<ChampionProfile> = (100..106)
        .map(|key| fighter(key, &[(Role::Top, 1.0)]))
        .collect();
    champions.push(fighter(110, &[(Role::Jungle, 1.0)]));
    champions.push(mage(111, &[(Role::Mid, 1.0)]));
    champions.push(marksman(112, &[(Role::Adc, 1.0)]));
    champions.push(enchanter(113, &[(Role::Support, 1.0)]));
    let profiles = set(champions);
    let analysis = AnalysisParams::default();
    let params = RecommendationParams {
        limit: 5,
        ..Default::default()
    };
    let output = recommend(&request(
        &format,
        6,
        &[],
        &[],
        &[],
        &profiles,
        &analysis,
        &params,
    ))
    .expect("recommendation");

    assert_eq!(output.picks.len(), 5);
    let tops = output
        .picks
        .iter()
        .filter(|pick| pick.role == Role::Top)
        .count();
    assert!(tops <= 2, "top picks en el top 5: {tops}");
    let best = output
        .picks
        .iter()
        .map(|pick| pick.ranking_score)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((output.picks[0].ranking_score - best).abs() < 1e-6);
}

#[test]
fn sensitivity_to_twenty_percent_weight_change_keeps_top_one() {
    let params = RecommendationParams::default();
    let mut weights = params.weights_mid_pick;
    let features_a = [
        FeatureValue::new(FeatureId::Need, 0.80, 1.0),
        FeatureValue::new(FeatureId::Synergy, 0.60, 1.0),
        FeatureValue::new(FeatureId::Safety, 0.50, 1.0),
    ];
    let features_b = [
        FeatureValue::new(FeatureId::Need, 0.30, 1.0),
        FeatureValue::new(FeatureId::Synergy, 0.40, 1.0),
        FeatureValue::new(FeatureId::Safety, 0.60, 1.0),
    ];
    let base_a = score(&features_a, &weights, &[], 1.0, &params).expect("a");
    let base_b = score(&features_b, &weights, &[], 1.0, &params).expect("b");
    assert!(base_a.score > base_b.score);

    weights.need *= 1.20;
    let shifted_a = score(&features_a, &weights, &[], 1.0, &params).expect("a");
    let shifted_b = score(&features_b, &weights, &[], 1.0, &params).expect("b");
    assert!(shifted_a.score > shifted_b.score);

    weights = params.weights_mid_pick;
    weights.synergy *= 0.80;
    let shifted_a = score(&features_a, &weights, &[], 1.0, &params).expect("a");
    let shifted_b = score(&features_b, &weights, &[], 1.0, &params).expect("b");
    assert!(shifted_a.score > shifted_b.score);
}

#[test]
fn a_full_recommendation_list_is_fast_enough() {
    // Cuarenta campeones y un draft a medias: el peor caso razonable del piloto.
    let bases = fixture_set().champions.into_values().collect::<Vec<_>>();
    let mut profiles = Vec::new();
    for copy in 0..5u16 {
        for base in &bases {
            let mut profile = base.clone();
            profile.key = ChampionKey(100 + copy * 10 + base.key.0);
            profiles.push(profile);
        }
    }
    let profiles = set(profiles);
    let format = DraftFormat::standard();
    let analysis = AnalysisParams::default();
    let params = RecommendationParams::default();
    let blue = [pick(1, Role::Top), pick(2, Role::Mid)];
    let red = [pick(3, Role::Adc), pick(4, Role::Support)];
    let used = ["1", "2", "3", "4"]
        .iter()
        .map(|key| ChampionKey(key.parse().expect("clave")))
        .collect::<Vec<_>>();
    let req = request(
        &format, 17, &blue, &red, &used, &profiles, &analysis, &params,
    );

    recommend(&req).expect("calentamiento");
    let started = std::time::Instant::now();
    for _ in 0..10 {
        recommend(&req).expect("recommendation");
    }
    let average = started.elapsed() / 10;
    assert!(
        average < std::time::Duration::from_millis(100),
        "las recomendaciones tardaron {average:?} de media"
    );
}
