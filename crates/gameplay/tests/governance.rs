use jinglun_gameplay::*;
use std::sync::OnceLock;
fn fixture() -> Game {
    static GAME: OnceLock<Game> = OnceLock::new();
    GAME.get_or_init(|| Game::confirmed_game().unwrap()).clone()
}
#[test]
fn passive_game_runs_real_nine_stages_and_load_replays_business() {
    let mut game = fixture();
    let original = game.map().clone();
    assert_eq!(
        game.balance("treasury", &Asset::Money),
        Some(money(55.0).unwrap())
    );
    for month in 1..=24 {
        let receipt = game.end_month(month).unwrap();
        assert!(!receipt.foundation_mode);
        assert_eq!(receipt.stages.len(), 9);
        if month == 12 {
            game = Game::load(&game.save().unwrap()).unwrap();
        }
    }
    let s = game.governance().unwrap();
    let report = s.report.as_ref().unwrap();
    println!(
        "B2 {:?}, treasury={}, recurring={:?}, factories={:?}",
        report.grade,
        currency(game.balance("treasury", &Asset::Money).unwrap()),
        report.final_three_recurring,
        s.factories
            .values()
            .map(|f| (f.site, f.started_month, f.phase))
            .collect::<Vec<_>>()
    );
    assert_eq!(report.grade, GoalGrade::NotAchieved);
    assert!(!report.fiscal_recovered);
    assert!(game.balance("treasury", &Asset::Money).unwrap() > 0);
    assert_eq!(game.map(), &original);
    assert!(!game.system_receipts().is_empty());
    let reloaded = Game::load(&game.save().unwrap()).unwrap();
    assert_eq!(reloaded.save().unwrap(), game.save().unwrap());
}
#[test]
fn plans_preview_and_undo_are_pure_and_failed_commands_atomic() {
    let mut game = fixture();
    let initial = game.save().unwrap();
    let before = game.scorecard(Group::A, 8).unwrap();
    let forecast = game.treasury_forecast().unwrap();
    game.queue_governance(
        "sub",
        GovernanceCommand::SetSubsidy {
            site: 8,
            cap: money(40.0).unwrap(),
        },
    )
    .unwrap();
    let after = game.scorecard(Group::A, 8).unwrap();
    assert!(after.gaps["cash"] < before.gaps["cash"]);
    game.withdraw_governance("sub").unwrap();
    assert_eq!(game.scorecard(Group::A, 8).unwrap(), before);
    assert_eq!(game.treasury_forecast().unwrap(), forecast);
    assert_eq!(game.save().unwrap(), initial);
    assert!(
        game.queue_governance("bad", GovernanceCommand::OpenRegion { site: 2 })
            .is_err()
    );
    assert_eq!(game.save().unwrap(), initial);
}

#[test]
fn opening_sources_and_material_owners_are_explicit() {
    let game = fixture();
    for site in [8, 29, 39, 74] {
        let view = game.cell(site).unwrap();
        println!(
            "site {site} land={} nonzero_sources={} total_water_slots={}",
            view.land_slots,
            view.water_sources
                .values()
                .filter(|(_, slots)| *slots > 0)
                .count(),
            view.water_sources
                .values()
                .map(|(_, slots)| *slots)
                .sum::<u64>()
        );
    }
    assert_eq!(
        game.balance("households", &Asset::Material("food".into())),
        Some(20)
    );
    assert_eq!(
        game.balance("capital_inventory", &Asset::Material("construction".into())),
        Some(12)
    );
    assert_eq!(
        game.balance("treasury", &Asset::Material("food".into())),
        Some(0)
    );
}

