//! Motor de recomendaciones. En la Fase 1 solo expone la versión del motor;
//! la puntuación multicriterio se implementa en la Fase 3.

pub const ENGINE_VERSION: &str = "0.1.0-phase1";

pub fn engine_version() -> &'static str {
    ENGINE_VERSION
}

#[cfg(test)]
mod tests {
    #[test]
    fn engine_version_is_set() {
        assert!(!super::engine_version().is_empty());
    }
}
