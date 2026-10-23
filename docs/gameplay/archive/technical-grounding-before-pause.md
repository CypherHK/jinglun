# 已查证技术边界，暂停交接

**治理玩法域未开工，等待用户的新规则。** 用户最新目标优先：建设内陆第二产业中心，玩家从 2–3 个内陆候选中选址，地块口径仍待确定。`minimal-governance-v2.md` 第 1 节及固定 10 块不再是有效目标，v2 不能作为实施授权。本文仅保存暂停前的只读代码证据，不是待执行架构或实施方案。

查证基线为 `/workspace/jinglun-gameplay` 的 `05a5fd3bbbbabab98d4aaa02a14daf5bbae232da`。已读指定 Cloud harness、How skill/explainer prompt、v2、handoff、架构 A/B，以及下列实际源码；未改源码、未运行构建或测试、未创建 PR。

## 已核实的地图字段与单位

- `crates/world/src/lib.rs:19` 的生成器版本是 `0.6.0`。`MapConfig` 接受 100–2000 地块，默认 500，见 `:32`、`:64`。`WorldMap`、`Cell` 等为 Serialize-only，并含 `&'static str`，见 `:105`、`:111`、`:178`；不能假定直接增加 Deserialize 就能承载旧档。完整地图应保留为 owned 数据，读档不依赖再次运行生成器；这是历史技术方向，尚未实现。
- 水源 `id` 是全世界唯一的环境排水出口采样编号，不是水源数组下标，也不是湖盆 ID。水源由出口 ID 集合生成，地块通过 `water_source_ids` 共享引用，见 `crates/world/src/water.rs:182`、`crates/world/src/hydrology.rs:388`、`crates/world/src/environment.rs:771`。
- `freshwater_ceiling` 是原始模型水量；乘 `environment.scale.water_unit_m3` 才是立方米/月。尺度公式见 `crates/world/src/construction.rs:5`、`:14`，水源字段与淡水筛选见 `crates/world/src/water.rs:151`、`:161`。地下水基流已含在出口包络，不能另加一份；跨地块、跨国实际取水须共同受唯一水源上限约束，见 `docs/WATER_RESOURCES.md:31`。没有供水设施或水权从自然关联中自动产生。
- `construction.buildable_area_km2` 已是全地块平方公里面积乘适建比例，不可再次乘 `land_fraction`，见 `crates/world/src/environment.rs:690`。`is_land` 只表示陆域比例至少 50%，见 `:723`、`:731`；不能用它断言混合海陆地块完全没有真实陆域候选。
- `construction.cost_multiplier` 可为 null，表示没有适用候选条件；自然成本系数不是最终工程报价。`construction_months` 是候选施工窗口的平均指标，不是项目工期，见 `crates/world/src/construction.rs:25`、`:91`，`crates/world/src/environment.rs:697`。
- `edge.passage` 是自然候选廊道，保留海、湖、河相交标记；不表示已有道路、运力或通行权，见 `crates/world/src/construction.rs:39`、`:167` 与 `docs/GEOLOGY_SOIL_CONSTRUCTION.md:64`。
- 矿藏 `reserve_units` 是有限游戏资源单位，不是吨，也不是自动存在的可用建材库存，见 `crates/world/src/geology.rs:28` 与 `docs/GEOLOGY_SOIL_CONSTRUCTION.md:29`。

## 可保留的边界与未证实内容

用户保留资金/物资守恒、补贴托管、完整拒因、次月生效及共享水源包络方向。此前分析指出两种失败应区分：整个推进操作失败须保留旧状态；企业拒投可形成月报，但该被拒项目不得扣补贴、自付资金或材料。托管转施工实付不能再次扣可用余额，取消只退未用部分。这些是已讨论约束，尚无治理代码或运行验证。

当前 workspace 只有 world/server 两个 Rust crate，公共已有 serde/serde_json；见 `Cargo.toml:1`、`:10`。服务器地图生成路由使用 4096 字节请求上限及两个生成许可，见 `crates/server/src/main.rs:74`；本次没有改动，玩法 HTTP 协议与传输上限尚未定。

未验证任何完整治理回放、投资事务、存取续算、24 月路线平衡或新目标可玩性。已读 handoff 记载过候选地图可行性结果，但本轮未取得其地图样本重新核验，不能将该历史记录当作新目标的选址或地块分配依据。