#[test]
fn water_renewal_continues_work_but_waits_for_water_before_activation() {
    let calibration = GovernanceCalibration {
        water_priority_fraction: 1.0,
        ..Default::default()
    };
    let mut game = Game::confirmed_game_with_calibration(calibration).unwrap();
    for month in 1..=7 {
        game.end_month(month).unwrap();
    }
    game.queue_governance(
        "house",
        GovernanceCommand::BuildPublic {
            id: "house".into(),
            site: 8,
            kind: PublicWork::Housing,
            road_edge: None,
        },
    )
    .unwrap();
    game.end_month(8).unwrap();
    assert_eq!(
        game.governance().unwrap().projects["house"].worked_months,
        1
    );
    game.queue_governance(
        "renew",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    game.end_month(9).unwrap();
    let s = game.governance().unwrap();
    assert_eq!(s.projects["house"].worked_months, 2);
    assert!(!s.projects["house"].active);
    assert_eq!(
        game.reservations()["public/house"].phase,
        AllocationPhase::WaitingForWater
    );
    let paid_before = game.escrow_remaining("public/house").unwrap();
    assert_eq!(paid_before, 0);
    game.end_month(10).unwrap();
    assert!(!game.governance().unwrap().projects["house"].active);
    game = Game::load(&game.save().unwrap()).unwrap();
    for month in 11..=25 {
        game.end_month(month).unwrap();
    }
    assert!(game.governance().unwrap().projects["house"].active);
    assert_eq!(
        game.reservations()["public/house"].phase,
        AllocationPhase::Operational
    );
}

#[test]
fn energy_long_term_has_gross_payments_unexpired_liability_and_private_resale() {
    let mut game = fixture();
    for month in 1..=8 {
        game.end_month(month).unwrap();
    }
    game.queue_governance(
        "renew",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::EnergyLongTerm,
        },
    )
    .unwrap();
    game.end_month(9).unwrap();
    let obligation = game.payable("resource_energy_long_term").unwrap();
    assert_eq!(
        obligation.total_committed,
        money(20.0 * 0.17 * 24.0).unwrap()
    );
    assert_eq!(obligation.paid, money(3.4).unwrap());
    assert_eq!(
        game.governance().unwrap().last_energy_unused_bill,
        money(3.4).unwrap()
    );
    for month in 10..=24 {
        game.end_month(month).unwrap();
    }
    let report = game.governance().unwrap().report.as_ref().unwrap();
    assert_eq!(report.future_energy_liability, money(3.4 * 8.0).unwrap());
    assert_eq!(
        report.treasury_net_energy_liability,
        game.balance("treasury", &Asset::Money).unwrap() as i128
            - game.payable("resource_energy_long_term").unwrap().unpaid as i128
    );
    assert_eq!(
        Game::load(&game.save().unwrap()).unwrap().save().unwrap(),
        game.save().unwrap()
    );
}

fn rewrite_checked_save(game: &Game, change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(&game.save().unwrap()).unwrap();
    change(&mut value["payload"]);
    let bytes = serde_json::to_vec(&value["payload"]).unwrap();
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    });
    value["checksum"] = format!("fnv1a64:{hash:016x}").into();
    serde_json::to_vec(&value).unwrap()
}
#[test]
fn business_replay_rejects_balanced_counterfeit_receipts_and_plausible_state_changes() {
    let mut game = fixture();
    game.end_month(1).unwrap();
    let state_tamper = rewrite_checked_save(&game, |payload| {
        payload["governance"]["security"] = 63.0.into()
    });
    assert!(Game::load(&state_tamper).is_err());
    let balanced_tamper = rewrite_checked_save(&game, |payload| {
        let receipts = payload["system_receipts"].as_array_mut().unwrap();
        let receipt = receipts
            .iter_mut()
            .find(|r| r["reason"] == "existing_nonmanufacturing_employer_wages")
            .unwrap();
        let old = receipt["delta"].as_i64().unwrap();
        receipt["delta"] = (old + 1).into();
        receipt["external_counterbalance"] = (-(old + 1)).into();
        let cash = payload["ledger"]["accounts"]["households"]["money"]
            .as_u64()
            .unwrap();
        payload["ledger"]["accounts"]["households"]["money"] = (cash + 1).into();
    });
    assert!(Game::load(&balanced_tamper).is_err());
}
#[test]
fn currency_quantization_is_central_and_checked() {
    assert_eq!(money(100_000_000.00000001).unwrap(), 10_000_000_000_000_001);
    assert_eq!(money(0.0000000049).unwrap(), 0);
    assert_eq!(money(0.000000005).unwrap(), 1);
    assert_eq!(currency(money(55.0).unwrap()), 55.0);
    for invalid in [f64::NAN, f64::INFINITY, -0.1, u64::MAX as f64] {
        assert!(money(invalid).is_err());
    }
}

