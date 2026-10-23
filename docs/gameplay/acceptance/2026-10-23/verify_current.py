import json, pathlib, time, traceback, urllib.request, hashlib, os, argparse, shutil
from playwright.sync_api import sync_playwright
parser=argparse.ArgumentParser(description='Verify candidate comparison against production frontend and real Rust map API; requires Python Playwright and existing Chromium.')
parser.add_argument('--output',default=os.environ.get('ACCEPTANCE_OUTPUT',str(pathlib.Path(__file__).parent)))
parser.add_argument('--base-url',default=os.environ.get('ACCEPTANCE_BASE_URL','http://127.0.0.1:4173'))
parser.add_argument('--api-url',default=os.environ.get('ACCEPTANCE_API_URL','http://127.0.0.1:3001'))
parser.add_argument('--chromium',default=os.environ.get('ACCEPTANCE_CHROMIUM',shutil.which('chromium') or shutil.which('chromium-browser') or 'chromium'))
args=parser.parse_args()
OUT=pathlib.Path(args.output).resolve(); OUT.mkdir(parents=True,exist_ok=True)
CONFIG={'seed':'经纶·综合治理·01','cell_count':100,'relaxations':2,'land_ratio':0.45}
results={'scope':'Fresh verification of 05a5fd3; historical handoff results were not reused. Script authored for this run.','checks':[], 'page_errors':[], 'console_errors':[]}
def check(name, value, evidence=None):
    results['checks'].append({'name':name,'passed':bool(value),'evidence':evidence})
    assert value, name
req=urllib.request.Request(args.api_url.rstrip('/')+'/api/maps/generate', data=json.dumps(CONFIG).encode(),headers={'Content-Type':'application/json'})
t0=time.perf_counter()
with urllib.request.urlopen(req) as r: raw=r.read()
response=json.loads(raw); world=response['map']
(OUT/'map-response-100.json').write_bytes(raw)
compact=json.dumps(world,ensure_ascii=False,separators=(',',':')).encode()
(OUT/'map-sample-100.json').write_bytes(compact)
results['sample']={'path':str(OUT/'map-sample-100.json'),'bytes':len(compact),'sha256':hashlib.sha256(compact).hexdigest(),'api_response_bytes':len(raw),'generation_ms':response['meta']['generation_ms'],'request_elapsed_s':time.perf_counter()-t0,'config':world['config'],'generator_version':world['generator_version'],'cells':len(world['cells']),'water_sources':len(world['environment']['water_sources'])}
# Prefer a shared source with a non-zero freshwater budget to make duplicate accounting meaningful.
pair=None
for s in world['environment']['water_sources']:
    members=[c['id'] for c in world['cells'] if s['id'] in c['water_source_ids'] and c['land_fraction']>0]
    if len(members)>=2 and sum(m['freshwater_ceiling'] for m in s['months'])>0:
        pair=members[:2]; break
