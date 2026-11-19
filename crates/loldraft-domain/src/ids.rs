use serde::{Deserialize, Serialize};
use std::fmt;

/// Identificador estable de Riot para un campeón (por ejemplo, 266 para Aatrox).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChampionKey(pub u16);

impl fmt::Display for ChampionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Top,
    Jungle,
    Mid,
    Adc,
    Support,
}

impl Role {
    pub const ALL: [Role; 5] = [Role::Top, Role::Jungle, Role::Mid, Role::Adc, Role::Support];

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Top => "top",
            Role::Jungle => "jungle",
            Role::Mid => "mid",
            Role::Adc => "adc",
            Role::Support => "support",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Role::Top => "Top",
            Role::Jungle => "Jungla",
            Role::Mid => "Mid",
            Role::Adc => "ADC",
            Role::Support => "Support",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Role::Top => 0,
            Role::Jungle => 1,
            Role::Mid => 2,
            Role::Adc => 3,
            Role::Support => 4,
        }
    }

    pub fn from_code(value: &str) -> Option<Self> {
        match value {
            "top" => Some(Role::Top),
            "jungle" => Some(Role::Jungle),
            "mid" => Some(Role::Mid),
            "adc" => Some(Role::Adc),
            "support" => Some(Role::Support),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Blue,
    Red,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Side::Blue => "blue",
            Side::Red => "red",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Side::Blue => "Azul",
            Side::Red => "Rojo",
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Side::Blue => Side::Red,
            Side::Red => Side::Blue,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Ban,
    Pick,
}

impl ActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActionKind::Ban => "ban",
            ActionKind::Pick => "pick",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftAction {
    pub side: Side,
    pub kind: ActionKind,
    pub champion_key: ChampionKey,
}

impl DraftAction {
    pub fn new(side: Side, kind: ActionKind, champion_key: ChampionKey) -> Self {
        Self {
            side,
            kind,
            champion_key,
        }
    }
}
