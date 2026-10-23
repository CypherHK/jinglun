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
    assert!(
        game.queue_governance("clinic", GovernanceCommand::OpenRegion { site: 29 })
            .is_err()
    );
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
    assert_eq!(f.domestic_sales, 100.0);
    assert_eq!(f.export_sales, 0.0);
    println!("D028 passive capital first domestic sales: M12 site39 domestic=100 export=0");
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
    assert!(
        !report
            .failures
            .iter()
            .any(|r| r == "last_three_month_livelihood_or_security_guardrail_failed")
    );
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
        assert!(
            game.queue_governance(
                format!("too-early-{month}"),
                GovernanceCommand::SetInstitution {
                    kind: Institution::LaborProtection,
                    enabled: false,
                }
            )
            .is_err()
        );
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
    early_public_conditions_from(fixture(), site)
}
fn early_public_conditions_from(mut game: Game, site: u32) -> Game {
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
    assert!(
        card.gaps
            .iter()
            .all(|(name, gap)| name == "cash" || *gap == 0.0)
    );
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
    println!(
        "B5 default M4 site8 cash_gap={} subsidy={} other_gaps={:?}",
        currency(card.cash_gap),
        currency(card.cash_gap),
        card.gaps
    );
    assert!(subsidized.scorecard(Group::B, 8).unwrap().conditional_ready);
    waiting.end_month(4).unwrap();
    subsidized.end_month(4).unwrap();
    assert!(
        !waiting
            .governance()
            .unwrap()
            .intents
            .values()
            .any(|i| i.site == 8 && i.group == Group::B)
    );
    assert!(
        subsidized
            .governance()
            .unwrap()
            .intents
            .values()
            .any(|i| i.site == 8 && i.group == Group::B && i.created_month == 4)
    );
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
    assert!(
        game.queue_governance(
            "too-late",
            GovernanceCommand::RenewAid {
                choice: RenewalChoice::WaterPriority
            }
        )
        .is_err()
    );
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
    assert!(
        recognized
            .promised_projects
            .contains(&"independent-8-power".into())
    );
    assert_eq!(recognized.gaps["power"], 0.0);
    advance(&mut game, 9);
    assert!(game.governance().unwrap().projects["independent-8-power"].active);
    assert_eq!(game.scorecard(Group::B, 8).unwrap().gaps["power"], 0.0);
    advance(&mut game, 12);
    public_project(&mut game, 26, "deadline-water", PublicWork::Water);
    assert_eq!(game.month(), 13);
    assert!(
        game.scorecard(Group::B, 26)
            .unwrap()
            .promised_projects
            .is_empty()
    );
    assert!(
        !game
            .scorecard(Group::A, 26)
            .unwrap()
            .promised_projects
            .is_empty()
    );
    game.end_month(13).unwrap();
    assert_eq!(game.month(), 14);
    assert!(
        !game
            .scorecard(Group::B, 26)
            .unwrap()
            .promised_projects
            .is_empty()
    );
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

#[test]
fn passive_report_names_the_missing_industrial_scale_and_fiscal_recovery() {
    let mut game = fixture();
    advance(&mut game, 24);
    let report = game.governance().unwrap().report.as_ref().unwrap();
    assert_eq!(report.grade, GoalGrade::NotAchieved);
    assert!(!report.fiscal_recovered);
    for reason in [
        "selected_inland_output_never_reached_second_rank_and_capital_ratio",
        "last_three_month_scale_and_unsubsidized_profit_not_sustained",
        "fiscal_recovery_separate_last_three_recurring_negative",
    ] {
        assert!(
            report.failures.iter().any(|failure| failure == reason),
            "report omitted reason {reason}"
        );
    }
    assert!(report.causes.iter().any(
        |cause| cause.effect == "transition_aid_expires_month_13" && cause.deadline == Some(12)
    ));
    assert!(game.balance("treasury", &Asset::Money).unwrap() > 0);
}

#[test]
fn same_inland_route_one_factory_pays_more_energy_and_earns_less_with_long_contract() {
    let mut common = early_public_conditions(29);
    public_project(&mut common, 29, "paired-vocational", PublicWork::Vocational);
    common
        .queue_governance("route-one-ordinary", GovernanceCommand::SignEnergy)
        .unwrap();
    common
        .queue_governance(
            "route-one-subsidy",
            GovernanceCommand::SetSubsidy {
                site: 29,
                cap: common.scorecard(Group::B, 29).unwrap().cash_gap,
            },
        )
        .unwrap();
    common.end_month(4).unwrap();
    let intent = common
        .governance()
        .unwrap()
        .intents
        .values()
        .find(|intent| intent.group == Group::B && intent.site == 29)
        .unwrap()
        .clone();
    common
        .queue_governance(
            "route-one-accept",
            GovernanceCommand::AcceptIntent {
                id: intent.id,
                deadline: 24,
            },
        )
        .unwrap();
    advance(&mut common, 8);
    let mut energy = common.clone();
    let mut decline = common;
    for (game, choice) in [
        (&mut energy, RenewalChoice::EnergyLongTerm),
        (&mut decline, RenewalChoice::Decline),
    ] {
        game.queue_governance(
            "only-difference-renewal",
            GovernanceCommand::RenewAid { choice },
        )
        .unwrap();
        advance(game, 14);
    }
    let matched = |game: &Game| {
        game.governance()
            .unwrap()
            .factories
            .values()
            .find(|factory| factory.group == Group::B && factory.site == 29)
            .unwrap()
            .clone()
    };
    let ef = matched(&energy);
    let df = matched(&decline);
    assert_eq!(ef.id, df.id);
    assert_eq!(ef.started_month, df.started_month);
    assert_eq!(ef.activate_month, df.activate_month);
    assert_eq!(ef.phase, FactoryPhase::Operating);
    assert_eq!(df.phase, FactoryPhase::Operating);
    assert_eq!(ef.output, 100.0);
    assert_eq!(ef.output, df.output);
    assert_eq!(ef.workers, df.workers);
    assert_eq!(ef.operating_subsidy, 0);
    assert_eq!(df.operating_subsidy, 0);
    // Confirmed D022 price ladder; ordinary purchases are prepaid, so compare
    // the energy consumed by this matched factory rather than purchase timing.
    let confirmed: Value =
        serde_json::from_str(include_str!("../data/jinglun-040-initial-draft.json")).unwrap();
    let price = |name: &str| confirmed["parameters"][name]["value"].as_f64().unwrap();
    let quantity = ef.output / price("new_factory_output") * price("new_factory_energy");
    let energy_cost = money(quantity * price("long_energy_price")).unwrap();
    let decline_cost = money(
        df.ordinary_energy_used * price("ordinary_energy_price")
            + df.maritime_energy_used * price("maritime_energy_price"),
    )
    .unwrap();
    assert_eq!(ef.ordinary_energy_used, 0.0);
    assert_eq!(ef.maritime_energy_used, 0.0);
    assert!(energy_cost > decline_cost);
    assert!(ef.after_tax_profit < df.after_tax_profit);
    assert_eq!(
        df.after_tax_profit - ef.after_tax_profit,
        (energy_cost - decline_cost) as i128
    );
    println!(
        "B10 paired M14 site29 output={} E energy={} profit={} Decline energy={} profit={}",
        ef.output,
        currency(energy_cost),
        currency(ef.after_tax_profit as u64),
        currency(decline_cost),
        currency(df.after_tax_profit as u64)
    );
}

fn default_cooperation_water_counterfactual(choice: RenewalChoice) -> Game {
    let mut opening = fixture();
    opening
        .queue_governance(
            "default-education",
            GovernanceCommand::SetServiceBudget {
                kind: Service::Education,
                monthly: 0,
            },
        )
        .unwrap();
    opening
        .queue_governance(
            "default-target",
            GovernanceCommand::SelectCandidate { site: 29 },
        )
        .unwrap();
    opening
        .queue_governance("default-cooperation", GovernanceCommand::SignCooperation)
        .unwrap();
    let mut game = early_public_conditions_from(opening, 29);
    let mut accepted = std::collections::BTreeSet::new();
    for month in 4..=24 {
        game.queue_governance(
            format!("default-cap-{month}"),
            GovernanceCommand::SetSubsidy {
                site: 29,
                cap: money(40.0).unwrap(),
            },
        )
        .unwrap();
        if month == 9 {
            game.queue_governance("default-renewal", GovernanceCommand::RenewAid { choice })
                .unwrap();
        }
        let intents: Vec<_> = game
            .governance()
            .unwrap()
            .intents
            .values()
            .filter(|i| i.site == 29 && !accepted.contains(&i.group))
            .cloned()
            .collect();
        for i in intents {
            if game
                .queue_governance(
                    format!("default-accept-{:?}-{month}", i.group),
                    GovernanceCommand::AcceptIntent {
                        id: i.id,
                        deadline: 24,
                    },
                )
                .is_ok()
            {
                accepted.insert(i.group);
            }
        }
        game.end_month(month).unwrap();
    }
    game
}

#[test]
fn default_calibration_report_traces_water_against_identical_cooperation_plan() {
    let water = default_cooperation_water_counterfactual(RenewalChoice::WaterPriority);
    let decline = default_cooperation_water_counterfactual(RenewalChoice::Decline);
    let ws = water.governance().unwrap();
    let ds = decline.governance().unwrap();
    let operating = |s: &GovernanceState| {
        s.factories
            .values()
            .filter(|f| f.site == 29 && f.phase == FactoryPhase::Operating)
            .count()
    };
    println!(
        "B13 default W={} Decline={} grades={:?}/{:?}",
        operating(ws),
        operating(ds),
        ws.report.as_ref().unwrap().grade,
        ds.report.as_ref().unwrap().grade
    );
    assert_eq!(operating(ws), 1);
    assert_eq!(operating(ds), 2);
    assert_ne!(ws.report.as_ref().unwrap().grade, GoalGrade::Achieved);
    assert_eq!(ds.report.as_ref().unwrap().grade, GoalGrade::Achieved);
    assert!(
        ws.report
            .as_ref()
            .unwrap()
            .causes
            .iter()
            .any(|f| f.month == 9
                && f.effect == "water_priority_limits_next_factory"
                && f.causes.contains(&"aid/renewal".into()))
    );
}

/// D028's expected capital-plus-two-candidate topology needs three plants.
/// This is a frozen-input feasibility diagnostic, not a passing B8 scenario.
#[test]
fn default_labor_pool_diagnostic_cannot_staff_three_standard_new_factories() {
    let game = fixture();
    let state = game.governance().unwrap();
    let idle: u32 = state.sites.values().map(|s| s.idle_workers).sum();
    let confirmed: Value =
        serde_json::from_str(include_str!("../data/jinglun-040-initial-draft.json")).unwrap();
    let workers = confirmed["parameters"]["new_factory_workers"]["value"]
        .as_u64()
        .unwrap() as u32;
    assert_eq!(idle, 295);
    assert_eq!(workers, 100);
    assert!(idle < workers * 3);
    println!(
        "D028 frozen-labor diagnostic: idle={idle}, workers_per_factory={workers}, simultaneous_new_factory_upper_bound={}",
        idle / workers
    );
}

/// Bounded D028 public-command search: 2 sites x 3 preparation months x
/// 4 security-cut months, with otherwise identical cooperation restart pairs.
/// Diagnostic only; it checks the domestic cap, not B8 acceptance. Months
/// 25..32 must never count as the 24-month slice passing.
#[test]
fn d028_default_order_rescue_bounded_diagnostic() {
    labor_trial_order_rescue_domain(false);
}

#[test]
fn labor_trial_on_order_rescue_bounded_diagnostic() {
    labor_trial_order_rescue_domain(true);
}

fn labor_trial_order_rescue_domain(enabled: bool) {
    let calibration = GovernanceCalibration::default();
    assert_eq!(calibration.initial_skilled_idle_share, 0.25);
    assert_eq!(calibration.water_priority_fraction, 0.92);
    assert_eq!(calibration.cooperation_export_orders_per_month, 100);
    for site in [8, 29] {
        for cap_month in [1, 6, 10] {
            for cut_month in [8, 11, 14, 17] {
                let mut opening =
                    Game::confirmed_game_with_labor_trial(calibration.clone(), enabled).unwrap();
                opening
                    .queue_governance(
                        "diagnostic-education",
                        GovernanceCommand::SetServiceBudget {
                            kind: Service::Education,
                            monthly: money(1.0).unwrap(),
                        },
                    )
                    .unwrap();
                opening
                    .queue_governance("diagnostic-cooperation", GovernanceCommand::SignCooperation)
                    .unwrap();
                opening
                    .queue_governance(
                        "diagnostic-target",
                        GovernanceCommand::SelectCandidate { site },
                    )
                    .unwrap();
                let mut game = opening;
                let mut accepted = std::collections::BTreeSet::new();
                let mut recovering: Option<Game> = None;
                let mut rescue_accepted = std::collections::BTreeSet::new();
                let mut warning = None;
                let mut resumed = false;
                for month in 1..=32 {
                    if month == cap_month {
                        game.queue_governance(
                            "diagnostic-admit",
                            GovernanceCommand::OpenRegion { site },
                        )
                        .unwrap();
                        public_project(&mut game, site, "housing", PublicWork::Housing);
                        public_project(&mut game, site, "clinic", PublicWork::Clinic);
                    }
                    if month == cap_month + 1 {
                        public_project(&mut game, site, "road", PublicWork::Road);
                        public_project(&mut game, site, "water", PublicWork::Water);
                    }
                    if month == cap_month + 3 {
                        public_project(&mut game, site, "power", PublicWork::Power);
                        public_project(&mut game, site, "vocational", PublicWork::Vocational);
                    }
                    if month >= cap_month + 3 {
                        game.queue_governance(
                            format!("diagnostic-cap-{month}"),
                            GovernanceCommand::SetSubsidy {
                                site,
                                cap: money(40.0).unwrap(),
                            },
                        )
                        .unwrap();
                    }
                    if month == 9 {
                        game.queue_governance(
                            "diagnostic-renewal",
                            GovernanceCommand::RenewAid {
                                choice: RenewalChoice::Decline,
                            },
                        )
                        .unwrap();
                    }
                    if month == cut_month {
                        game.queue_governance(
                            "diagnostic-cut",
                            GovernanceCommand::SetSecurity { monthly: 0 },
                        )
                        .unwrap();
                    }
                    let intents: Vec<_> = game
                        .governance()
                        .unwrap()
                        .intents
                        .values()
                        .filter(|i| i.site == site && !accepted.contains(&i.group))
                        .cloned()
                        .collect();
                    for intent in intents {
                        if game
                            .queue_governance(
                                format!("diagnostic-accept-{:?}-{month}", intent.group),
                                GovernanceCommand::AcceptIntent {
                                    id: intent.id,
                                    deadline: 24,
                                },
                            )
                            .is_ok()
                        {
                            accepted.insert(intent.group);
                        }
                    }
                    // Restore safety once it falls below 49. This still permits
                    // actual suspension, while avoiding unnecessary extra damage.
                    // Existing contract qualification is not accelerated.
                    if month > cut_month && game.governance().unwrap().security < 49.0 {
                        game.queue_governance(
                            format!("diagnostic-safety-{month}"),
                            GovernanceCommand::SetSecurity {
                                monthly: money(6.0).unwrap(),
                            },
                        )
                        .unwrap();
                    }
                    if let Some(rescue) = recovering.as_mut() {
                        // Keep the same subsidy and acceptance policy in both
                        // branches; signing cooperation is the only difference.
                        if month >= cap_month + 3 {
                            rescue
                                .queue_governance(
                                    format!("diagnostic-cap-{month}"),
                                    GovernanceCommand::SetSubsidy {
                                        site,
                                        cap: money(40.0).unwrap(),
                                    },
                                )
                                .unwrap();
                        }
                        let intents: Vec<_> = rescue
                            .governance()
                            .unwrap()
                            .intents
                            .values()
                            .filter(|i| i.site == site && !rescue_accepted.contains(&i.group))
                            .cloned()
                            .collect();
                        for intent in intents {
                            if rescue
                                .queue_governance(
                                    format!("diagnostic-accept-{:?}-{month}", intent.group),
                                    GovernanceCommand::AcceptIntent {
                                        id: intent.id,
                                        deadline: 24,
                                    },
                                )
                                .is_ok()
                            {
                                rescue_accepted.insert(intent.group);
                            }
                        }
                        if !resumed && rescue.governance().unwrap().security >= 50.0 {
                            rescue
                                .queue_governance(
                                    "diagnostic-resign",
                                    GovernanceCommand::SignCooperation,
                                )
                                .unwrap();
                            println!(
                                "B8 trial={enabled} re-sign site={site} prepare={cap_month} cut={cut_month} M{month}"
                            );
                            resumed = true;
                        }
                        rescue.end_month(month).unwrap();
                    }
                    game.end_month(month).unwrap();
                    let state = game.governance().unwrap();
                    if recovering.is_none()
                        && state.cooperation_start.is_none()
                        && month > cut_month
                    {
                        println!(
                            "B8 trial={enabled} suspended site={site} prepare={cap_month} cut={cut_month} M{month}"
                        );
                        rescue_accepted = accepted.clone();
                        recovering = Some(game.clone());
                    }
                    if warning.is_none()
                        && state
                            .factories
                            .values()
                            .any(|f| f.site == site && f.phase == FactoryPhase::ClosureWarned)
                    {
                        warning = Some(month);
                    }
                    let labor: u32 = state
                        .sites
                        .values()
                        .map(|s| s.idle_workers + s.teachers)
                        .sum::<u32>()
                        + state
                            .factories
                            .values()
                            .filter(|f| f.phase != FactoryPhase::Closed)
                            .map(|f| f.workers)
                            .sum::<u32>();
                    let recruited: u32 = state
                        .labor_trial_sources
                        .values()
                        .map(|s| s.cumulative_employed_recruits)
                        .sum();
                    assert_eq!(labor, 295 + recruited);
                    verify_trial_labor(&game);
                    let domestic: f64 = state.factories.values().map(|f| f.domestic_sales).sum();
                    assert!(domestic <= 200.0 + 1e-9);
                    if month == 24 {
                        let bytes = game.save().unwrap();
                        let saved: Value = serde_json::from_slice(&bytes).unwrap();
                        let applied = &saved["payload"]["applied"];
                        let gross_existing: u64 = [Group::A, Group::B]
                            .iter()
                            .map(|group| {
                                applied[format!("existing/wages/{group:?}/24")]["command"]
                                    ["Transfer"]["amount"]
                                    .as_u64()
                                    .unwrap()
                            })
                            .sum();
                        let remaining_wages = game
                            .system_receipts()
                            .iter()
                            .rev()
                            .find(|r| r.reason == "existing_nonmanufacturing_employer_wages")
                            .unwrap()
                            .delta as u64;
                        let actual_tax =
                            applied["existing/payroll/24"]["command"]["Transfer"]["amount"]
                                .as_u64()
                                .unwrap();
                        assert_eq!(
                            actual_tax,
                            money(currency(remaining_wages) * 0.01).unwrap()
                                + money(currency(gross_existing) * 0.01).unwrap()
                        );
                        let loaded = Game::load(&bytes).unwrap();
                        assert_eq!(loaded.save().unwrap(), bytes);
                        let mut replay = loaded;
                        let mut original = game.clone();
                        replay.end_month(25).unwrap();
                        original.end_month(25).unwrap();
                        assert_eq!(replay.save().unwrap(), original.save().unwrap());
                        println!(
                            "B8 trial={enabled} sources site={site} prepare={cap_month} cut={cut_month} {:?}",
                            state.labor_trial_sources
                        );
                        println!(
                            "B8 trial={enabled} M24 scorecards site={site} prepare={cap_month} cut={cut_month} treasury={:?} A={:?} B={:?}",
                            game.balance("treasury", &Asset::Money),
                            game.scorecard(Group::A, site).unwrap().gaps,
                            game.scorecard(Group::B, site).unwrap().gaps
                        );
                    }
                    if warning.is_some() || month >= 18 {
                        let fs: Vec<_> = state
                            .factories
                            .values()
                            .map(|f| {
                                (
                                    f.site,
                                    f.id.as_str(),
                                    f.first_operated_month,
                                    f.domestic_sales,
                                    f.export_sales,
                                    f.output,
                                    f.after_tax_profit,
                                    f.loss_months,
                                    f.phase,
                                )
                            })
                            .collect();
                        let rescue_fs = recovering.as_ref().map(|r| {
                            r.governance()
                                .unwrap()
                                .factories
                                .values()
                                .map(|f| {
                                    (f.site, f.output, f.after_tax_profit, f.loss_months, f.phase)
                                })
                                .collect::<Vec<_>>()
                        });
                        println!(
                            "B8 trial={enabled} diagnostic site={site} cap={cap_month} cut={cut_month} M{month} security={} export={} warning={warning:?} resumed={resumed} factories={fs:?} rescue={rescue_fs:?}",
                            state.security, state.export_orders
                        );
                    }
                }
            }
        }
    }
}

/// A bounded public-command counterfactual, not a claim that B8 order rescue passes.
/// Both new factories are retrofitted; loss of cooperation removes export orders
/// while domestic import substitution remains the D022-confirmed 200 units.
#[test]
fn default_double_retrofit_cooperation_order_loss_remains_profitable_diagnostic() {
    double_retrofit_trial_diagnostic(false);
}

#[test]
fn labor_trial_on_double_retrofit_diagnostic() {
    double_retrofit_trial_diagnostic(true);
}

fn double_retrofit_trial_diagnostic(enabled: bool) {
    let mut opening =
        Game::confirmed_game_with_labor_trial(GovernanceCalibration::default(), enabled).unwrap();
    opening
        .queue_governance(
            "order-test-budget",
            GovernanceCommand::SetServiceBudget {
                kind: Service::Education,
                monthly: 0,
            },
        )
        .unwrap();
    opening
        .queue_governance(
            "order-test-target",
            GovernanceCommand::SelectCandidate { site: 29 },
        )
        .unwrap();
    opening
        .queue_governance("order-test-cooperation", GovernanceCommand::SignCooperation)
        .unwrap();
    let mut common = early_public_conditions_from(opening, 29);
    common
        .queue_governance(
            "order-test-cap",
            GovernanceCommand::SetSubsidy {
                site: 29,
                cap: money(40.0).unwrap(),
            },
        )
        .unwrap();
    let mut accepted = std::collections::BTreeSet::new();
    for month in 4..=17 {
        if month == 9 {
            common
                .queue_governance(
                    "order-test-refresh",
                    GovernanceCommand::SetSubsidy {
                        site: 29,
                        cap: money(40.0).unwrap(),
                    },
                )
                .unwrap();
            common
                .queue_governance(
                    "order-test-decline",
                    GovernanceCommand::RenewAid {
                        choice: RenewalChoice::Decline,
                    },
                )
                .unwrap();
        }
        let intents: Vec<_> = common
            .governance()
            .unwrap()
            .intents
            .values()
            .cloned()
            .collect();
        for intent in intents {
            if intent.site == 29
                && !accepted.contains(&intent.group)
                && (intent.group == Group::B || month >= 10)
            {
                common
                    .queue_governance(
                        format!("order-test-accept-{:?}", intent.group),
                        GovernanceCommand::AcceptIntent {
                            id: intent.id,
                            deadline: 24,
                        },
                    )
                    .unwrap();
                accepted.insert(intent.group);
            }
        }
        if month == 13 {
            common
                .queue_governance(
                    "order-test-labor",
                    GovernanceCommand::SetInstitution {
                        kind: Institution::LaborProtection,
                        enabled: true,
                    },
                )
                .unwrap();
        }
        common.end_month(month).unwrap();
    }
    let factories: Vec<_> = common
        .governance()
        .unwrap()
        .factories
        .values()
        .filter(|f| f.site == 29 && f.phase == FactoryPhase::Operating)
        .map(|f| f.id.clone())
        .collect();
    println!(
        "B8 retrofit trial={enabled} candidate operating count={}",
        factories.len()
    );
    if !enabled {
        assert_eq!(factories.len(), 2);
    }
    for id in factories {
        common
            .queue_governance(
                format!("order-test-retrofit-{id}"),
                GovernanceCommand::Retrofit { factory: id },
            )
            .unwrap();
    }
    public_project(&mut common, 29, "order-test-extra-road", PublicWork::Road);
    public_project(&mut common, 29, "order-test-extra-power", PublicWork::Power);
    let mut maintained = common.clone();
    let mut suspended = common;
    suspended
        .queue_governance(
            "order-test-security-cut",
            GovernanceCommand::SetSecurity { monthly: 0 },
        )
        .unwrap();
    let mut observed_order_loss_months = 0;
    for month in 18..=29 {
        let orders_at_opening = suspended.governance().unwrap().export_orders;
        maintained.end_month(month).unwrap();
        suspended.end_month(month).unwrap();
        verify_trial_labor(&suspended);
        let state = suspended.governance().unwrap();
        let kept = maintained.governance().unwrap();
        let lost_output: f64 = state
            .factories
            .values()
            .filter(|f| f.site == 29)
            .map(|f| f.output)
            .sum();
        let lost_exports: f64 = state
            .factories
            .values()
            .filter(|f| f.site == 29)
            .map(|f| f.export_sales)
            .sum();
        let kept_output: f64 = kept
            .factories
            .values()
            .filter(|f| f.site == 29)
            .map(|f| f.output)
            .sum();
        let kept_exports: f64 = kept
            .factories
            .values()
            .filter(|f| f.site == 29)
            .map(|f| f.export_sales)
            .sum();
        println!(
            "B8 trial={enabled} default retrofit diagnostic M{month}: maintained output={kept_output} exports={kept_exports}; suspended output={lost_output} exports={lost_exports}; months_after_24_are_diagnostic_only={}",
            month > 24
        );
        if state.cooperation_start.is_none() && orders_at_opening == 0.0 {
            observed_order_loss_months += 1;
            let factories: Vec<_> = state.factories.values().filter(|f| f.site == 29).collect();
            let output: f64 = factories.iter().map(|f| f.output).sum();
            let domestic: f64 = factories.iter().map(|f| f.domestic_sales).sum();
            let exports: f64 = factories.iter().map(|f| f.export_sales).sum();
            if !enabled {
                assert_eq!(domestic, 200.0);
                assert_eq!(exports, 0.0);
                assert_eq!(output, 200.0);
                assert!(factories.iter().all(|f| f.after_tax_profit > 0
                    && f.loss_months == 0
                    && f.phase == FactoryPhase::Operating));
            }
            println!(
                "B8 trial={enabled} bounded order-loss M{month}: output={output}, factories={:?}",
                factories
                    .iter()
                    .map(|f| (
                        f.id.as_str(),
                        f.output,
                        f.domestic_sales,
                        f.export_sales,
                        currency(f.after_tax_profit as u64)
                    ))
                    .collect::<Vec<_>>()
            );
        }
    }
    let kept = maintained.governance().unwrap();
    let lost = suspended.governance().unwrap();
    assert!(kept.export_orders > 0.0);
    assert_eq!(lost.export_orders, 0.0);
    assert!(
        lost.facts
            .iter()
            .any(|f| f.effect == "cooperation_suspended")
    );
    let kept_output: f64 = kept
        .factories
        .values()
        .filter(|f| f.site == 29)
        .map(|f| f.output)
        .sum();
    let lost_output: f64 = lost
        .factories
        .values()
        .filter(|f| f.site == 29)
        .map(|f| f.output)
        .sum();
    println!(
        "B8 trial={enabled} bounded counterfactual: maintained output={kept_output}, suspended output={lost_output}, suspension_month={:?}, final_export_orders={}/{}",
        lost.facts
            .iter()
            .find(|f| f.effect == "cooperation_suspended")
            .map(|f| f.month),
        kept.export_orders,
        lost.export_orders
    );
    assert!(observed_order_loss_months >= 6);
    if !enabled {
        assert_eq!(kept_output, 240.0);
        assert_eq!(
            kept.factories
                .values()
                .filter(|f| f.site == 29)
                .map(|f| f.export_sales)
                .sum::<f64>(),
            40.0
        );
        assert!(kept_output > lost_output);
        assert_eq!(lost_output, 200.0);
        assert!(
            lost.factories
                .values()
                .all(|f| f.phase != FactoryPhase::ClosureWarned)
        );
    }
}

/// Explicit historical skill-share fixture isolates D025 water behavior. This
/// is not evidence that D026 default calibration builds two factories by M12.
fn historical_skill_share_two_operating_factories_before_late_water(fraction: f64) -> Game {
    let calibration = GovernanceCalibration {
        initial_skilled_idle_share: 1.0,
        water_priority_fraction: fraction,
        ..GovernanceCalibration::default()
    };
    let opening = Game::confirmed_game_with_calibration(calibration).unwrap();
    let mut game = early_public_conditions_from(opening, 29);
    game.queue_governance(
        "late-water-cap",
        GovernanceCommand::SetSubsidy {
            site: 29,
            cap: money(40.0).unwrap(),
        },
    )
    .unwrap();
    game.end_month(4).unwrap();
    let first = game
        .governance()
        .unwrap()
        .intents
        .values()
        .find(|i| i.group == Group::B && i.site == 29)
        .unwrap()
        .id
        .clone();
    game.queue_governance(
        "late-water-first",
        GovernanceCommand::AcceptIntent {
            id: first,
            deadline: 11,
        },
    )
    .unwrap();
    game.end_month(5).unwrap();
    game.queue_governance(
        "late-water-refresh",
        GovernanceCommand::SetSubsidy {
            site: 29,
            cap: money(40.0).unwrap(),
        },
    )
    .unwrap();
    game.end_month(6).unwrap();
    let second = game
        .governance()
        .unwrap()
        .intents
        .values()
        .find(|i| i.group == Group::A && i.site == 29)
        .unwrap()
        .id
        .clone();
    game.queue_governance(
        "late-water-second",
        GovernanceCommand::AcceptIntent {
            id: second,
            deadline: 11,
        },
    )
    .unwrap();
    advance(&mut game, 11);
    let factories: Vec<_> = game
        .governance()
        .unwrap()
        .factories
        .values()
        .filter(|f| f.site == 29)
        .collect();
    assert_eq!(factories.len(), 2);
    assert!(
        factories
            .iter()
            .all(|f| f.phase == FactoryPhase::Operating && f.output == 100.0)
    );
    assert_ne!(
        factories[0].first_operated_month,
        factories[1].first_operated_month
    );
    game
}

#[test]
fn historical_skill_fixture_late_water_suspends_new_factories_but_protects_initial_industry() {
    // Explicit .94 pressure fixture; the pending default may retain one factory.
    let mut game = historical_skill_share_two_operating_factories_before_late_water(0.94);
    let ids: Vec<_> = game
        .governance()
        .unwrap()
        .factories
        .values()
        .filter(|f| f.site == 29)
        .map(|f| f.id.clone())
        .collect();
    let initial: Vec<_> = ["initial-industry/39", "initial-industry/74"]
        .iter()
        .map(|id| ((*id).to_string(), game.reservations()[*id].clone()))
        .collect();
    let opening_cash = std::collections::BTreeMap::from([
        (Group::A, game.balance("group_a", &Asset::Money).unwrap()),
        (Group::B, game.balance("group_b", &Asset::Money).unwrap()),
    ]);
    game.queue_governance(
        "late-water-renew",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    game.end_month(12).unwrap();
    for id in &ids {
        let factory = &game.governance().unwrap().factories[id];
        assert!(factory.water_suspended);
        assert_eq!(factory.output, 0.0);
        assert_eq!(factory.loss_months, 1);
        let retained = money(if factory.group == Group::A {
            2.2
        } else {
            2.475
        })
        .unwrap();
        assert_eq!(
            game.balance(factory.group.account(), &Asset::Money)
                .unwrap(),
            opening_cash[&factory.group] + retained - money(5.6).unwrap()
        );
        assert_eq!(
            factory.after_tax_profit,
            -(money(2.0 + 3.6).unwrap() as i128)
        );
        assert_eq!(
            game.reservations()[id].phase,
            AllocationPhase::OperationalWaitingForWater
        );
    }
    for (id, reservation) in initial {
        assert_eq!(game.reservations()[&id], reservation);
    }
    let history = game.governance().unwrap().history.last().unwrap();
    assert!(history.output_by_site[&39] >= 300.0 && history.output_by_site[&74] >= 125.0);
    advance(&mut game, 17);
    for id in &ids {
        let factory = &game.governance().unwrap().factories[id];
        assert_eq!(factory.phase, FactoryPhase::ClosureWarned);
        assert_eq!(factory.loss_months, 6);
    }
    game.end_month(18).unwrap();
    assert!(
        ids.iter()
            .all(|id| game.governance().unwrap().factories[id].phase == FactoryPhase::Closed)
    );
}

#[test]
fn historical_skill_and_pending_water_fixture_suspends_later_operated_factory_first() {
    let total = fixture()
        .cell(29)
        .unwrap()
        .water_sources
        .values()
        .map(|(_, slots)| *slots)
        .max()
        .unwrap();
    assert!(total > 2);
    // Pending technical fixture leaves exactly two slots: the earlier housing
    // and the earlier factory. It does not propose a production balance value.
    let fraction = (total as f64 - 2.25) / total as f64;
    let mut game = historical_skill_share_two_operating_factories_before_late_water(fraction);
    let mut before: Vec<_> = game
        .governance()
        .unwrap()
        .factories
        .values()
        .filter(|f| f.site == 29)
        .cloned()
        .collect();
    before.sort_by_key(|f| (f.first_operated_month, f.id.clone()));
    assert!(before[0].first_operated_month < before[1].first_operated_month);
    let source = game.reservations()[&before[0].id].source;
    assert_eq!(game.reservations()[&before[1].id].source, source);
    game.queue_governance(
        "ordered-water-renew",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    game.end_month(12).unwrap();
    let state = game.governance().unwrap();
    let earlier = &state.factories[&before[0].id];
    let later = &state.factories[&before[1].id];
    assert!(!earlier.water_suspended);
    assert!(earlier.output > 0.0);
    assert!(later.water_suspended);
    assert_eq!(later.output, 0.0);
    assert_eq!(earlier.first_operated_month, before[0].first_operated_month);
    assert_eq!(later.first_operated_month, before[1].first_operated_month);
    println!(
        "D025 pending technical fraction={fraction}, source={source}, total={total}, first={:?}, later={:?}",
        earlier.first_operated_month, later.first_operated_month
    );
}

#[test]
fn historical_skill_fixture_housing_loses_and_recovers_capacity_without_resetting_first_operation()
{
    let mut game = historical_skill_share_two_operating_factories_before_late_water(1.0);
    let project = "independent-29-housing";
    let before = game.governance().unwrap().projects[project].clone();
    let site_before = game.governance().unwrap().sites[&29].clone();
    game.queue_governance(
        "all-new-water-renew",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    game.end_month(12).unwrap();
    let state = game.governance().unwrap();
    assert!(state.projects[project].water_suspended);
    assert_eq!(
        state.sites[&29].housing_capacity,
        site_before.housing_capacity - state.calibration.housing_workers
    );
    assert_eq!(state.sites[&29].population, site_before.population);
    assert_eq!(
        state.projects[project].first_operated_month,
        before.first_operated_month
    );
    let saved = game.save().unwrap();
    game = Game::load(&saved).unwrap();
    advance(&mut game, 25);
    let state = game.governance().unwrap();
    assert!(!state.projects[project].water_suspended);
    assert_eq!(
        state.sites[&29].housing_capacity,
        site_before.housing_capacity
    );
    assert_eq!(state.sites[&29].population, site_before.population);
    assert_eq!(
        state.projects[project].first_operated_month,
        before.first_operated_month
    );
}

fn verify_trial_labor(game: &Game) {
    let state = game.governance().unwrap();
    let baseline = fixture();
    let opening = baseline.governance().unwrap();
    assert_eq!(
        state.sites.values().map(|s| s.population).sum::<u32>(),
        opening.sites.values().map(|s| s.population).sum::<u32>()
    );
    let employed: u32 = state
        .labor_trial_sources
        .values()
        .map(|s| s.nonmanufacturing_workers)
        .sum();
    let opening_employed: u32 = opening
        .labor_trial_sources
        .values()
        .map(|s| s.nonmanufacturing_workers)
        .sum();
    let mobile = state
        .sites
        .values()
        .map(|s| s.idle_workers + s.teachers)
        .sum::<u32>()
        + state
            .factories
            .values()
            .filter(|f| f.phase != FactoryPhase::Closed)
            .map(|f| f.workers)
            .sum::<u32>();
    assert_eq!(mobile + employed + 425, 295 + opening_employed + 425);
    if game.month() > 1 {
        let displaced: u32 = state
            .labor_trial_sources
            .values()
            .map(|s| s.cumulative_employed_recruits)
            .sum();
        let receipt = game
            .system_receipts()
            .iter()
            .rev()
            .find(|r| r.reason == "existing_nonmanufacturing_employer_wages")
            .unwrap();
        assert_eq!(
            receipt.delta,
            money(105.52).unwrap() as i128 - money(displaced as f64 * 0.02).unwrap() as i128
        );
    }
    // Public causal records expose which sources supplied each migration pass.
    let month = game.month().saturating_sub(1);
    let movements: Vec<_> = state
        .facts
        .iter()
        .filter(|f| {
            f.month == month
                && (f.effect.starts_with("domestic_idle_migration:")
                    || f.effect.starts_with("domestic_employed_migration:"))
        })
        .collect();
    let mut employed_seen = false;
    for movement in &movements {
        if movement.effect.starts_with("domestic_employed") {
            employed_seen = true;
        } else {
            assert!(
                !employed_seen,
                "idle pass occurred after recruiting an occupied job"
            );
        }
    }
    if state.labor_trial_enabled
        && !state
            .factories
            .values()
            .any(|f| f.phase == FactoryPhase::Closed)
    {
        for movement in movements
            .iter()
            .filter(|f| f.effect.starts_with("domestic_employed"))
        {
            let destination: u32 = movement.causes[1]
                .strip_prefix("site/")
                .unwrap()
                .parse()
                .unwrap();
            for (origin, source) in &state.sites {
                if *origin == destination || source.idle_workers == 0 {
                    continue;
                }
                let mut outbound = 0u32;
                let mut inbound = 0u32;
                for f in &movements {
                    let count: u32 = f.effect.split(':').nth(1).unwrap().parse().unwrap();
                    if f.causes[0] == format!("site/{origin}") {
                        outbound += count;
                    }
                    if f.causes[1] == format!("site/{origin}") {
                        inbound += count;
                    }
                }
                let opening_population = source.population + outbound - inbound;
                let monthly_limit = (opening_population as f64 * 0.01).floor() as u32;
                let record = &state.labor_trial_sources[origin];
                assert!(
                    outbound >= monthly_limit
                        || record.cumulative_outbound == record.initial_labor_force / 10,
                    "employed recruited despite reachable idle at {origin}"
                );
            }
        }
    }
    for (id, source) in &state.labor_trial_sources {
        assert_eq!(
            source.initial_nonmanufacturing_workers,
            opening.labor_trial_sources[id].initial_nonmanufacturing_workers
        );
        if state.labor_trial_enabled {
            let mut outbound = 0;
            let mut recruited = 0;
            let mut worker_months = 0;
            for f in state
                .facts
                .iter()
                .filter(|f| f.causes.first() == Some(&format!("site/{id}")))
            {
                if f.effect.starts_with("domestic_idle_migration:")
                    || f.effect.starts_with("domestic_employed_migration:")
                {
                    let count: u32 = f.effect.split(':').nth(1).unwrap().parse().unwrap();
                    outbound += count;
                    if f.effect.starts_with("domestic_employed") {
                        recruited += count;
                        worker_months += count as u64 * (game.month() - f.month) as u64;
                    }
                }
            }
            assert_eq!(source.cumulative_outbound, outbound);
            assert_eq!(source.cumulative_employed_recruits, recruited);
            assert_eq!(
                source.lost_worker_months, worker_months,
                "original wages resumed or were deducted twice"
            );
        }
        assert!(source.cumulative_outbound <= source.initial_labor_force / 10);
        assert_eq!(
            source.nonmanufacturing_workers + source.cumulative_employed_recruits,
            source.initial_nonmanufacturing_workers
        );
        assert_eq!(
            source.cumulative_wage_loss,
            money(source.lost_worker_months as f64 * 0.02).unwrap()
        );
        assert_eq!(
            source.cumulative_payroll_tax_loss,
            money(source.lost_worker_months as f64 * 0.0002).unwrap()
        );
    }
}

#[test]
fn labor_trial_explicit_off_replays_default_public_commands_identically() {
    let mut default = fixture();
    let mut off =
        Game::confirmed_game_with_labor_trial(GovernanceCalibration::default(), false).unwrap();
    for month in 1..=24 {
        assert_eq!(default.save().unwrap(), off.save().unwrap());
        default.end_month(month).unwrap();
        off.end_month(month).unwrap();
    }
    assert_eq!(default.save().unwrap(), off.save().unwrap());
}
