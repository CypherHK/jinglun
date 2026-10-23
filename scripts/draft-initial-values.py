#!/usr/bin/env python3
"""Generate an inactive D017/D018/D020 arithmetic proposal. No Game initialization imports this file."""
import argparse
import hashlib
import json
import math
from fractions import Fraction
from pathlib import Path

STATUS = 'pending_user_confirmation'
PLAYER = [8, 9, 26, 29, 38, 39, 63, 74, 83, 91]
DESIGN_FILES = ['DECISIONS.md', 'handoffs/2026-10-23-first-governance-slice.md']


def value(number, unit, principle, formula, inputs, origin='derived_proposal'):
    return dict(value=number, unit=unit, principle=principle, formula=formula,
                inputs=inputs, origin=origin, status=STATUS, activation='disabled',
                assumption='Inactive arithmetic proposal; listed inputs and formulas hold only under the section assumptions. No runtime rule or accepted calibration is implied.')


def group(**items):
    return dict(status=STATUS, activation='disabled', **items)


def generate(map_path, design_dir):
    raw = map_path.read_bytes()
    world = json.loads(raw)
    assert world['config']['seed'] == 'jinglun-040'
    assert len(world['cells']) == 100
    cells = {c['id']: c for c in world['cells']}
    assert len(cells) == 100 and all(i in cells for i in PLAYER)
    assert hashlib.sha256(raw).hexdigest() == '4c8714aaca565e0c4b932420aa320cb92a901ef63c29a897e507540db4de37a8', 'Map differs from calibrated evidence'
    expected_design = {
        'DECISIONS.md': 'fb7ae18e12f99b6fe653d4b056f9d175d8994dc3420f39e3fe915e89ddeffa66',
        'handoffs/2026-10-23-first-governance-slice.md': '120a5638035b90cd31101ae312611d92cfca23c2b38056540167db2bab66e5f1'}
    for name, expected in expected_design.items():
        assert hashlib.sha256((design_dir/name).read_bytes()).hexdigest() == expected, 'Design source changed: '+name
    parameters = {}

    def parameter(name, number, unit, principle, reason):
        parameters[name] = value(number, unit, principle, 'proposed free parameter',
                                 {'reason': reason}, 'proposed_free_parameter')
        return number

    total = parameter('total_population', 10000, 'person', 'D017.1', 'Adjustable gameplay scale, not real map population.')
    boost = parameter('capital_weight_multiplier', 2, 'ratio', 'D017.1', 'Public capital weight, gives approximately forty percent.')
    factory_output = parameter('new_factory_output', 100, 'goods/month', 'D017.2', 'Reference factory scale.')
    labor_need = parameter('new_factory_workers', 100, 'person', 'D017.5', 'Reference factory labor need.')
    labor_share = parameter('labor_share', 0.6, 'ratio', 'D017.5', 'Proposed working population share.')
    idle_share = parameter('idle_labor_share', 0.05, 'ratio', 'D017.5', 'Unemployed share of labor force.')
    wage = parameter('monthly_wage', 0.02, 'currency/person/month', 'D017.3', 'Existing employment pays household income.')
    goods_budget_share = parameter('goods_budget_share', 0.6, 'ratio', 'D017.3', 'Household income fraction available for manufactured goods.')
    price = parameter('goods_price', 0.1, 'currency/goods', 'D017.3', 'Fixed proposal price.')
    base = parameter('factory_base_capital', 40, 'currency', 'D017.7;D010', 'Starting point from v2, not approved.')
    buffer = parameter('operating_cash_buffer', 10, 'currency', 'D017.7', 'Working cash remains after construction.')
    cash_a = parameter('group_a_starting_cash', 50, 'currency', 'D017.7', 'Opening equity endowment proposal, not monthly injection.')
    cash_b = parameter('group_b_starting_cash', 55, 'currency', 'D017.7', 'Opening equity endowment proposal, not monthly injection.')
    weights = {i: Fraction(str(cells[i]['construction']['natural_support_index'])) * (boost if i == 39 else 1) for i in PLAYER}
    quotas = {i: total * w / sum(weights.values()) for i, w in weights.items()}
    population = {i: math.floor(q) for i, q in quotas.items()}
    for i in sorted(PLAYER, key=lambda i: (-(quotas[i] - population[i]), i))[:total - sum(population.values())]:
        population[i] += 1
    regions = {}
    for i in PLAYER:
        p = population[i]
        labor = math.floor(Fraction(p) * Fraction(str(labor_share)))
        idle = math.floor(Fraction(labor) * Fraction(str(idle_share)))
        output = 3 * factory_output if i == 39 else 1.25 * factory_output if i == 74 else 0
        regions[str(i)] = group(
            natural_support_index=value(cells[i]['construction']['natural_support_index'], 'index', 'D017.1', f'map.cells[id={i}].construction.natural_support_index', {'raw_cell_id': i}, 'natural_map'),
            population=value(p, 'person', 'D017.1', 'floor(total*weight/sum(weights)) plus deterministic largest remainder; ties ascending raw ID', {'total': total, 'weight': float(weights[i]), 'all_weights': {str(k): float(v) for k, v in weights.items()}}),
            labor_force=value(labor, 'person', 'D017.5', 'floor(population*labor_share)', {'population': p, 'labor_share': labor_share}),
            non_labor=value(p-labor, 'person', 'D017.5', 'population-labor_force', {'population': p, 'labor_force': labor}),
            idle=value(idle, 'person', 'D020.4;D017.5', 'floor(labor_force*idle_labor_share); no absolute cap', {'labor_force': labor, 'idle_labor_share': idle_share}),
            employed=value(labor-idle, 'person', 'D017.5', 'labor_force-idle', {'labor_force': labor, 'idle': idle}),
            manufacturing_output=value(output, 'goods/month', 'D017.2', '39:3*new_factory_output;74:1.25*new_factory_output;others:0', {'new_factory_output': factory_output}),
        )
    employed = sum(r['employed']['value'] for r in regions.values())
    domestic_output = sum(r['manufacturing_output']['value'] for r in regions.values())
    imports = 2 * factory_output
    income = employed * wage
    budget = income * goods_budget_share
    demand = domestic_output + imports
    m = lambda n, u, p, f, **inputs: value(n, u, p, f, inputs, 'proposed_free_parameter' if f.startswith('proposed') else 'derived_proposal')
    demand_group = group(
        wage_income=m(income, 'currency/month', 'D017.3', 'sum(employed)*monthly_wage', employed=employed, monthly_wage=wage),
        goods_budget=m(budget, 'currency/month', 'D017.3', 'wage_income*goods_budget_share', wage_income=income, goods_budget_share=goods_budget_share),
        payable_demand_ceiling=m(budget/price, 'goods/month', 'D017.3', 'goods_budget/goods_price', goods_budget=budget, goods_price=price),
        initial_domestic_orders=m(demand, 'goods/month', 'D017.3', 'existing_domestic_output+initial_imports; must be <= payable ceiling', existing_domestic_output=domestic_output, initial_imports=imports),
        initial_imports=m(imports, 'goods/month', 'D017.3', '2*new_factory_output', new_factory_output=factory_output),
        initial_export_orders=m(0, 'goods/month', 'D017.3', 'zero until future cooperation, no gifted orders'),
        assumption='Existing services and manufacturing pay all recorded wages. Sector balance sheets and external seller accounts are not built. Household purchases have a finite income budget. Import substitution displaces at most initial_imports; demand ceiling is not an order guarantee.')
    ordinary = parameter('ordinary_energy_price', 0.1, 'currency/energy', 'D017.4', 'Resource-country ordinary contract.')
    sea = parameter('maritime_energy_price', 0.135, 'currency/energy', 'D017.4', 'Sea alternative costs more.')
    long_price = parameter('long_energy_price', 0.17, 'currency/energy', 'D017.4;D018', 'Renewal costs more than sea.')
    factory_energy = parameter('new_factory_energy', 10, 'energy/month', 'D017.4', 'Separate incremental requirement.')
    service_energy = parameter('existing_service_energy', 20, 'energy/month', 'D017.4', 'Existing nonmanufacturing demand.')
    energy_need = service_energy + domestic_output/factory_output*factory_energy
    energy = group(ordinary_price=parameters['ordinary_energy_price'], maritime_price=parameters['maritime_energy_price'], long_price=parameters['long_energy_price'],
                   existing_supply=m(energy_need, 'energy/month', 'D017.4', 'existing_demand; ordinary resource-country imports, no spare supply', existing_demand=energy_need),
                   existing_demand=m(energy_need, 'energy/month', 'D017.4', 'existing_service_energy+existing_output/new_factory_output*new_factory_energy', existing_service_energy=service_energy, existing_output=domestic_output, new_factory_output=factory_output, new_factory_energy=factory_energy),
                   incremental_factory_demand=parameters['new_factory_energy'],
                   renewal_quantity=m(2*factory_energy, 'energy/month', 'D017.4;D018', '2*new_factory_energy; draft only, no contract execution', new_factory_energy=factory_energy))
    tax_rate = parameter('manufacturing_sales_tax_rate', 0.12, 'ratio', 'D020.1;D017.6', 'Proposed gross sales tax jointly calibrated with low payroll rate to put gap/spending in 20–25%. No profit tax.')
    payroll_rate = parameter('payroll_tax_rate', 0.01, 'ratio', 'D020.1;D017.6', 'Proposed household gross-wage withholding, not an additional employer charge; jointly calibrated with sales tax.')
    factory_sales_tax = factory_output*price*tax_rate
    factory_payroll_tax = labor_need*wage*payroll_rate
    factory_tax = factory_sales_tax+factory_payroll_tax
    taxes = domestic_output*price*tax_rate + income*payroll_rate
    gap = 1.3*factory_tax
    spending = taxes+gap
    aid = parameter('monthly_transition_aid', gap, 'currency/month', 'D020.1;D017.6', 'Set equal to structural gap: zero baseline aid-period recurring balance; external payer not implemented.')
    fiscal = group(
        reference_factory_sales_tax=m(factory_sales_tax, 'currency/month', 'D017.6', 'new_factory_output*goods_price*manufacturing_sales_tax_rate', new_factory_output=factory_output, goods_price=price, manufacturing_sales_tax_rate=tax_rate),
        reference_factory_incremental_payroll_tax=m(factory_payroll_tax, 'currency/month', 'D017.6', 'new_factory_workers*monthly_wage*payroll_tax_rate; all reference wages assumed net new taxable income', new_factory_workers=labor_need, monthly_wage=wage, payroll_tax_rate=payroll_rate),
        reference_factory_tax=m(factory_tax, 'currency/month', 'D017.6', 'reference_factory_sales_tax+reference_factory_incremental_payroll_tax', reference_factory_sales_tax=factory_sales_tax, reference_factory_incremental_payroll_tax=factory_payroll_tax),
        reference_factory_tax_assumption='Pending proposal. All 100 reference factory workers previously had no taxable wages, so their wages are entirely net new taxable income. Recruit only domestic previously idle workers, available in aggregate; migration and employment displacement are not simulated. This does not claim that any single region has enough idle workers. Both the sales tax and payroll tax proposal require user confirmation.',
        existing_tax_revenue=m(taxes, 'currency/month', 'D017.6', 'existing_output*goods_price*manufacturing_sales_tax_rate+wage_income*payroll_tax_rate', existing_output=domestic_output, goods_price=price, manufacturing_sales_tax_rate=tax_rate, wage_income=income, payroll_tax_rate=payroll_rate),
        recurring_spending=m(spending, 'currency/month', 'D017.6', 'existing_tax_revenue+1.3*reference_factory_tax', existing_tax_revenue=taxes, reference_factory_tax=factory_tax),
        post_aid_gap=m(gap, 'currency/month', 'D017.6', 'recurring_spending-existing_tax_revenue', recurring_spending=spending, existing_tax_revenue=taxes),
        aid=parameters['monthly_transition_aid'],
        aid_period_balance=m(aid-gap, 'currency/month', 'D017.6', 'monthly_transition_aid-post_aid_gap', monthly_transition_aid=aid, post_aid_gap=gap),
        spending_origin='Proposed existing recurring obligations envelope, apportioned below. No unfunded recurrent injection. Aid payer is an unbuilt external resource-country account.',
        spending_breakdown=group(**{name:m(spending*share, 'currency/month', 'D017.6', 'recurring_spending*proposed_share', recurring_spending=spending, proposed_share=share) for name,share in [('health',.3),('education',.25),('safety',.25),('maintenance',.2)]}))
    capital = {}
    for i in (8,29):
        cost = cells[i]['construction']['cost_multiplier']
        cap = base*cost
        capital[str(i)] = group(natural_cost=value(cost,'ratio','D017.7;D010',f'map.cells[id={i}].construction.cost_multiplier',{'raw_cell_id':i},'natural_map'),
            factory_capital=m(cap,'currency','D017.7;D010','factory_base_capital*natural_cost',factory_base_capital=base,natural_cost=cost),
            cash_gaps=group(**{name:m(cap+buffer-cash,'currency','D017.7','factory_capital+operating_cash_buffer-group_starting_cash',factory_capital=cap,operating_cash_buffer=buffer,group_starting_cash=cash) for name,cash in [('group_a',cash_a),('group_b',cash_b)]}))
    food_rate = parameter('food_consumption_per_person', .001, 'food/person/month', 'D017.8', 'Fixed population coverage arithmetic, migration not simulated.')
    food_price = parameter('food_price', .02, 'currency/food', 'D017.8', 'Background household food purchase price.')
    consumption = total*food_rate
    food = group(initial_inventory=m(consumption*2,'food','D017.8','2*monthly_consumption',monthly_consumption=consumption),
        background_imports=m(consumption,'food/month','D017.8','monthly_consumption',monthly_consumption=consumption),
        monthly_consumption=m(consumption,'food/month','D017.8','total_population*food_consumption_per_person',total_population=total,food_consumption_per_person=food_rate),
        horizon_balance=m(consumption*2,'food','D017.8','initial_inventory+24*(background_imports-monthly_consumption)',initial_inventory=consumption*2,background_imports=consumption,monthly_consumption=consumption),
        monthly_import_bill=m(consumption*food_price,'currency/month','D017.8','background_imports*food_price',background_imports=consumption,food_price=food_price),
        assumption='Opening inventory is already-paid household property. Households pay stable maritime food sellers from wage income outside the goods budget. External seller and settlement accounts are not implemented. Coverage assumes fixed population and uninterrupted imports, not a simulation result.')
    material = group(
        initial_inventory=m(12,'material','D017.9','proposed opening paid capital inventory',proposal=12),
        capital_monthly_output=m(6,'material/month','D017.9','proposed existing capital production',proposal=6),
        monthly_imports=m(2,'material/month','D017.9','proposed stable external supply',proposal=2),
        reference_project_total=m(12,'material/project','D017.9','monthly_use*duration',monthly_use=4,duration=3),
        reference_project_monthly_use=m(4,'material/project/month','D017.9','reference_project_total/duration',reference_project_total=12,duration=3),
        reference_project_duration=m(3,'month','D017.9','proposed comparison project duration',proposal=3),
        sustainable_parallel_projects=m(2,'project','D017.9','floor((capital_monthly_output+monthly_imports)/reference_project_monthly_use)',capital_monthly_output=6,monthly_imports=2,reference_project_monthly_use=4),
        assumption='Materials are paid from each future project capital budget, not a free treasury flow. Supplier prices and external accounts remain unbuilt. Two reference projects use the monthly flow exactly. Three projects burn four inventory units per month and exhaust the opening buffer after three months. Inventory is finite; heterogeneous projects require later calibration.')
    draft = group(schema='jinglun.initial-values-draft.v2', scenario='jinglun-040',
        provenance=dict(map_sha256=hashlib.sha256(raw).hexdigest(),generator_version=world['generator_version'],map_config=world['config'],raw_player_cell_ids=PLAYER,raw_capital_cell_id=39,raw_candidate_cell_ids=[8,29],
            source_hashes={name:hashlib.sha256((design_dir/name).read_bytes()).hexdigest() for name in DESIGN_FILES},
            design_input_requirement='External jinglun-design/design directory must contain the relative source_hashes paths. Supply --design-dir when the sibling directory is unavailable. Design files are read only and are not bundled with this product.',
            generator='scripts/draft-initial-values.py',
            generator_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()),
        scope='Arithmetic proposal only. No enterprise, pressure, todo behavior or Game activation. All economic values require user confirmation. Month arithmetic is not gameplay replay: no B1-B15 or experience acceptance claimed.',
        parameters=group(**parameters),regions=group(**regions),demand=demand_group,energy=energy,fiscal=fiscal,candidate_capital=group(**capital),food=food,materials=material)
    return d020_calibration(draft, cells)


