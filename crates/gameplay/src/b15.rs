use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

pub const MONEY_SCALE: i128 = 100_000_000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Choice {
    W,
    E,
    Decline,
}

const CHOICES: [Choice; 3] = [Choice::W, Choice::E, Choice::Decline];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GuardrailAggregation {
    SumOfThreeCounts,
    UnionOfMonths,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Metrics {
    pub attainment_month: Option<u32>,
    pub guardrail_months: [u32; 3],
    pub guardrail_union_months: u32,
    /// Caller supplies M24 treasury minus all unpaid future E obligations once.
    pub treasury_less_unpaid_future_e: i128,
    pub final_three_recurring: [i128; 3],
}

impl Metrics {
    pub fn recurring_sum_for_display(&self) -> Result<i128, ComparisonError> {
        self.final_three_recurring
            .iter()
            .try_fold(0_i128, |sum, value| sum.checked_add(*value))
            .ok_or(ComparisonError::Overflow)
    }

    fn guardrail_total(&self, aggregation: GuardrailAggregation) -> u64 {
        match aggregation {
            GuardrailAggregation::SumOfThreeCounts => {
                self.guardrail_months.iter().map(|x| u64::from(*x)).sum()
            }
            GuardrailAggregation::UnionOfMonths => u64::from(self.guardrail_union_months),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptOutcome {
    pub id: String,
    pub choice: Choice,
    pub metrics: Metrics,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToleranceStatus {
    PendingUserConfirmation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tolerances {
    pub status: ToleranceStatus,
    pub attainment_months: u32,
    pub guardrail_months: u32,
    pub treasury_minor_units: i128,
    pub recurring_minor_units: i128,
}

impl Tolerances {
    pub fn proposed(
        attainment_months: u32,
        guardrail_months: u32,
        treasury_minor_units: i128,
        recurring_minor_units: i128,
    ) -> Result<Self, ComparisonError> {
        let tolerances = Self {
            status: ToleranceStatus::PendingUserConfirmation,
            attainment_months,
            guardrail_months,
            treasury_minor_units,
            recurring_minor_units,
        };
        tolerances.validate()?;
        Ok(tolerances)
    }

    fn validate(&self) -> Result<(), ComparisonError> {
        if self.treasury_minor_units < 0 || self.recurring_minor_units < 0 {
            return Err(ComparisonError::NegativeTolerance);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Frontier {
    pub choice: Choice,
    pub script_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DominanceWitness {
    pub choice: Choice,
    pub script_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comparison {
    pub aggregation: GuardrailAggregation,
    pub tolerances: Tolerances,
    pub frontiers: Vec<Frontier>,
    pub witnesses: Vec<DominanceWitness>,
    pub all_equivalent: bool,
    /// Result for supplied scripts under proposed tolerances, not user acceptance.
    pub pass: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComparisonError {
    MissingChoice(Choice),
    DuplicateScriptId,
    NegativeTolerance,
    InconsistentGuardrailCounts,
    Overflow,
}

/// Compares only supplied scripts in the caller's disclosed bounded search.
/// A witness is weakly better than every script of both other choices, and
/// for each other choice is strictly better in some metric than some script.
/// Frontiers use exact Pareto dominance because tolerance dominance can cycle.
/// Choices tie when each pair of exact frontiers covers each other within the
/// tolerance bands. The three recurring months are three separate dimensions.
pub fn compare(
    scripts: &[ScriptOutcome],
    tolerances: &Tolerances,
    aggregation: GuardrailAggregation,
) -> Result<Comparison, ComparisonError> {
    tolerances.validate()?;
    let mut ids = BTreeSet::new();
    for script in scripts {
        if !ids.insert(&script.id) {
            return Err(ComparisonError::DuplicateScriptId);
        }
        let counts = script.metrics.guardrail_months;
        let union = u64::from(script.metrics.guardrail_union_months);
        if union < u64::from(*counts.iter().max().unwrap())
            || union > counts.iter().map(|x| u64::from(*x)).sum()
        {
            return Err(ComparisonError::InconsistentGuardrailCounts);
        }
    }
    let exact = Tolerances::proposed(0, 0, 0, 0)?;
    let mut fronts: Vec<Vec<&ScriptOutcome>> = Vec::new();
    for choice in CHOICES {
        let candidates: Vec<_> = scripts.iter().filter(|s| s.choice == choice).collect();
        if candidates.is_empty() {
            return Err(ComparisonError::MissingChoice(choice));
        }
        fronts.push(
            candidates
                .iter()
                .copied()
                .filter(|candidate| {
                    !candidates.iter().any(|other| {
                        let relation = dimensions(other, candidate, &exact, aggregation);
                        weakly_better(&relation) && strictly_better(&relation)
                    })
                })
                .collect(),
        );
    }
    let witnesses = scripts
        .iter()
        .filter(|candidate| {
            CHOICES
                .iter()
                .filter(|c| **c != candidate.choice)
                .all(|choice| {
                    let other: Vec<_> = scripts.iter().filter(|s| s.choice == *choice).collect();
                    other
                        .iter()
                        .all(|s| weakly_better(&dimensions(candidate, s, tolerances, aggregation)))
                        && other.iter().any(|s| {
                            strictly_better(&dimensions(candidate, s, tolerances, aggregation))
                        })
                })
        })
        .map(|s| DominanceWitness {
            choice: s.choice,
            script_id: s.id.clone(),
        })
        .collect::<Vec<_>>();
    let covers = |a: &[&ScriptOutcome], b: &[&ScriptOutcome]| {
        a.iter().all(|x| {
            b.iter().any(|y| {
                dimensions(x, y, tolerances, aggregation)
                    .iter()
                    .all(|r| *r == Ordering::Equal)
            })
        })
    };
    let all_equivalent = (0..3).all(|i| {
        (i + 1..3).all(|j| covers(&fronts[i], &fronts[j]) && covers(&fronts[j], &fronts[i]))
    });
    let frontiers = CHOICES
        .iter()
        .zip(fronts)
        .map(|(choice, scripts)| Frontier {
            choice: *choice,
            script_ids: scripts.iter().map(|s| s.id.clone()).collect(),
        })
        .collect();
    Ok(Comparison {
        aggregation,
        tolerances: tolerances.clone(),
        frontiers,
        pass: witnesses.is_empty() && !all_equivalent,
        witnesses,
        all_equivalent,
    })
}

fn weakly_better(dimensions: &[Ordering; 6]) -> bool {
    dimensions.iter().all(|x| *x != Ordering::Less)
}

fn strictly_better(dimensions: &[Ordering; 6]) -> bool {
    dimensions.contains(&Ordering::Greater)
}

fn higher(a: i128, b: i128, tolerance: i128) -> Ordering {
    if a.abs_diff(b) <= tolerance as u128 {
        Ordering::Equal
    } else {
        a.cmp(&b)
    }
}

fn dimensions(
    a: &ScriptOutcome,
    b: &ScriptOutcome,
    tolerance: &Tolerances,
    aggregation: GuardrailAggregation,
) -> [Ordering; 6] {
    let a = &a.metrics;
    let b = &b.metrics;
    let attainment = match (a.attainment_month, b.attainment_month) {
        (Some(a), Some(b)) => higher(
            i128::from(b),
            i128::from(a),
            i128::from(tolerance.attainment_months),
        ),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    };
    [
        attainment,
        higher(
            i128::from(b.guardrail_total(aggregation)),
            i128::from(a.guardrail_total(aggregation)),
            i128::from(tolerance.guardrail_months),
        ),
        higher(
            a.treasury_less_unpaid_future_e,
            b.treasury_less_unpaid_future_e,
            tolerance.treasury_minor_units,
        ),
        higher(
            a.final_three_recurring[0],
            b.final_three_recurring[0],
            tolerance.recurring_minor_units,
        ),
        higher(
            a.final_three_recurring[1],
            b.final_three_recurring[1],
            tolerance.recurring_minor_units,
        ),
        higher(
            a.final_three_recurring[2],
            b.final_three_recurring[2],
            tolerance.recurring_minor_units,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(id: &str, choice: Choice, month: Option<u32>, money: i128) -> ScriptOutcome {
        ScriptOutcome {
            id: id.into(),
            choice,
            metrics: Metrics {
                attainment_month: month,
                guardrail_months: [0; 3],
                guardrail_union_months: 0,
                treasury_less_unpaid_future_e: money,
                final_three_recurring: [0; 3],
            },
        }
    }

    fn exact(scripts: &[ScriptOutcome]) -> Comparison {
        compare(
            scripts,
            &Tolerances::proposed(0, 0, 0, 0).unwrap(),
            GuardrailAggregation::SumOfThreeCounts,
        )
        .unwrap()
    }

    #[test]
    fn strong_dominance_and_tradeoffs() {
        let mut scripts = vec![
            script("w", Choice::W, Some(16), 100),
            script("e", Choice::E, Some(17), 90),
            script("d", Choice::Decline, None, 80),
        ];
        let result = exact(&scripts);
        assert_eq!(
            result.witnesses,
            vec![DominanceWitness {
                choice: Choice::W,
                script_id: "w".into()
            }]
        );
        assert!(!result.pass);
        scripts[1].metrics.treasury_less_unpaid_future_e = 110;
        scripts[2].metrics.treasury_less_unpaid_future_e = 120;
        let result = exact(&scripts);
        assert!(result.witnesses.is_empty());
        assert!(result.pass);
    }

    #[test]
    fn equal_frontiers_detect_ties_despite_different_dominated_scripts() {
        let mut scripts = vec![];
        for (i, choice) in CHOICES.into_iter().enumerate() {
            scripts.push(script(&format!("best{i}"), choice, Some(18), 100));
            scripts.push(script(&format!("bad{i}"), choice, None, i as i128));
        }
        let result = exact(&scripts);
        assert!(result.all_equivalent);
        assert!(!result.pass);
        assert!(result.frontiers.iter().all(|f| f.script_ids.len() == 1));
        // The literal D023 quantifier can find a witness against dominated points
        // even when frontier tie detection independently rejects the experiment.
        assert_eq!(result.witnesses.len(), 3);
    }

    #[test]
    fn identical_outcome_sets_are_completely_tied() {
        let mut scripts = vec![];
        for (i, choice) in CHOICES.into_iter().enumerate() {
            scripts.push(script(&format!("fast{i}"), choice, Some(15), 10));
            scripts.push(script(&format!("rich{i}"), choice, Some(20), 100));
        }
        let result = exact(&scripts);
        assert!(result.all_equivalent);
        assert!(result.witnesses.is_empty());
        assert!(!result.pass);
    }

    #[test]
    fn tolerance_boundaries_negative_balances_and_nonattainment() {
        let tolerance = Tolerances::proposed(1, 1, 1_000_000, 1_000_000).unwrap();
        assert_eq!(MONEY_SCALE / 100, tolerance.treasury_minor_units);
        let a = script("a", Choice::W, Some(20), -1_000_000);
        let b = script("b", Choice::E, Some(21), -2_000_000);
        assert!(
            dimensions(&a, &b, &tolerance, GuardrailAggregation::UnionOfMonths)
                .iter()
                .all(|x| *x == Ordering::Equal)
        );
        let c = script("c", Choice::Decline, Some(22), -2_000_001);
        let comparison = dimensions(&a, &c, &tolerance, GuardrailAggregation::UnionOfMonths);
        assert_eq!(comparison[0], Ordering::Greater);
        assert_eq!(comparison[2], Ordering::Greater);
        let never = script("n", Choice::Decline, None, 0);
        let huge = Tolerances::proposed(u32::MAX, 0, 0, 0).unwrap();
        assert_eq!(
            dimensions(&a, &never, &huge, GuardrailAggregation::UnionOfMonths)[0],
            Ordering::Greater
        );
        assert!(Tolerances::proposed(0, 0, -1, 0).is_err());
        assert_eq!(higher(i128::MIN, i128::MAX, i128::MAX), Ordering::Less);
    }

    #[test]
    fn recurring_comparison_keeps_each_month_and_sum_is_display_only() {
        let mut a = script("a", Choice::W, Some(20), 0);
        let mut b = script("b", Choice::E, Some(20), 0);
        a.metrics.final_three_recurring = [-10, 20, 20];
        b.metrics.final_three_recurring = [0, 0, 0];
        assert_eq!(a.metrics.recurring_sum_for_display().unwrap(), 30);
        let result = dimensions(
            &a,
            &b,
            &Tolerances::proposed(0, 0, 0, 0).unwrap(),
            GuardrailAggregation::SumOfThreeCounts,
        );
        assert!(!weakly_better(&result));
        assert!(strictly_better(&result));
        a.metrics.final_three_recurring = [i128::MAX, 1, 0];
        assert_eq!(
            a.metrics.recurring_sum_for_display(),
            Err(ComparisonError::Overflow)
        );
    }

    #[test]
    fn full_precision_outcomes_roundtrip_without_money_conversion() {
        let mut outcome = script("precision", Choice::E, None, i128::MAX);
        outcome.metrics.final_three_recurring = [i128::MIN, -1, i128::MAX];
        let encoded = serde_json::to_string(&outcome).unwrap();
        let restored: ScriptOutcome = serde_json::from_str(&encoded).unwrap();
        assert_eq!(outcome, restored);
    }

    #[test]
    fn dominance_requires_comparison_to_every_opposing_script() {
        let scripts = vec![
            script("w", Choice::W, Some(15), 100),
            script("e1", Choice::E, Some(16), 90),
            script("e2", Choice::E, Some(14), 90),
            script("d", Choice::Decline, Some(17), 120),
        ];
        assert!(exact(&scripts).witnesses.is_empty());
    }

    #[test]
    fn guardrail_aggregation_can_change_result_and_missing_choices_fail() {
        let mut scripts = vec![
            script("w", Choice::W, Some(20), 100),
            script("e", Choice::E, Some(20), 100),
            script("d", Choice::Decline, Some(20), 100),
        ];
        scripts[0].metrics.guardrail_months = [2, 2, 2];
        scripts[0].metrics.guardrail_union_months = 2;
        for s in &mut scripts[1..] {
            s.metrics.guardrail_months = [1, 1, 1];
            s.metrics.guardrail_union_months = 3;
        }
        let tolerance = Tolerances::proposed(0, 0, 0, 0).unwrap();
        let sum = compare(&scripts, &tolerance, GuardrailAggregation::SumOfThreeCounts).unwrap();
        let union = compare(&scripts, &tolerance, GuardrailAggregation::UnionOfMonths).unwrap();
        assert!(sum.witnesses.iter().all(|s| s.choice != Choice::W));
        assert_eq!(union.witnesses[0].choice, Choice::W);
        assert_eq!(
            compare(
                &scripts[..2],
                &tolerance,
                GuardrailAggregation::UnionOfMonths
            ),
            Err(ComparisonError::MissingChoice(Choice::Decline))
        );
    }
}
