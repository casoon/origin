use crate::Connector;
use origin_domain::{AppError, ConnectorId, Result};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The connectors an application was built with.
///
/// Populated by the composition root and then read-only — there is no runtime
/// registration, so the set of external services a build can reach is fixed at compile
/// time and auditable.
#[derive(Debug, Clone, Default)]
pub struct ConnectorRegistry {
    connectors: BTreeMap<ConnectorId, Arc<dyn Connector>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, connector: Arc<dyn Connector>) {
        self.connectors.insert(connector.id(), connector);
    }

    pub fn get(&self, id: &ConnectorId) -> Option<Arc<dyn Connector>> {
        self.connectors.get(id).cloned()
    }

    /// Resolve a connector or fail with a configuration error naming it.
    pub fn require(&self, id: &ConnectorId) -> Result<Arc<dyn Connector>> {
        self.get(id).ok_or_else(|| {
            AppError::configuration(format!("this application has no connector `{id}`"))
        })
    }

    pub fn ids(&self) -> Vec<ConnectorId> {
        self.connectors.keys().cloned().collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Connector>> {
        self.connectors.values()
    }

    pub fn is_empty(&self) -> bool {
        self.connectors.is_empty()
    }
}

/// Constructed from the composition root.
impl FromIterator<Arc<dyn Connector>> for ConnectorRegistry {
    fn from_iter<I: IntoIterator<Item = Arc<dyn Connector>>>(connectors: I) -> Self {
        let mut registry = Self::new();
        for connector in connectors {
            registry.insert(connector);
        }
        registry
    }
}
