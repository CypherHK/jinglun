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

fn public_project(game: &mut Game, site: u32, suffix: &str, kind: PublicWork) {
    let id = format!("independent-{site}-{suffix}");
    let edge = if kind == PublicWork::Road {
        game.map()["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|edge| {
                let id = edge["id"].as_u64()? as u32;
                game.governance()
                    .unwrap()
                    .public_work_quote(game, site, kind, Some(id))
                    .ok()
                    .map(|(cost, _)| (cost, id))
            })
            .min()
            .map(|(_, id)| id)
    } else {
        None
    };
    game.queue_governance(
        id.clone(),
        GovernanceCommand::BuildPublic {
            id,
            site,
            kind,
            road_edge: edge,
        },
    )
    .unwrap();
}
fn early_public_conditions(site: u32) -> Game {
    let mut game = fixture();
    game.queue_governance("admit-candidate", GovernanceCommand::OpenRegion { site })
        .unwrap();
    public_project(&mut game, site, "housing", PublicWork::Housing);
    public_project(&mut game, site, "clinic", PublicWork::Clinic);
    game.end_month(1).unwrap();
    public_project(&mut game, site, "road", PublicWork::Road);
    public_project(&mut game, site, "water", PublicWork::Water);
    advance(&mut game, 3);
    public_project(&mut game, site, "power", PublicWork::Power);
    game
}

#[test]
fn early_subsidy_alone_changes_cash_gap_into_next_month_investment_intent() {
    let mut waiting = early_public_conditions(8);
    let card = waiting.scorecard(Group::B, 8).unwrap();
    assert!(card.gaps["cash"] > 0.0);
    assert!(card
        .gaps
        .iter()
        .all(|(name, gap)| name == "cash" || *gap == 0.0));
    let mut subsidized = waiting.clone();
    subsidized
        .queue_governance(
            "only-difference-subsidy",
            GovernanceCommand::SetSubsidy {
                site: 8,
                cap: card.cash_gap,
            },
        )
        .unwrap();
    assert!(subsidized.scorecard(Group::B, 8).unwrap().conditional_ready);
    waiting.end_month(4).unwrap();
    subsidized.end_month(4).unwrap();
    assert!(!waiting
        .governance()
        .unwrap()
        .intents
        .values()
        .any(|i| i.site == 8 && i.group == Group::B));
    assert!(subsidized
        .governance()
        .unwrap()
        .intents
        .values()
        .any(|i| i.site == 8 && i.group == Group::B && i.created_month == 4));
}

#[test]
fn investment_intent_expires_at_original_deadline_and_renewal_expires_at_twelve() {
    let mut game = early_public_conditions(8);
    let cap = game.scorecard(Group::B, 8).unwrap().cash_gap;
    game.queue_governance("expiry-cap", GovernanceCommand::SetSubsidy { site: 8, cap })
        .unwrap();
    game.end_month(4).unwrap();
    let intent = game
        .governance()
        .unwrap()
        .intents
        .values()
        .find(|i| i.site == 8 && i.group == Group::B)
        .unwrap()
        .clone();
    assert_eq!(intent.due_month, 5);
    game.end_month(5).unwrap();
    let s = game.governance().unwrap();
    assert!(!s.intents.contains_key(&intent.id));
    assert!(s.todos[&intent.id].resolved);
    assert_eq!(s.todos[&intent.id].due_month, 5);
    assert!(
        s.facts
            .iter()
            .any(|f| f.effect == "investment_intent_expired_rejected"
                && f.causes.contains(&intent.id))
    );
    advance(&mut game, 9);
    for month in 10..=12 {
        let todo = &game.governance().unwrap().todos["aid/renewal"];
        assert_eq!((todo.created_month, todo.due_month), (9, 12));
        assert!(!todo.resolved);
        game.end_month(month).unwrap();
    }
    assert!(game.governance().unwrap().todos["aid/renewal"].resolved);
    assert_eq!(game.governance().unwrap().renewal, None);
    let before = game.save().unwrap();
    assert!(game
        .queue_governance(
            "too-late",
            GovernanceCommand::RenewAid {
                choice: RenewalChoice::WaterPriority
            }
        )
        .is_err());
    assert_eq!(game.save().unwrap(), before);
    game.end_month(13).unwrap();
    assert_eq!(game.governance().unwrap().last_aid, 0);
}