def d020_calibration(d, cells):
    """Joint calibration ledger only: deliberately has no runtime/game imports."""
    v = lambda x: x['value']
    p = d['parameters']
    def m(n, unit, principle, formula, **inputs):
        return value(n, unit, principle, formula, inputs)
    def param(name, n, unit, principle, reason):
        p[name] = value(n, unit, principle, 'proposed free parameter',
                        {'reason': reason}, 'proposed_free_parameter')
        return n
    treasury = param('opening_treasury', 55, 'currency', 'D020.1',
                     'Joint feasibility: one maximum subsidy plus works reserve < treasury < distinct-group two-site subsidies; also covers E payments through M24.')
    works_base = param('early_public_works_base_budget', 7.5, 'currency', 'D020.1',
                       'Illustrative package: housing base 4 plus utilities base 3.5; neither is an implemented project price. No road or corridor cost is assumed.')
    works_reserve = param('early_public_works_reserve', 15, 'currency', 'D020.1',
                          'Round above 7.5 times the highest candidate natural cost multiplier; reserve includes headroom, not a recurring expense.')
    other_fixed = param('factory_other_fixed_monthly_cost', 3.6, 'currency/month', 'D020.2',
                        'Proposed fixed non-wage, non-energy services/maintenance expense; calibrated so long-price full-output profit is 15% of gross sales.')
    gap = v(d['fiscal']['post_aid_gap'])
    aid = v(d['fiscal']['aid'])
    spending = v(d['fiscal']['recurring_spending'])
    taxes = v(d['fiscal']['existing_tax_revenue'])
    factory_tax = v(d['fiscal']['reference_factory_tax'])
    output = v(p['new_factory_output'])
    workers = v(p['new_factory_workers'])
    price = v(p['goods_price'])
    wage_bill = workers*v(p['monthly_wage'])
    sales_rate = v(p['manufacturing_sales_tax_rate'])
    energy_per_factory = v(p['new_factory_energy'])
    contract_quantity = v(d['energy']['renewal_quantity'])
    long_price = v(p['long_energy_price'])
    new_tax = lambda n: n*factory_tax

    # Spending here is a reserved illustrative package; it is not spent in the no-action walks.
    packages = {}
    for i in (8, 29):
        natural = cells[i]['construction']['cost_multiplier']
        actual = works_base*natural
        packages[str(i)] = group(
            housing_base=m(4, 'currency', 'D020.1', 'proposed component of early_public_works_base_budget', base_package=works_base),
            utilities_base=m(3.5, 'currency', 'D020.1', 'early_public_works_base_budget-housing_base', early_public_works_base_budget=works_base, housing_base=4),
            natural_multiplier=d['candidate_capital'][str(i)]['natural_cost'],
            natural_adjusted_cost=m(actual, 'currency', 'D020.1;D010', 'early_public_works_base_budget*natural_multiplier', early_public_works_base_budget=works_base, natural_multiplier=natural),
            reserve_headroom=m(works_reserve-actual, 'currency', 'D020.1', 'early_public_works_reserve-natural_adjusted_cost', early_public_works_reserve=works_reserve, natural_adjusted_cost=actual))
    maximum_subsidy = max(v(d['candidate_capital'][str(i)]['cash_gaps'][g]) for i in (8,29) for g in ('group_a','group_b'))
    # One balance per group: the same opening cash cannot finance two first factories.
    allocations = []
    for ga, gb in [('group_a','group_b'), ('group_b','group_a')]:
        a = v(d['candidate_capital']['8']['cash_gaps'][ga])
        b = v(d['candidate_capital']['29']['cash_gaps'][gb])
        allocations.append(group(group_at_8=ga, group_at_29=gb,
            total_subsidy=m(a+b, 'currency', 'D020.1;D017.7', 'subsidy_at_8+subsidy_at_29; use each group opening cash once', subsidy_at_8=a, subsidy_at_29=b),
            treasury_shortfall=m(a+b-treasury, 'currency', 'D020.1', 'total_subsidy-opening_treasury; excludes public works', total_subsidy=a+b, opening_treasury=treasury)))
    minimum_pair = min(v(a['total_subsidy']) for a in allocations)
    d['capital_feasibility'] = group(early_works_packages=group(**packages),
        maximum_first_factory_subsidy=m(maximum_subsidy,'currency','D020.1','max(candidate cash_gaps)',candidate_ids=[8,29],groups=['group_a','group_b']),
        one_factory_plus_works_reserve=m(maximum_subsidy+works_reserve,'currency','D020.1','maximum_first_factory_subsidy+early_public_works_reserve',maximum_first_factory_subsidy=maximum_subsidy,early_public_works_reserve=works_reserve),
        remaining_after_maximum_package=m(treasury-maximum_subsidy-works_reserve,'currency','D020.1','opening_treasury-maximum_first_factory_subsidy-early_public_works_reserve',opening_treasury=treasury,maximum_first_factory_subsidy=maximum_subsidy,early_public_works_reserve=works_reserve),
        distinct_group_allocations=allocations,
        minimum_two_candidate_subsidy=m(minimum_pair,'currency','D020.1','min(distinct_group_allocations.total_subsidy)',allocation_count=len(allocations)),
        assumption='Each group contributes opening cash minus its own retained operating buffer once; no starting cash reuse, operating profit, new equity or treasury inflow. Public works are an illustrative pending package, not a complete playable route or B6 test. Treasury is an explicit scenario endowment, not monthly income.')

    profits = {}
    for mode in ('ordinary','maritime','long'):
        ep = v(p[mode+'_energy_price'])
        sales = output*price
        energy_bill = energy_per_factory*ep
        sales_tax = sales*sales_rate
        before_tax = sales-wage_bill-other_fixed-energy_bill
        after_tax = before_tax-sales_tax
        contribution = price*(1-sales_rate)-energy_per_factory/output*ep
        threshold = (wage_bill+other_fixed)/contribution
        profits[mode] = group(
            gross_sales=m(sales,'currency/month','D020.2','orders*goods_price',orders=output,goods_price=price),
            gross_wages=m(wage_bill,'currency/month','D020.2','workers*monthly_wage; fixed within this operating month',workers=workers,monthly_wage=v(p['monthly_wage'])),
            other_fixed_cost=p['factory_other_fixed_monthly_cost'],
            energy_cost=m(energy_bill,'currency/month','D020.2','orders/new_factory_output*new_factory_energy*energy_price',orders=output,new_factory_output=output,new_factory_energy=energy_per_factory,energy_price=ep),
            sales_tax=m(sales_tax,'currency/month','D020.2','gross_sales*manufacturing_sales_tax_rate',gross_sales=sales,manufacturing_sales_tax_rate=sales_rate),
            before_tax_profit=m(before_tax,'currency/month','D020.2','gross_sales-gross_wages-other_fixed_cost-energy_cost',gross_sales=sales,gross_wages=wage_bill,other_fixed_cost=other_fixed,energy_cost=energy_bill),
            after_tax_profit=m(after_tax,'currency/month','D020.2','before_tax_profit-sales_tax; no separate corporate profit tax proposed',before_tax_profit=before_tax,sales_tax=sales_tax),
            after_tax_margin=m(after_tax/sales,'ratio','D020.2','after_tax_operating_profit/gross_sales; denominator is sales before sales tax',after_tax_profit=after_tax,gross_sales=sales),
            before_tax_margin=m(before_tax/sales,'ratio','D020.2','before_tax_profit/gross_sales',before_tax_profit=before_tax,gross_sales=sales),
            contribution_per_order=m(contribution,'currency/goods','D020.2','goods_price*(1-sales_tax_rate)-energy_per_output*energy_price',goods_price=price,sales_tax_rate=sales_rate,energy_per_output=energy_per_factory/output,energy_price=ep),
            break_even_orders=m(threshold,'goods/month','D020.2','(gross_wages+other_fixed_cost)/contribution_per_order; losses strictly below threshold',gross_wages=wage_bill,other_fixed_cost=other_fixed,contribution_per_order=contribution),
            break_even_output_share=m(threshold/output,'ratio','D020.2','break_even_orders/new_factory_output',break_even_orders=threshold,new_factory_output=output),
            order_drop_to_break_even=m(1-threshold/output,'ratio','D020.2','1-break_even_orders/new_factory_output',break_even_orders=threshold,new_factory_output=output),
            maximum_whole_orders_still_loss=m(math.ceil(threshold)-1,'goods/month','D020.2','ceil(break_even_orders)-1',break_even_orders=threshold))
    d['factory_profit_arithmetic'] = group(cases=group(**profits),
        assumption='Unimplemented arithmetic only. Orders equal actual production and sales, between 0 and nameplate output; no inventory, dynamic pricing or automatic order guarantee. Energy and gross-sales tax vary linearly with actual orders. Gross wages and other operating cost stay fixed for this monthly comparison; reduced staffing/closure is not simulated. Payroll tax is withheld from gross wages and is not another firm expense. No operating subsidy or corporate profit tax. Profit means operating surplus after proposed sales tax, excluding depreciation, capital amortization, financing and capital expenditure; it is not complete accounting net income. Energy passed through by government is charged only for energy the new factory actually uses; unused take-or-pay energy belongs to the government cost ledger.')

    # Matched factory counts prevent attributing growth to the renewal choice.
    comparisons = {}
    for period, start, end, refusal_aid in [('M9_M12',9,12,aid),('M13_M24',13,24,0)]:
        rows = []
        for n in range(3):
            bought = contract_quantity*long_price
            sold = n*energy_per_factory*long_price
            unused = bought-sold
            e_net = taxes+new_tax(n)+aid-spending-bought+sold
            r_net = taxes+new_tax(n)+refusal_aid-spending
            rows.append(group(factory_count=n,
                additional_factory_taxes=m(new_tax(n),'currency/month','D020.3','factory_count*reference_factory_tax',factory_count=n,reference_factory_tax=factory_tax),
                e_aid=m(aid,'currency/month','D020.3','renewed aid assumed through M24',monthly_transition_aid=aid),
                refusal_aid=m(refusal_aid,'currency/month','D018.3;D020.3','aid through M12 inclusive; zero from M13',period_start=start,period_end=end,monthly_transition_aid=aid),
                e_government_purchase=m(bought,'currency/month','D020.3','renewal_quantity*long_energy_price',renewal_quantity=contract_quantity,long_energy_price=long_price),
                e_government_factory_sales=m(sold,'currency/month','D020.3','factory_count*new_factory_energy*long_energy_price',factory_count=n,new_factory_energy=energy_per_factory,long_energy_price=long_price),
                e_unused_energy_cost=m(unused,'currency/month','D020.3','government_purchase-government_factory_sales',government_purchase=bought,government_factory_sales=sold),
                e_net_recurring=m(e_net,'currency/month','D020.3','existing_tax+additional_factory_taxes+aid-recurring_spending-purchase+factory_sales',existing_tax=taxes,additional_factory_taxes=new_tax(n),aid=aid,recurring_spending=spending,purchase=bought,factory_sales=sold),
                refusal_net_recurring=m(r_net,'currency/month','D020.3','existing_tax+additional_factory_taxes+refusal_aid-recurring_spending',existing_tax=taxes,additional_factory_taxes=new_tax(n),refusal_aid=refusal_aid,recurring_spending=spending),
                e_minus_refusal=m(e_net-r_net,'currency/month','D020.3','e_net_recurring-refusal_net_recurring',e_net_recurring=e_net,refusal_net_recurring=r_net)))
        comparisons[period] = group(start_month=start,end_month=end,interval='inclusive',rows=rows)
    d['renewal_fiscal_comparison'] = group(periods=group(**comparisons),
        assumption='Matched 0/1/2 full-output factories in E and refusal. All reference workers come from previously idle domestic labor, so full factory payroll tax is incremental. Factories pay for their own energy; government only books E purchases and E sales, never deducts private energy costs twice. Refusal uses maritime energy privately, outside government budget. Excludes construction, subsidy and migration costs equally from both sides; this is a recurring-flow comparison, not a feasible investment timeline or B15. Renewed aid is assumed through M24 without any unconfirmed W fee; W effects on water/targets are not simulated.')

    walks = {}
    for route in ('W','E','refuse'):
        balance = treasury
        rows = []
        for month in range(1,25):
            month_aid = aid if month <= 12 or route in ('W','E') else 0
            purchase = contract_quantity*long_price if route == 'E' and month >= 9 else 0
            net = round(taxes+month_aid-spending-purchase,10)
            opening = balance
            balance = round(balance+net,10)
            rows.append(group(month=month,
                opening_treasury=m(opening,'currency','D020.1;D018.4','previous month closing treasury; M1 opening_treasury',month=month),
                tax_revenue=d['fiscal']['existing_tax_revenue'],
                aid=m(month_aid,'currency/month','D020.1;D018.4;D018.3','M1–12 all routes; W/E renewal extends through M24 inclusive',route=route,month=month,monthly_transition_aid=aid),
                recurring_spending=d['fiscal']['recurring_spending'],
                government_energy_purchase=m(purchase,'currency/month','D020.3;D020.1;D018.4','E only: M9–32 inclusive, renewal_quantity*long_energy_price; report stops at M24',route=route,month=month,renewal_quantity=contract_quantity,long_energy_price=long_price),
                government_energy_sales=m(0,'currency/month','D020.3;D020.1;D018.4','no new factories, therefore zero sales',factory_count=0),
                unused_energy_cost=m(purchase,'currency/month','D020.3;D020.1;D018.4','government_energy_purchase-government_energy_sales',government_energy_purchase=purchase,government_energy_sales=0),
                net_recurring=m(net,'currency/month','D020.1;D018.4','tax_revenue+aid-recurring_spending-government_energy_purchase+government_energy_sales',tax_revenue=taxes,aid=month_aid,recurring_spending=spending,government_energy_purchase=purchase,government_energy_sales=0),
                closing_treasury=m(balance,'currency','D020.1;D018.4','opening_treasury+net_recurring; rounded to 10 decimals for arithmetic display',opening_treasury=opening,net_recurring=net)))
        remaining = 8 if route == 'E' else 0
        future = remaining*contract_quantity*long_price
        walks[route] = group(monthly=rows,
            m24_treasury=m(balance,'currency','D020.1;D018.4','M24 closing_treasury',month=24),
            m24_treasury_to_recurring_spending=m(balance/spending,'month','D020.1;D018.4','M24 treasury/base recurring spending; excludes future energy payments',m24_treasury=balance,recurring_spending=spending),
            future_contract_months=m(remaining,'month','D018.2;D020.1;D018.4','E: M25–32 inclusive, still legally committed; others zero',contract_start=9 if route=='E' else None,contract_end=32 if route=='E' else None,report_end=24),
            future_gross_purchase_commitment=m(future,'currency','D018.2;D020.1;D018.4','future_contract_months*renewal_quantity*long_energy_price; not overdue debt',future_contract_months=remaining,renewal_quantity=contract_quantity,long_energy_price=long_price),
            overdue_at_m24=m(0,'currency','D020.1;D018.4','all due M1–24 payments covered by nonnegative treasury',route=route),
            note=('M24 buffer is thin; under unchanged zero-factory conditions M25 energy payment alone exceeds cash. Future aid beyond M24 is unspecified; no stop-aid, forced cut or debt behavior is simulated.' if route=='E' else 'No future take-or-pay commitment.'))
    d['month_1_to_24_no_investment_arithmetic'] = group(routes=group(**walks),
        assumption='Only M9 W/E/refusal decision, then no construction, new factory, hiring or discretionary energy purchase. E contractual automatic purchases are mandatory. All intervals inclusive: M1–8 before choice; M9 choice before month settlement and immediate first purchase; 24 payments M9–32. Original aid M1–12; W/E renew aid to at least M24, a pending timing assumption. W has no proposed cash fee. Fixed tax base, spending, population and imports; no forced cuts needed through M24. No migration is executed. No gameplay execution, B2/B3/B10/B15, dominance or experiential claim.')

    idle_total = sum(v(d['regions'][str(i)]['idle']) for i in PLAYER)
    migration = {}
    for i in (8,29):
        local = v(d['regions'][str(i)]['idle'])
        need = workers-local
        migration[str(i)] = group(local_idle=d['regions'][str(i)]['idle'],
            local_idle_in_factory_units=m(local/workers,'ratio','D020.4','local_idle/new_factory_workers',local_idle=local,new_factory_workers=workers),
            first_factory_migration_need=m(need,'person','D020.4','new_factory_workers-local_idle',new_factory_workers=workers,local_idle=local),
            other_domestic_idle=m(idle_total-local,'person','D020.4','total_domestic_idle-local_idle',total_domestic_idle=idle_total,local_idle=local),
            migration_need_to_available_share=m(need/(idle_total-local),'ratio','D020.4','first_factory_migration_need/other_domestic_idle',first_factory_migration_need=need,other_domestic_idle=idle_total-local))
    d['domestic_labor_arithmetic'] = group(candidates=group(**migration),
        total_idle=m(idle_total,'person','D020.4','sum(region idle), no absolute cap',player_cell_ids=PLAYER),
        capital_immediate_factory_labor_share=m(v(d['regions']['39']['idle'])/workers,'ratio','D020.4','capital_idle/new_factory_workers',capital_idle=v(d['regions']['39']['idle']),new_factory_workers=workers),
        assumption='D020 overrides D017.5 old every-region-below-one-factory wording: only inland candidates must remain below 100; capital has at least 100. Any migrant is removed from idle labor in another owned cell and added to destination labor/population once; foreign arrivals zero, total domestic population conserved. Nationwide idle labor is 295: enough for two 100-worker factories, fewer than three. This checks aggregate availability only; housing, service capacity, skilled labor, migration speed and enterprise selection remain unimplemented.')

    ratio = lambda n, f, **ins: m(n,'ratio','D020.1;D017',f,**ins)
    d['calibration_actual_ratios'] = group(
        capital_population_share=ratio(v(d['regions']['39']['population'])/v(p['total_population']),'capital_population/total_population',capital_population=v(d['regions']['39']['population']),total_population=v(p['total_population'])),
        coastal_74_output_in_factory_units=ratio(v(d['regions']['74']['manufacturing_output'])/output,'coastal_74_output/new_factory_output',coastal_74_output=v(d['regions']['74']['manufacturing_output']),new_factory_output=output),
        import_output_in_factory_units=ratio(v(d['demand']['initial_imports'])/output,'initial_imports/new_factory_output',initial_imports=v(d['demand']['initial_imports']),new_factory_output=output),
        gap_to_recurring_spending=ratio(gap/spending,'post_aid_gap/recurring_spending',post_aid_gap=gap,recurring_spending=spending),
        gap_in_factory_tax_units=ratio(gap/factory_tax,'post_aid_gap/reference_factory_tax',post_aid_gap=gap,reference_factory_tax=factory_tax),
        aid_to_gap=ratio(aid/gap,'monthly_aid/post_aid_gap',monthly_aid=aid,post_aid_gap=gap),
        aid_period_balance_to_spending=ratio((aid-gap)/spending,'aid_period_balance/recurring_spending',aid_period_balance=aid-gap,recurring_spending=spending),
        maximum_package_to_treasury=ratio((maximum_subsidy+works_reserve)/treasury,'one_factory_plus_works_reserve/opening_treasury',one_factory_plus_works_reserve=maximum_subsidy+works_reserve,opening_treasury=treasury),
        minimum_pair_subsidy_to_treasury=ratio(minimum_pair/treasury,'minimum_two_candidate_subsidy/opening_treasury',minimum_two_candidate_subsidy=minimum_pair,opening_treasury=treasury),
        long_full_output_margin=profits['long']['after_tax_margin'],
        ordinary_full_output_margin=profits['ordinary']['after_tax_margin'],
        maritime_full_output_margin=profits['maritime']['after_tax_margin'],
        actual_idle_share=m(idle_total/sum(v(d['regions'][str(i)]['labor_force']) for i in PLAYER),'ratio','D020.4','sum(floor(region_labor*5%))/sum(region_labor); rounding lowers aggregate slightly',total_idle=idle_total,total_labor=sum(v(d['regions'][str(i)]['labor_force']) for i in PLAYER)),
        food_monthly_import_to_consumption=ratio(v(d['food']['background_imports'])/v(d['food']['monthly_consumption']),'background_imports/monthly_food_consumption',background_imports=v(d['food']['background_imports']),monthly_food_consumption=v(d['food']['monthly_consumption'])),
        materials_flow_in_project_units=d['materials']['sustainable_parallel_projects'])
    d['calibration_review'] = dict(owner='user', status=STATUS, activation='disabled',
        review_date=None, scheduling='User has not specified a review date; unapproved and unscheduled.',
        limitation='Joint arithmetic satisfies numerical ratios only. A 20–25% gap does not prove cuts must hit health/safety: spending-to-guardrail mapping is unimplemented, so B3 and experience remain unverified. It does not establish guarded spending cuts, complete-route funding, migration/enterprise behavior, or absence of a dominant renewal choice.')
    return d


