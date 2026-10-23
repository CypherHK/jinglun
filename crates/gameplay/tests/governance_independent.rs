//! Independent public-API regressions for D009, D022, D023 and B14.
//! No state injection: every accepted state comes from commands and month ends.
use jinglun_gameplay::*;
use serde_json::Value;
use std::sync::OnceLock;

fn fixture() -> Game {
    static GAME: OnceLock<Game> = OnceLock::new();
    GAME.get_or_init(|| Game::confirmed_game().unwrap()).clone()
}
fn advance(game: &mut Game, through: u32) {
    while game.month() <= through {
        game.end_month(game.month()).unwrap();
    }
}
fn snapshot(game: &Game) -> Value {
    serde_json::from_slice(&game.save().unwrap()).unwrap()
}

#[test]
fn monthly_plan_preserves_live_ledger_and_failed_dependency_withdrawal_is_atomic() {
    let mut game = fixture();
    let before = snapshot(&game);
    game.queue_governance(
        "clinic",
        GovernanceCommand::BuildPublic {
            id: "clinic-test".into(),
            site: 29,
            kind: PublicWork::Clinic,
            road_edge: None,
        },
    )
    .unwrap();
    game.queue_governance(
        "pause",
        GovernanceCommand::PauseProject {
            id: "clinic-test".into(),
            paused: true,
        },
    )
    .unwrap();
    let planned = game.save().unwrap();
    let after = snapshot(&game);
    for key in [
        "ledger",
        "reservations",
        "system_receipts",
        "month",
        "pending",
        "available",
    ] {
        assert_eq!(
            before["payload"][key], after["payload"][key],
            "live {key} changed before month end"
        );
    }
    for key in ["sites", "projects", "budgets", "decision_log", "todos"] {
        assert_eq!(
            before["payload"]["governance"][key], after["payload"]["governance"][key],
            "live governance {key} changed before month end"
        );
    }
    assert!(game.withdraw_governance("clinic").is_err());
    assert_eq!(game.save().unwrap(), planned);
    assert!(game
        .queue_governance("clinic", GovernanceCommand::OpenRegion { site: 29 })
        .is_err());
    assert_eq!(game.save().unwrap(), planned);
    game.withdraw_governance("pause").unwrap();
    game.withdraw_governance("clinic").unwrap();
    assert_eq!(snapshot(&game), before);
}

#[test]
fn forecast_matches_actual_month_end_and_planned_save_roundtrips_without_side_effects() {
    let mut game = fixture();
    let base_card = game.scorecard(Group::A, 8).unwrap();
    let base_forecast = game.treasury_forecast().unwrap();
    let initial = game.save().unwrap();
    game.queue_governance(
        "subsidy",
        GovernanceCommand::SetSubsidy {
            site: 8,
            cap: money(40.0).unwrap(),
        },
    )
    .unwrap();
    game.queue_governance(
        "clinic",
        GovernanceCommand::BuildPublic {
            id: "forecast-clinic".into(),
            site: 29,
            kind: PublicWork::Clinic,
            road_edge: None,
        },
    )
    .unwrap();
    let planned = game.save().unwrap();
    assert!(game.scorecard(Group::A, 8).unwrap().gaps["cash"] < base_card.gaps["cash"]);
    let forecast = game.treasury_forecast().unwrap();
    assert_ne!(forecast, base_forecast);
    assert_eq!(
        game.save().unwrap(),
        planned,
        "preview must not change any serialized state"
    );
    let mut replay = Game::load(&planned).unwrap();
    assert_eq!(replay.save().unwrap(), planned);
    assert_eq!(replay.treasury_forecast().unwrap(), forecast);
    for point in forecast {
        replay.end_month(point.month).unwrap();
        let state = replay.governance().unwrap();
        assert_eq!(
            replay.balance("treasury", &Asset::Money),
            Some(point.treasury)
        );
        assert_eq!(state.last_recurring_balance, point.recurring_balance);
        assert_eq!(state.last_aid, point.aid);
    }
    game.withdraw_governance("clinic").unwrap();
    game.withdraw_governance("subsidy").unwrap();
    assert_eq!(game.save().unwrap(), initial);
    assert_eq!(game.treasury_forecast().unwrap(), base_forecast);
    assert_eq!(game.scorecard(Group::A, 8).unwrap(), base_card);
}

