//! The demo's one feature area.
//!
//! It stands in for what a real product module does: read settings, produce data,
//! cache it, decide whether something is wrong, and tell the rest of the application
//! through typed events.
//!
//! Note what it does *not* do: it never touches Tauri, never opens a database, never
//! shows a notification itself. It asks the platform.

use async_trait::async_trait;
use origin_app::{ApplicationModule, ModuleRegistry, Platform};
use origin_core::{
    AccountId, Alert, ConnectorId, Health, Metric, MetricKey, Result, Severity, Unit,
};
use origin_events::{AlertRaised, AlertResolved, PlatformEvent};
use origin_platform::{Notification, Urgency};
use origin_settings::Setting;
use origin_storage::{StorageKey, namespace};
use origin_sync::{SyncContext, SyncPolicy, SyncReport, SyncResult, SyncSource, SyncTarget};
use serde::Serialize;
use std::sync::Arc;
use time::Duration;
use tokio::sync::Mutex;

/// Above this value the demo reports a warning.
const WARN_ABOVE: Setting<f64> = Setting::new("demo.warn_above", || 60.0);
/// Above this value it raises an alert and notifies the user.
const CRITICAL_ABOVE: Setting<f64> = Setting::new("demo.critical_above", || 85.0);

/// One fingerprint for one problem: repeated raises update the same alert instead of
/// notifying the user again (ADR-0005 / §32).
const ALERT_FINGERPRINT: &str = "demo.load.critical";

const LATEST_KEY: &str = "latest";
const CACHE_TTL: Duration = Duration::minutes(5);

/// The demo has no real account, but every target is account-scoped (ADR-0016) — so it
/// uses a fixed local one rather than inventing an exception.
const LOCAL_ACCOUNT: &str = "local";

/// How often the engine refreshes. A real product would tune this per target.
const REFRESH_INTERVAL: Duration = Duration::seconds(30);

/// What the UI renders.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct PulseSnapshot {
    pub health: Health,
    pub metric: Option<Metric>,
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Default)]
struct PulseState {
    alerts: Vec<Alert>,
    seed: u64,
}

/// The service the module provides.
///
/// Alerts are kept in this module on purpose. Every product will want alerts, but
/// until a second and third product actually need them there is no evidence for what
/// a shared alert engine should look like (ADR-0009, Rule of Three).
#[derive(Debug)]
pub struct PulseService {
    platform: Platform,
    state: Mutex<PulseState>,
}

impl PulseService {
    fn new(platform: Platform) -> Self {
        Self {
            platform,
            state: Mutex::new(PulseState::default()),
        }
    }

    fn connector() -> ConnectorId {
        ConnectorId::new("demo")
    }

    fn account() -> AccountId {
        AccountId::new(LOCAL_ACCOUNT)
    }

    /// The one thing this module synchronises.
    pub fn target() -> SyncTarget {
        SyncTarget::new(Self::connector(), Self::account(), "load")
    }

    fn cache_key() -> StorageKey {
        StorageKey::new(
            namespace::account(&Self::connector(), &Self::account(), "pulse"),
            LATEST_KEY,
        )
    }

    /// Current state without contacting anything.
    pub async fn snapshot(&self) -> Result<PulseSnapshot> {
        let metric: Option<Metric> = self.platform.cache.get(&Self::cache_key()).await?;

        let alerts: Vec<Alert> = self
            .state
            .lock()
            .await
            .alerts
            .iter()
            .filter(|alert| alert.is_visible())
            .cloned()
            .collect();

        let health = match &metric {
            None => Health::Unknown,
            Some(metric) => self.health_for(metric.value).await?,
        };

        Ok(PulseSnapshot {
            health,
            metric,
            alerts,
        })
    }

    /// Ask the engine to sync now.
    ///
    /// The engine decides nothing about *how* — that is [`PulseSource`] — but it owns
    /// single-flight, state and events, so the UI must not call the source directly.
    pub async fn refresh(&self) -> Result<PulseSnapshot> {
        self.platform.sync.sync_now(&Self::target()).await?;
        self.snapshot().await
    }

