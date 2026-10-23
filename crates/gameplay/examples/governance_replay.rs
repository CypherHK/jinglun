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
    commitment_span: u32,
}

impl Script {
    fn id(&self) -> String {
        let base = format!(
            "s{}-r{}-{:?}-p{}-c{}-m{}-o{}",
            self.site,
            self.route,
            self.choice,
            self.prepare_month,
            self.subsidy_currency,
            self.renewal_month,
            self.project_order
        );
        if self.commitment_span == 4 {
            base
        } else {
            format!("{base}-d{}", self.commitment_span)
        }
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
    government_capital: GovernmentCapital,
    trade_audit: TradeAudit,
}

#[derive(Clone, Debug, Default, Serialize)]
struct CapitalAccount {
    historical_gross_committed: u64,
    actually_paid: u64,
    refunded: u64,
    remaining_escrow: u64,
    net_committed: u64,
}

impl CapitalAccount {
    fn add(&mut self, total: u64, paid: u64, cancelled: bool) -> std::result::Result<(), String> {
        let rest = total
            .checked_sub(paid)
            .ok_or("capital escrow paid exceeds total")?;
        for (field, value) in [
            (&mut self.historical_gross_committed, total),
            (&mut self.actually_paid, paid),
            (&mut self.refunded, if cancelled { rest } else { 0 }),
            (&mut self.remaining_escrow, if cancelled { 0 } else { rest }),
            (
                &mut self.net_committed,
                if cancelled { paid } else { total },
            ),
        ] {
            *field = field
                .checked_add(value)
                .ok_or("government capital overflow")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize)]
struct GovernmentCapital {
    public_works: CapitalAccount,
    construction_subsidies: CapitalAccount,
    direct_construction_fees: u64,
    total: CapitalAccount,
}

#[derive(Clone, Debug, Default, Serialize)]
struct TradeAudit {
    domestic_sales_actual_payment: u64,
    export_sales_actual_payment: u64,
    export_sales_months: BTreeSet<u32>,
}

fn financial_audit(game: &Game) -> std::result::Result<(GovernmentCapital, TradeAudit), String> {
    let snapshot: Value = serde_json::from_slice(&game.save().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let escrows = snapshot
        .pointer("/payload/ledger/escrows")
        .and_then(Value::as_object)
        .ok_or("save lacks escrow audit data")?;
    let mut capital = GovernmentCapital::default();
    let mut trade = TradeAudit::default();
    for (id, escrow) in escrows {
        if escrow["payer"] != "treasury" {
            continue;
        }
        let total = escrow["total"]
            .as_u64()
            .ok_or("invalid capital escrow total")?;
        let paid = escrow["paid"]
            .as_u64()
            .ok_or("invalid capital escrow paid")?;
        let cancelled = escrow["cancelled"]
            .as_bool()
            .ok_or("invalid capital escrow cancellation")?;
        let component = if id.starts_with("public/") {
            &mut capital.public_works
        } else if id.starts_with("subsidy/") {
            &mut capital.construction_subsidies
        } else {
            return Err(format!("unclassified treasury capital escrow {id}"));
        };
        component.add(total, paid, cancelled)?;
        capital.total.add(total, paid, cancelled)?;
    }
    let applied = snapshot
        .pointer("/payload/applied")
        .and_then(Value::as_object)
        .ok_or("save lacks command audit data")?;
    for (command_id, entry) in applied {
        let Some(transfer) = entry.pointer("/command/Transfer") else {
            continue;
        };
        if command_id.starts_with("sales/") && transfer["asset"] == "money" {
            let amount = transfer["amount"].as_u64().ok_or("invalid sales payment")?;
            if transfer["from"] == "foreign_buyer" {
                trade.export_sales_actual_payment = trade
                    .export_sales_actual_payment
                    .checked_add(amount)
                    .ok_or("export sales overflow")?;
                let month = entry
                    .pointer("/receipt/month")
                    .and_then(Value::as_u64)
                    .and_then(|m| u32::try_from(m).ok())
                    .ok_or("invalid export payment month")?;
                trade.export_sales_months.insert(month);
            } else if transfer["from"] == "households" {
                trade.domestic_sales_actual_payment = trade
                    .domestic_sales_actual_payment
                    .checked_add(amount)
                    .ok_or("domestic sales overflow")?;
            }
        }
        if transfer["from"] == "treasury"
            && transfer["to"] == "construction"
            && transfer["asset"] == "money"
        {
            let amount = transfer["amount"]
                .as_u64()
                .ok_or("invalid direct construction payment")?;
            capital.direct_construction_fees = capital
                .direct_construction_fees
                .checked_add(amount)
                .ok_or("construction fee overflow")?;
            capital.total.add(amount, amount, false)?;
        }
    }
    Ok((capital, trade))
}

#[derive(Clone, Debug, Serialize)]
struct CapitalOutcome {
    script_id: String,
    choice: b15::Choice,
    site: u32,
    route: u32,
    attainment_month: u32,
    capital: GovernmentCapital,
}

fn capital_frontier(outcomes: &[CapitalOutcome]) -> Vec<CapitalOutcome> {
    outcomes
        .iter()
        .filter(|candidate| {
            !outcomes.iter().any(|other| {
                other.attainment_month <= candidate.attainment_month
                    && other.capital.total.net_committed <= candidate.capital.total.net_committed
                    && (other.attainment_month < candidate.attainment_month
                        || other.capital.total.net_committed
                            < candidate.capital.total.net_committed)
            })
        })
        .cloned()
        .collect()
}

fn route_capital_summary(outcomes: &[CapitalOutcome]) -> Value {
    let fastest = outcomes.iter().map(|o| o.attainment_month).min();
    let cheapest = outcomes.iter().map(|o| o.capital.total.net_committed).min();
    let fastest_capital = outcomes
        .iter()
        .filter(|o| Some(o.attainment_month) == fastest)
        .map(|o| o.capital.total.net_committed)
        .min();
    let cheapest_month = outcomes
        .iter()
        .filter(|o| Some(o.capital.total.net_committed) == cheapest)
        .map(|o| o.attainment_month)
        .min();
    json!({"achieved_count":outcomes.len(),"fastest_month":fastest,"minimum_net_committed":cheapest,
        "fastest_then_cheapest":outcomes.iter().filter(|o|Some(o.attainment_month)==fastest && Some(o.capital.total.net_committed)==fastest_capital).collect::<Vec<_>>(),
        "cheapest_then_fastest":outcomes.iter().filter(|o|Some(o.capital.total.net_committed)==cheapest && Some(o.attainment_month)==cheapest_month).collect::<Vec<_>>(),
        "exact_pareto_frontier":capital_frontier(outcomes)})
}

fn b1_comparison(outcomes: &[CapitalOutcome], choice: b15::Choice, site: u32) -> (Value, bool) {
    let routes: Vec<Vec<_>> = [1, 2]
        .into_iter()
        .map(|route| {
            outcomes
                .iter()
                .filter(|o| o.choice == choice && o.site == site && o.route == route)
                .cloned()
                .collect()
        })
        .collect();
    let fronts: Vec<_> = routes.iter().map(|r| capital_frontier(r)).collect();
    let mut pairs = vec![];
    let mut counterexamples = vec![];
    for r1 in &fronts[0] {
        for r2 in &fronts[1] {
            if r1.attainment_month.saturating_add(2) <= r2.attainment_month
                && r2.capital.total.net_committed < r1.capital.total.net_committed
            {
                pairs.push(json!({"route1":r1,"route2":r2}));
            }
            if r2.attainment_month <= r1.attainment_month
                && r2.capital.total.net_committed <= r1.capital.total.net_committed
            {
                counterexamples.push(json!({"route1":r1,"route2":r2,"relation":"route2_no_slower_and_no_more_capital"}));
            }
            if r1.attainment_month <= r2.attainment_month
                && r1.capital.total.net_committed <= r2.capital.total.net_committed
            {
                counterexamples.push(json!({"route1":r1,"route2":r2,"relation":"route1_no_slower_and_no_more_capital"}));
            }
        }
    }
    let fastest: Vec<_> = routes
        .iter()
        .map(|r| r.iter().map(|o| o.attainment_month).min())
        .collect();
    let cheapest: Vec<_> = routes
        .iter()
        .map(|r| r.iter().map(|o| o.capital.total.net_committed).min())
        .collect();
    let faster = fastest[0]
        .zip(fastest[1])
        .is_some_and(|(a, b)| a.saturating_add(2) <= b);
    let cheaper = cheapest[0].zip(cheapest[1]).is_some_and(|(a, b)| b < a);
    fn canonical_fastest(route: &[CapitalOutcome]) -> Option<&CapitalOutcome> {
        route.iter().min_by_key(|o| {
            (
                o.attainment_month,
                o.capital.total.net_committed,
                o.script_id.clone(),
            )
        })
    }
    fn canonical_cheapest(route: &[CapitalOutcome]) -> Option<&CapitalOutcome> {
        route.iter().min_by_key(|o| {
            (
                o.capital.total.net_committed,
                o.attainment_month,
                o.script_id.clone(),
            )
        })
    }
    let selected_r1 = canonical_fastest(&routes[0]);
    let selected_r2_cheap = canonical_cheapest(&routes[1]);
    let selected_r2_fast = canonical_fastest(&routes[1]);
    let pair_pass = |a: Option<&CapitalOutcome>, b: Option<&CapitalOutcome>| {
        a.zip(b).is_some_and(|(a, b)| {
            a.attainment_month.saturating_add(2) <= b.attainment_month
                && b.capital.total.net_committed < a.capital.total.net_committed
        })
    };
    let fastest_vs_cheapest_pass = pair_pass(selected_r1, selected_r2_cheap);
    let both_fastest_pass = pair_pass(selected_r1, selected_r2_fast);
    let pass = fastest_vs_cheapest_pass;
    (
        json!({"site":site,"choice":choice,"route1":route_capital_summary(&routes[0]),"route2":route_capital_summary(&routes[1]),"route1_fastest_at_least_two_months_earlier":faster,"route2_minimum_capital_strictly_lower":cheaper,"efficient_fast_expensive_slow_cheap_pairs":pairs,"counterexamples":counterexamples,"fastest_vs_cheapest_pass":fastest_vs_cheapest_pass,"both_fastest_diagnostic_pass":both_fastest_pass,"stronger_independent_minima_diagnostic":faster && cheaper && !pairs.is_empty()}),
        pass,
    )
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
                        deadline: month.saturating_add(script.commitment_span).min(24),
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
        let factories: Vec<_> = state.factories.values().map(|f|json!({"id":f.id,"group":f.group,"site":f.site,"phase":f.phase,"start":f.started_month,"activate":f.activate_month,"output":f.output,"after_tax_profit":f.after_tax_profit,"operating_subsidy":f.operating_subsidy,"water_suspended":f.water_suspended,"first_operated_month":f.first_operated_month})).collect();
        monthly.push(json!({"record":record,"group_cash":{"A":game.balance(Group::A.account(), &Asset::Money),"B":game.balance(Group::B.account(), &Asset::Money)},"factories":factories,"candidate_site":state.sites.get(&script.site),"unused_energy_bill":state.last_energy_unused_bill,"projects":state.projects.values().collect::<Vec<_>>()}));
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
    let (government_capital, trade_audit) = financial_audit(&game)?;
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
        government_capital,
        trade_audit,
    })
}

fn run() -> std::result::Result<u8, String> {
    let mut args = std::env::args().skip(1);
    let mut quick = false;
    let mut summary_only = false;
    let mut selected_script = None;
    let mut water_share = None;
    let mut skilled_idle_share = None;
    let mut export_orders = None;
    let mut commitment_span = 4_u32;
    let mut require_acceptance = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--quick" => quick = true,
            "--summary-only" => summary_only = true,
            "--require-acceptance" => require_acceptance = true,
            "--script" => selected_script = Some(args.next().ok_or("--script needs an ID")?),
            "--skilled-idle-share" => {
                skilled_idle_share = Some(
                    args.next()
                        .ok_or("--skilled-idle-share needs a ratio")?
                        .parse::<f64>()
                        .map_err(|e| e.to_string())?,
                )
            }
            "--commitment-span" => {
                commitment_span = args
                    .next()
                    .ok_or("--commitment-span needs months")?
                    .parse::<u32>()
                    .map_err(|e| e.to_string())?
            }
            "--export-orders" => {
                export_orders = Some(
                    args.next()
                        .ok_or("--export-orders needs a quantity")?
                        .parse::<u32>()
                        .map_err(|e| e.to_string())?,
                )
            }
            "--water-share" => {
                water_share = Some(
                    args.next()
                        .ok_or("--water-share needs a ratio")?
                        .parse::<f64>()
                        .map_err(|e| e.to_string())?,
                )
            }
            _ => {
                return Err(
                    "usage: governance_replay [--quick] [--summary-only] [--script ID] [--water-share RATIO] [--require-acceptance] [--skilled-idle-share RATIO] [--export-orders QUANTITY] [--commitment-span MONTHS]".into(),
                );
            }
        }
    }
    if require_acceptance && selected_script.is_some() {
        emit(
            &json!({"kind":"acceptance_result","status":"not_evaluated","reason":"single-script mode cannot evaluate aggregate B1/B15 acceptance","exit_code":2}),
        )?;
        return Ok(2);
    }
    let mut replay_count = 0;
    let mut all_b15_pass = true;
    let mut b15_aggregation_pending = false;
    let mut b15_execution_blocked = false;
    let preparation: Vec<u32> = if quick { vec![1] } else { vec![1, 6, 13] };
    let subsidy: Vec<u64> = if quick { vec![40] } else { vec![0, 20, 40] };
    let renewal: Vec<u32> = if quick { vec![9] } else { vec![9, 12] };
    let order: Vec<u32> = if quick { vec![0] } else { vec![0, 1] };
    let tolerance = b15::Tolerances::proposed(0, 0, b15::MONEY_SCALE / 100, b15::MONEY_SCALE / 100)
        .map_err(|e| format!("{e:?}"))?;
    let mut calibration = GovernanceCalibration::default();
    if let Some(share) = water_share {
        calibration.water_priority_fraction = share;
    }
    if let Some(share) = skilled_idle_share {
        calibration.initial_skilled_idle_share = share;
    }
    if let Some(orders) = export_orders {
        calibration.cooperation_export_orders_per_month = orders;
        let document = initial_values::confirmed_document().map_err(|e| e.to_string())?;
        let unit_price = money(
            initial_values::number(&document, "/parameters/goods_price")
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        calibration.foreign_buyer_monthly_budget = unit_price
            .checked_mul(u64::from(orders))
            .ok_or("export buyer budget overflow")?;
    }
    let base = Game::confirmed_game_with_calibration(calibration).map_err(|e| e.to_string())?;
    emit(
        &json!({"kind":"search_domain","require_acceptance":require_acceptance,"selected_script":selected_script,"pending_water_share_override":water_share,"pending_skilled_idle_share_override":skilled_idle_share,"pending_export_orders_override":export_orders,"export_budget_policy":"When export quantity is overridden, foreign buyer monthly budget is exactly quantity times D022 confirmed goods unit price in fixed money units.","preparation_months":preparation,"subsidy_currency":subsidy,"renewal_months":renewal,"project_orders":order,"choices":["W","E","Decline"],"project_sequences":{"r1_o0":["Housing","Road","Power","Water","Clinic"],"r1_o1":["Road","Power","Housing","Clinic","Water","Vocational"],"r2_o0":["Vocational","Housing","Clinic","Road","Power","Water"],"r2_o1":["Housing","Clinic","Vocational","Power","Road","Water"]},"commitment_span":commitment_span,"horizon":24,"money_scale":b15::MONEY_SCALE,"tolerances":tolerance,"calibration":base.governance().ok_or("missing governance")?.calibration,"global_optimality_claim":false,"retry_policy":"Every uncompleted prescribed action retries monthly; successful IDs are never repeated. Intents only at selected candidate; deadline min(month+commitment_span,24). Renewal retries only to M12. Execution errors have no metrics and block the whole B15 group.","route1":"housing/road/power/water/clinic; order1 additionally vocational last; subsidy cap, ordinary energy contract from M3","route2":"vocational/housing/clinic/road/power/water, subsidy cap, cooperation from M2","frontiers":"exact Pareto; tolerance pairwise bidirectional coverage","attainment":"Some only when final M24 grade Achieved; raw first sustained window separately reported"}),
    )?;
    let mut b1_execution_blocked = false;
    let mut capital_outcomes = vec![];
    for site in [8, 29] {
        for route in [1, 2] {
            let mut outcomes = vec![];
            let mut capital_records = vec![];
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
                                    commitment_span,
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
                                        b1_execution_blocked = true;
                                        let failure = json!({"kind":"script_execution_error","script":script,"execution_error":error});
                                        execution_errors.push(failure.clone());
                                        emit(&failure)?;
                                        continue;
                                    }
                                };
                                capital_records.push(json!({"script_id":result.script.id(),"choice":choice,"capital":result.government_capital,"trade_audit":result.trade_audit,"execution_error":result.execution_error}));
                                if result.execution_error.is_some() {
                                    b1_execution_blocked = true;
                                    fatal_count += 1;
                                }
                                if let Some(outcome) = &result.outcome {
                                    if let Some(month) = outcome.metrics.attainment_month {
                                        achieved += 1;
                                        capital_outcomes.push(CapitalOutcome {
                                            script_id: outcome.id.clone(),
                                            choice,
                                            site,
                                            route,
                                            attainment_month: month,
                                            capital: result.government_capital.clone(),
                                        });
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
                all_b15_pass = false;
                b15_execution_blocked = true;
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
            let aggregation_sensitive = sum.pass != union.pass
                || sum.witnesses != union.witnesses
                || sum.frontiers != union.frontiers
                || sum.all_equivalent != union.all_equivalent;
            all_b15_pass &= sum.pass && union.pass;
            b15_aggregation_pending |= aggregation_sensitive;
            emit(
                &json!({"kind":"bounded_b15","site":site,"route":route,"scripts":outcomes.len(),"counts":counts,"sum":sum,"union":union,"aggregation_sensitive":aggregation_sensitive,"outcomes":outcomes,"government_capital_records":capital_records,"government_capital_outcomes":capital_outcomes.iter().filter(|o|o.site==site && o.route==route).collect::<Vec<_>>()}),
            )?;
        }
    }
    if selected_script.is_some() {
        if replay_count == 0 {
            return Err("script ID outside disclosed search domain".into());
        }
        emit(
            &json!({"kind":"single_script_complete","count":replay_count,"b15":"not_evaluated_for_single_script"}),
        )?;
        return Ok(0);
    }
    let mut b1 = vec![];
    let mut all_reached = true;
    for choice in [b15::Choice::Decline, b15::Choice::W, b15::Choice::E] {
        for site in [8, 29] {
            let (comparison, pass) = b1_comparison(&capital_outcomes, choice, site);
            if choice == b15::Choice::Decline {
                all_reached &= pass;
            }
            b1.push(comparison);
        }
    }
    let eight_fastest = capital_outcomes
        .iter()
        .filter(|o| o.site == 8 && o.choice == b15::Choice::Decline)
        .map(|o| o.attainment_month)
        .min();
    let eight_w = capital_outcomes
        .iter()
        .any(|o| o.site == 8 && o.choice == b15::Choice::W);
    let twenty_nine_w = capital_outcomes
        .iter()
        .any(|o| o.site == 29 && o.choice == b15::Choice::W);
    all_reached &= eight_fastest.is_some_and(|month| month <= 20) && eight_w && !twenty_nine_w;
    emit(
        &json!({"kind":"bounded_b1","decision":"D026","comparison_policy_status":"chat_confirmed_2026-10-24","selection_confirmation":"User confirmed in current chat: route1 fastest, route2 cheapest, disclose all frontiers. R1 ties use minimum net capital; R2 ties use earliest attainment.","primary_choice":"Decline","same_disclosed_search_domain":true,"capital_metric":"net committed = actual paid + remaining escrow; historical gross and refunds separately retained","selection_policy":"Primary Decline: R1 earliest attained then minimum net capital; R2 minimum net capital attained within M24 then earliest. Full exact month/capital frontiers, alternate fastest-vs-fastest diagnosis and counterexamples disclosed; no dominated script establishes a pair.","status":if b1_execution_blocked {"blocked_not_evaluated"} else {"evaluated_under_proposed_policy"},"results":b1,"eight_fastest_month":eight_fastest,"eight_fastest_by_m20":eight_fastest.is_some_and(|month|month<=20),"eight_w_achieved":eight_w,"twenty_nine_w_achieved":twenty_nine_w,"pass":if b1_execution_blocked {None} else {Some(all_reached)},"not_experience_acceptance":true}),
    )?;
    let passed = all_reached && !b1_execution_blocked && all_b15_pass && !b15_aggregation_pending;
    let status = if b1_execution_blocked || b15_execution_blocked {
        "blocked_not_evaluated"
    } else if b15_aggregation_pending {
        "pending_guardrail_aggregation"
    } else if passed {
        "passed"
    } else {
        "failed"
    };
    let exit_code = if require_acceptance && !passed { 2 } else { 0 };
    emit(
        &json!({"kind":"acceptance_result","status":status,"b1_pass":if b1_execution_blocked {None} else {Some(all_reached)},"b1_selection_policy":"chat_confirmed_route1_fastest_route2_cheapest","all_b15_pass":all_b15_pass,"aggregation_pending":b15_aggregation_pending,"exit_code":exit_code,"not_experience_acceptance":true}),
    )?;
    Ok(exit_code)
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(i32::from(code)),
        Err(error) => {
            let _ = emit(&json!({"kind":"driver_error","error":error}));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod driver_tests {
    use super::*;

    fn outcome(id: &str, route: u32, month: u32, capital: u64) -> CapitalOutcome {
        let mut account = CapitalAccount::default();
        account.add(capital, capital, false).unwrap();
        CapitalOutcome {
            script_id: id.into(),
            choice: b15::Choice::Decline,
            site: 8,
            route,
            attainment_month: month,
            capital: GovernmentCapital {
                total: account,
                ..GovernmentCapital::default()
            },
        }
    }

    #[test]
    fn commitment_span_is_explicit_in_nondefault_script_identity() {
        let mut script = Script {
            site: 8,
            route: 1,
            choice: b15::Choice::Decline,
            prepare_month: 1,
            subsidy_currency: 20,
            renewal_month: 9,
            project_order: 1,
            commitment_span: 4,
        };
        assert_eq!(script.id(), "s8-r1-Decline-p1-c20-m9-o1");
        script.commitment_span = 8;
        assert_eq!(script.id(), "s8-r1-Decline-p1-c20-m9-o1-d8");
    }

    #[test]
    fn capital_refunds_are_not_counted_as_actual_payment() {
        let mut account = CapitalAccount::default();
        account.add(100, 30, true).unwrap();
        account.add(50, 10, false).unwrap();
        assert_eq!(account.historical_gross_committed, 150);
        assert_eq!(account.actually_paid, 40);
        assert_eq!(account.refunded, 70);
        assert_eq!(account.remaining_escrow, 40);
        assert_eq!(account.net_committed, 80);
    }

    #[test]
    fn dominated_expensive_route_cannot_manufacture_a_cheap_route_pair() {
        let outcomes = vec![
            outcome("r1-fast-cheap", 1, 10, 20),
            outcome("r1-slow-wasteful", 1, 14, 30),
            outcome("r2", 2, 16, 25),
        ];
        let (report, pass) = b1_comparison(&outcomes, b15::Choice::Decline, 8);
        assert!(!pass);
        assert!(
            report["efficient_fast_expensive_slow_cheap_pairs"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            report["route1"]["exact_pareto_frontier"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(!report["counterexamples"].as_array().unwrap().is_empty());
    }

    #[test]
    fn reports_both_selection_policies_without_hiding_fast_route_two() {
        let outcomes = vec![
            outcome("r1", 1, 10, 20),
            outcome("r2-fast", 2, 11, 10),
            outcome("r2-cheap", 2, 14, 5),
        ];
        let (report, _) = b1_comparison(&outcomes, b15::Choice::Decline, 8);
        assert_eq!(report["fastest_vs_cheapest_pass"], true);
        assert_eq!(report["both_fastest_diagnostic_pass"], false);
        assert_eq!(
            report["route2"]["exact_pareto_frontier"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