#[test]
fn passive_enterprise_self_funds_capital_without_intent_or_government_subsidy() {
    let mut game = fixture();
    assert_eq!(
        game.balance("treasury", &Asset::Money),
        Some(money(55.0).unwrap())
    );
    let s = game.governance().unwrap();
    assert!(s.sites[&39].admitted && s.sites[&74].admitted);
    assert!(!s.sites[&8].admitted && !s.sites[&29].admitted);
    advance(&mut game, 8);
    assert!(game.governance().unwrap().factories.is_empty());
    game.end_month(9).unwrap();
    let s = game.governance().unwrap();
    assert_eq!(s.factories.len(), 1);
    let f = s.factories.values().next().unwrap();
    assert_eq!((f.group, f.site, f.started_month), (Group::B, 39, 9));
    assert_eq!(f.subsidy, 0);
    assert!(f.subsidy_escrow.is_none());
    assert!(s.intents.is_empty() && s.commitments.is_empty());
    assert!(s.todos.values().all(|t| t.kind != TodoKind::Intent));
    advance(&mut game, 12);
    let f = game
        .governance()
        .unwrap()
        .factories
        .values()
        .next()
        .unwrap();
    assert_eq!(f.phase, FactoryPhase::Operating);
    assert_eq!(f.output, 100.0);
    assert_eq!(f.operating_subsidy, 0);
}

#[test]
fn payroll_withholding_is_inside_gross_wages_not_an_extra_employer_charge() {
    let mut game = fixture();
    advance(&mut game, 11);
    let opening = game.balance(Group::B.account(), &Asset::Money).unwrap();
    game.end_month(12).unwrap();
    let closing = game.balance(Group::B.account(), &Asset::Money).unwrap();
    let s = game.governance().unwrap();
    let f = s.factories.values().find(|f| f.group == Group::B).unwrap();
    assert!(f.after_tax_profit > 0);
    // D022 retained income 2.475; retention share .5. Payroll tax is
    // withheld from gross wage, per confirmed factory_profit_arithmetic.
    let retained = money(currency(f.after_tax_profit as u64) * 0.5).unwrap();
    assert_eq!(closing - opening, money(2.475).unwrap() + retained);
}

#[test]
fn energy_remaining_obligation_is_gross_24_month_contract_for_early_and_late_signature() {
    for sign_month in [9, 12] {
        let mut game = fixture();
        advance(&mut game, sign_month - 1);
        game.queue_governance(
            "long-energy",
            GovernanceCommand::RenewAid {
                choice: RenewalChoice::EnergyLongTerm,
            },
        )
        .unwrap();
        advance(&mut game, 24);
        let report = game.governance().unwrap().report.as_ref().unwrap();
        let cash = game.balance("treasury", &Asset::Money).unwrap() as i128;
        // M9..M32 inclusive: after M24 eight gross payments remain.
        // M12..M35 inclusive: eleven remain. Do not net expected resale.
        let remaining = sign_month - 1;
        let payable = game.payable("resource_energy_long_term").unwrap();
        let gross_monthly = money(20.0 * 0.17).unwrap();
        assert_eq!(payable.total_committed, gross_monthly * 24);
        assert_eq!(payable.paid + payable.unpaid, payable.total_committed);
        assert_eq!(
            payable.unpaid,
            gross_monthly * remaining as u64 + payable.arrears
        );
        let resumed = Game::load(&game.save().unwrap()).unwrap();
        assert_eq!(
            resumed.payable("resource_energy_long_term").unwrap(),
            payable
        );
        assert_eq!(
            cash - report.treasury_net_energy_liability,
            money(20.0 * 0.17).unwrap() as i128 * remaining as i128
        );
    }
}

#[test]
fn report_keeps_historical_guard_failures_but_fiscal_recovery_is_separate() {
    let mut game = fixture();
    game.queue_governance("early-cut", GovernanceCommand::SetSecurity { monthly: 0 })
        .unwrap();
    advance(&mut game, 4);
    game.queue_governance(
        "restore",
        GovernanceCommand::SetSecurity {
            monthly: money(6.0).unwrap(),
        },
    )
    .unwrap();
    advance(&mut game, 24);
    let s = game.governance().unwrap();
    let report = s.report.as_ref().unwrap();
    assert!(!report.security_failed_months.is_empty());
    assert!(s.history.iter().rev().take(3).all(|r| r.security_ok));
    assert_eq!(report.grade, GoalGrade::NotAchieved);
    assert_eq!(
        report.fiscal_recovered,
        s.history
            .iter()
            .rev()
            .take(3)
            .all(|r| r.recurring_balance >= 0)
    );
    assert!(!report
        .failures
        .iter()
        .any(|r| r == "last_three_month_livelihood_or_security_guardrail_failed"));
}

