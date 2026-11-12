//! Necesidades de la composición: suelos, balance de daño y amenazas del rival.
//!
//! Las reglas son funciones tipadas. Las necesidades se combinan con `max`, no
//! con suma, para que una sola amenaza no dispare diez carencias artificiales.

use super::params::{AnalysisParams, Dimension};
use super::team::TeamAggregate;
use crate::taxonomy::{ArchetypeId, TraitId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedStatus {
    /// La carencia está confirmada: incluso la cota superior queda por debajo.
    Confirmed,
    /// El intervalo cruza el umbral: puede faltar, pero no se afirma.
    Possible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreatRuleId {
    EnemyEngage,
    EnemyEngageFrontline,
    EnemyPoke,
    EnemySiege,
    EnemyFrontline,
    EnemyAssassin,
    EnemyCatch,
    EnemySplitpush,
    EnemyWombo,
    EnemyEarly,
    EnemyScaling,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NeedSource {
    Baseline { floor: f32 },
    EnemyThreat { rule: ThreatRuleId, threshold: f32 },
    DamageBalance { share: f32, threshold: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Need {
    pub dimension: Dimension,
    pub amount: f32,
    pub status: NeedStatus,
    /// Una necesidad puede tener varias razones; se conservan para explicarla.
    pub sources: Vec<NeedSource>,
}

#[derive(Debug, Default)]
struct Accum {
    base: f32,
    base_status: Option<NeedStatus>,
    context: f32,
    context_status: Option<NeedStatus>,
    sources: Vec<NeedSource>,
}

pub(crate) fn compute(
    team: &TeamAggregate,
    enemy: Option<&TeamAggregate>,
    params: &AnalysisParams,
) -> Vec<Need> {
    if team.picks == 0 {
        return Vec::new();
    }

    let mut needs: BTreeMap<Dimension, Accum> = BTreeMap::new();

    for (id, floor) in &params.archetype_floors {
        add_baseline(&mut needs, team, Dimension::Archetype(*id), *floor);
    }
    for (id, floor) in &params.trait_floors {
        add_baseline(&mut needs, team, Dimension::Trait(*id), *floor);
    }

    add_damage_needs(&mut needs, team);

    if let Some(enemy) = enemy {
        add_enemy_threats(&mut needs, team, enemy);
    }

    let mut result = Vec::new();
    for (dimension, accum) in needs {
        let (amount, status) = if accum.context > accum.base {
            (accum.context, accum.context_status)
        } else {
            (accum.base, accum.base_status)
        };
        let amount = (params.priority(dimension) * amount).clamp(0.0, 1.0);
        if amount <= 0.001 {
            continue;
        }
        let mut sources = accum.sources;
        sources.sort_by_key(source_key);
        sources.dedup_by(|left, right| source_key(left) == source_key(right));
        result.push(Need {
            dimension,
            amount,
            status: status.unwrap_or(NeedStatus::Possible),
            sources,
        });
    }

    result.sort_by(|left, right| {
        right
            .amount
            .total_cmp(&left.amount)
            .then_with(|| left.dimension.cmp(&right.dimension))
    });
    result
}

fn source_key(source: &NeedSource) -> String {
    match source {
        NeedSource::Baseline { floor } => format!("baseline:{floor:.3}"),
        NeedSource::EnemyThreat { rule, threshold } => {
            format!("threat:{}:{threshold:.3}", rule_label(*rule))
        }
        NeedSource::DamageBalance { share, threshold } => {
            format!("damage:{share:.3}:{threshold:.3}")
        }
    }
}

fn rule_label(rule: ThreatRuleId) -> &'static str {
    match rule {
        ThreatRuleId::EnemyEngage => "enemy_engage",
        ThreatRuleId::EnemyEngageFrontline => "enemy_engage_frontline",
        ThreatRuleId::EnemyPoke => "enemy_poke",
        ThreatRuleId::EnemySiege => "enemy_siege",
        ThreatRuleId::EnemyFrontline => "enemy_frontline",
        ThreatRuleId::EnemyAssassin => "enemy_assassin",
        ThreatRuleId::EnemyCatch => "enemy_catch",
        ThreatRuleId::EnemySplitpush => "enemy_splitpush",
        ThreatRuleId::EnemyWombo => "enemy_wombo",
        ThreatRuleId::EnemyEarly => "enemy_early",
        ThreatRuleId::EnemyScaling => "enemy_scaling",
    }
}

fn add_baseline(
    needs: &mut BTreeMap<Dimension, Accum>,
    team: &TeamAggregate,
    dimension: Dimension,
    floor: f32,
) {
    let Some(coverage) = team.coverage(dimension) else {
        return;
    };
    if coverage.total == 0 || coverage.value_low >= floor {
        return;
    }
    let amount = floor - coverage.value_low;
    let status = if coverage.value_high < floor {
        NeedStatus::Confirmed
    } else {
        NeedStatus::Possible
    };
    let entry = needs.entry(dimension).or_default();
    if amount > entry.base {
        entry.base = amount;
        entry.base_status = Some(status);
    }
    entry.sources.push(NeedSource::Baseline { floor });
}

fn add_damage_needs(needs: &mut BTreeMap<Dimension, Accum>, team: &TeamAggregate) {
    let Some(damage) = &team.damage else {
        return;
    };
    // Con menos del 60 % del peso de oro conocido no se afirma un desbalance.
    if damage.known_weight_ratio < 0.60 {
        return;
    }
    let status = if damage.known_weight_ratio >= 0.80 {
        NeedStatus::Confirmed
    } else {
        NeedStatus::Possible
    };
    let threshold = 0.30;
    for (share, dimension) in [
        (damage.magic, Dimension::Trait(TraitId::DamageMagic)),
        (damage.physical, Dimension::Trait(TraitId::DamagePhysical)),
    ] {
        if share >= threshold {
            continue;
        }
        let amount = (threshold - share) / threshold * 0.60;
        if amount <= 0.0 {
            continue;
        }
        let entry = needs.entry(dimension).or_default();
        if amount > entry.context {
            entry.context = amount;
            entry.context_status = Some(status);
        }
        entry
            .sources
            .push(NeedSource::DamageBalance { share, threshold });
    }
}

fn add_enemy_threats(
    needs: &mut BTreeMap<Dimension, Accum>,
    team: &TeamAggregate,
    enemy: &TeamAggregate,
) {
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyEngage,
        &[(Dimension::Archetype(ArchetypeId::Engage), 0.60)],
        &[(Dimension::Archetype(ArchetypeId::Disengage), 0.40)],
        &[
            (Dimension::Archetype(ArchetypeId::Disengage), 0.55),
            (Dimension::Archetype(ArchetypeId::Peel), 0.30),
        ],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyEngageFrontline,
        &[(Dimension::Archetype(ArchetypeId::Engage), 0.60)],
        &[(Dimension::Archetype(ArchetypeId::Frontline), 0.45)],
        &[(Dimension::Archetype(ArchetypeId::Frontline), 0.45)],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyPoke,
        &[(Dimension::Archetype(ArchetypeId::Poke), 0.55)],
        &[
            (Dimension::Archetype(ArchetypeId::Engage), 0.45),
            (Dimension::Archetype(ArchetypeId::Sustain), 0.35),
        ],
        &[
            (Dimension::Archetype(ArchetypeId::Engage), 0.50),
            (Dimension::Archetype(ArchetypeId::Sustain), 0.30),
        ],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemySiege,
        &[(Dimension::Archetype(ArchetypeId::Siege), 0.50)],
        &[(Dimension::Trait(TraitId::Waveclear), 0.40)],
        &[
            (Dimension::Trait(TraitId::Waveclear), 0.40),
            (Dimension::Archetype(ArchetypeId::Engage), 0.25),
        ],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyFrontline,
        &[(Dimension::Archetype(ArchetypeId::Frontline), 0.60)],
        &[(Dimension::Archetype(ArchetypeId::Dps), 0.55)],
        &[
            (Dimension::Archetype(ArchetypeId::Dps), 0.40),
            (Dimension::Archetype(ArchetypeId::AntiTank), 0.45),
        ],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyAssassin,
        &[
            (Dimension::Archetype(ArchetypeId::Assassin), 0.60),
            (Dimension::Archetype(ArchetypeId::Burst), 0.60),
        ],
        &[(Dimension::Archetype(ArchetypeId::Peel), 0.40)],
        &[
            (Dimension::Archetype(ArchetypeId::Peel), 0.45),
            (Dimension::Archetype(ArchetypeId::Frontline), 0.25),
        ],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyCatch,
        &[(Dimension::Archetype(ArchetypeId::Catch), 0.55)],
        &[(Dimension::Trait(TraitId::Mobility), 0.40)],
        &[
            (Dimension::Trait(TraitId::Mobility), 0.35),
            (Dimension::Archetype(ArchetypeId::Utility), 0.25),
        ],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemySplitpush,
        &[(Dimension::Archetype(ArchetypeId::Splitpush), 0.55)],
        &[(Dimension::Archetype(ArchetypeId::Catch), 0.40)],
        &[
            (Dimension::Archetype(ArchetypeId::Catch), 0.40),
            (Dimension::Trait(TraitId::Waveclear), 0.30),
        ],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyWombo,
        &[(Dimension::Archetype(ArchetypeId::WomboCombo), 0.55)],
        &[(Dimension::Archetype(ArchetypeId::Disengage), 0.45)],
        &[(Dimension::Archetype(ArchetypeId::Disengage), 0.50)],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyEarly,
        &[(Dimension::Archetype(ArchetypeId::EarlyGame), 0.60)],
        &[(Dimension::Trait(TraitId::EarlyPower), 0.45)],
        &[
            (Dimension::Archetype(ArchetypeId::EarlyGame), 0.35),
            (Dimension::Trait(TraitId::LaneSafety), 0.30),
        ],
    );
    add_rule_needs(
        needs,
        team,
        enemy,
        ThreatRuleId::EnemyScaling,
        &[(Dimension::Archetype(ArchetypeId::Scaling), 0.65)],
        &[(Dimension::Archetype(ArchetypeId::EarlyGame), 0.45)],
        &[(Dimension::Archetype(ArchetypeId::EarlyGame), 0.35)],
    );
}

fn add_rule_needs(
    needs: &mut BTreeMap<Dimension, Accum>,
    team: &TeamAggregate,
    enemy: &TeamAggregate,
    rule: ThreatRuleId,
    threats: &[(Dimension, f32)],
    own_deficits: &[(Dimension, f32)],
    targets: &[(Dimension, f32)],
) {
    // Amenaza: basta con que una de las dimensiones declaradas sea posible.
    let mut threat_confirmed = false;
    let mut threat_possible = false;
    let mut primary_threshold: f32 = 0.0;
    for (dimension, threshold) in threats {
        if let Some(confirmed) = enemy_threat_level(enemy, *dimension, *threshold) {
            threat_possible = true;
            threat_confirmed |= confirmed;
            primary_threshold = primary_threshold.max(*threshold);
        }
    }
    if !threat_possible {
        return;
    }

    let mut deficit_confirmed = true;
    let mut deficit_possible = true;
    for (dimension, threshold) in own_deficits {
        match own_deficit_level(team, *dimension, *threshold) {
            Some(confirmed) => {
                deficit_confirmed &= confirmed;
                deficit_possible = true;
            }
            None => {
                deficit_possible = false;
            }
        }
    }
    if !deficit_possible {
        return;
    }

    let status = if threat_confirmed && deficit_confirmed {
        NeedStatus::Confirmed
    } else {
        NeedStatus::Possible
    };

    for (dimension, base_amount) in targets {
        add_context_need(
            needs,
            team,
            *dimension,
            *base_amount,
            status,
            NeedSource::EnemyThreat {
                rule,
                threshold: primary_threshold,
            },
        );
    }
}

fn add_context_need(
    needs: &mut BTreeMap<Dimension, Accum>,
    team: &TeamAggregate,
    dimension: Dimension,
    base_amount: f32,
    status: NeedStatus,
    source: NeedSource,
) {
    let target_low = team
        .coverage(dimension)
        .map(|coverage| coverage.value_low)
        .unwrap_or(0.0);
    // Si la herramienta ya está cubierta, la regla no inventa una carencia.
    if target_low >= 0.55 {
        return;
    }
    let amount = base_amount * (1.0 - target_low);
    if amount <= 0.001 {
        return;
    }
    let entry = needs.entry(dimension).or_default();
    if amount > entry.context {
        entry.context = amount;
        entry.context_status = Some(status);
    }
    entry.sources.push(source);
}

fn enemy_threat_level(enemy: &TeamAggregate, dimension: Dimension, threshold: f32) -> Option<bool> {
    let coverage = enemy.coverage(dimension)?;
    if coverage.total == 0 {
        return None;
    }
    if coverage.value_low >= threshold {
        Some(true)
    } else if coverage.value_high >= threshold {
        Some(false)
    } else {
        None
    }
}

fn own_deficit_level(team: &TeamAggregate, dimension: Dimension, threshold: f32) -> Option<bool> {
    let coverage = team.coverage(dimension)?;
    if coverage.total == 0 {
        return None;
    }
    if coverage.value_high < threshold {
        Some(true)
    } else if coverage.value_low < threshold {
        Some(false)
    } else {
        None
    }
}
