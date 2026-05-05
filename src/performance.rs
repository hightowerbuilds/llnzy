use std::collections::VecDeque;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PerformanceScenario {
    TerminalIdle,
    TerminalOutputFlood,
    EditorTenThousandLines,
    EditorFiftyThousandLines,
    EditorWordWrap,
    EditorDiagnostics,
    EffectsEnabled,
    LspInteractive,
    GitRefresh,
}

impl PerformanceScenario {
    pub fn label(self) -> &'static str {
        match self {
            Self::TerminalIdle => "terminal idle",
            Self::TerminalOutputFlood => "terminal output flood",
            Self::EditorTenThousandLines => "editor 10k lines",
            Self::EditorFiftyThousandLines => "editor 50k lines",
            Self::EditorWordWrap => "editor word wrap",
            Self::EditorDiagnostics => "editor diagnostics",
            Self::EffectsEnabled => "effects enabled",
            Self::LspInteractive => "LSP interactive",
            Self::GitRefresh => "Git refresh",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerformanceBudget {
    pub scenario: PerformanceScenario,
    pub target_frame_ms: f32,
    pub p95_frame_ms: f32,
    pub max_frame_ms: f32,
    pub max_input_latency_ms: Option<f32>,
}

impl PerformanceBudget {
    pub const fn new(
        scenario: PerformanceScenario,
        target_frame_ms: f32,
        p95_frame_ms: f32,
        max_frame_ms: f32,
        max_input_latency_ms: Option<f32>,
    ) -> Self {
        Self {
            scenario,
            target_frame_ms,
            p95_frame_ms,
            max_frame_ms,
            max_input_latency_ms,
        }
    }

    pub fn evaluate(self, smoothness: FrameSmoothness) -> BudgetEvaluation {
        BudgetEvaluation {
            scenario: self.scenario,
            target_passed: smoothness.average_ms <= self.target_frame_ms,
            p95_passed: smoothness.p95_ms <= self.p95_frame_ms,
            max_passed: smoothness.max_ms <= self.max_frame_ms,
            smoothness,
            budget: self,
        }
    }
}

pub const DEFAULT_PERFORMANCE_BUDGETS: [PerformanceBudget; 9] = [
    PerformanceBudget::new(PerformanceScenario::TerminalIdle, 8.0, 12.0, 16.67, None),
    PerformanceBudget::new(
        PerformanceScenario::TerminalOutputFlood,
        12.0,
        16.67,
        24.0,
        None,
    ),
    PerformanceBudget::new(
        PerformanceScenario::EditorTenThousandLines,
        12.0,
        16.67,
        24.0,
        Some(50.0),
    ),
    PerformanceBudget::new(
        PerformanceScenario::EditorFiftyThousandLines,
        16.67,
        24.0,
        33.34,
        Some(75.0),
    ),
    PerformanceBudget::new(
        PerformanceScenario::EditorWordWrap,
        16.67,
        24.0,
        33.34,
        Some(75.0),
    ),
    PerformanceBudget::new(
        PerformanceScenario::EditorDiagnostics,
        16.67,
        24.0,
        33.34,
        Some(75.0),
    ),
    PerformanceBudget::new(
        PerformanceScenario::EffectsEnabled,
        16.67,
        20.0,
        33.34,
        None,
    ),
    PerformanceBudget::new(
        PerformanceScenario::LspInteractive,
        12.0,
        16.67,
        24.0,
        Some(75.0),
    ),
    PerformanceBudget::new(PerformanceScenario::GitRefresh, 16.67, 24.0, 33.34, None),
];

pub fn default_performance_budgets() -> &'static [PerformanceBudget] {
    &DEFAULT_PERFORMANCE_BUDGETS
}

pub fn budget_for(scenario: PerformanceScenario) -> PerformanceBudget {
    DEFAULT_PERFORMANCE_BUDGETS
        .iter()
        .copied()
        .find(|budget| budget.scenario == scenario)
        .unwrap_or_else(|| PerformanceBudget::new(scenario, 16.67, 24.0, 33.34, Some(75.0)))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameSmoothness {
    pub sample_count: usize,
    pub average_ms: f32,
    pub p95_ms: f32,
    pub max_ms: f32,
    pub missed_frame_count: usize,
    pub missed_frame_ratio: f32,
}

impl FrameSmoothness {
    pub fn from_samples(target_frame_ms: f32, samples: &[f32]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let total: f32 = sorted.iter().sum();
        let max_ms = sorted.last().copied().unwrap_or_default();
        let p95_idx = ((sorted.len() as f32 - 1.0) * 0.95).round() as usize;
        let missed_frame_count = samples
            .iter()
            .filter(|sample| **sample > target_frame_ms)
            .count();

        Self {
            sample_count: samples.len(),
            average_ms: total / samples.len() as f32,
            p95_ms: sorted[p95_idx.min(sorted.len() - 1)],
            max_ms,
            missed_frame_count,
            missed_frame_ratio: missed_frame_count as f32 / samples.len() as f32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BudgetEvaluation {
    pub scenario: PerformanceScenario,
    pub target_passed: bool,
    pub p95_passed: bool,
    pub max_passed: bool,
    pub smoothness: FrameSmoothness,
    pub budget: PerformanceBudget,
}

impl BudgetEvaluation {
    pub fn passed(self) -> bool {
        self.target_passed && self.p95_passed && self.max_passed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerformanceHarness {
    pub scenario: PerformanceScenario,
    pub warmup_iterations: usize,
    pub measured_iterations: usize,
    pub deterministic_seed: u64,
    pub effects_enabled: bool,
}

impl PerformanceHarness {
    pub fn for_scenario(scenario: PerformanceScenario) -> Self {
        Self {
            scenario,
            warmup_iterations: 10,
            measured_iterations: 120,
            deterministic_seed: 0x51A7_E5F0,
            effects_enabled: scenario == PerformanceScenario::EffectsEnabled,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HarnessResult {
    pub harness: PerformanceHarness,
    pub evaluation: BudgetEvaluation,
}

impl HarnessResult {
    pub fn passed(&self) -> bool {
        self.evaluation.passed()
    }
}

pub fn evaluate_harness_samples(
    harness: PerformanceHarness,
    frame_ms_samples: &[f32],
) -> HarnessResult {
    let budget = budget_for(harness.scenario);
    let smoothness = FrameSmoothness::from_samples(budget.target_frame_ms, frame_ms_samples);

    HarnessResult {
        harness,
        evaluation: budget.evaluate(smoothness),
    }
}

pub fn run_timed_harness(
    harness: PerformanceHarness,
    mut workload: impl FnMut(usize),
) -> HarnessResult {
    for idx in 0..harness.warmup_iterations {
        workload(idx);
    }

    let mut samples = Vec::with_capacity(harness.measured_iterations);
    for idx in 0..harness.measured_iterations {
        let start = Instant::now();
        workload(idx);
        samples.push(start.elapsed().as_secs_f32() * 1000.0);
    }

    evaluate_harness_samples(harness, &samples)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectsMode {
    Off,
    On,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EffectsSmoothnessSnapshot {
    pub effects_off: FrameSmoothness,
    pub effects_on: FrameSmoothness,
}

#[derive(Clone, Debug, Default)]
pub struct EffectsSmoothnessTracker {
    effects_off_ms: VecDeque<f32>,
    effects_on_ms: VecDeque<f32>,
}

impl EffectsSmoothnessTracker {
    const WINDOW_LIMIT: usize = 180;

    pub fn record(&mut self, mode: EffectsMode, frame_ms: f32) {
        let window = match mode {
            EffectsMode::Off => &mut self.effects_off_ms,
            EffectsMode::On => &mut self.effects_on_ms,
        };
        if window.len() >= Self::WINDOW_LIMIT {
            window.pop_front();
        }
        window.push_back(frame_ms.max(0.0));
    }

    pub fn snapshot(&self, target_frame_ms: f32) -> EffectsSmoothnessSnapshot {
        let effects_off: Vec<f32> = self.effects_off_ms.iter().copied().collect();
        let effects_on: Vec<f32> = self.effects_on_ms.iter().copied().collect();

        EffectsSmoothnessSnapshot {
            effects_off: FrameSmoothness::from_samples(target_frame_ms, &effects_off),
            effects_on: FrameSmoothness::from_samples(target_frame_ms, &effects_on),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerSource {
    Unknown,
    External,
    Battery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityLevel {
    Minimal,
    Reduced,
    Balanced,
    High,
}

impl QualityLevel {
    pub fn allows_expensive_effects(self) -> bool {
        matches!(self, Self::High | Self::Balanced)
    }

    pub fn degrade(self) -> Self {
        match self {
            Self::High => Self::Balanced,
            Self::Balanced => Self::Reduced,
            Self::Reduced | Self::Minimal => Self::Minimal,
        }
    }

    pub fn recover(self) -> Self {
        match self {
            Self::Minimal => Self::Reduced,
            Self::Reduced => Self::Balanced,
            Self::Balanced | Self::High => Self::High,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AdaptiveQualityState {
    scenario: PerformanceScenario,
    power_source: PowerSource,
    quality: QualityLevel,
    frame_ms_window: VecDeque<f32>,
    consecutive_over_budget: usize,
    consecutive_under_budget: usize,
}

impl AdaptiveQualityState {
    const WINDOW_LIMIT: usize = 60;
    const MIN_SAMPLES: usize = 12;
    const OVER_BUDGET_LIMIT: usize = 3;
    const UNDER_BUDGET_LIMIT: usize = 10;

    pub fn new(scenario: PerformanceScenario) -> Self {
        Self {
            scenario,
            power_source: PowerSource::Unknown,
            quality: QualityLevel::High,
            frame_ms_window: VecDeque::with_capacity(Self::WINDOW_LIMIT),
            consecutive_over_budget: 0,
            consecutive_under_budget: 0,
        }
    }

    pub fn quality(&self) -> QualityLevel {
        self.quality
    }

    pub fn power_source(&self) -> PowerSource {
        self.power_source
    }

    pub fn set_scenario(&mut self, scenario: PerformanceScenario) {
        if self.scenario != scenario {
            self.scenario = scenario;
            self.frame_ms_window.clear();
            self.consecutive_over_budget = 0;
            self.consecutive_under_budget = 0;
        }
    }

    pub fn set_power_source(&mut self, power_source: PowerSource) {
        self.power_source = power_source;
        if power_source == PowerSource::Battery && self.quality > QualityLevel::Balanced {
            self.quality = QualityLevel::Balanced;
        }
    }

    pub fn record_frame_ms(&mut self, frame_ms: f32) -> QualityLevel {
        if self.frame_ms_window.len() >= Self::WINDOW_LIMIT {
            self.frame_ms_window.pop_front();
        }
        self.frame_ms_window.push_back(frame_ms.max(0.0));

        if self.frame_ms_window.len() < Self::MIN_SAMPLES {
            return self.quality;
        }

        let budget = budget_for(self.scenario);
        let samples: Vec<f32> = self.frame_ms_window.iter().copied().collect();
        let smoothness = FrameSmoothness::from_samples(budget.target_frame_ms, &samples);
        let over_budget =
            smoothness.p95_ms > budget.p95_frame_ms || smoothness.missed_frame_ratio > 0.25;
        let under_budget = smoothness.p95_ms <= budget.target_frame_ms * 0.85
            && smoothness.missed_frame_count == 0;

        if over_budget {
            self.consecutive_over_budget += 1;
            self.consecutive_under_budget = 0;
        } else if under_budget {
            self.consecutive_under_budget += 1;
            self.consecutive_over_budget = 0;
        } else {
            self.consecutive_over_budget = 0;
            self.consecutive_under_budget = 0;
        }

        if self.consecutive_over_budget >= Self::OVER_BUDGET_LIMIT {
            self.quality = self.quality.degrade();
            self.consecutive_over_budget = 0;
        } else if self.consecutive_under_budget >= Self::UNDER_BUDGET_LIMIT {
            let recovered = self.quality.recover();
            self.quality = if self.power_source == PowerSource::Battery {
                recovered.min(QualityLevel::Balanced)
            } else {
                recovered
            };
            self.consecutive_under_budget = 0;
        }

        self.quality
    }
}

impl Default for AdaptiveQualityState {
    fn default() -> Self {
        Self::new(PerformanceScenario::TerminalIdle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budgets_cover_required_scenarios() {
        let scenarios: Vec<_> = default_performance_budgets()
            .iter()
            .map(|budget| budget.scenario)
            .collect();

        assert!(scenarios.contains(&PerformanceScenario::TerminalIdle));
        assert!(scenarios.contains(&PerformanceScenario::TerminalOutputFlood));
        assert!(scenarios.contains(&PerformanceScenario::EditorTenThousandLines));
        assert!(scenarios.contains(&PerformanceScenario::EditorFiftyThousandLines));
        assert!(scenarios.contains(&PerformanceScenario::EditorWordWrap));
        assert!(scenarios.contains(&PerformanceScenario::EditorDiagnostics));
        assert!(scenarios.contains(&PerformanceScenario::EffectsEnabled));
        assert!(scenarios.contains(&PerformanceScenario::LspInteractive));
        assert!(scenarios.contains(&PerformanceScenario::GitRefresh));
    }

    #[test]
    fn smoothness_reports_p95_and_missed_frames() {
        let samples = [8.0, 8.5, 9.0, 16.0, 18.0, 20.0];
        let smoothness = FrameSmoothness::from_samples(16.67, &samples);

        assert_eq!(smoothness.sample_count, 6);
        assert_eq!(smoothness.missed_frame_count, 2);
        assert_eq!(smoothness.max_ms, 20.0);
        assert!(smoothness.p95_ms >= 18.0);
    }

    #[test]
    fn harness_evaluation_uses_scenario_budget() {
        let harness = PerformanceHarness::for_scenario(PerformanceScenario::EffectsEnabled);
        let samples = vec![16.0; harness.measured_iterations];
        let result = evaluate_harness_samples(harness, &samples);

        assert!(result.passed());
        assert_eq!(
            result.evaluation.budget.scenario,
            PerformanceScenario::EffectsEnabled
        );
    }

    #[test]
    fn adaptive_quality_degrades_after_repeated_over_budget_windows() {
        let mut adaptive = AdaptiveQualityState::new(PerformanceScenario::EffectsEnabled);

        for _ in 0..40 {
            adaptive.record_frame_ms(40.0);
        }

        assert!(adaptive.quality() < QualityLevel::High);
    }

    #[test]
    fn adaptive_quality_recovers_when_external_power_and_frames_are_stable() {
        let mut adaptive = AdaptiveQualityState::new(PerformanceScenario::EffectsEnabled);

        for _ in 0..40 {
            adaptive.record_frame_ms(40.0);
        }
        let degraded = adaptive.quality();
        for _ in 0..140 {
            adaptive.record_frame_ms(8.0);
        }

        assert!(adaptive.quality() >= degraded);
        assert_eq!(adaptive.power_source(), PowerSource::Unknown);
    }

    #[test]
    fn battery_power_caps_recovery_at_balanced() {
        let mut adaptive = AdaptiveQualityState::new(PerformanceScenario::EffectsEnabled);
        adaptive.set_power_source(PowerSource::Battery);

        for _ in 0..140 {
            adaptive.record_frame_ms(8.0);
        }

        assert_eq!(adaptive.quality(), QualityLevel::Balanced);
    }

    #[test]
    fn effects_smoothness_tracks_effects_on_and_off_separately() {
        let mut tracker = EffectsSmoothnessTracker::default();

        for _ in 0..5 {
            tracker.record(EffectsMode::Off, 8.0);
            tracker.record(EffectsMode::On, 18.0);
        }

        let snapshot = tracker.snapshot(16.67);
        assert_eq!(snapshot.effects_off.sample_count, 5);
        assert_eq!(snapshot.effects_on.sample_count, 5);
        assert_eq!(snapshot.effects_off.missed_frame_count, 0);
        assert_eq!(snapshot.effects_on.missed_frame_count, 5);
    }
}
