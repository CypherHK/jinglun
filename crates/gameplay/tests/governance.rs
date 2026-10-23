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
    assert_eq!(
        game.governance().unwrap().projects["house"].first_operated_month,
        None
    );
    game = Game::load(&game.save().unwrap()).unwrap();
    for month in 11..=25 {
        game.end_month(month).unwrap();
    }
    assert!(game.governance().unwrap().projects["house"].active);
    assert_eq!(
        game.governance().unwrap().projects["house"].first_operated_month,
        Some(25)
    );
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
    // Isolate existing cash and contract regressions from D026's pending skills calibration.
    let calibration = GovernanceCalibration {
        initial_skilled_idle_share: 1.0,
        ..Default::default()
    };
    public_factory_fixture(calibration, renewal, through, 9, 10)
}
fn public_factory_fixture(
    calibration: GovernanceCalibration,
    renewal: RenewalChoice,
    through: u32,
    renewal_month: u32,
    accept_a_from: u32,
) -> Game {
    let mut game = Game::confirmed_game_with_calibration(calibration).unwrap();
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
        if month == renewal_month {
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
                month >= accept_a_from
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
    for factory in state
        .factories
        .values()
        .filter(|f| f.phase == FactoryPhase::Operating)
    {
        assert!((factory.ordinary_energy_used - 3.0).abs() < 1e-8);
        assert!((factory.maritime_energy_used - 7.0).abs() < 1e-8);
        assert_eq!(factory.after_tax_profit, money(1.955).unwrap() as i128);
    }
    assert_eq!(
        game.balance("group_b", &Asset::Money).unwrap() - before,
        money(2.475 + 1.955 * 0.5).unwrap()
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
            .filter(|r| {
                r.source == source
                    && !matches!(
                        r.phase,
                        AllocationPhase::WaitingForWater
                            | AllocationPhase::OperationalWaitingForWater
                            | AllocationPhase::Retired
                    )
            })
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

#[test]
fn ordinary_energy_is_prepaid_then_delivered_next_month_with_shared_limit() {
    let mut game = fixture();
    for month in 1..=24 {
        if month == 3 {
            game.queue_governance("ordinary", GovernanceCommand::SignEnergy)
                .unwrap();
        }
        game.end_month(month).unwrap();
        let due: u64 = game
            .governance()
            .unwrap()
            .ordinary_energy_deliveries
            .iter()
            .filter(|(due, _, _)| *due == month + 1)
            .map(|(_, _, q)| q)
            .sum();
        assert!(due <= money(6.0).unwrap());
        if month == 11 {
            assert_eq!(due, money(6.0).unwrap());
            assert_eq!(
                game.balance("group_b", &Asset::Material("ordinary_energy".into()))
                    .unwrap_or(0),
                0
            );
            assert!(
                game.system_receipts()
                    .iter()
                    .all(|r| r.reason != "ordinary_energy_delivery")
            );
        }
        if month >= 12 {
            let f = game
                .governance()
                .unwrap()
                .factories
                .values()
                .find(|f| f.group == Group::B && f.phase == FactoryPhase::Operating)
                .unwrap();
            assert_eq!(f.ordinary_energy_used, 6.0);
            assert_eq!(f.maritime_energy_used, 4.0);
            assert_eq!(f.after_tax_profit, money(2.06).unwrap() as i128);
            assert_eq!(
                game.balance("group_b", &Asset::Material("ordinary_energy".into())),
                Some(0)
            );
        }
    }
    assert!(
        game.system_receipts()
            .iter()
            .any(|r| r.reason == "ordinary_energy_delivery"
                && r.month == 12
                && r.external_party == "resource_country_energy_producer")
    );
    assert!(
        game.system_receipts()
            .iter()
            .any(|r| r.reason == "new_factory_energy"
                && r.delta < 0
                && r.external_party == "maritime_energy_exporter")
    );
    let bytes = game.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
}

#[test]
fn b8_closure_retires_land_but_stops_water_use_and_replays() {
    let calibration = GovernanceCalibration {
        labor_protection_wage_multiplier: 2.0,
        ..GovernanceCalibration::default()
    };
    let mut game = Game::confirmed_game_with_calibration(calibration).unwrap();
    for month in 1..=11 {
        game.end_month(month).unwrap();
    }
    game.queue_governance(
        "loss-cause",
        GovernanceCommand::SetInstitution {
            kind: Institution::LaborProtection,
            enabled: true,
        },
    )
    .unwrap();
    for month in 12..=18 {
        game.end_month(month).unwrap();
    }
    let f = game
        .governance()
        .unwrap()
        .factories
        .values()
        .find(|f| f.group == Group::B)
        .unwrap();
    assert_eq!(f.phase, FactoryPhase::ClosureWarned);
    let id = f.id.clone();
    let before = game.reservations()[&id].clone();
    let used_before: u64 = game
        .reservations()
        .values()
        .filter(|r| {
            r.source == before.source
                && !matches!(
                    r.phase,
                    AllocationPhase::WaitingForWater
                        | AllocationPhase::OperationalWaitingForWater
                        | AllocationPhase::Retired
                )
        })
        .map(|r| r.water)
        .sum();
    let idle_before = game.governance().unwrap().sites[&f.site].idle_workers;
    let site = f.site;
    let workers = f.workers;
    game.end_month(19).unwrap();
    assert_eq!(
        game.governance().unwrap().factories[&id].phase,
        FactoryPhase::Closed
    );
    let retired = &game.reservations()[&id];
    assert_eq!(retired.phase, AllocationPhase::Retired);
    assert_eq!(retired.land, before.land);
    assert!(retired.land > 0);
    let used_after: u64 = game
        .reservations()
        .values()
        .filter(|r| {
            r.source == before.source
                && !matches!(
                    r.phase,
                    AllocationPhase::WaitingForWater
                        | AllocationPhase::OperationalWaitingForWater
                        | AllocationPhase::Retired
                )
        })
        .map(|r| r.water)
        .sum();
    assert_eq!(used_before - used_after, before.water);
    assert_eq!(
        game.governance().unwrap().sites[&site].idle_workers,
        idle_before + workers
    );
    let bytes = game.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
}

#[test]
fn long_term_supply_does_not_consume_or_erase_prepaid_ordinary_inventory() {
    let mut game = early_two_factory_fixture(RenewalChoice::EnergyLongTerm, 9);
    let ordinary = Asset::Material("ordinary_energy".into());
    let stock = game.balance("group_b", &ordinary).unwrap_or(0);
    assert!(stock > 0);
    for month in 10..=24 {
        game.end_month(month).unwrap();
        assert_eq!(game.balance("group_b", &ordinary).unwrap_or(0), stock);
        let factories = &game.governance().unwrap().factories;
        let mut energy = 0.0;
        for f in factories
            .values()
            .filter(|f| f.phase == FactoryPhase::Operating)
        {
            assert_eq!(f.ordinary_energy_used, 0.0);
            assert_eq!(f.maritime_energy_used, 0.0);
            energy += f.output / 10.0;
        }
        assert!(energy <= 20.0 + 1e-8);
    }
    let bytes = game.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
}

#[test]
fn security_follows_budget_tiers_and_recovery_cap() {
    let mut game = fixture();
    let baseline = game.governance().unwrap().budgets[&Service::Safety];
    assert_eq!(game.governance().unwrap().security, 60.0);
    let schedule = [
        (baseline, 60.0),
        (baseline * 3 / 2, 61.0),
        (baseline / 2, 60.0),
        (baseline / 4, 57.0),
        (baseline, 57.0),
    ];
    for (index, (monthly, expected)) in schedule.into_iter().enumerate() {
        game.queue_governance(
            format!("safety/{index}"),
            GovernanceCommand::SetSecurity { monthly },
        )
        .unwrap();
        game.end_month(index as u32 + 1).unwrap();
        assert_eq!(game.governance().unwrap().security, expected);
    }
    game.queue_governance(
        "recover",
        GovernanceCommand::SetSecurity {
            monthly: baseline * 3 / 2,
        },
    )
    .unwrap();
    for month in 6..=20 {
        game.end_month(month).unwrap();
        assert_eq!(
            game.governance().unwrap().security,
            (57.0 + (month - 5) as f64).min(70.0)
        );
    }
    let bytes = game.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
}

#[test]
fn autonomous_location_matches_shortest_public_payback_then_site() {
    let mut game = fixture();
    let mut observed = 0;
    for month in 1..=12 {
        let mut expected = std::collections::BTreeMap::new();
        for group in [Group::A, Group::B] {
            let mut cards: Vec<_> = game
                .governance()
                .unwrap()
                .sites
                .keys()
                .map(|site| game.scorecard(group, *site).unwrap())
                .filter(|c| c.autonomous_ready)
                .collect();
            cards.sort_by(|a, b| {
                a.static_payback_months
                    .unwrap()
                    .total_cmp(&b.static_payback_months.unwrap())
                    .then_with(|| a.site.cmp(&b.site))
            });
            if let Some(card) = cards.first() {
                expected.insert(group, card.site);
            }
        }
        game.end_month(month).unwrap();
        for f in game
            .governance()
            .unwrap()
            .factories
            .values()
            .filter(|f| f.started_month == month)
        {
            assert_eq!(expected.get(&f.group), Some(&f.site));
            observed += 1;
        }
    }
    assert!(observed > 0);
}

#[test]
fn undefined_clinical_population_rejects_switch_atomically_and_old_true_flags() {
    let mut game = fixture();
    let before = game.save().unwrap();
    for enabled in [true, false] {
        assert!(matches!(
            game.queue_governance(
                "unsupported",
                GovernanceCommand::SetInstitution {
                    kind: Institution::ClinicalCoverage,
                    enabled
                }
            ),
            Err(Error::Invalid(
                "clinical coverage switching is outside the first governance slice"
            ))
        ));
        assert_eq!(game.save().unwrap(), before);
    }
    let forged = rewrite_checked_save(&game, |payload| {
        payload["governance"]["clinical_coverage_institution"] = true.into();
    });
    assert!(Game::load(&forged).is_err());
    let old = rewrite_checked_save(&game, |payload| {
        payload["governance"]["clinical_coverage_institution"] = true.into();
        payload["governance"]["rules_version"] = "governance-rules-2".into();
    });
    assert!(Game::load(&old).is_err());
}

#[test]
fn b10_late_water_signature_protects_only_initial_and_suspends_newest_operations() {
    let calibration = GovernanceCalibration {
        initial_skilled_idle_share: 1.0,
        water_priority_fraction: 0.90,
        ..Default::default()
    };
    let mut game = public_factory_fixture(calibration, RenewalChoice::WaterPriority, 11, 12, 1);
    let mut operations: Vec<_> = game
        .governance()
        .unwrap()
        .factories
        .values()
        .filter(|f| f.site == 29 && f.phase == FactoryPhase::Operating)
        .map(|f| (f.first_operated_month.unwrap(), f.id.clone()))
        .collect();
    operations.sort();
    println!("late W before operations={operations:?}");
    assert_eq!(operations.len(), 2);
    let housing_first = game.governance().unwrap().projects["work/Housing"].first_operated_month;
    let baseline = game.save().unwrap();
    game.queue_governance(
        "late_W",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    let forecast = game.treasury_forecast().unwrap();
    game.withdraw_governance("late_W").unwrap();
    assert_eq!(game.save().unwrap(), baseline);
    game.queue_governance(
        "late_W",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    assert_eq!(forecast, game.treasury_forecast().unwrap());
    game.end_month(12).unwrap();
    assert_eq!(
        forecast[0].treasury,
        game.balance("treasury", &Asset::Money).unwrap()
    );
    assert_eq!(
        forecast[0].recurring_balance,
        game.governance().unwrap().last_recurring_balance
    );
    let state = game.governance().unwrap();
    let oldest = &state.factories[&operations[0].1];
    let newest = &state.factories[&operations[1].1];
    assert!(!oldest.water_suspended);
    assert!(oldest.output > 0.0);
    assert!(newest.water_suspended);
    assert_eq!(newest.output, 0.0);
    assert_eq!(newest.loss_months, 1);
    assert_eq!(
        newest.after_tax_profit,
        -(money(2.0 + 3.6).unwrap() as i128)
    );
    assert_eq!(newest.first_operated_month, Some(operations[1].0));
    assert_eq!(
        state.projects["work/Housing"].first_operated_month,
        housing_first
    );
    for id in ["initial-industry/39", "initial-industry/74"] {
        assert_eq!(game.reservations()[id].phase, AllocationPhase::Operational);
    }
    let value: serde_json::Value = serde_json::from_slice(&game.save().unwrap()).unwrap();
    for admission in value["payload"]["admissions"].as_object().unwrap().values() {
        assert!(
            admission["grandfathered"]
                .as_array()
                .unwrap()
                .iter()
                .all(|id| id.as_str().unwrap().starts_with("initial-industry/"))
        );
    }
    let bytes = game.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
    // Stop supplying new water for seven loss months; the normal exit path still runs.
    for month in 13..=18 {
        game.end_month(month).unwrap();
    }
    let closed = &game.governance().unwrap().factories[&operations[1].1];
    assert_eq!(closed.phase, FactoryPhase::Closed);
    assert_eq!(closed.first_operated_month, Some(operations[1].0));
    assert_eq!(
        game.reservations()[&operations[1].1].phase,
        AllocationPhase::Retired
    );
    for month in 19..=25 {
        game.end_month(month).unwrap();
    }
    assert_eq!(
        game.governance().unwrap().factories[&operations[1].1].phase,
        FactoryPhase::Closed
    );
    assert!(
        game.governance()
            .unwrap()
            .report
            .as_ref()
            .unwrap()
            .causes
            .iter()
            .any(|fact| fact.effect == "closed_factory_retains_land"
                && fact.causes.contains(&operations[1].1))
    );
}

#[test]
fn water_suspended_housing_withdraws_capacity_without_moving_people_and_restores_once() {
    let calibration = GovernanceCalibration {
        initial_skilled_idle_share: 1.0,
        water_priority_fraction: 1.0,
        ..Default::default()
    };
    let mut game = public_factory_fixture(calibration, RenewalChoice::WaterPriority, 11, 12, 1);
    let site_before = game.governance().unwrap().sites[&29].clone();
    let first = game.governance().unwrap().projects["work/Housing"].first_operated_month;
    assert!(first.is_some());
    game.queue_governance(
        "late_W",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    game.end_month(12).unwrap();
    let state = game.governance().unwrap();
    assert!(state.projects["work/Housing"].water_suspended);
    assert_eq!(state.sites[&29].population, site_before.population);
    assert_eq!(
        state.sites[&29].housing_capacity + state.calibration.housing_workers,
        site_before.housing_capacity
    );
    assert!(!state.history.last().unwrap().housing_ok);
    let bytes = game.save().unwrap();
    game = Game::load(&bytes).unwrap();
    for month in 13..=25 {
        game.end_month(month).unwrap();
    }
    let state = game.governance().unwrap();
    assert!(!state.projects["work/Housing"].water_suspended);
    assert_eq!(state.projects["work/Housing"].first_operated_month, first);
    assert_eq!(
        state.sites[&29].housing_capacity,
        site_before.housing_capacity
    );
    assert_eq!(
        game.reservations()["public/work/Housing"].phase,
        AllocationPhase::Operational
    );
    let bytes = game.save().unwrap();
    let mut resumed = Game::load(&bytes).unwrap();
    game.end_month(26).unwrap();
    resumed.end_month(26).unwrap();
    assert_eq!(game.save().unwrap(), resumed.save().unwrap());
    assert_eq!(
        game.governance().unwrap().sites[&29].housing_capacity,
        site_before.housing_capacity
    );
}

#[test]
fn d026_rule_parameters_convert_only_existing_idle_workers_and_fund_fixed_export_orders() {
    let mut game = fixture();
    let state = game.governance().unwrap();
    assert_eq!(state.sites[&39].skilled_workers, 30);
    assert_eq!(state.sites[&74].skilled_workers, 7);
    assert_eq!(state.calibration.initial_skilled_idle_share, 0.25);
    assert_eq!(state.calibration.cooperation_export_orders_per_month, 100);
    assert_eq!(
        state.calibration.foreign_buyer_monthly_budget,
        money(10.0).unwrap()
    );
    for month in 1..=6 {
        if month == 2 {
            game.queue_governance("cooperate", GovernanceCommand::SignCooperation)
                .unwrap();
        }
        game.end_month(month).unwrap();
    }
    assert_eq!(game.governance().unwrap().export_orders, 100.0);
    assert!(game.balance("foreign_buyer", &Asset::Money).unwrap() >= money(10.0).unwrap());
    let old = rewrite_checked_save(&game, |payload| {
        payload["governance"]["rules_version"] = "governance-rules-3".into();
        payload["governance"]["calibration"]
            .as_object_mut()
            .unwrap()
            .remove("initial_skilled_idle_share");
    });
    assert!(matches!(
        Game::load(&old),
        Err(Error::Invalid("unsupported governance rules version"))
    ));
}

#[test]
fn housing_and_factories_share_one_actual_operation_water_priority_order() {
    let calibration = GovernanceCalibration {
        // A larger protected cash floor legally cuts services to fund the extra house.
        protected_treasury: money(20.0).unwrap(),
        initial_skilled_idle_share: 1.0,
        water_priority_fraction: 0.90,
        ..Default::default()
    };
    let mut game = public_factory_fixture(calibration, RenewalChoice::WaterPriority, 8, 12, 1);
    game.queue_governance(
        "extra_housing",
        GovernanceCommand::BuildPublic {
            id: "newest_house".into(),
            site: 29,
            kind: PublicWork::Housing,
            road_edge: None,
        },
    )
    .unwrap();
    for month in 9..=11 {
        game.end_month(month).unwrap();
    }
    let before = game.governance().unwrap().sites[&29].clone();
    assert_eq!(
        game.governance().unwrap().projects["newest_house"].first_operated_month,
        Some(11)
    );
    game.queue_governance(
        "late_W",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    game.end_month(12).unwrap();
    let state = game.governance().unwrap();
    assert!(state.projects["newest_house"].water_suspended);
    assert!(!state.projects["work/Housing"].water_suspended);
    assert_eq!(state.sites[&29].population, before.population);
    assert_eq!(
        state.sites[&29].housing_capacity + state.calibration.housing_workers,
        before.housing_capacity
    );
    let mut plants: Vec<_> = state.factories.values().filter(|f| f.site == 29).collect();
    plants.sort_by_key(|f| (f.first_operated_month, f.id.clone()));
    assert_eq!(plants.len(), 2);
    assert!(!plants[0].water_suspended);
    assert!(plants[1].water_suspended);
    let bytes = game.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
}

#[test]
fn b10_in_construction_factories_finish_but_wait_without_operating_losses() {
    let calibration = GovernanceCalibration {
        initial_skilled_idle_share: 1.0,
        water_priority_fraction: 0.94,
        ..Default::default()
    };
    let mut game = public_factory_fixture(calibration, RenewalChoice::WaterPriority, 1, 9, 1);
    let mut accepted = std::collections::BTreeSet::new();
    for month in 2..=8 {
        for kind in [PublicWork::Power, PublicWork::Water] {
            let id = format!("work/{kind:?}");
            if !game.governance().unwrap().projects.contains_key(&id) {
                let _ = game.queue_governance(
                    format!("retry/{kind:?}/{month}"),
                    GovernanceCommand::BuildPublic {
                        id,
                        site: 29,
                        kind,
                        road_edge: None,
                    },
                );
            }
        }

        if month == 2 {
            game.queue_governance(
                "pause_road",
                GovernanceCommand::PauseProject {
                    id: "work/Road".into(),
                    paused: true,
                },
            )
            .unwrap();
        }
        if month == 4 {
            game.queue_governance(
                "finish_clinic",
                GovernanceCommand::BuildPublic {
                    id: "work/Clinic".into(),
                    site: 29,
                    kind: PublicWork::Clinic,
                    road_edge: None,
                },
            )
            .unwrap();
        }
        if month == 5 {
            game.queue_governance(
                "resume_road",
                GovernanceCommand::PauseProject {
                    id: "work/Road".into(),
                    paused: false,
                },
            )
            .unwrap();
        }
        let intents: Vec<_> = game
            .governance()
            .unwrap()
            .intents
            .values()
            .filter(|i| i.site == 29)
            .cloned()
            .collect();
        for intent in intents {
            if !accepted.contains(&intent.group)
                && game
                    .queue_governance(
                        format!("accept/{}", intent.id),
                        GovernanceCommand::AcceptIntent {
                            id: intent.id,
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
    let ids: Vec<_> = game
        .governance()
        .unwrap()
        .factories
        .values()
        .filter(|f| f.site == 29)
        .map(|f| f.id.clone())
        .collect();
    assert_eq!(ids.len(), 2);
    let progress: std::collections::BTreeMap<_, _> = ids
        .iter()
        .map(|id| {
            let f = &game.governance().unwrap().factories[id];
            assert_eq!(f.phase, FactoryPhase::Construction);
            assert!(f.worked_months > 0 && f.worked_months < f.construction_months);
            (id.clone(), f.worked_months)
        })
        .collect();
    game.queue_governance(
        "renew",
        GovernanceCommand::RenewAid {
            choice: RenewalChoice::WaterPriority,
        },
    )
    .unwrap();
    game.end_month(9).unwrap();
    for id in &ids {
        let f = &game.governance().unwrap().factories[id];
        assert_eq!(f.worked_months, progress[id] + 1);
        assert_eq!(f.first_operated_month, None);
        assert_eq!(
            game.reservations()[id].phase,
            AllocationPhase::WaitingForWater
        );
    }
    for month in 10..=24 {
        game.end_month(month).unwrap();
    }
    for id in &ids {
        let f = &game.governance().unwrap().factories[id];
        assert_eq!(f.worked_months, f.construction_months);
        assert_eq!(game.escrow_remaining(&f.capital_escrow), Some(0));
        assert_eq!(f.phase, FactoryPhase::Construction);
        assert!(!f.water_suspended);
        assert_eq!(f.loss_months, 0);
        assert_eq!(f.output, 0.0);
        assert_eq!(f.first_operated_month, None);
    }
    let bytes = game.save().unwrap();
    game = Game::load(&bytes).unwrap();
    game.end_month(25).unwrap();
    for id in ids {
        let f = &game.governance().unwrap().factories[&id];
        assert_eq!(f.phase, FactoryPhase::Operating);
        assert_eq!(f.first_operated_month, Some(25));
        assert!(f.activate_month < 25);
    }
    let bytes = game.save().unwrap();
    assert_eq!(Game::load(&bytes).unwrap().save().unwrap(), bytes);
}
