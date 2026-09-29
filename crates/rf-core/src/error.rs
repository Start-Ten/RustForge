//! 引擎统一错误类型（IF-007 / IF-008）。

use crate::Entity;

/// 引擎统一错误（IF-007）。
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("entity {0} is not alive (stale generation or despawned)")]
    EntityDead(Entity),

    #[error("component `{0}` is not registered")]
    ComponentNotRegistered(&'static str),

    #[error("stale handle: slot generation mismatch")]
    StaleHandle,

    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),

    #[error("not yet supported: {what} (planned: {priority})")]
    NotYetSupported { what: &'static str, priority: &'static str },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid data: {0}")]
    InvalidData(String),

    #[error("{0}")]
    Message(String),
}

/// 统一 Result 别名（IF-008）。
pub type Result<T, E = EngineError> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = EngineError::EntityDead(Entity::new(3, 1));
        assert!(e.to_string().contains("#3v1"));
        let n = EngineError::NotYetSupported { what: "fbx", priority: "P1" };
        assert!(n.to_string().contains("fbx"));
    }
}