#[test]
fn breached_promise_ignores_new_work_for_six_months_but_recognizes_completed_capacity() {
    let mut game = early_public_conditions(8);
    let cap = game.scorecard(Group::B, 8).unwrap().cash_gap;
    game.queue_governance(
        "promise-cap",
        GovernanceCommand::SetSubsidy { site: 8, cap },
    )
    .unwrap();
    game.end_month(4).unwrap();
    let intent = game
        .governance()
        .unwrap()
        .intents
        .values()
        .find(|i| i.site == 8 && i.group == Group::B)
        .unwrap()
        .clone();
    game.queue_governance(
        "accept",
        GovernanceCommand::AcceptIntent {
            id: intent.id.clone(),
            deadline: 7,
        },
    )
    .unwrap();
    game.queue_governance(
        "delay-power",
        GovernanceCommand::PauseProject {
            id: "independent-8-power".into(),
            paused: true,
        },
    )
    .unwrap();
    advance(&mut game, 7);
    let s = game.governance().unwrap();
    assert_eq!(s.commitments[&intent.id].status, CommitmentStatus::Breached);
    assert_eq!(s.distrust_until.get(&Group::B), Some(&13));
    assert!(!s.distrust_until.contains_key(&Group::A));
    assert!(s.facts.iter().any(|f| f.month == 7
        && f.effect == "conditional_promise_breached_six_month_group_distrust"
        && f.causes.contains(&intent.id)));
    game.queue_governance(
        "resume-power",
        GovernanceCommand::PauseProject {
            id: "independent-8-power".into(),
            paused: false,
        },
    )
    .unwrap();
    let ignored = game.scorecard(Group::B, 8).unwrap();
    let recognized = game.scorecard(Group::A, 8).unwrap();
    assert!(ignored.promised_projects.is_empty());
    assert!(ignored.gaps["power"] > 0.0);
    assert!(recognized
        .promised_projects
        .contains(&"independent-8-power".into()));
    assert_eq!(recognized.gaps["power"], 0.0);
    advance(&mut game, 9);
    assert!(game.governance().unwrap().projects["independent-8-power"].active);
    assert_eq!(game.scorecard(Group::B, 8).unwrap().gaps["power"], 0.0);
    advance(&mut game, 12);
    public_project(&mut game, 26, "deadline-water", PublicWork::Water);
    assert_eq!(game.month(), 13);
    assert!(game
        .scorecard(Group::B, 26)
        .unwrap()
        .promised_projects
        .is_empty());
    assert!(!game
        .scorecard(Group::A, 26)
        .unwrap()
        .promised_projects
        .is_empty());
    game.end_month(13).unwrap();
    assert_eq!(game.month(), 14);
    assert!(!game
        .scorecard(Group::B, 26)
        .unwrap()
        .promised_projects
        .is_empty());
}

#[test]
fn hidden_fourth_intent_keeps_deadline_and_defaults_despite_three_visible_limit() {
    let mut game = early_public_conditions(8);
    for site in [39, 8] {
        game.queue_governance(
            format!("hidden-subsidy-{site}"),
            GovernanceCommand::SetSubsidy {
                site,
                cap: money(40.0).unwrap(),
            },
        )
        .unwrap();
    }
    game.end_month(4).unwrap();
    let s = game.governance().unwrap();
    let intents: Vec<_> = s.intents.values().cloned().collect();
    assert_eq!(intents.len(), 4);
    let visible = s.visible_todos(game.month());
    assert_eq!(visible.len(), 3);
    let hidden = intents
        .iter()
        .find(|i| !visible.iter().any(|t| t.id == i.id))
        .unwrap()
        .clone();
    assert_eq!(hidden.due_month, 5);
    game.end_month(5).unwrap();
    let s = game.governance().unwrap();
    assert!(!s.intents.contains_key(&hidden.id));
    assert_eq!(s.todos[&hidden.id].due_month, 5);
    assert!(s.todos[&hidden.id].resolved);
    assert!(s.facts.iter().any(|f| f.month == 5
        && f.effect == "investment_intent_expired_rejected"
        && f.causes.contains(&hidden.id)));
}

#[test]
fn two_candidate_subsidies_cannot_both_commit_under_shared_budget_and_capacity() {
    let mut game = fixture();
    for site in [8, 29] {
        game.queue_governance(
            format!("dual-open-{site}"),
            GovernanceCommand::OpenRegion { site },
        )
        .unwrap();
    }
    public_project(&mut game, 8, "dual-house", PublicWork::Housing);
    public_project(&mut game, 8, "dual-clinic", PublicWork::Clinic);
    game.end_month(1).unwrap();
    public_project(&mut game, 8, "dual-road", PublicWork::Road);
    public_project(&mut game, 8, "dual-water", PublicWork::Water);
    advance(&mut game, 3);
    public_project(&mut game, 8, "dual-power", PublicWork::Power);
    public_project(&mut game, 29, "dual-house", PublicWork::Housing);
    game.end_month(4).unwrap();
    public_project(&mut game, 29, "dual-clinic", PublicWork::Clinic);
    game.end_month(5).unwrap();
    public_project(&mut game, 29, "dual-road", PublicWork::Road);
    game.end_month(6).unwrap();
    public_project(&mut game, 29, "dual-water", PublicWork::Water);
    public_project(&mut game, 29, "dual-power", PublicWork::Power);
    for (group, site) in [(Group::A, 8), (Group::B, 29)] {
        let card = game.scorecard(group, site).unwrap();
        game.queue_governance(
            format!("dual-cap-{site}"),
            GovernanceCommand::SetSubsidy {
                site,
                cap: card.cash_gap,
            },
        )
        .unwrap();
    }
    game.end_month(7).unwrap();
    let intents: Vec<_> = [(Group::B, 29), (Group::A, 8)]
        .iter()
        .map(|(group, site)| {
            game.governance()
                .unwrap()
                .intents
                .values()
                .find(|i| i.group == *group && i.site == *site)
                .unwrap()
                .clone()
        })
        .collect();
    game.queue_governance(
        "lock-first",
        GovernanceCommand::AcceptIntent {
            id: intents[0].id.clone(),
            deadline: 12,
        },
    )
    .unwrap();
    let first_locked = game.save().unwrap();
    let result = game.queue_governance(
        "lock-second",
        GovernanceCommand::AcceptIntent {
            id: intents[1].id.clone(),
            deadline: 12,
        },
    );
    println!("B6 actual second acceptance result: {result:?}");
    assert!(
        matches!(result, Err(Error::Insufficient(_))),
        "second subsidy result: {result:?}"
    );
    assert_eq!(game.save().unwrap(), first_locked);
    game.end_month(8).unwrap();
    let state = game.governance().unwrap();
    let commitment = &state.commitments[&intents[0].id];
    assert!(commitment.subsidy > 0);
    assert!(
        game.escrow_remaining(commitment.escrow.as_ref().unwrap())
            .unwrap()
            > 0
    );
    assert!(!state.commitments.contains_key(&intents[1].id));
}
