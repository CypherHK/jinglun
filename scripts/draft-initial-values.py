#!/usr/bin/env python3
"""Generate an inactive D017 arithmetic proposal. No Game initialization imports this file."""
import argparse
import hashlib
import json
import math
from fractions import Fraction
from pathlib import Path

STATUS = 'pending_user_confirmation'
PLAYER = [8, 9, 26, 29, 38, 39, 63, 74, 83, 91]
DESIGN = Path('/Users/wangyucheng/Projects/jinglun-design/design')


def value(number, unit, principle, formula, inputs, origin='derived_proposal'):
    return dict(value=number, unit=unit, principle=principle, formula=formula,
                inputs=inputs, origin=origin, status=STATUS, activation='disabled')


def group(**items):
    return dict(status=STATUS, activation='disabled', **items)


def generate(map_path):
    raw = map_path.read_bytes()
    world = json.loads(raw)
    assert world['config']['seed'] == 'jinglun-040'
    assert len(world['cells']) == 100
    cells = {c['id']: c for c in world['cells']}
    assert len(cells) == 100 and all(i in cells for i in PLAYER)
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
    idle_cap = parameter('idle_labor_cap', 30, 'person', 'D017.5', 'Existing employment leaves fewer idle workers than one factory needs.')
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
        idle = min(idle_cap, math.floor(Fraction(labor) * Fraction(str(idle_share))))
        output = 3 * factory_output if i == 39 else 1.25 * factory_output if i == 74 else 0
        regions[str(i)] = group(
            natural_support_index=value(cells[i]['construction']['natural_support_index'], 'index', 'D017.1', f'map.cells[id={i}].construction.natural_support_index', {'raw_cell_id': i}, 'natural_map'),
            population=value(p, 'person', 'D017.1', 'floor(total*weight/sum(weights)) plus deterministic largest remainder; ties ascending raw ID', {'total': total, 'weight': float(weights[i]), 'all_weights': {str(k): float(v) for k, v in weights.items()}}),
            labor_force=value(labor, 'person', 'D017.5', 'floor(population*labor_share)', {'population': p, 'labor_share': labor_share}),
            non_labor=value(p-labor, 'person', 'D017.5', 'population-labor_force', {'population': p, 'labor_force': labor}),
            idle=value(idle, 'person', 'D017.5', 'min(idle_labor_cap,floor(labor_force*idle_labor_share))', {'labor_force': labor, 'idle_labor_share': idle_share, 'idle_labor_cap': idle_cap}),
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
    sea = parameter('maritime_energy_price', 0.15, 'currency/energy', 'D017.4', 'Sea alternative costs more.')
    long_price = parameter('long_energy_price', 0.2, 'currency/energy', 'D017.4;D018', 'Renewal costs more than sea.')
    factory_energy = parameter('new_factory_energy', 10, 'energy/month', 'D017.4', 'Separate incremental requirement.')
    service_energy = parameter('existing_service_energy', 20, 'energy/month', 'D017.4', 'Existing nonmanufacturing demand.')
    energy_need = service_energy + domestic_output/factory_output*factory_energy
    energy = group(ordinary_price=parameters['ordinary_energy_price'], maritime_price=parameters['maritime_energy_price'], long_price=parameters['long_energy_price'],
                   existing_supply=m(energy_need, 'energy/month', 'D017.4', 'existing_demand; ordinary resource-country imports, no spare supply', existing_demand=energy_need),
                   existing_demand=m(energy_need, 'energy/month', 'D017.4', 'existing_service_energy+existing_output/new_factory_output*new_factory_energy', existing_service_energy=service_energy, existing_output=domestic_output, new_factory_output=factory_output, new_factory_energy=factory_energy),
                   incremental_factory_demand=parameters['new_factory_energy'],
                   renewal_quantity=m(2*factory_energy, 'energy/month', 'D017.4;D018', '2*new_factory_energy; draft only, no contract execution', new_factory_energy=factory_energy))
    tax_rate = parameter('manufacturing_sales_tax_rate', 0.1, 'ratio', 'D017.6', 'Proposed gross sales tax, not profit tax.')
    payroll_rate = parameter('payroll_tax_rate', 0.1, 'ratio', 'D017.6', 'Existing wage tax base.')
    factory_tax = factory_output*price*tax_rate
    taxes = domestic_output*price*tax_rate + income*payroll_rate
    gap = 1.3*factory_tax
    spending = taxes+gap
    aid = parameter('monthly_transition_aid', 4, 'currency/month', 'D017.6', 'External resource-country transfer proposal, expiry handled elsewhere.')
    fiscal = group(
        reference_factory_tax=m(factory_tax, 'currency/month', 'D017.6', 'new_factory_output*goods_price*manufacturing_sales_tax_rate', new_factory_output=factory_output, goods_price=price, manufacturing_sales_tax_rate=tax_rate),
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
    return group(schema='jinglun.initial-values-draft.v1', scenario='jinglun-040',
        provenance=dict(map_sha256=hashlib.sha256(raw).hexdigest(),generator_version=world['generator_version'],map_config=world['config'],raw_player_cell_ids=PLAYER,raw_capital_cell_id=39,raw_candidate_cell_ids=[8,29],
            source_hashes={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [DESIGN/'DECISIONS.md',DESIGN/'handoffs/2026-10-23-first-governance-slice.md']},generator='scripts/draft-initial-values.py',
            generator_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()),
        scope='Arithmetic proposal only. No enterprise, pressure, todo behavior or Game activation. All economic values require user confirmation. No B1-B15 replay or experience acceptance claimed.',
        parameters=group(**parameters),regions=group(**regions),demand=demand_group,energy=energy,fiscal=fiscal,candidate_capital=group(**capital),food=food,materials=material)


def verify(d):
    v = lambda x: x['value']
    rs = [d['regions'][str(i)] for i in PLAYER]
    total = v(d['parameters']['total_population'])
    assert sum(v(r['population']) for r in rs) == total
    assert .38 <= v(d['regions']['39']['population'])/total <= .42
    for r in rs:
        assert v(r['population']) == v(r['non_labor'])+v(r['labor_force'])
        assert v(r['labor_force']) == v(r['employed'])+v(r['idle'])
        assert 0 <= v(r['idle']) < v(d['parameters']['new_factory_workers'])
    outputs = sorted((v(d['regions'][str(i)]['manufacturing_output']), i) for i in PLAYER)
    assert [x[1] for x in outputs[-2:]] == [74,39]
    assert 1 <= outputs[-2][0]/v(d['parameters']['new_factory_output']) <= 1.5
    assert all(v(d['regions'][str(i)]['manufacturing_output']) == 0 for i in (8,29))
    assert v(d['demand']['initial_imports']) == 2*v(d['parameters']['new_factory_output'])
    assert v(d['demand']['initial_domestic_orders']) <= v(d['demand']['payable_demand_ceiling'])
    assert v(d['energy']['ordinary_price']) < v(d['energy']['maritime_price']) < v(d['energy']['long_price'])
    assert v(d['energy']['existing_supply']) == v(d['energy']['existing_demand'])
    assert math.isclose(v(d['fiscal']['post_aid_gap'])/v(d['fiscal']['reference_factory_tax']),1.3)
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
            if 'value' in x:
                assert x['status']==STATUS and x['activation']=='disabled'
                assert x['principle'].startswith('D017.') and x['unit'] and x['formula']
                assert 'inputs' in x and 'origin' in x
            for y in x.values(): walk(y)
        elif isinstance(x,list):
            for y in x: walk(y)
    walk(d)
    for k in ['parameters','regions','demand','energy','fiscal','candidate_capital','food','materials']:
        assert d[k]['status']==STATUS and d[k]['activation']=='disabled'


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--map',type=Path,required=True,help='Complete exported WorldMap JSON, raw cell IDs.')
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--check',action='store_true',help='Validate and compare output without writing.')
    args=parser.parse_args()
    draft=generate(args.map)
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
    print('PASS: D017 arithmetic draft checks only; pending_user_confirmation; activation disabled.')


if __name__=='__main__':
    main()