    /// Produce a new reading and react to it. Called by the engine, never directly.
    async fn fetch(&self) -> Result<SyncReport> {
        let now = self.platform.clock.now();
        let value = self.next_value(now.unix_timestamp_nanos() as u64).await;

        let metric = Metric::new(MetricKey::new("demo.load"), value, Unit::Percent, now);

        self.platform
            .cache
            .put(&Self::cache_key(), &metric, Some(CACHE_TTL))
            .await?;

        let health = self.health_for(value).await?;
        self.reconcile_alert(health, value, now).await?;

        // No SyncCompleted event is published here: the engine does that, with the
        // sync id and timing it owns.
        tracing::debug!(value, ?health, "pulse refreshed");
        Ok(SyncReport::changed(1))
    }

    async fn health_for(&self, value: f64) -> Result<Health> {
        let warn = self.platform.settings.get(&WARN_ABOVE).await?;
        let critical = self.platform.settings.get(&CRITICAL_ABOVE).await?;

        Ok(if value >= critical {
            Health::Critical
        } else if value >= warn {
            Health::Warning
        } else {
            Health::Healthy
        })
    }

    /// Raise, keep or resolve the alert for the current health.
    async fn reconcile_alert(
        &self,
        health: Health,
        value: f64,
        now: time::OffsetDateTime,
    ) -> Result<()> {
        let mut state = self.state.lock().await;
        let existing = state
            .alerts
            .iter_mut()
            .find(|alert| alert.fingerprint == ALERT_FINGERPRINT && alert.is_visible());

        match (health, existing) {
            (Health::Critical, Some(_)) => {
                // Already active: no second notification for the same problem.
                let _ = self
                    .platform
                    .events
                    .publish(PlatformEvent::AlertRaised(AlertRaised {
                        alert: state
                            .alerts
                            .iter()
                            .find(|alert| alert.fingerprint == ALERT_FINGERPRINT)
                            .cloned()
                            .expect("just matched"),
                        deduplicated: true,
                    }));
            }
            (Health::Critical, None) => {
                let alert = Alert::new(
                    ALERT_FINGERPRINT,
                    Severity::Critical,
                    "Load is critical",
                    now,
                )
                .with_body(format!("Demo load reached {value:.0} %"))
                .with_connector(Self::connector());

                state.alerts.push(alert.clone());
                drop(state);

                self.platform
                    .notifications
                    .notify(
                        Notification::new(&alert.title)
                            .with_body(alert.body.clone().unwrap_or_default())
                            .with_urgency(Urgency::Critical)
                            .with_tag(ALERT_FINGERPRINT),
                    )
                    .await?;

                let _ = self
                    .platform
                    .events
                    .publish(PlatformEvent::AlertRaised(AlertRaised {
                        alert,
                        deduplicated: false,
                    }));
            }
            (_, Some(alert)) => {
                alert.resolve(now);
                let id = alert.id.clone();
                drop(state);

                let _ = self
                    .platform
                    .events
                    .publish(PlatformEvent::AlertResolved(AlertResolved {
                        alert: id,
                        at: now,
                    }));
            }
            (_, None) => {}
        }

        Ok(())
    }

    /// A deterministic pseudo-random reading.
    ///
    /// Stands in for an external API so the demo needs no credentials to be useful.
    async fn next_value(&self, seed_material: u64) -> f64 {
        let mut state = self.state.lock().await;
        if state.seed == 0 {
            state.seed = seed_material | 1;
        }
        // Numerical Recipes LCG — good enough for a demo, and reproducible.
        state.seed = state
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);

        f64::from((state.seed >> 33) as u32 % 10_001) / 100.0
    }
}

/// Adapts the service to the sync engine.
///
/// Note what it does not contain: no interval, no retry, no backoff, no check whether
/// the machine is online. Those belong to the engine (ADR-0017).
#[derive(Debug)]
struct PulseSource {
    service: Arc<PulseService>,
}

#[async_trait]
impl SyncSource for PulseSource {
    async fn sync(&self, _context: &SyncContext) -> Result<SyncResult> {
        Ok(SyncResult::Updated(self.service.fetch().await?))
    }
}