fn loss_warning_game(restore_in_warning_month: bool) -> Game {
    let calibration = GovernanceCalibration {
        labor_protection_wage_multiplier: 2.0,
        ..GovernanceCalibration::default()
    };
    let mut game = Game::confirmed_game_with_calibration(calibration).unwrap();
    advance(&mut game, 11);
    game.queue_governance(
        "wage-policy",
        GovernanceCommand::SetInstitution {
            kind: Institution::LaborProtection,
            enabled: true,
        },
    )
    .unwrap();
    for month in 12..=18 {
        if month == 18 && restore_in_warning_month {
            game.queue_governance(
                "restore-costs",
                GovernanceCommand::SetInstitution {
                    kind: Institution::LaborProtection,
                    enabled: false,
                },
            )
            .unwrap();
        }
        game.end_month(month).unwrap();
        let f = game
            .governance()
            .unwrap()
            .factories
            .values()
            .find(|f| f.group == Group::B)
            .unwrap();
        assert_eq!(f.loss_months, month.saturating_sub(12));
        if (16..=18).contains(&month) {
            assert!(
                f.output > 0.0 && f.output < 100.0,
                "reduction must lower output while the wage loss cause remains: {}",
                f.output
            );
        }
        assert_eq!(
            f.phase,
            match month {
                12..=14 => FactoryPhase::Operating,
                15..=17 => FactoryPhase::Reduced,
                _ => FactoryPhase::ClosureWarned,
            }
        );
    }
    let saved = game.save().unwrap();
    let resumed = Game::load(&saved).unwrap();
    assert_eq!(resumed.save().unwrap(), saved);
    resumed
}

#[test]
fn consecutive_loss_reduction_warning_and_next_month_closure_are_distinct() {
    let mut game = loss_warning_game(false);
    let f = game
        .governance()
        .unwrap()
        .factories
        .values()
        .find(|f| f.group == Group::B)
        .unwrap();
    assert_eq!(f.close_month, Some(19));
    let closure = game
        .governance()
        .unwrap()
        .todos
        .values()
        .find(|t| t.kind == TodoKind::Closure)
        .unwrap();
    assert_eq!(closure.due_month, 19);
    game.end_month(19).unwrap();
    let f = game
        .governance()
        .unwrap()
        .factories
        .values()
        .find(|f| f.group == Group::B)
        .unwrap();
    assert_eq!(f.phase, FactoryPhase::Closed);
}

#[test]
fn warning_month_removal_of_loss_cause_and_full_orders_can_rescue_factory() {
    let mut game = loss_warning_game(true);
    game.end_month(19).unwrap();
    let f = game
        .governance()
        .unwrap()
        .factories
        .values()
        .find(|f| f.group == Group::B)
        .unwrap();
    // The only loss cause was the temporary wage policy. The original full
    // orders and original maritime energy can support the factory again.
    assert_eq!(f.phase, FactoryPhase::Operating);
    assert_eq!(f.loss_months, 0);
    assert!(f.after_tax_profit > 0);
    assert_eq!(f.close_month, None);
}

#[test]
fn load_rejects_forged_business_facts_even_with_valid_checksum() {
    let mut game = fixture();
    game.queue_governance("open-inland", GovernanceCommand::OpenRegion { site: 29 })
        .unwrap();
    advance(&mut game, 12);
    assert!(game.governance().unwrap().sites[&29].admitted);
    let original = snapshot(&game);
    for pointer in [
        "/payload/governance/sites/39/admitted",
        "/payload/governance/sites/29/admitted",
        "/payload/governance/history/0/security_ok",
        "/payload/governance/factories/factory~11/operating_subsidy",
    ] {
        let mut forged = original.clone();
        // Mutate only fields that actually exist; never accept a synthetic
        // initial state or use a forged save to drive gameplay.
        let value = forged.pointer_mut(pointer).unwrap();
        *value = match value {
            Value::Bool(v) => Value::Bool(!*v),
            Value::Number(_) => Value::from(1),
            _ => panic!("unexpected test fixture field"),
        };
        let bytes = serde_json::to_vec(&forged["payload"]).unwrap();
        let hash = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
        });
        forged["checksum"] = Value::String(format!("fnv1a64:{hash:016x}"));
        assert!(
            Game::load(&serde_json::to_vec(&forged).unwrap()).is_err(),
            "accepted forged {pointer}"
        );
    }
    assert_eq!(snapshot(&game), original);
}

#[test]
fn institution_announcement_waits_one_month_and_cooldown_rejections_are_atomic() {
    let mut game = fixture();
    game.queue_governance(
        "announce",
        GovernanceCommand::SetInstitution {
            kind: Institution::LaborProtection,
            enabled: true,
        },
    )
    .unwrap();
    game.end_month(1).unwrap();
    assert!(!game.governance().unwrap().labor_protection);
    for month in [2, 3] {
        let before = game.save().unwrap();
        assert!(game
            .queue_governance(
                format!("too-early-{month}"),
                GovernanceCommand::SetInstitution {
                    kind: Institution::LaborProtection,
                    enabled: false,
                }
            )
            .is_err());
        assert_eq!(game.save().unwrap(), before);
        game.end_month(month).unwrap();
        assert!(game.governance().unwrap().labor_protection);
    }
    game.queue_governance(
        "announce-end",
        GovernanceCommand::SetInstitution {
            kind: Institution::LaborProtection,
            enabled: false,
        },
    )
    .unwrap();
    game.end_month(4).unwrap();
    assert!(game.governance().unwrap().labor_protection);
    game.end_month(5).unwrap();
    assert!(!game.governance().unwrap().labor_protection);
}