def verify_d020(d):
    v = lambda x: x['value']
    p = d['parameters']
    ratios = d['calibration_actual_ratios']
    assert 'idle_labor_cap' not in p
    assert v(d['regions']['39']['idle']) >= v(p['new_factory_workers'])
    assert [v(d['regions'][str(i)]['idle']) for i in (39,74,8,29)] == [122,31,19,26]
    for i in (8,29):
        labor = d['domestic_labor_arithmetic']['candidates'][str(i)]
        assert 0 < v(labor['first_factory_migration_need']) <= v(labor['other_domestic_idle'])
        assert v(d['regions'][str(i)]['idle']) < v(p['new_factory_workers'])
    assert .20 <= v(ratios['gap_to_recurring_spending']) <= .25
    assert math.isclose(v(ratios['aid_to_gap']),1)
    assert math.isclose(v(ratios['gap_in_factory_tax_units']),1.3)
    assert abs(v(ratios['aid_period_balance_to_spending'])) < .001
    assert v(ratios['maximum_package_to_treasury']) < 1 < v(ratios['minimum_pair_subsidy_to_treasury'])
    for row in d['capital_feasibility']['distinct_group_allocations']:
        assert row['group_at_8'] != row['group_at_29']
        assert v(row['treasury_shortfall']) > 0
    for i in (8,29):
        assert v(d['capital_feasibility']['early_works_packages'][str(i)]['reserve_headroom']) >= 0
    # Test an order strictly below/above break even, not only full-output equality.
    for row in d['factory_profit_arithmetic']['cases'].values():
        if not isinstance(row,dict):
            continue
        assert .15-1e-12 <= v(row['after_tax_margin']) <= .25
        threshold = v(row['break_even_orders'])
        fixed = v(row['gross_wages'])+v(row['other_fixed_cost'])
        contribution = v(row['contribution_per_order'])
        assert 0 < threshold < v(p['new_factory_output'])
        assert (threshold-1)*contribution-fixed < 0 < (threshold+1)*contribution-fixed
        assert v(row['maximum_whole_orders_still_loss'])*contribution-fixed < 0
        assert (v(row['maximum_whole_orders_still_loss'])+1)*contribution-fixed >= 0
    period_rows = d['renewal_fiscal_comparison']['periods']
    for period in ('M9_M12','M13_M24'):
        rows = period_rows[period]['rows']
        assert len(rows) == 3
        for n,row in enumerate(rows):
            assert row['factory_count'] == n
            assert v(row['e_unused_energy_cost']) >= 0
            assert math.isclose(v(row['e_government_purchase'])-v(row['e_government_factory_sales']),v(row['e_unused_energy_cost']))
            aid_difference = 0 if period=='M9_M12' else v(p['monthly_transition_aid'])
            assert math.isclose(v(row['e_minus_refusal']),aid_difference-v(row['e_unused_energy_cost']),abs_tol=1e-12)
        assert math.isclose(v(rows[2]['e_unused_energy_cost']),0)
    # Pre-expiry E has no fiscal advantage; post-expiry only two factories beat refusal.
    assert all(v(r['e_minus_refusal']) <= 1e-12 for r in period_rows['M9_M12']['rows'])
    assert [v(r['e_minus_refusal']) > 0 for r in period_rows['M13_M24']['rows']] == [False,False,True]
    routes = d['month_1_to_24_no_investment_arithmetic']['routes']
    for route in ('W','E','refuse'):
        rows = routes[route]['monthly']
        assert [r['month'] for r in rows] == list(range(1,25))
        last = v(p['opening_treasury'])
        for r in rows:
            assert math.isclose(v(r['opening_treasury']),last,abs_tol=1e-9)
            net = v(r['tax_revenue'])+v(r['aid'])-v(r['recurring_spending'])-v(r['government_energy_purchase'])+v(r['government_energy_sales'])
            assert math.isclose(v(r['net_recurring']),net,abs_tol=1e-9)
            last = v(r['closing_treasury'])
            assert last >= 0 and math.isclose(last,v(r['opening_treasury'])+net,abs_tol=1e-9)
        assert math.isclose(v(routes[route]['m24_treasury']),last)
        assert v(routes[route]['overdue_at_m24']) == 0
    e = routes['E']['monthly']
    r = routes['refuse']['monthly']
    assert v(e[7]['government_energy_purchase']) == 0 < v(e[8]['government_energy_purchase'])
    assert v(r[11]['aid']) > 0 == v(r[12]['aid'])
    assert sum(v(x['government_energy_purchase'])>0 for x in e) == 16
    assert v(routes['E']['future_contract_months'])+16 == 24
    assert math.isclose(v(routes['E']['future_gross_purchase_commitment']),27.2)
    assert math.isclose(v(routes['E']['m24_treasury']),.6,abs_tol=1e-9)
    assert math.isclose(v(routes['W']['m24_treasury']),55)
    assert math.isclose(v(routes['refuse']['m24_treasury']),55-12*v(p['monthly_transition_aid']))
    assert v(routes['E']['m24_treasury']) < v(e[-1]['government_energy_purchase'])


