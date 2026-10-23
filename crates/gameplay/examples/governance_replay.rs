use jinglun_gameplay::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

#[path = "../src/b15.rs"]
mod b15;

#[derive(Clone, Debug, Serialize)]
struct Script {
    site: u32,
    route: u32,
    choice: b15::Choice,
    prepare_month: u32,
    subsidy_currency: u64,
    renewal_month: u32,
    project_order: u32,
}

impl Script {
    fn id(&self) -> String {
        format!(
            "s{}-r{}-{:?}-p{}-c{}-m{}-o{}",
            self.site,
            self.route,
            self.choice,
            self.prepare_month,
            self.subsidy_currency,
            self.renewal_month,
            self.project_order
        )
    }
}

#[derive(Serialize)]
struct Replay {
    script: Script,
    outcome: Option<b15::ScriptOutcome>,
    first_sustained_window: Option<u32>,
    report: Option<GovernanceReport>,
    execution_error: Option<String>,
    recurring_sum_for_display: Option<i128>,
    failed_actions: Vec<Value>,
    monthly: Vec<Value>,
    final_scorecards: Vec<Scorecard>,
}

fn emit(value: &impl Serialize) -> std::result::Result<(), String> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    writeln!(output).map_err(|e| e.to_string())
}

fn road_edge(game: &Game, site: u32) -> std::result::Result<u32, String> {
    let state = game.governance().ok_or("missing governance state")?;
    game.map()["edges"]
        .as_array()
        .ok_or("missing map edges")?
        .iter()
        .filter_map(|edge| {
            let a = u32::try_from(edge["a"].as_u64()?).ok()?;
            let b = u32::try_from(edge["b"].as_u64()?).ok()?;
            let cost = edge.pointer("/passage/cost_multiplier")?.as_f64()?;
            let id = u32::try_from(edge["id"].as_u64()?).ok()?;
            ((a == site || b == site)
                && state.sites.contains_key(&a)
                && state.sites.contains_key(&b)
                && cost.is_finite()
                && cost > 0.0)
                .then_some((cost, id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|x| x.1)
        .ok_or_else(|| format!("site {site}: no buildable domestic road corridor"))
}

fn attempt(
    game: &mut Game,
    id: String,
    command: GovernanceCommand,
    failed: &mut Vec<Value>,
) -> bool {
    let month = game.month();
    match game.queue_governance(id.clone(), command.clone()) {
        Ok(()) => true,
        Err(error) => {
            failed.push(json!({"month":month,"id":id,"command":command,"error":error.to_string()}));
            false
        }
    }
}

fn replay(base: &Game, script: Script) -> std::result::Result<Replay, String> {
    let mut game = base.clone();
    let road = road_edge(&game, script.site)?;
    let mut done = BTreeSet::new();
    let mut failed = vec![];
    let mut monthly = vec![];
    let mut fatal = None;
    let mut works = match (script.route, script.project_order) {
        (1, 0) => vec![
            PublicWork::Housing,
            PublicWork::Road,
            PublicWork::Power,
            PublicWork::Water,
            PublicWork::Clinic,
            PublicWork::Vocational,
        ],
        (1, _) => vec![
            PublicWork::Road,
            PublicWork::Power,
            PublicWork::Housing,
            PublicWork::Clinic,
            PublicWork::Water,
            PublicWork::Vocational,
        ],
        (2, 0) => vec![
            PublicWork::Vocational,
            PublicWork::Housing,
            PublicWork::Clinic,
            PublicWork::Road,
            PublicWork::Power,
            PublicWork::Water,
        ],
        _ => vec![
            PublicWork::Housing,
            PublicWork::Clinic,
            PublicWork::Vocational,
            PublicWork::Power,
            PublicWork::Road,
            PublicWork::Water,
        ],
    };
    if script.route == 1 && script.project_order == 0 {
        works.retain(|kind| *kind != PublicWork::Vocational);
    }
    for month in 1..=24 {
        if month == 1 {
            for (id, command) in [(
                "candidate",
                GovernanceCommand::SelectCandidate { site: script.site },
            )] {
                if !attempt(&mut game, id.into(), command, &mut failed) {
                    fatal = Some(format!("mandatory initial command {id} rejected"));
                    break;
                }
            }
        }
        if fatal.is_some() {
            break;
        }
        if month >= script.prepare_month {
            for (id, command) in [
                ("admit", GovernanceCommand::OpenRegion { site: script.site }),
                (
                    "subsidy",
                    GovernanceCommand::SetSubsidy {
                        site: script.site,
                        cap: script.subsidy_currency * MONEY_SCALE,
                    },
                ),
            ] {
                if !done.contains(id) && attempt(&mut game, id.into(), command, &mut failed) {
                    done.insert(id.into());
                }
            }
            if script.route == 2
                && month >= 2
                && !done.contains("cooperation")
                && attempt(
                    &mut game,
                    "cooperation".into(),
                    GovernanceCommand::SignCooperation,
                    &mut failed,
                )
            {
                done.insert("cooperation".into());
            }
            if script.route == 1
                && month >= 3
                && !done.contains("energy")
                && attempt(
                    &mut game,
                    "energy".into(),
                    GovernanceCommand::SignEnergy,
                    &mut failed,
                )
            {
                done.insert("energy".into());
            }
        }
        if month >= script.renewal_month && month <= 12 && !done.contains("renewal") {
            let choice = match script.choice {
                b15::Choice::W => RenewalChoice::WaterPriority,
                b15::Choice::E => RenewalChoice::EnergyLongTerm,
                b15::Choice::Decline => RenewalChoice::Decline,
            };
            if attempt(
                &mut game,
                "renewal".into(),
                GovernanceCommand::RenewAid { choice },
                &mut failed,
            ) {
                done.insert("renewal".into());
            }
        }
        if month >= script.prepare_month {
            for kind in &works {
                let id = format!("public/{kind:?}");
                if done.contains(&id) {
                    continue;
                }
                if attempt(
                    &mut game,
                    format!("build/{kind:?}"),
                    GovernanceCommand::BuildPublic {
                        id: id.clone(),
                        site: script.site,
                        kind: *kind,
                        road_edge: (*kind == PublicWork::Road).then_some(road),
                    },
                    &mut failed,
                ) {
                    done.insert(id);
                }
            }
            let state = game.governance().ok_or("missing governance state")?;
            let mut intents: Vec<_> = state
                .intents
                .values()
                .filter(|i| i.site == script.site)
                .cloned()
                .collect();
            intents.sort_by_key(|i| (i.subsidy, i.id.clone()));
            for intent in intents {
                let id = format!("accept/{}", intent.id);
                if done.contains(&id) {
                    continue;
                }
                if attempt(
                    &mut game,
                    id.clone(),
                    GovernanceCommand::AcceptIntent {
                        id: intent.id,
                        deadline: (month + 4).min(24),
                    },
                    &mut failed,
                ) {
                    done.insert(id);
                }
            }
        }
        if let Err(error) = game.end_month(month) {
            fatal = Some(format!("month {month}: {error}"));
            break;
        }
        let state = game.governance().ok_or("missing governance state")?;
        let record = state.history.last().ok_or("month produced no history")?;
        let factories: Vec<_> = state.factories.values().map(|f|json!({"id":f.id,"group":f.group,"site":f.site,"phase":f.phase,"start":f.started_month,"activate":f.activate_month,"output":f.output,"after_tax_profit":f.after_tax_profit,"operating_subsidy":f.operating_subsidy})).collect();
        monthly.push(json!({"record":record,"group_cash":{"A":game.balance(Group::A.account(), &Asset::Money),"B":game.balance(Group::B.account(), &Asset::Money)},"factories":factories,"candidate_site":state.sites.get(&script.site),"unused_energy_bill":state.last_energy_unused_bill}));
    }
    let state = game.governance().ok_or("missing governance state")?;
    let report = state.report.clone();
    let history = &state.history;
    let union = history
        .iter()
        .filter(|r| !r.housing_ok || !r.clinic_ok || !r.security_ok)
        .count() as u32;
    let guards = [
        history.iter().filter(|r| !r.housing_ok).count() as u32,
        history.iter().filter(|r| !r.clinic_ok).count() as u32,
        history.iter().filter(|r| !r.security_ok).count() as u32,
    ];
    let first = report.as_ref().and_then(|r| r.first_sustained_month);
    let achieved = report
        .as_ref()
        .is_some_and(|r| r.grade == GoalGrade::Achieved)
        && fatal.is_none();
    let final_three = report
        .as_ref()
        .and_then(|r| <[i128; 3]>::try_from(r.final_three_recurring.clone()).ok());
    let outcome = if fatal.is_some() || history.len() != 24 || final_three.is_none() {
        if fatal.is_none() {
            fatal = Some("incomplete month history or final report".into());
        }
        None
    } else {
        Some(b15::ScriptOutcome {
            id: script.id(),
            choice: script.choice,
            metrics: b15::Metrics {
                attainment_month: if achieved { first } else { None },
                guardrail_months: guards,
                guardrail_union_months: union,
                treasury_less_unpaid_future_e: report
                    .as_ref()
                    .ok_or("missing final report")?
                    .treasury_net_energy_liability,
                final_three_recurring: final_three.ok_or("missing final recurring months")?,
            },
        })
    };
    let recurring_sum_for_display = outcome
        .as_ref()
        .and_then(|outcome| outcome.metrics.recurring_sum_for_display().ok());
    let final_scorecards = [Group::A, Group::B]
        .into_iter()
        .map(|g| game.scorecard(g, script.site))
        .collect::<Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(Replay {
        outcome,
        script,
        first_sustained_window: first,
        report,
        execution_error: fatal,
        recurring_sum_for_display,
        failed_actions: failed,
        monthly,
        final_scorecards,
    })
}

fn run() -> std::result::Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut quick = false;
    let mut summary_only = false;
    let mut selected_script = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--quick" => quick = true,
            "--summary-only" => summary_only = true,
            "--script" => selected_script = Some(args.next().ok_or("--script needs an ID")?),
            _ => {
                return Err(
                    "usage: governance_replay [--quick] [--summary-only] [--script ID]".into(),
                );
            }
        }
    }
    let mut replay_count = 0;
    let preparation: Vec<u32> = if quick { vec![1] } else { vec![1, 6, 13] };
    let subsidy: Vec<u64> = if quick { vec![40] } else { vec![0, 20, 40] };
    let renewal: Vec<u32> = if quick { vec![9] } else { vec![9, 12] };
    let order: Vec<u32> = if quick { vec![0] } else { vec![0, 1] };
    let tolerance = b15::Tolerances::proposed(0, 0, b15::MONEY_SCALE / 100, b15::MONEY_SCALE / 100)
        .map_err(|e| format!("{e:?}"))?;
    let base = Game::confirmed_game().map_err(|e| e.to_string())?;
    emit(
        &json!({"kind":"search_domain","selected_script":selected_script,"preparation_months":preparation,"subsidy_currency":subsidy,"renewal_months":renewal,"project_orders":order,"choices":["W","E","Decline"],"project_sequences":{"r1_o0":["Housing","Road","Power","Water","Clinic"],"r1_o1":["Road","Power","Housing","Clinic","Water","Vocational"],"r2_o0":["Vocational","Housing","Clinic","Road","Power","Water"],"r2_o1":["Housing","Clinic","Vocational","Power","Road","Water"]},"horizon":24,"money_scale":b15::MONEY_SCALE,"tolerances":tolerance,"calibration":base.governance().ok_or("missing governance")?.calibration,"global_optimality_claim":false,"retry_policy":"Every uncompleted prescribed action retries monthly; successful IDs are never repeated. Intents only at selected candidate; deadline min(month+4,24). Renewal retries only to M12. Execution errors have no metrics and block the whole B15 group.","route1":"housing/road/power/water/clinic; order1 additionally vocational last; subsidy cap, ordinary energy contract from M3","route2":"vocational/housing/clinic/road/power/water, subsidy cap, cooperation from M2","frontiers":"exact Pareto; tolerance pairwise bidirectional coverage","attainment":"Some only when final M24 grade Achieved; raw first sustained window separately reported"}),
    )?;
    let mut b1_execution_blocked = false;
    let mut best_decline: BTreeMap<(u32, u32), Option<(u32, String)>> = BTreeMap::new();
    for site in [8, 29] {
        for route in [1, 2] {
            let mut outcomes = vec![];
            let mut counts = BTreeMap::new();
            let mut execution_errors = vec![];
            for choice in [b15::Choice::W, b15::Choice::E, b15::Choice::Decline] {
                let mut achieved = 0;
                let mut fatal_count = 0;
                for prepare_month in &preparation {
                    for cap in &subsidy {
                        for renewal_month in &renewal {
                            for project_order in &order {
                                let script = Script {
                                    site,
                                    route,
                                    choice,
                                    prepare_month: *prepare_month,
                                    subsidy_currency: *cap,
                                    renewal_month: *renewal_month,
                                    project_order: *project_order,
                                };
                                if selected_script
                                    .as_ref()
                                    .is_some_and(|id| *id != script.id())
                                {
                                    continue;
                                }
                                replay_count += 1;
                                let result = match replay(&base, script.clone()) {
                                    Ok(result) => result,
                                    Err(error) => {
                                        fatal_count += 1;
                                        b1_execution_blocked |= choice == b15::Choice::Decline;
                                        let failure = json!({"kind":"script_execution_error","script":script,"execution_error":error});
                                        execution_errors.push(failure.clone());
                                        emit(&failure)?;
                                        continue;
                                    }
                                };
                                if result.execution_error.is_some() {
                                    b1_execution_blocked |= choice == b15::Choice::Decline;
                                    fatal_count += 1;
                                }
                                if let Some(outcome) = &result.outcome {
                                    if let Some(month) = outcome.metrics.attainment_month {
                                        achieved += 1;
                                        if choice == b15::Choice::Decline {
                                            let entry =
                                                best_decline.entry((site, route)).or_default();
                                            if entry.as_ref().is_none_or(|old| month < old.0) {
                                                *entry = Some((month, outcome.id.clone()));
                                            }
                                        }
                                    }
                                    outcomes.push(outcome.clone());
                                } else {
                                    execution_errors
                                        .push(json!({"script":result.script,"error":result.execution_error}));
                                }
                                if !summary_only || selected_script.is_some() {
                                    emit(&json!({"kind":"script_replay","replay":result}))?;
                                }
                            }
                        }
                    }
                }
                counts.insert(
                    format!("{choice:?}"),
                    json!({"achieved":achieved,"fatal":fatal_count}),
                );
            }
            if selected_script.is_some() {
                continue;
            }
            if !execution_errors.is_empty() {
                emit(
                    &json!({"kind":"bounded_b15","site":site,"route":route,"status":"blocked_not_evaluated","counts":counts,"execution_errors":execution_errors,"completed_outcomes":outcomes}),
                )?;
                continue;
            }
            let sum = b15::compare(
                &outcomes,
                &tolerance,
                b15::GuardrailAggregation::SumOfThreeCounts,
            )
            .map_err(|e| format!("B15 sum {e:?}"))?;
            let union = b15::compare(
                &outcomes,
                &tolerance,
                b15::GuardrailAggregation::UnionOfMonths,
            )
            .map_err(|e| format!("B15 union {e:?}"))?;
            emit(
                &json!({"kind":"bounded_b15","site":site,"route":route,"scripts":outcomes.len(),"counts":counts,"sum":sum,"union":union,"aggregation_sensitive":sum.pass!=union.pass || sum.witnesses!=union.witnesses || sum.frontiers!=union.frontiers || sum.all_equivalent!=union.all_equivalent,"outcomes":outcomes}),
            )?;
        }
    }
    if selected_script.is_some() {
        if replay_count == 0 {
            return Err("script ID outside disclosed search domain".into());
        }
        return emit(
            &json!({"kind":"single_script_complete","count":replay_count,"b15":"not_evaluated_for_single_script"}),
        );
    }
    let mut b1 = vec![];
    let mut all_reached = true;
    for site in [8, 29] {
        let preferred = if site == 8 { 1 } else { 2 };
        let preferred_best = best_decline.get(&(site, preferred)).cloned().flatten();
        let other_best = best_decline.get(&(site, 3 - preferred)).cloned().flatten();
        let delta = preferred_best
            .as_ref()
            .zip(other_best.as_ref())
            .map(|(a, b)| i64::from(b.0) - i64::from(a.0));
        let pass = delta.is_some_and(|d| d >= 2);
        all_reached &= pass;
        b1.push(json!({"site":site,"preferred_route":preferred,"preferred_best":preferred_best,"other_best":other_best,"preferred_advantage_months":delta,"pass":pass}));
    }
    emit(
        &json!({"kind":"bounded_b1","choice":"Decline","same_disclosed_search_domain":true,"status":if b1_execution_blocked {"blocked_not_evaluated"} else {"evaluated"},"results":b1,"pass":if b1_execution_blocked {None} else {Some(all_reached)},"not_experience_acceptance":true}),
    )
}

fn main() {
    if let Err(error) = run() {
        let _ = emit(&json!({"kind":"driver_error","error":error}));
        std::process::exit(1);
    }
}
