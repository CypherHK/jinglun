# 地区对比字段边界（05a5fd3）

本轮查看的精确源码行均相对于提交 `05a5fd3bbbbabab98d4aaa02a14daf5bbae232da`，路径为仓库相对路径。

| 显示或字段 | 定义与计算 | 源码位置 |
| --- | --- | --- |
| 共享水源身份 | 按 `WorldMap.environment.water_sources` 遍历一次；通过候选 cell.water_source_ids 列出成员，不按候选地复制同一来源。UI编号是数组位置加1，来源身份仍为id。 | frontend/src/comparison/model.ts:30–38 |
| `natural_outflow` | 该月水源自然外排量，模型游戏水量单位。 | crates/world/src/water.rs:152–168 |
| `ecological_reserve` | 自然外排量的70%。这是保守游戏规则，源码明确不作为水文学或监管标准。 | crates/world/src/water.rs:162–170 |
| `planning_ceiling` | 自然外排减生态保留，即最多30%；不是已分配取水权或工程供水量。 | crates/world/src/water.rs:164–170 |
| `freshwater_ceiling`／可规划淡水 | 仅水质为fresh时取planning_ceiling，否则0；对比UI实际读取该字段。 | crates/world/src/water.rs:171–175；frontend/src/comparison/model.ts:36；frontend/src/comparison/Comparison.tsx:97 |
| 水量换算 | 原值乘 `environment.scale.water_unit_m3` 再除1,000,000，显示百万立方米；全年为12个月合计，月份为对应单月。单位比例来自采样面积平方公里乘1000。 | frontend/src/comparison/model.ts:13–17、33–37；crates/world/src/construction.rs:14–20；frontend/src/i18n/zh-CN.ts:100–107 |
| 地下储水／回补河道 | 年内储水为12个月月末storage_mm均值；全年baseflow_mm合计。月份分别取当月storage_mm与baseflow_mm。仅land_fraction>0显示。单位为陆域平均毫米，不代表井深或可直接抽取量。 | frontend/src/comparison/model.ts:23–28；frontend/src/i18n/zh-CN.ts:84–97 |
| 建设与农业条件 | 全年静态字段直接读取；有任意陆域的混合地块保留，纯海洋显示不适用。适建面积不代表审批或建设完成。 | frontend/src/comparison/Comparison.tsx:13、40–50；frontend/src/i18n/zh-CN.ts:69–70 |
| 资源潜力 | `resources[].potential`，0–100相对禀赋指数；并非储量、产量、月产能或企业库存。 | crates/world/src/lib.rs:105–108；crates/world/src/environment.rs:819–845；frontend/src/comparison/Comparison.tsx:52；frontend/src/i18n/zh-CN.ts:238、241 |
| 矿藏估计 | `minerals[].reserve_units`为游戏资源单位的地质估计存量，并非已探明吨数或可采储量。品位、埋深、开采难度为估计字段。 | crates/world/src/geology.rs:28–36；frontend/src/comparison/Comparison.tsx:53–58；frontend/src/i18n/zh-CN.ts:44 |
| 实际产出 | 当前WorldMap、地区对比中没有经济实际产出、已建供水、采矿产能、企业库存或月度治理结算字段。不能将上述规划上限、潜力和地质估计当作实际产出。对比界面明确提示。 | frontend/src/types.ts:1–130；frontend/src/i18n/zh-CN.ts:18；crates/world/src/environment.rs:822–823 |

候选列表在失败时保留来自成功地图的状态；只有请求成功后递增generationSerial，对比组件以该值为key重建，从而清空同seed的新地图候选。参见 frontend/src/ui/App.tsx:46–63、372–373。