#[derive(Debug)]
pub struct PulseModule;

impl ApplicationModule for PulseModule {
    fn id(&self) -> &'static str {
        "pulse"
    }

    fn register(&self, registry: &mut ModuleRegistry) -> Result<()> {
        let platform = registry.platform().clone();
        let service = Arc::new(PulseService::new(platform.clone()));

        platform.sync.register(
            PulseService::target(),
            SyncPolicy::every(REFRESH_INTERVAL),
            Arc::new(PulseSource {
                service: service.clone(),
            }),
        );

        registry.provide(service);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use origin_app::ApplicationBuilder;
    use origin_core::testing::FakeClock;
    use origin_platform::testing::RecordingNotificationService;
    use time::macros::datetime;
    use ts_rs::{Config, TS};

    #[test]
    fn the_checked_in_product_contract_matches_rust() {
        let config = Config::default().with_large_int("number");
        let expected = format!(
            "// Generated from PulseSnapshot in src-tauri/src/pulse.rs. Do not edit.\n\n\
             import type {{ Alert, Health, Metric }} from \"@origin/client\";\n\n\
             export {}\n",
            PulseSnapshot::decl(&config)
        );

        assert_eq!(
            include_str!("../../src/pulse.generated.ts"),
            expected,
            "the product IPC contract drifted; regenerate pulse.generated.ts from the Rust declaration"
        );
    }

    /// The whole feature is exercised without starting Tauri — the quality gate from
    /// ADR-0002.
    async fn application() -> (Arc<PulseService>, Arc<RecordingNotificationService>) {
        let notifications = Arc::new(RecordingNotificationService::new());
        let application = ApplicationBuilder::in_memory()
            .clock(Arc::new(FakeClock::new(datetime!(2026-08-23 10:00 UTC))))
            .notifications(notifications.clone())
            .module(PulseModule)
            .build()
            .expect("build application");

        (
            application.require::<PulseService>().unwrap(),
            notifications,
        )
    }

    #[tokio::test]
    async fn an_unrefreshed_application_reports_unknown_health() {
        let (pulse, _) = application().await;

        let snapshot = pulse.snapshot().await.unwrap();

        assert_eq!(snapshot.health, Health::Unknown);
        assert!(snapshot.metric.is_none());
    }

    #[tokio::test]
    async fn refreshing_caches_a_metric() {
        let (pulse, _) = application().await;

        let snapshot = pulse.refresh().await.unwrap();

        let metric = snapshot.metric.expect("a metric was produced");
        assert_eq!(metric.key.as_str(), "demo.load");
        assert_eq!(metric.at, datetime!(2026-08-23 10:00 UTC));
        assert_ne!(snapshot.health, Health::Unknown);
    }

    #[tokio::test]
    async fn a_critical_reading_notifies_once_and_resolves_when_it_recovers() {
        let (pulse, notifications) = application().await;
        let now = datetime!(2026-08-23 10:00 UTC);

        // Force the critical branch without depending on the pseudo-random value.
        pulse
            .reconcile_alert(Health::Critical, 92.0, now)
            .await
            .unwrap();
        pulse
            .reconcile_alert(Health::Critical, 94.0, now)
            .await
            .unwrap();

        assert_eq!(
            notifications.sent().len(),
            1,
            "the same problem must not notify twice"
        );
        assert_eq!(pulse.snapshot().await.unwrap().alerts.len(), 1);

        pulse
            .reconcile_alert(Health::Healthy, 12.0, now)
            .await
            .unwrap();

        assert!(
            pulse.snapshot().await.unwrap().alerts.is_empty(),
            "a recovered alert must stop being visible"
        );
    }

    #[tokio::test]
    async fn thresholds_come_from_settings() {
        let (pulse, _) = application().await;

        assert_eq!(pulse.health_for(70.0).await.unwrap(), Health::Warning);

        pulse
            .platform
            .settings
            .set(&WARN_ABOVE, &90.0)
            .await
            .unwrap();

        assert_eq!(
            pulse.health_for(70.0).await.unwrap(),
            Health::Healthy,
            "raising the threshold must change the verdict"
        );
    }
}