fn domestic_road(game: &Game, site: u32) -> u32 {
    let owned = &game.governance().unwrap().sites;
    game.map()["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| {
            let a = edge["a"].as_u64().unwrap() as u32;
            let b = edge["b"].as_u64().unwrap() as u32;
            (a == site || b == site)
                && owned.contains_key(&a)
                && owned.contains_key(&b)
                && edge["passage"]["cost_multiplier"].as_f64().is_some()
        })
        .min_by(|a, b| {
            a["passage"]["cost_multiplier"]
                .as_f64()
                .unwrap()
                .total_cmp(&b["passage"]["cost_multiplier"].as_f64().unwrap())
        })
        .unwrap()["id"]
        .as_u64()
        .unwrap() as u32
}
fn early_two_factory_fixture(renewal: RenewalChoice, through: u32) -> Game {
    let mut game = fixture();
    let site = 29;
    let road = domestic_road(&game, site);
    game.queue_governance("target", GovernanceCommand::SelectCandidate { site })
        .unwrap();
    game.queue_governance("open", GovernanceCommand::OpenRegion { site })
        .unwrap();
    game.queue_governance(
        "cap",
        GovernanceCommand::SetSubsidy {
            site,
            cap: money(40.0).unwrap(),
        },
    )
    .unwrap();
    let mut built = std::collections::BTreeSet::new();
    let mut accepted = std::collections::BTreeSet::new();
    for month in 1..=through {
        if month == 3 {
            game.queue_governance("ordinary", GovernanceCommand::SignEnergy)
                .unwrap();
        }
        if month == 9 {
            game.queue_governance("renew", GovernanceCommand::RenewAid { choice: renewal })
                .unwrap();
            game.queue_governance(
                "fresh_terms",
                GovernanceCommand::SetSubsidy {
                    site,
                    cap: money(40.0).unwrap(),
                },
            )
            .unwrap();
        }
        for kind in [
            PublicWork::Housing,
            PublicWork::Road,
            PublicWork::Power,
            PublicWork::Water,
            PublicWork::Clinic,
        ] {
            if built.contains(&kind) {
                continue;
            }
            let id = format!("work/{kind:?}");
            if game
                .queue_governance(
                    format!("build/{kind:?}"),
                    GovernanceCommand::BuildPublic {
                        id: id.clone(),
                        site,
                        kind,
                        road_edge: if kind == PublicWork::Road {
                            Some(road)
                        } else {
                            None
                        },
                    },
                )
                .is_ok()
            {
                built.insert(kind);
                if kind == PublicWork::Clinic {
                    game.queue_governance(
                        "clinic_accelerate",
                        GovernanceCommand::ExpediteProject {
                            id,
                            additional_months: 1,
                        },
                    )
                    .unwrap();
                }
            }
        }
        let intents: Vec<_> = game
            .governance()
            .unwrap()
            .intents
            .values()
            .cloned()
            .collect();
        for intent in intents {
            if accepted.contains(&intent.group) {
                continue;
            }
            let allowed = if intent.group == Group::B {
                true
            } else {
                month >= 10
            };
            if allowed
                && game
                    .queue_governance(
                        format!("accept/{}", intent.id),
                        GovernanceCommand::AcceptIntent {
                            id: intent.id.clone(),
                            deadline: month + 4,
                        },
                    )
                    .is_ok()
            {
                accepted.insert(intent.group);
            }
        }
        game.end_month(month).unwrap();
    }
    game
}
#[test]
fn b3_late_budget_response_has_three_month_recovery_without_terminating() {
    let mut game = fixture();
    for month in 1..=12 {
        game.end_month(month).unwrap();
    }
    game.queue_governance(
        "education_rebalance",
        GovernanceCommand::SetServiceBudget {
            kind: Service::Education,
            monthly: money(1.0).unwrap(),
        },
    )
    .unwrap();
    for month in 13..=24 {
        game.end_month(month).unwrap();
    }
    let state = game.governance().unwrap();
    let restored = state
        .history
        .windows(3)
        .find(|window| {
            window[0].month >= 13 && window.iter().all(|record| record.recurring_balance >= 0)
        })
        .unwrap()[2]
        .month;
    assert!((15..=18).contains(&restored));
    assert!(state.report.as_ref().unwrap().fiscal_recovered);
    assert!(game.balance("treasury", &Asset::Money).unwrap() > 0);
    assert_eq!(game.month(), 25);
    let before = game.save().unwrap();
    game.queue_governance(
        "education_rebalance",
        GovernanceCommand::SetServiceBudget {
            kind: Service::Education,
            monthly: money(1.0).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(game.save().unwrap(), before);
    assert!(
        game.queue_governance(
            "education_rebalance",
            GovernanceCommand::SetServiceBudget {
                kind: Service::Education,
                monthly: money(2.0).unwrap()
            }
        )
        .is_err()
    );
    assert_eq!(game.save().unwrap(), before);
}
#[test]
fn b4_early_factory_revenue_softens_the_month_thirteen_aid_break() {
    let game = early_two_factory_fixture(RenewalChoice::Decline, 24);
    let state = game.governance().unwrap();
    let before = &state.history[11];
    let after = &state.history[12];
    assert_eq!(before.aid, money(1.586).unwrap());
    assert_eq!(after.aid, 0);
    let difference = (after.recurring_balance - before.recurring_balance).unsigned_abs();
    let pending_small_threshold = money(0.4).unwrap() as u128;
    println!(
        "B4 M12={} M13={} difference={} pending threshold=0.4",
        before.recurring_balance, after.recurring_balance, difference
    );
    assert!(difference <= pending_small_threshold);
    assert!(
        state
            .factories
            .values()
            .any(|factory| factory.activate_month == 13)
    );
}
#[test]
fn b9_operating_receipts_never_pay_subsidy_into_company_cash() {
    let mut game = early_two_factory_fixture(RenewalChoice::Decline, 23);
    let before = game.balance("group_b", &Asset::Money).unwrap();
    game.end_month(24).unwrap();
    let state = game.governance().unwrap();
    assert!(
        state
            .factories
            .values()
            .any(|f| f.subsidy > 0 && f.phase == FactoryPhase::Operating)
    );
    for factory in state
        .factories
        .values()
        .filter(|f| f.phase == FactoryPhase::Operating)
    {
        assert_eq!(factory.operating_subsidy, 0);
        if let Some(escrow) = &factory.subsidy_escrow {
            assert_eq!(game.escrow_remaining(escrow), Some(0));
        }
    }
    assert_eq!(
        game.balance("group_b", &Asset::Money).unwrap() - before,
        money(2.475 + 2.2 * 0.5).unwrap()
    );
    assert!(
        !game
            .system_receipts()
            .iter()
            .any(|receipt| receipt.account == "group_b" && receipt.reason.contains("subsidy"))
    );
}
#[test]
fn b12_existing_industry_and_cross_border_water_share_the_real_capacity_and_corridor() {
    let game = early_two_factory_fixture(RenewalChoice::WaterPriority, 10);
    assert_eq!(game.reservations()["initial-industry/39"].water, 3);
    assert_eq!(game.reservations()["initial-industry/74"].water, 2);
    let cell = game.cell(8).unwrap();
    assert_eq!(cell.land_slots, game.rules().land_min);
    assert!(
        cell.buildable_area_km2 / game.rules().land_km2_per_slot < game.rules().land_min as f64
    );
    let mut shared = false;
    for source in game
        .reservations()
        .values()
        .map(|r| r.source)
        .collect::<std::collections::BTreeSet<_>>()
    {
        let reservations: Vec<_> = game
            .reservations()
            .values()
            .filter(|r| r.source == source && r.phase != AllocationPhase::WaitingForWater)
            .collect();
        let quota = game.map()["environment"]["water_sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"].as_u64() == Some(source as u64))
            .unwrap();
        let minimum = quota["months"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["freshwater_ceiling"].as_f64().unwrap())
            .fold(f64::INFINITY, f64::min);
        let scale = game.map()["environment"]["scale"]["water_unit_m3"]
            .as_f64()
            .unwrap();
        let slots = ((minimum * scale / game.rules().water_m3_per_slot).floor() as u64)
            .clamp(game.rules().water_min, game.rules().water_max);
        assert!(reservations.iter().map(|r| r.water).sum::<u64>() <= slots);
        let countries = reservations
            .iter()
            .filter(|r| r.water > 0)
            .map(|r| r.country.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        shared |= countries.contains("player") && countries.contains("resource");
    }
    assert!(shared);
    let project = &game.governance().unwrap().projects["work/Road"];
    let edge = game.map()["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| edge["id"].as_u64() == Some(project.road_edge.unwrap() as u64))
        .unwrap();
    let expected = money(
        1.5 * game.cell(project.site).unwrap().cost_multiplier.unwrap()
            * edge["passage"]["cost_multiplier"].as_f64().unwrap(),
    )
    .unwrap();
    assert_eq!(project.cost, expected);
    assert_eq!(project.construction_months, 3);
}
#[test]
fn b13_report_traces_water_choice_against_the_same_build_plan_counterfactual() {
    let water = early_two_factory_fixture(RenewalChoice::WaterPriority, 24);
    let declined = early_two_factory_fixture(RenewalChoice::Decline, 24);
    let ws = water.governance().unwrap();
    let ds = declined.governance().unwrap();
    let water_factories = ws
        .factories
        .values()
        .filter(|f| f.site == 29 && f.phase == FactoryPhase::Operating)
        .count();
    let declined_factories = ds
        .factories
        .values()
        .filter(|f| f.site == 29 && f.phase == FactoryPhase::Operating)
        .count();
    println!("B13 W plants={water_factories} Decline plants={declined_factories}");
    assert_eq!(water_factories, 1);
    assert_eq!(declined_factories, 2);
    let report = ws.report.as_ref().unwrap();
    assert_ne!(report.grade, GoalGrade::Achieved);
    assert_eq!(ds.report.as_ref().unwrap().grade, GoalGrade::Achieved);
    assert!(report.causes.iter().any(|fact| fact.month == 9
        && fact.effect == "water_priority_limits_next_factory"
        && fact.causes.contains(&"aid/renewal".into())));
    assert_eq!(
        ws.decision_log["renew"],
        (
            9,
            GovernanceCommand::RenewAid {
                choice: RenewalChoice::WaterPriority
            }
        )
    );
}