def verify(d):
    v = lambda x: x['value']
    rs = [d['regions'][str(i)] for i in PLAYER]
    total = v(d['parameters']['total_population'])
    assert sum(v(r['population']) for r in rs) == total
    assert .38 <= v(d['regions']['39']['population'])/total <= .42
    for r in rs:
        assert v(r['population']) == v(r['non_labor'])+v(r['labor_force'])
        assert v(r['labor_force']) == v(r['employed'])+v(r['idle'])
        assert 0 <= v(r['idle']) <= v(r['labor_force'])
        assert v(r['idle']) == math.floor(Fraction(v(r['labor_force']))*Fraction(str(v(d['parameters']['idle_labor_share']))))
    outputs = sorted((v(d['regions'][str(i)]['manufacturing_output']), i) for i in PLAYER)
    assert [x[1] for x in outputs[-2:]] == [74,39]
    assert 1 <= outputs[-2][0]/v(d['parameters']['new_factory_output']) <= 1.5
    assert all(v(d['regions'][str(i)]['manufacturing_output']) == 0 for i in (8,29))
    assert v(d['demand']['initial_imports']) == 2*v(d['parameters']['new_factory_output'])
    assert v(d['demand']['initial_domestic_orders']) <= v(d['demand']['payable_demand_ceiling'])
    assert v(d['energy']['ordinary_price']) < v(d['energy']['maritime_price']) < v(d['energy']['long_price'])
    assert v(d['energy']['existing_supply']) == v(d['energy']['existing_demand'])
    fiscal=d['fiscal']
    params=d['parameters']
    assert math.isclose(v(fiscal['reference_factory_sales_tax']),v(params['new_factory_output'])*v(params['goods_price'])*v(params['manufacturing_sales_tax_rate']))
    assert math.isclose(v(fiscal['reference_factory_incremental_payroll_tax']),v(params['new_factory_workers'])*v(params['monthly_wage'])*v(params['payroll_tax_rate']))
    assert v(fiscal['reference_factory_incremental_payroll_tax']) > 0
    assert math.isclose(v(fiscal['reference_factory_tax']),v(fiscal['reference_factory_sales_tax'])+v(fiscal['reference_factory_incremental_payroll_tax']))
    assert math.isclose(v(fiscal['post_aid_gap'])/v(fiscal['reference_factory_tax']),1.3)
    assert math.isclose(v(fiscal['recurring_spending'])-v(fiscal['existing_tax_revenue']),v(fiscal['post_aid_gap']))
    assert math.isclose(v(fiscal['aid_period_balance']),v(fiscal['aid'])-v(fiscal['post_aid_gap']))
    assert math.isclose(sum(v(x) for x in d['fiscal']['spending_breakdown'].values() if isinstance(x,dict)),v(d['fiscal']['recurring_spending']))
    for i in (8,29):
        assert all(v(x)>0 for x in d['candidate_capital'][str(i)]['cash_gaps'].values() if isinstance(x,dict))
    f=d['food']
    for month in range(25):
        assert v(f['initial_inventory'])+month*(v(f['background_imports'])-v(f['monthly_consumption']))>=0
    available_food_budget = v(d['demand']['wage_income'])*(1-v(d['parameters']['payroll_tax_rate']))-v(d['demand']['goods_budget'])
    assert v(f['monthly_import_bill']) <= available_food_budget
    m=d['materials']
    assert v(m['reference_project_total']) == v(m['reference_project_monthly_use'])*v(m['reference_project_duration'])
    assert (v(m['capital_monthly_output'])+v(m['monthly_imports']))/v(m['reference_project_monthly_use']) == 2
    def walk(x):
        if isinstance(x,dict):
            if 'status' in x:
                assert x['status']==STATUS and x['activation']=='disabled'
            if 'value' in x:
                assert isinstance(x['value'], (int,float)) and math.isfinite(x['value'])
                assert x['status']==STATUS and x['activation']=='disabled'
                assert x['principle'].startswith(('D017.', 'D018.', 'D020.')) and x['unit'] and x['formula']
                assert x['assumption']
                assert 'inputs' in x and 'origin' in x
            for y in x.values(): walk(y)
        elif isinstance(x,list):
            for y in x: walk(y)
    verify_d020(d)
    walk(d)
    for k in ['parameters','regions','demand','energy','fiscal','candidate_capital','food','materials']:
        assert d[k]['status']==STATUS and d[k]['activation']=='disabled'


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--map',type=Path,required=True,help='Complete exported WorldMap JSON, raw cell IDs.')
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--design-dir',type=Path,default=Path(__file__).resolve().parents[2]/'jinglun-design'/'design',
                        help='Read-only external design directory. Defaults to jinglun-design/design beside the product repository.')
    parser.add_argument('--check',action='store_true',help='Validate and compare output without writing.')
    args=parser.parse_args()
    missing=[name for name in DESIGN_FILES if not (args.design_dir/name).is_file()]
    if missing:
        parser.error('Design inputs unavailable. Pass --design-dir pointing to jinglun-design/design containing: '+', '.join(DESIGN_FILES))
    draft=generate(args.map, args.design_dir)
    verify(draft)
    text=json.dumps(draft,ensure_ascii=False,indent=2,sort_keys=True)+'\n'
    if args.check:
        actual=json.loads(args.output.read_text())
        verify(actual)
        if args.output.read_text()!=text:
            parser.error('Output differs from deterministic regeneration or source hashes changed.')
    else:
        args.output.parent.mkdir(parents=True,exist_ok=True)
        args.output.write_text(text)
    print('PASS: D017/D018/D020 arithmetic draft checks only; pending_user_confirmation; activation disabled.')


if __name__=='__main__':
    main()