assert pair
mixed=next(c['id'] for c in world['cells'] if 0<c['land_fraction']<1 and c['id'] not in pair)
ocean=next(c['id'] for c in world['cells'] if c['land_fraction']==0 and c['id'] not in pair)
ids=pair+[mixed,ocean]
results['candidate_ids']=ids
with sync_playwright() as p:
    browser=p.chromium.launch(executable_path=args.chromium,headless=True,args=['--no-sandbox'])
    page=browser.new_page(viewport={'width':1600,'height':1000},reduced_motion='reduce')
    page.on('pageerror',lambda e:results['page_errors'].append(str(e)))
    page.on('console',lambda m:results['console_errors'].append(m.text) if m.type=='error' else None)
    page.add_init_script('localStorage.setItem("jinglun.map-settings.v1",'+json.dumps(json.dumps(CONFIG,ensure_ascii=False))+');')
    try:
        page.goto(args.base_url)
        page.locator('#cell-select option').nth(99).wait_for(state='attached',timeout=45000)
        page.wait_for_function('!document.querySelector(".map-overlay")',timeout=15000)
        comp=page.locator('.comparison'); add=comp.get_by_role('button',name='加入所选地块',exact=True)
        comp.locator('summary').click()
        def select(i): page.locator('#cell-select').select_option(str(i))
        def names(): return comp.locator('thead').first.locator('th').all_text_contents()[1:]
        for i in ids[:2]:
            select(i); add.click()
        check('two ordered candidates',names()==[f'地块 {i+1}' for i in ids[:2]],names())
        select(ids[0]);check('duplicate cannot be pinned',add.is_disabled() and '已在候选列表中' in comp.inner_text())
        for i in ids[2:]: select(i);add.click()
        select(next(c['id'] for c in world['cells'] if c['id'] not in ids))
        check('four-candidate bound and order',names()==[f'地块 {i+1}' for i in ids] and add.is_disabled() and '最多比较四处' in comp.inner_text(),names())
        def water_expected(month=None):
            sources=[]
            for index,s in enumerate(world['environment']['water_sources']):
                members=[i for i in ids if s['id'] in world['cells'][i]['water_source_ids']]
                if members:
                    months=s['months'] if month is None else [s['months'][month]]
                    scale=world['environment']['scale']['water_unit_m3']/1e6
                    sources.append({'number':index+1,'id':s['id'],'members':members,'ceiling':sum(m['freshwater_ceiling'] for m in months)*scale,'reserve':sum(m['ecological_reserve'] for m in months)*scale})
            return sources
        def fmt(v): return page.evaluate('(v)=>new Intl.NumberFormat("zh-CN",{maximumFractionDigits:1}).format(v)',0 if v == 0 else v)
        def water_check(month=None):
            expected=water_expected(month); cards=comp.locator('.lake-card');check('water sources unique '+str(month),cards.count()==len(expected),{'actual':cards.count(),'expected':len(expected),'shared_sources':sum(len(s['members'])>1 for s in expected)})
            for idx,s in enumerate(expected):
                text=cards.nth(idx).inner_text()
                assert f'共享流域 {s["number"]}' in text
                assert '涉及候选地 '+ '、'.join(f'地块 {i+1}' for i in s['members']) in text
                expected_text=f'可规划淡水 {fmt(s["ceiling"])} · 生态保留 {fmt(s["reserve"])}'
                assert expected_text in text,(s,repr(text),repr(expected_text))
            check('all water members and numeric conversion '+str(month),True,expected[:3])
        water_check()
        landtable=comp.locator('table').nth(1); resourcetable=comp.locator('table').nth(2)
        land_before=landtable.inner_text();resource_before=resourcetable.inner_text()
        ocean_index=ids.index(ocean)+1
        check('ocean land metrics unavailable',all(row.locator('td').nth(ocean_index-1).inner_text()=='不适用或无数据' for row in landtable.locator('tbody tr').all()))
        mixed_index=ids.index(mixed)
        actual=landtable.locator('tbody tr').nth(0).locator('td').nth(mixed_index).inner_text()
        check('mixed-cell land metrics retained',actual==fmt(world['cells'][mixed]['construction']['area_km2'])+' 平方公里',actual)
        page.locator('.layer-dock button').nth(7).click();layer=page.locator('.map-stage').get_attribute('data-layer')
        comp.locator('select').select_option('0')
        check('comparison period preserves layer and static metrics',page.locator('.map-stage').get_attribute('data-layer')==layer and names()==[f'地块 {i+1}' for i in ids] and landtable.inner_text().split('\n',1)[1]==land_before.split('\n',1)[1] and resourcetable.inner_text().split('\n',1)[1]==resource_before.split('\n',1)[1])
        water_check(0)
        natural=comp.locator('table').first
        monthly_values=natural.locator('tbody tr').nth(3).locator('td').all_text_contents()
        check('monthly temperatures read actual map months',monthly_values==[fmt(world['cells'][i]['climate']['months'][0]['temperature_c'])+' °C' for i in ids],monthly_values)
        page.locator('.layer-dock button').nth(2).click();page.get_by_role('button',name='播放四季流转',exact=True).click();page.wait_for_timeout(1000)
        check('playback retains candidates',names()==[f'地块 {i+1}' for i in ids])
        comp.locator('select').select_option('annual');page.wait_for_timeout(1000)
        check('explicit comparison period stops playback',comp.locator('select').input_value()=='annual' and page.get_by_role('button',name='播放四季流转',exact=True).count()==1)
        page.get_by_role('combobox',name='外观主题').select_option('light');selected=page.locator('#cell-select').input_value()
        page.locator('.side-drawer.right .drawer-tab').click();page.locator('.side-drawer.right .drawer-tab').click()
        check('theme and drawer toggles retain pins and selection',names()==[f'地块 {i+1}' for i in ids] and page.locator('#cell-select').input_value()==selected)
        comp.get_by_role('button',name=f'查看地块 {ids[1]+1}',exact=True).click()
        check('inspect pin changes selection only',page.locator('#cell-select').input_value()==str(ids[1]) and len(names())==4)
        comp.locator('summary').scroll_into_view_if_needed();page.screenshot(path=str(OUT/'desktop-light.png'))
        # Native horizontal scrolling from a keyboard-focusable region, no global month shortcut.
        page.set_viewport_size({'width':390,'height':844});page.wait_for_timeout(250)
        page.locator('.side-drawer.right .drawer-tab').click();page.wait_for_timeout(250)
        region=comp.locator('.comparison-scroll').first
        region.scroll_into_view_if_needed();region.focus();region.evaluate('(e)=>e.scrollLeft=0')
        period=comp.locator('select').input_value();layer=page.locator('.map-stage').get_attribute('data-layer')
        region.press('ArrowRight');page.wait_for_timeout(250)
        scroll=region.evaluate('(e)=>({scrollLeft:e.scrollLeft,clientWidth:e.clientWidth,scrollWidth:e.scrollWidth,firstLeft:e.querySelector("th").getBoundingClientRect().left,regionLeft:e.getBoundingClientRect().left})')
        check('390px native keyboard scroll preserves period and layer',scroll['scrollLeft']>0 and period==comp.locator('select').input_value() and layer==page.locator('.map-stage').get_attribute('data-layer'),scroll)
        for key in ['ArrowLeft','ArrowUp','ArrowDown','Space','Home','End','PageUp','PageDown']:
            region.press(key)
        check('all scroll keys isolated from global playback shortcuts',period==comp.locator('select').input_value() and page.get_by_role('button',name='播放四季流转',exact=True).count()==1)
        region.evaluate('(e)=>e.scrollLeft=e.scrollWidth');page.wait_for_timeout(100)
        sticky=region.evaluate('(e)=>({firstLeft:e.querySelector("th").getBoundingClientRect().left,regionLeft:e.getBoundingClientRect().left,scrollLeft:e.scrollLeft})')
        check('390px sticky metric column and no document horizontal overflow',abs(sticky['firstLeft']-sticky['regionLeft'])<2 and page.evaluate('document.documentElement.scrollWidth <= innerWidth'),sticky)
        for theme in ['light','dark']:
            page.get_by_role('combobox',name='外观主题').select_option(theme)
            page.wait_for_function('(theme)=>document.documentElement.dataset.theme===theme',arg=theme)
            page.wait_for_timeout(400)
            region.scroll_into_view_if_needed()
            results.setdefault('theme_screenshots',[]).append({'theme':theme,'actual_theme':page.locator('html').get_attribute('data-theme'),'panel_background':comp.evaluate('(e)=>getComputedStyle(e).getPropertyValue("--bg-panel")'),'file':f'mobile-{theme}.png'})
            page.screenshot(path=str(OUT/f'mobile-{theme}.png'))
        # Remove candidate with focus; then refill before generation checks.
        remove=comp.get_by_role('button',name=f'移除地块 {ids[0]+1}',exact=True);remove.click()
        check('remove restores focus to adjacent remove control',page.evaluate('document.activeElement.textContent')==f'移除地块 {ids[1]+1}' and names()==[f'地块 {i+1}' for i in ids[1:]])
        page.set_viewport_size({'width':1600,'height':1000});page.wait_for_timeout(200)
        select(ids[0]);add.click();expected_names=names();selection=page.locator('#cell-select').input_value()
        if 'collapsed' in page.locator('.side-drawer.left').get_attribute('class'):page.locator('.side-drawer.left .drawer-tab').click()
        def fail(route): route.fulfill(status=503,content_type='application/json',body='{"error":{"code":"error.generation","params":{}}}')
        page.route('**/api/maps/generate',fail)
        page.get_by_role('button',name='生成这片世界',exact=True).click();page.locator('.error-symbol').wait_for()
        check('failed generation preserves pins and selected map',names()==expected_names and page.locator('#cell-select').input_value()==selection and page.locator('#cell-select option').count()==100)
        page.screenshot(path=str(OUT/'failed-generation-preserved.png'))
        page.unroute('**/api/maps/generate',fail)
        with page.expect_response(lambda r:'/api/maps/generate' in r.url and r.status==200,timeout=45000):page.get_by_role('button',name='重新尝试',exact=True).click()
        page.wait_for_function('document.querySelector(".comparison-controls strong").textContent==="候选地 0/4"')
        check('same-seed real successful generation clears pins',comp.locator('thead').count()==0 and '候选地 0/4' in comp.inner_text())
        page.wait_for_function('!document.querySelector(".map-overlay")')
        select(ids[0]);add.click();select(ids[1]);add.click();comp.get_by_role('button',name='清空候选地',exact=True).click();page.wait_for_timeout(100)
        check('clear all restores focus',comp.locator('thead').count()==0 and page.evaluate('document.activeElement.textContent')=='加入所选地块')
        check('mobile theme screenshots differ',hashlib.sha256((OUT/'mobile-light.png').read_bytes()).hexdigest()!=hashlib.sha256((OUT/'mobile-dark.png').read_bytes()).hexdigest())
        check('no uncaught browser exceptions',not results['page_errors'],results['page_errors'])
        results['passed']=True
    except Exception:
        results['passed']=False;results['failure']=traceback.format_exc();page.screenshot(path=str(OUT/'failure.png'));print(results['failure'])
    finally:
        results['browser_version']=browser.version;browser.close();(OUT/'browser-results.json').write_text(json.dumps(results,ensure_ascii=False,indent=2))
print(json.dumps({'passed':results.get('passed'),'checks':len(results['checks']),'sample':results['sample']},ensure_ascii=False))
raise SystemExit(0 if results.get('passed') else 1)
