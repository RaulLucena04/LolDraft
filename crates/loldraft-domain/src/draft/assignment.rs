use crate::champion::Champion;
use crate::ids::Role;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleAssignment {
    /// Rol asignado a cada campeón, en el mismo orden que la entrada.
    pub roles: Vec<Option<Role>>,
    /// Suma de `proficiency` de la mejor asignación.
    pub score: f32,
    /// Separación relativa entre la mejor y la segunda mejor asignación.
    pub confidence: f32,
    pub assigned: usize,
}

/// Asigna roles de forma óptima (5! = 120 combinaciones como máximo).
///
/// No usa asignación codiciosa: dos flex picks pueden compartir roles y una
/// elección local puede bloquear la mejor composición global.
pub fn assign_roles(champions: &[&Champion]) -> RoleAssignment {
    assign_roles_with_locks(champions, &vec![None; champions.len()])
}

/// Igual que [`assign_roles`], pero respeta roles fijados manualmente.
///
/// Un `Some(role)` obliga a ese campeón a ocupar ese rol. Si dos bloqueos
/// compiten por el mismo rol, el segundo queda sin asignar en lugar de
/// producir una composición inválida; la UI puede avisar de la carencia.
pub fn assign_roles_with_locks(champions: &[&Champion], locks: &[Option<Role>]) -> RoleAssignment {
    let mut search = Search {
        champions,
        locks,
        current: vec![None; champions.len()],
        best_roles: vec![None; champions.len()],
        best_score: 0.0,
        second_score: 0.0,
    };
    search.run(0, 0, 0.0);

    let assigned = search
        .best_roles
        .iter()
        .filter(|role| role.is_some())
        .count();
    let confidence = if search.best_score > 0.0 {
        (1.0 - search.second_score / search.best_score).clamp(0.0, 1.0)
    } else {
        0.0
    };

    RoleAssignment {
        roles: search.best_roles,
        score: search.best_score,
        confidence,
        assigned,
    }
}

struct Search<'a> {
    champions: &'a [&'a Champion],
    locks: &'a [Option<Role>],
    current: Vec<Option<Role>>,
    best_roles: Vec<Option<Role>>,
    best_score: f32,
    second_score: f32,
}

impl Search<'_> {
    fn run(&mut self, index: usize, used_mask: u8, score: f32) {
        if index == self.champions.len() {
            self.record(score);
            return;
        }

        // Un rol fijado manualmente no compite con las alternativas del campeón.
        if let Some(lock) = self.locks.get(index).copied().flatten() {
            let bit = 1u8 << lock.index();
            if used_mask & bit == 0 {
                self.current[index] = Some(lock);
                let proficiency = self.champions[index]
                    .role_fit(lock)
                    .map(|fit| fit.proficiency)
                    .unwrap_or(1.0);
                self.run(index + 1, used_mask | bit, score + proficiency);
            } else {
                // Dos bloqueos compiten por el mismo rol: se deja sin asignar.
                self.current[index] = None;
                self.run(index + 1, used_mask, score);
            }
            self.current[index] = None;
            return;
        }

        // Opción de no asignar rol (campeón sin rol viable o rol ya ocupado).
        self.current[index] = None;
        self.run(index + 1, used_mask, score);

        for fit in &self.champions[index].roles {
            if fit.proficiency <= 0.0 {
                continue;
            }
            let bit = 1u8 << fit.role.index();
            if used_mask & bit != 0 {
                continue;
            }
            self.current[index] = Some(fit.role);
            self.run(index + 1, used_mask | bit, score + fit.proficiency);
        }

        self.current[index] = None;
    }

    fn record(&mut self, score: f32) {
        const EPSILON: f32 = 1e-6;

        if score > self.best_score + EPSILON {
            self.second_score = self.best_score;
            self.best_score = score;
            self.best_roles.clone_from(&self.current);
        } else if (score - self.best_score).abs() <= EPSILON {
            // Hay al menos dos asignaciones igual de buenas: el rol es ambiguo.
            if self.current != self.best_roles {
                self.second_score = score;
            }
        } else if score > self.second_score {
            self.second_score = score;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::champion::{Champion, RoleFit};
    use crate::ids::ChampionKey;

    fn champion(key: u16, roles: &[(Role, f32)]) -> Champion {
        Champion {
            key: ChampionKey(key),
            slug: format!("champion-{key}"),
            name: format!("Champion {key}"),
            title: String::new(),
            image: String::new(),
            tags: Vec::new(),
            roles: roles
                .iter()
                .map(|(role, proficiency)| RoleFit {
                    role: *role,
                    proficiency: *proficiency,
                    confidence: 1.0,
                    source: "test".to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn assigns_distinct_roles() {
        let a = champion(1, &[(Role::Top, 1.0), (Role::Mid, 0.9)]);
        let b = champion(2, &[(Role::Mid, 1.0)]);
        let c = champion(3, &[(Role::Jungle, 0.8)]);
        let assignment = assign_roles(&[&a, &b, &c]);

        assert_eq!(assignment.assigned, 3);
        assert_eq!(assignment.roles[0], Some(Role::Top));
        assert_eq!(assignment.roles[1], Some(Role::Mid));
        assert_eq!(assignment.roles[2], Some(Role::Jungle));
        assert!((assignment.score - 2.8).abs() < 1e-6);
    }

    #[test]
    fn flex_pick_moves_to_second_role_to_unblock_teammate() {
        let flex = champion(1, &[(Role::Top, 1.0), (Role::Mid, 0.95)]);
        let mid_only = champion(2, &[(Role::Mid, 1.0)]);
        let assignment = assign_roles(&[&flex, &mid_only]);

        assert_eq!(assignment.roles[0], Some(Role::Top));
        assert_eq!(assignment.roles[1], Some(Role::Mid));
        assert!((assignment.score - 2.0).abs() < 1e-6);
    }

    #[test]
    fn champion_without_roles_is_unassigned() {
        let a = champion(1, &[]);
        let assignment = assign_roles(&[&a]);
        assert_eq!(assignment.roles, vec![None]);
        assert_eq!(assignment.assigned, 0);
        assert_eq!(assignment.confidence, 0.0);
    }

    #[test]
    fn tied_assignments_have_low_confidence() {
        let a = champion(1, &[(Role::Top, 1.0)]);
        let b = champion(2, &[(Role::Top, 1.0)]);
        let assignment = assign_roles(&[&a, &b]);
        assert_eq!(assignment.assigned, 1);
        assert_eq!(assignment.confidence, 0.0);
    }

    #[test]
    fn manual_lock_forces_a_flex_pick_into_its_second_role() {
        let flex = champion(1, &[(Role::Top, 1.0), (Role::Mid, 0.9)]);
        let mid_only = champion(2, &[(Role::Mid, 1.0)]);
        let assignment = assign_roles_with_locks(&[&flex, &mid_only], &[Some(Role::Mid), None]);

        assert_eq!(assignment.roles[0], Some(Role::Mid));
        assert_eq!(assignment.roles[1], None);
        assert_eq!(assignment.assigned, 1);
    }

    #[test]
    fn conflicting_locks_leave_the_second_champion_unassigned() {
        let a = champion(1, &[(Role::Top, 1.0)]);
        let b = champion(2, &[(Role::Top, 1.0)]);
        let assignment = assign_roles_with_locks(&[&a, &b], &[Some(Role::Top), Some(Role::Top)]);

        assert_eq!(assignment.roles[0], Some(Role::Top));
        assert_eq!(assignment.roles[1], None);
        assert_eq!(assignment.assigned, 1);
    }
}
