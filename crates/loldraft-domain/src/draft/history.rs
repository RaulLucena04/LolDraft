use crate::champion::ChampionCatalog;
use crate::draft::format::DraftFormat;
use crate::draft::state::DraftState;
use crate::error::DraftError;
use crate::ids::{DraftAction, Side};

/// Sesión de draft con historial de deshacer/rehacer.
///
/// El estado es inmutable desde fuera: cada acción produce una instantánea nueva.
/// Con 20 acciones por draft, clonar el estado es más simple y seguro que un
/// sistema de deltas, y el coste es despreciable.
#[derive(Debug, Clone)]
pub struct DraftSession {
    side: Side,
    state: DraftState,
    undo_stack: Vec<DraftState>,
    redo_stack: Vec<DraftState>,
}

impl DraftSession {
    pub fn new(format: DraftFormat, side: Side) -> Self {
        Self {
            side,
            state: DraftState::new(format),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn standard(side: Side) -> Self {
        Self::new(DraftFormat::standard(), side)
    }

    pub fn from_state(state: DraftState, side: Side) -> Self {
        Self {
            side,
            state,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Reconstruye una sesión desde sus acciones aplicadas y conserva el historial
    /// de deshacer. Se usa al reabrir una sesión guardada: el estado persistido
    /// solo contiene las acciones, no la pila de instantáneas.
    pub fn replay(
        format: DraftFormat,
        side: Side,
        actions: &[DraftAction],
    ) -> Result<Self, DraftError> {
        let mut session = Self::new(format, side);
        for action in actions {
            session.apply(*action)?;
        }
        Ok(session)
    }

    pub fn side(&self) -> Side {
        self.side
    }

    pub fn state(&self) -> &DraftState {
        &self.state
    }

    pub fn apply(&mut self, action: DraftAction) -> Result<(), DraftError> {
        let mut next = self.state.clone();
        next.apply(action)?;
        self.undo_stack
            .push(std::mem::replace(&mut self.state, next));
        self.redo_stack.clear();
        Ok(())
    }

    pub fn apply_in(
        &mut self,
        action: DraftAction,
        catalog: &ChampionCatalog,
    ) -> Result<(), DraftError> {
        let mut next = self.state.clone();
        next.apply_in(action, catalog)?;
        self.undo_stack
            .push(std::mem::replace(&mut self.state, next));
        self.redo_stack.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack
            .push(std::mem::replace(&mut self.state, previous));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack
            .push(std::mem::replace(&mut self.state, next));
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ActionKind, ChampionKey};

    fn action(side: Side, kind: ActionKind, key: u16) -> DraftAction {
        DraftAction::new(side, kind, ChampionKey(key))
    }

    #[test]
    fn undo_and_redo_restore_state() {
        let mut session = DraftSession::standard(Side::Blue);
        session
            .apply(action(Side::Blue, ActionKind::Ban, 1))
            .unwrap();
        session
            .apply(action(Side::Red, ActionKind::Ban, 2))
            .unwrap();
        assert_eq!(session.state().actions().len(), 2);
        assert!(session.can_undo());

        assert!(session.undo());
        assert_eq!(session.state().actions().len(), 1);
        assert!(session.can_redo());

        assert!(session.redo());
        assert_eq!(session.state().actions().len(), 2);
        assert!(!session.can_redo());
    }

    #[test]
    fn replay_restores_state_and_undo_history() {
        let actions = [
            action(Side::Blue, ActionKind::Ban, 1),
            action(Side::Red, ActionKind::Ban, 2),
        ];
        let mut session =
            DraftSession::replay(DraftFormat::standard(), Side::Blue, &actions).unwrap();
        assert_eq!(session.state().actions(), actions.as_slice());
        assert_eq!(session.undo_depth(), 2);
        assert!(!session.can_redo());

        assert!(session.undo());
        assert_eq!(session.state().actions().len(), 1);
    }

    #[test]
    fn new_action_clears_redo_stack() {
        let mut session = DraftSession::standard(Side::Blue);
        session
            .apply(action(Side::Blue, ActionKind::Ban, 1))
            .unwrap();
        session.undo();
        assert!(session.can_redo());

        session
            .apply(action(Side::Blue, ActionKind::Ban, 2))
            .unwrap();
        assert!(!session.can_redo());
    }

    #[test]
    fn invalid_action_does_not_change_history() {
        let mut session = DraftSession::standard(Side::Blue);
        let error = session
            .apply(action(Side::Red, ActionKind::Ban, 1))
            .unwrap_err();
        assert!(matches!(error, DraftError::WrongStep { .. }));
        assert_eq!(session.state().actions().len(), 0);
        assert!(!session.can_undo());
    }
}
