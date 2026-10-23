# jinglun 本地接手交接

交接日期：2026-10-23，新加坡时间。本文件优先于同目录 `archive/` 中的历史草案和旧交接。

## 记录职责

用户 2026-10-23 明确：设计决策只写在 `/Users/wangyucheng/Projects/jinglun-design/design/`，由 Claude 与用户讨论后写，Codex 只读该目录。实现进度只由 Codex 写入本文件，不另写 LOCAL-PROMPT 或项目地图等并行状态记录。设计规则与确认依据以 design/ 为准，本文件只记录实施状态与约束。

本次 Codex 曾在 design/ 追加接手状态与 D005。随后 Claude 接续编辑，将该条改为 D008 并加入后续说明；检查撤回时发现这些并发改动，补丁未应用。Codex 已停止写入 design/，没有覆盖 Claude 当前内容，该目录的整理归 Claude。LOCAL-PROMPT 恢复为原交接提交的历史文本，当前实现进度以本文件为准；本次生成的并行项目地图已移除。

## 当前实施范围与进度

2026-10-23 地图升级审查与提交：已核对另一 Agent 的两阶段地图改动，完成上游组合分支审查。用户随后明确授权公开提交，已创建正式 [上游 PR #2](https://github.com/CircuitMurderer/jinglun/pull/2)。[正文](natural-map-upstream-pr-body.md)和审查证据保存在本目录，详情见本文件末尾“地图升级审查与提交结果”。PR 开放、非 draft、无合并冲突、未合并。没有把地图代码集成到 Develop，也没有修改治理玩法或设计记录。

2026-10-23：用户在本地接手后的消息中提供底层开工授权截图，并指定 GPT-6.1 Sol 子代理负责具体开发，根代理负责整体把控与验收。授权仅覆盖 `jinglun-040` 100 地块及规格边界的剧本绑定、账本与托管、名额计算、月结骨架和版本化完整存档。完整可玩切片、企业决策、待办、外交续期、内参和 UI 不在本次实施范围。

开工核实：本地与远端 `Develop` 均为 `69f86cdc1014233c623663a36bc3801ba88cc218`，工作区干净；最新上游 `main` 为 `20a276e5620c26436f47120fdb4f5dcb754745d6`，产品源码一致，仅交接资料有差异。最终代码集成并推送 Develop；子代理只在独立 worktree 或不重叠模块工作，不修改原功能评审分支。

只读规格为 `/Users/wangyucheng/Projects/jinglun-design/design/handoffs/2026-10-23-first-governance-slice.md`，最初读取时 SHA-256 为 `79f8a2dd13b0a9c05e76e1ab3b1ab41b2c09ad24e3fc35a6b2c91db84b4b1bfd`。用户随后补充授权，已重读更新稿与 D017–D019；规格新 SHA-256 为 `3f41f8fb0be94f82b90eda69cb734de845e76671f6d958b651bf138209781c8b`，DECISIONS 为 `44b820ffbfc00bc8319f4691bd73664c23e65ae4920145d75a85791899af92d7`。规格的旧“未授权”标记未改写，以用户本轮消息限定实施范围。

补充范围：第 8 节第 1–3 项规则已确认，只有第 4 项内参模板、排序与风险阈值继续留空。底层需承载 D018 的固定照付不议义务、只限新增的水名额与 D019 的截止时间和默认结果数据，但企业、压力、待办生成及默认效果仍不执行。另起草 D017 九条原则对应的初始数值，每项附原则、公式和假设，统一待用户确认且不接入正式初始化。B15 已知，但本轮不实现其完整玩法回放。

结构比较与独立评判已完成，采用私有 Game 聚合与原子命令，完整地图快照入档；不采用全量事件重放系统。托管按累计进度的累计应付减已付计算，避免分期舍入及重复支付。具体代码由 GPT-6.1 Sol 在独立 worktree 实现；第二名 GPT-6.1 Sol 只负责数值草案 JSON 与标准库生成脚本，文件不重叠。根代理负责审查、集成与独立验收。

- 前置核对：固定上游、规格与 raw cell ID 口径，保留原地图与 4096 字节生成接口边界。
- 独立工作：技术方案比较只读并行；代码交给单一实现负责人，再按不重叠模块拆分。
- 共享状态：月度状态、账本、名额各有单一写入边界；根代理独占本文件，设计目录只读。
- 最小拆分：一个独立 Rust 玩法域；先验证底层事务、存读档和空月推进，不提前接入未确认的产品规则或 HTTP/UI。

2026-10-23 底层实现与独立技术验收已完成，集成到 `Develop`：主实现 `4d36e7b`，数值草案 `7aa81ec`，税收口径修正 `431a21a`，可移植生成器 `84ad310`，水名额修正及最终代码提交 `a26657e`。产物保存在 `crates/gameplay/` 与 `scripts/draft-initial-values.py`，本文件是唯一实施记录。数值确认、完整玩法实现和体验验收仍未完成，不能将本轮底层交付视为可玩切片。未安装或升级依赖。原有 Cloud 验收只证明地区对比，不继承为本次治理底层通过项。

2026-10-23 D020 改稿：用户审阅 `fae1fb6` 后，明确授权仅修改初始草案 JSON、生成脚本和本文件。已重新完整读取规格及 D017、D018、D020；当前 DECISIONS SHA-256 为 `fb7ae18e12f99b6fe653d4b056f9d175d8994dc3420f39e3fe915e89ddeffa66`，规格为 `120a5638035b90cd31101ae312611d92cfca23c2b38056540167db2bab66e5f1`，均已写入草案来源。改稿提交 `6c392a9`、`66b4ff3` 已集成到 `Develop`，草案 schema 为 `jinglun.initial-values-draft.v2`；应用仍为 `0.6.2-gameplay`。以下数值摘要替代 `fae1fb6` 首稿摘要，所有数值继续 `pending_user_confirmation` / `disabled`。Codex 没有改写设计目录，也未将 D020 接入运行时。

### 本轮产物与边界

- [`crates/gameplay/`](../../crates/gameplay/) 是独立 Rust 底层：完整自然地图与固定剧本绑定；现金和材料守恒转账；累计进度托管支付及剩余退款；土地和全局共享水源预留；固定九阶段空月结；命令幂等与失败回滚；次月能力标记；完整地图版本存读档。
- D018 只提供固定数量、单价和期限的应付款记录及显式支付、欠款投影，以及限制新增用水的名额结构。限制从记录当月起生效，截止后解除；已投运用水豁免快照与对应命令结果一起存档并核对，不支持未来排期。D019 只保存创建月、截止月、显式默认结果、优先级和选项。没有自动执行合同、企业、压力、每月三项分发或到期默认效果。
- 存档 schema 为 `1`，剧本为 `jinglun-040-foundation-1`，规则为 `foundation-quota-sample-1`，地图生成器仍为 `0.6.0`。载入保留完整 owned JSON，包括未知自然字段，不调用生成器重建地图；FNV 仅检查意外损坏，不是安全签名。
- 规则参数仍为 `Unconfirmed`；初始化必须显式传入 `Draft` / `Unconfirmed` 账户。样本土地参数 `100000 km²/名额、下限 3、上限 8` 和水参数 `400000000 m³/名额、下限 0、上限 8` 都没有获得平衡性批准，也不证明 B1 或 B15 可达。
- 原始 ID 8 与 29 均为内陆候选，39 为沿海首都。实际固定边界的连通分量是 player 1、resource 1、maritime 2；按规格保留原边界，未自行修图。
- WorldMap 的 Serialize-only、原生成接口 4096 字节上限、前端季节预览和现有服务均保持原实现；没有新增 HTTP、UI 或部署。

### D017 / D020 初始数值草案：全部待确认、未生效

完整产物为 [`jinglun-040-initial-draft.json`](../../crates/gameplay/data/jinglun-040-initial-draft.json)，生成器为 [`draft-initial-values.py`](../../scripts/draft-initial-values.py)。每项经济数值包含原则、单位、公式、输入、来源类型和 `pending_user_confirmation` / `disabled`。JSON 不被 Game 初始化引用。以下是供审阅的摘要，不是新增设计决策；确认与设计记录归用户和 Claude。

| 原则 | 草案及计算口径 |
| --- | --- |
| D017.1 人口 | 总量 10000；按自然承载指数分配，首都权重乘 2，最大余数取整。首都 4087（40.87%）；8 号 652、29 号 870、74 号 1043。逐地人口与劳动分项见 JSON。 |
| D017.2 产业 | 参考新厂 100 单位/月；首都 300，74 号 125（1.25 厂），其余为 0，包含 8、29。 |
| D017.3 需求 | 进口 200/月（2 厂）；国内初始订单 625/月。取消闲置上限后既有工资为 114.02/月，其中 60% 可用于制成品，单价 0.1，支付上限 684.12/月；初始出口订单为 0。 |
| D017.4、D020.2 能源 | 普通、海运、长约单价为 0.1 / 0.135 / 0.17；既有供需均 62.5/月，无富余。每新厂满产需 10/月；长约固定购买 20/月（2 厂用量）。 |
| D017.5、D020.4 劳动 | 劳动人口占 60%，闲置只按 5% 向下取整，无绝对上限。首都 / 74 / 8 / 29 号分别 122 / 31 / 19 / 26 人；相当于 1.22 / 0.31 / 0.19 / 0.26 厂所需。全国劳动力 5996、闲置 295，实际闲置率约 4.92%。 |
| D017.6、D020.1 财政 | 草案销售税率 12%、工资税率 1%。单厂销售税 1.2，加净增工资税 0.02，总计 1.22/月；停援缺口 `1.3 × 1.22 = 1.586/月`。既有税 6.2402、经常支出 7.8262，缺口占支出 **20.2653%**。援助恰为 1.586，援助/缺口为 100%，不作为时援助期经常结余为 0。 |
| D017.7 现金 | 两集团现金 50 / 55；建厂基准资本 40 乘真实自然成本，加运营缓冲 10。8 号资金缺口约 38.24797 / 33.24797，29 号约 27.67507 / 22.67507。 |
| D017.8 食品 | 库存 20，稳定进口与消费均 10/月；固定人口及不断供假设下第 24 月库存仍为 20。每月进口账单 0.2，按草案税后家庭预算可支付。 |
| D017.9 建材 | 库存 12，首都生产 6/月、进口 2/月。参考工程用料 12、工期 3 月，每月需 4；持续月流量支持 2 项并行，3 项连续施工 3 月耗尽开局库存。 |

D020 开局国库提案为 **55**。首厂最大补贴为 38.24797，加早期公共工程预留 15，共 53.24797，占国库 **96.8145%**，余 1.75203。公共工程仅为待确认示意包：住房基准 4 加公用设施基准 3.5，乘候选自然成本，8 / 29 号分别为 14.67149 / 12.68908；预留 15 已覆盖，未假设道路廊道价格。两集团初始现金各使用一次时，两候选各补一厂最少需 60.92304，占国库 **110.7692%**，尚未计公共工程即不足。不能把同一集团的开局现金重复用于两座厂；这些算术也不证明完整路线预算或 B6 已通过。

**工厂利润与订单边界。** 满产月产量 100，销售单价 0.1；100 人工资合计 2，另列固定非能源运营成本 3.6。能源和销售税随实际订单变化，工资与其余运营费在比较月内不变。工资税由工人从毛工资支付，不再重复扣企业。下表“利润”是扣草案销售税后的运营盈余，不含折旧、资本摊销、融资和建厂资本支出，不是完整会计净利润。

| 能源档 | 满产能源费/月 | 月运营盈余 | 盈余 / 销售额 | 盈亏平衡订单/月 | 相对 100 单位订单的下降阈值 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 普通 | 1.00 | 2.20 | 22% | 71.7949 | 28.2051% |
| 海运 | 1.35 | 1.85 | 18.5% | 75.1678 | 24.8322% |
| 长约 | 1.70 | 1.50 | 15% | 78.8732 | 21.1268% |

订单下降**严格超过**对应比例才开始亏损，等于时为盈亏平衡。公式为 `订单 × [0.1 × (1 − 12%) − 每单位能源成本] − 2 − 3.6`。订单即实际生产销售，不模拟库存、动态价格、裁员或关厂；该边界不构成 B8 验收。

**长约政府购销。** 政府每月购入 20 能源、支付 3.4；每座满产新厂按长约价支付 1.7，未使用部分单列为国库成本。下表比较双方具有相同满产厂数、相同新增税基的经常收支，不重复扣企业能源费，也不假设这些厂的建设、补贴与迁移已经完成。

| 满产新厂数 | 政府向新厂销售收入/月 | 国库承担未用能源/月 | 接受 E：月净收支 | 拒绝：第 9–12 月净收支 | 拒绝：第 13–24 月净收支 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 | 0 | 3.4 | -3.4 | 0 | -1.586 |
| 1 | 1.7 | 1.7 | -0.48 | +1.22 | -0.366 |
| 2 | 3.4 | 0 | +2.44 | +2.44 | +0.854 |

因此，停援后 0 / 1 座新厂接受 E 的月净收支比拒绝分别少 1.814 / 0.114；两座厂时多 1.586。只支持“约两厂后 E 有财政收益”的校准方向，不比较目标、护栏或全部经营后果。

**只在第 9 月作一次续期选择，此后不投资的 24 月算术。** 开局国库 55；人口、税基、经常支出和背景进口固定。原援助覆盖第 1–12 月，W/E 按“续援至少覆盖到第 24 月”的待确认时点假设计算；W 未另设现金费用。E 在第 9 月结算前签约并立即首付，24 期覆盖第 9–32 月。政府履行长约购买，不是玩家另作投资。

| 第 9 月选择 | 第 9–12 月净收支/月 | 第 13–24 月净收支/月 | 第 24 月末国库 | 第 24 月末 / 开局国库 |
| --- | ---: | ---: | ---: | ---: |
| W 优先取水 | 0 | 0 | 55 | 100% |
| E 能源长约 | -3.4 | -3.4 | 0.6 | 1.0909% |
| 都拒绝 | 0 | -1.586 | 35.968 | 65.3964% |

E 在第 9–24 月已付 `16 × 3.4 = 54.4`，到第 24 月没有到期欠款，但还剩第 25–32 月 **8 期、27.2 的未来总购买义务**。无新厂时未来能源仍由国库负担。剩余 0.6 只相当于约 0.0767 个月的基础经常支出；若继续相同静态条件，第 25 月资金不足。第 24 月后的援助未擅自定义，草案不执行自动削减、欠款、停援或违约效果。

三条路线的完整 72 条月度算式保存在 JSON。W 的账面结余最高，未体现后续住房和工厂被限制用水的代价；该结果**不是 B15 回放，也不证明 W 是最优路线**。缺口占支出 20.2653% 只证明数值比例，不能单独证明削减必触诊疗、安全护栏；支出与护栏映射仍未实现，B3 和体验须后续验证。

参考工人的新增工资税假设为：工人来自本国原无应税工资的闲置劳动，新厂工资全部属于净增；迁入者仅来自本国其他地块，须等量扣除来源地闲置及人口，不设外来移民。8 / 29 号首厂需从外地迁入 81 / 74 人，对应其他本国地块闲置池 276 / 269 人；仅验证总量充足，不模拟住房、服务、技能或迁移速度。全国 295 人够两座百人工厂，不够三座；不能重复计算同一闲置人口。工资来源、既有服务业收入、外部进口卖家和援助付款方仍是开局假设，尚未建立完整部门账本。Python 浮点只用于草案算术，正式底层整数账本不导入本稿金额。

数值确认负责人为用户，设计落笔负责人为 Claude；Codex 记录建议复核日 2026-10-24，用户尚未指定排期或确认数值，JSON 因此保留未排期。未收到确认前持续禁用。第 8 节第 4 项内参模板、排序与风险阈值继续留空；完整 B1–B15 玩法回放与人工体验未执行，不以草案算术或此前底层测试代替。

### 本地验证与复跑

本次 D020 改稿由 GPT-6.1 Sol 在独立 worktree 修改，另一名 GPT-6.1 Sol 独立审查。根代理在主工作区复跑最终 `--check`，另用标准库 `Fraction` 按独立公式核对落盘 JSON：财政、三档利润和订单阈值、0/1/2 厂两阶段政府流量、三条路线全部 72 月记录、未来义务和最新设计/脚本哈希均通过。提前激活草案、重复集团初始现金、截断剩余合同期和错误亏损阈值四项负面变异均被拒绝。这是草案生成与算术验证，不调用游戏月结或玩法回放。

此前底层代码 `a26657e` 在主工作区独立通过 33 项 Rust 测试（gameplay 11、server 1、world 21）及 gameplay Clippy `--no-deps -D warnings`；另通过 18 项原前端测试和真实地图示例。独立 GPT-6.1 Sol 审查识别出的两项水名额 P2 已修正并复测关闭。这些是上一轮验证，本次 D020 只改草案、脚本和本文件，没有重跑或新增 Rust/UI 覆盖。

根代理重新导出的完整地图 SHA-256 为 `4c8714aaca565e0c4b932420aa320cb92a901ef63c29a897e507540db4de37a8`。示例证明托管付款、次月标记、完整地图深相等及读档后继续到第 3 月；这不是完整玩法体验。复跑命令（在产品根目录执行，不安装依赖）：

```sh
cargo test --workspace --locked --offline
cargo clippy -p jinglun-gameplay --all-targets --no-deps --locked --offline -- -D warnings
npm run test:web
cargo run -p jinglun-gameplay --example foundation --locked --offline -- target/jinglun-040-map.json
python3 scripts/draft-initial-values.py --map target/jinglun-040-map.json --output crates/gameplay/data/jinglun-040-initial-draft.json --check
```

草案生成器默认只读同级 `jinglun-design/design`；其他目录布局须显式传 `--design-dir`。它会核对当前设计输入哈希，设计文件变更后不会把旧稿无条件判为一致。原始完整地图位于可再生的 `target/`，由示例导出；耐久产物为上面的代码、JSON 与本文件记录。

## 本地接手记录

2026-10-23：已在 `/Users/wangyucheng/Projects/jinglun` 从交接提交 `381bffaf9b257b96cdc94b5f676f60390a0507be` 接手 `Develop`。已创建正式[上游 PR #1](https://github.com/CircuitMurderer/jinglun/pull/1)，与 [fork 评审 PR #1](https://github.com/CypherHK/jinglun/pull/1) 互链，两者均已附加到当前本地 Codex 任务。接手记录保存在本文件，创建页面见 [upstream-pr-created.png](upstream-pr-created.png)。当时治理暂停；后续有限底层授权及实现见本文件当前进度。

最终核实时，作者 CircuitMurderer 已于 2026-10-23 16:08 新加坡时间自行合并上游 PR，提交为 `20a276e5620c26436f47120fdb4f5dcb754745d6`。GitHub API 的 `merged_by`、`merged_at` 与上游 main 均已核实，页面见 [upstream-pr-merged.png](upstream-pr-merged.png)。agent 没有调用合并操作。原 fork 评审 PR 仍开放。

本地 `origin` 指向 CypherHK fork，`upstream` 指向 CircuitMurderer 上游。原设计工作区 `/Users/wangyucheng/Projects/jinglun-design` 及其中的只读 `reference/` 保留；其 `reference/main` 仍在 `c2dc9f0`，未 fetch、切分支或改写。设计工作区没有 Git，本次没有初始化外层仓库。

## 完整玩法的停止点

**完整玩法仍未开工。** 本轮仅实施上方已授权的底层；未确认部分继续等待用户提供规则与授权，不按 archive/ 中的旧 v2 扩展范围。

设计讨论正在由 Claude 继续更新。接手时的地块口径描述是历史状态，不能替代当前 [设计决策](../../../jinglun-design/design/DECISIONS.md) 与 [设计上下文](../../../jinglun-design/design/CONTEXT.md)。这两个链接指向同级本地设计工作区；远端阅读时以用户提供的 design/ 确认版为准。Codex 不在此重写设计决策，也不从设计文件的存在推断实施授权。

## 仓库、分支与提交

| 用途 | 位置或提交 |
| --- | --- |
| 用户 fork | https://github.com/CypherHK/jinglun |
| 上游 | https://github.com/CircuitMurderer/jinglun |
| 已有 fork 评审 PR | https://github.com/CypherHK/jinglun/pull/1 |
| 正式上游 PR | https://github.com/CircuitMurderer/jinglun/pull/1；由 CircuitMurderer 于 16:08 合并 |
| 地区对比功能分支 | `gameplay/0.6.1-candidate-comparison` |
| 功能提交 | `05a5fd3bbbbabab98d4aaa02a14daf5bbae232da` |
| 上游接手时基线 | `c2dc9f0d9ab404214952264c9313970750c423a5` |
| 上游当前 `main`，作者合并后已 fetch | `20a276e5620c26436f47120fdb4f5dcb754745d6` |
| fork PR 目标分支 | `gameplay/0.6.1-review-base`，固定在 `c2dc9f0` |
| fork `main` | `fb92387f308810239b36174cb27b5778e572bb6b`，没有推进 |
| 后续工作分支 | `Develop`，大小写精确如此；原地区对比及交接之后加入本轮已授权底层 |
| 应用 / 生成器版本 | `0.6.2-gameplay` / `0.6.0` |

原交接提交 `381bffaf` 不含治理实现；地区对比评审分支未改变。完整原功能 diff 在 [`candidate-comparison.patch.gz`](candidate-comparison.patch.gz)（gzip 压缩的原始 diff），10 个文件、274 行新增、4 行删除。可以用下面命令复核，避免把 `Develop` 的后续提交误算进功能差异：

```sh
git diff c2dc9f0d9ab404214952264c9313970750c423a5..05a5fd3bbbbabab98d4aaa02a14daf5bbae232da
git diff c2dc9f0d9ab404214952264c9313970750c423a5..05a5fd3bbbbabab98d4aaa02a14daf5bbae232da -- crates Cargo.toml Cargo.lock
```

第二条输出为空。地区对比没有修改 Rust 后端、生成接口、地图生成器或依赖版本。

## PR 处理结果与下一步

作者 CircuitMurderer 已于 2026-10-23 15:31 新加坡时间批准 fork PR，留言 “No change in backend, right? looks good.”：

- [作者批准与原文](https://github.com/CypherHK/jinglun/pull/1#pullrequestreview-5625236683)
- [本轮已回复，确认后端未改](https://github.com/CypherHK/jinglun/pull/1#issuecomment-6360041471)

Cloud 阶段网络已恢复，但连接器 REST 和 CLI GraphQL 创建上游 PR 均返回 `Resource not accessible by integration`，当时没有创建成功。该历史失败不再代表当前 PR 状态。

本地接手时，`gh auth status` 报告 CypherHK 的 token invalid。连接器身份为 CypherHK，能读取仓库，但本次创建仍返回 `403 Resource not accessible by integration`。核实最新上游和全部状态 PR 搜索为空后，通过 Chrome 已登录的 CypherHK 页面完成创建，没有修改认证或全局配置。

随后 HTTPS push 的凭据读取失败，SSH 返回 `Permission denied (publickey)`。用户修复本机 gh 认证后，再次执行 `gh auth status` 和 `gh api user` 已确认 CypherHK 登录有效，具备 `repo` scope。这项修复由用户完成；agent 没有更改认证配置。后续可直接使用 gh，连接器此前的 403 作为历史记录保留。

Git 默认 osxkeychain 仍返回 `failed to get: -25293`。本次用 `git -c credential.helper= -c 'credential.helper=!gh auth git-credential' push origin Develop` 成功推送文档，覆盖只对该命令生效，没有更改 Git 配置。后续若遇同一钥匙串问题，可沿用该命令。

创建后用 GitHub API 复核：目标 `CircuitMurderer/jinglun:main`，base SHA `c2dc9f0d9ab404214952264c9313970750c423a5`；head 为 `CypherHK/jinglun:gameplay/0.6.1-candidate-comparison`，SHA `05a5fd3bbbbabab98d4aaa02a14daf5bbae232da`；1 个提交、10 个文件、274 行新增、4 行删除。PR 为 open、非 draft、未合并。正文与 [`upstream-pr-body.md`](upstream-pr-body.md) 一致，保留 `@CircuitMurderer`，没有夹带 Develop 文档。

上游正文链接原 fork 评审；原 fork PR 正文和[回链评论](https://github.com/CypherHK/jinglun/pull/1#issuecomment-6360384922)链接正式上游 PR。创建时的 open 状态与随后作者合并分开记录。agent 未关闭原 fork PR、执行合并或部署。

地区对比提交任务已完成。本轮底层范围与未确认玩法的区别见本文件“当前实施范围与进度”。

## 实现状态

原接手基线 `69f86cd` 中，治理相关类型、代码、测试：**没有**。检索范围为功能提交的 `crates/`、`frontend/`、`tests/` 及完整文件树。现有 `crates/world/src/lakes.rs::step` 属于水文计算，不能当作治理月结。本轮新增实现与验证在当前进度处记录。

地区对比已实现：按加入顺序保留 2—4 个候选、按月/全年比较、共享水源 ID 去重、生成成功清空、失败保留，以及键盘和窄屏支持。相关源码和测试都在 `05a5fd3`：

- `frontend/src/comparison/Comparison.tsx`、`model.ts`、`comparison.css`、`index.ts`
- `frontend/src/ui/App.tsx`、`Inspector.tsx`、`frontend/src/i18n/zh-CN.ts`
- `tests/comparison.test.mjs`
- `package.json`、`package-lock.json` 只调整应用版本

治理原文完整保存在 [`archive/minimal-governance-v2.md`](archive/minimal-governance-v2.md)，未改写规则、状态或数值。它是已被最新指示暂停的提案，未变成代码。两份架构草图与旧交接同在 `archive/`，仅用于回溯。旧交接中的网络限制、路径及“下一实施单元”授权已过时。

## 原地区对比的 Cloud 验证证据

本节产品运行结果均由 Cloud 提供，本地没有重新运行其类型检查、生产构建或浏览器产品验收。新 clone 没有 `node_modules`；已检测到 Node `v25.2.1` 与 Cargo `1.98.1`，没有安装或升级依赖。本地底层开发后新跑的 Rust、前端测试和示例单独记录于上方“本地验证与复跑”。

旧 Cloud 环境曾通过类型检查、18 项前端测试、22 项 Rust 基线测试、生产构建及真实后端 Chromium 验收。这些是历史结果。

2026-10-23 的 Cloud 地区对比验收通过类型检查、18 项前端测试、22 项 Rust 基线测试、生产构建，以及真实后端与 Chromium 154 的 24 项浏览器检查。浏览器场景、截图、观察脚本修正和未覆盖项记录在 [`acceptance/2026-10-23/REPORT.md`](acceptance/2026-10-23/REPORT.md)。只以该报告及对应日志表述这些结果，不把历史测试累计成新覆盖率。完整地图和 API 响应以 gzip 原样压缩保存；深浅主题、失败现场截图与可复跑脚本也已携带。Cloud 样本只是技术诊断样本，不是本轮固定治理剧本。

## 保留的工程约束

- 作者负责地图设计和生成，我们负责 gameplay。应用版本采用 `X.X.X-gameplay`；地图生成器、规则、剧本和存档版本分别管理。
- 账本守恒、补贴托管、全部拒绝原因、次月生效和共享水源包络方向继续有效。本轮验证其底层结构；完整企业拒因、补贴档位和玩法效果仍不实现。
- 100 地块完整地图的本轮诊断样本为 6,500,066 字节，原 API envelope 约 6.5 MB。不能因地块数少就假设载荷很小。
- 现有生成接口上限为 4096 字节。未来存档/玩法路由如需大请求，应单独设计，不能误改为全局大上限。
- 存档必须保存完整地图；不能依赖新版生成器重新生成。`WorldMap` 含 `&'static str` 且仅实现 `Serialize`，不要直接假设能派生 `Deserialize`。本轮采用 owned JSON 保存原图、派生私有玩法投影，保留原类型。
- 地图季节预览与游戏月份必须分开。现有月份切换不推进治理时间。
- 水源 ID 是环境出口采样 ID，不能当作数组下标。淡水规划包络需要乘 `environment.scale.water_unit_m3` 才是立方米；地下水基流已包含，不能重复相加。
- 建设候选面积不是已占用土地、已建设施或实际产出；天然关联水源不代表水权、管线或实际供水。具体字段定义见本轮验收报告。
- 不自动安装或升级依赖，不自动合并 PR，不部署。不能把原 Cloud 验收或本轮底层存读档测试继承为完整治理回放、企业投资或体验“通过”。

## 本地环境与 Pstack

用户已要求转回本地开发，取代此前“所有项目工作在 Cloud”的环境要求。接手阶段完成 clone、交接阅读、范围核对、上游 PR 创建和记录；后续按明确授权实施本文件顶部的底层任务。

先检查本地 `AGENTS.md`、Git remotes、分支、commit、工作区和最新上游；保留用户未提交的工作。不要假设本地与 Cloud 共用 `/workspace`。取得 fork 的 `Develop` 后，先读本文件。

Pstack 仍为主导套件，所有 subagent 必须使用 **GPT-6.1 Sol (`gpt-6.1-sol`)**。这条用户明确选择优先于历史 HARNESS 中 `inherit-parent` 的默认规则。并行写作使用独立 worktree 或互不重叠的文件范围。

本地接手已确认下列路径存在，并读取 HARNESS、角色配置与适用技能。接手采用 Session pickup、Opening a PR、unslop、technical-writing 和 Prove It Works；底层开发读取并应用 Feature、How、Architect、Arena、Swarm、Model the Domain、Separate Before Serializing Shared State、Prove It Works 与 No Comments。两个结构候选由 GPT-6.1 Sol 独立评判；代码和数值各有独立 GPT-6.1 Sol 审查，根代理完成集成及本地验证。没有运行会安装依赖的辅助脚本。`cursor-team-kit/deslop` 未安装，按 HARNESS 跳过，没有伪称执行；保留解释边界与不变量所需注释。

- `/Users/wangyucheng/agent-skills/cursor-plugins/pstack`
- `/Users/wangyucheng/agent-skills/pstack-entrypoints/HARNESS.md`
- `~/.cursor/rules/pstack-models.mdc`

先读取实际本地映射及适用技能。若路径失效，寻找已有安装；无法取得时明确报告，不能声称已读。Cloud 原完整 281 文件导入目录已丢失，本轮只从旧线程恢复了相关文本，包括 `CLOUD-HARNESS.md`、Feature、How、Architect、Arena、验证和 PR 流程；没有完整恢复整套技能。`cursor-team-kit/deslop` 未取得，也没有执行。不要为迁移自动安装套件或运行会安装依赖的 Pstack 脚本。

Cloud 使用已有缓存完成构建，没有安装或升级依赖。本地需检测自己的 Node、Rust、依赖和浏览器状态，不要照抄 Cloud 绝对路径。通常检查命令为 `npm run check`、`npm run test:web`、`npm run build` 与 `cargo test --workspace --locked --offline`；离线缓存缺失应报告，不自动联网安装。

## 可回读的上下文

- 原架构与开发线程：`threadId=01a16705-3ca5-74bb-824b-8d99d88cd0bc`，`hostId=durable`，标题“梳理 jinglun 架构与进度”。
- 本轮 Cloud 网络与接手线程：`threadId=01a167a4-11b5-76c7-91b1-b2279efc70b9`，`hostId=durable`。

原文件从旧线程的未截断命令输出恢复；v2 另外应用了旧线程最后一次“国家比较→地区比较”的文字修正。没有原文件的独立 hash 可比，不声称完成原磁盘字节校验。

## 地图升级审查与提交结果

2026-10-23：用户要求简单审查地图两阶段改动、准备提交给原作者，并讨论后续进化空间。审查与 PR 正文完成后，用户回答“现在创建正式上游 PR”，已完成发布。未修改地图代码、合并或部署。设计目录和 reference 保持只读。本节是实施与验证记录，后续方向只是对话建议，未升级为设计决策。

- 上游目标为 `CircuitMurderer/jinglun:main`，本轮 GitHub 核实为 `20a276e5620c26436f47120fdb4f5dcb754745d6`。
- 提交来源为 `CypherHK:feat/natural-map-upstream-ready`，远端及本地均为 `ad573def8500072485ac82d0b5aaece5e2d2b118`。仅含 `a2e119f` 提案、`69f113e` 第一阶段、`ad573de` 第二阶段，共 40 个变更文件；没有夹带 fork 的 gameplay 交接资料，也没有当前 Develop 的治理底层。
- [Fork PR #3](https://github.com/CypherHK/jinglun/pull/3) 仍开放、未合并，base 为 `67bcf0f4facdf7e6fc6ff93ae86c265d8480652f`，head 为 `42650b50d25ad6a84ebd37a0c64d2f885576effd`。GitHub 报告可合并。创建前核对上游 PR 列表，只有原地区比较 PR #1，没有重复地图 PR。
- 正式 [上游 PR #2](https://github.com/CircuitMurderer/jinglun/pull/2) 标题为 `feat(map): add optional natural terrain with shaded relief`。正文为 [natural-map-upstream-pr-body.md](natural-map-upstream-pr-body.md)。GitHub 连接器创建返回 `403 Resource not accessible by integration`，改用本机 `gh`，身份核实为 CypherHK，成功创建。API 复核目标与 head 完整 SHA、3 个提交、40 个文件、正文一致、`open`、`draft=false`、`merged=false`、`mergeable=true`、`mergeable_state=clean`。正式 PR 与 fork PR #3 均已附到当前 Codex 任务。
- 作者审阅与合并决定归 CircuitMurderer，用户跟进；建议用户 2026-10-24 复核，非作者或用户的承诺。未冒充作者批准或完成美术验收。

### 审查与本轮独立验证

Standards 和 Spec 由两个 GPT-6.1 Sol 子代理分别只读审查，均未发现确定的阻塞项。前者检查 i18n、数据职责、纹理缓存和河网生命周期；后者对照两份提案检查海岸、湖泊、阴影、缩放分级、降级与数据不变。根代理核对组合分支和全部差异；这是一轮有限审查，不是零缺陷保证。

在运行预览的第二阶段 checkout `/private/tmp/jinglun-map-phase2.dpZLCX` 独立执行 `npm run test:web` 和 `npm run build`，28 项前端测试、TypeScript 与 Vite 构建通过。Rust 默认项目缓存缺失，随后明确使用已有 Phase 1 缓存，执行 `CARGO_HOME=/private/tmp/jinglun-map-phase1.ORewgo/.cache/cargo CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline`，29 项 world 和 1 项 server 测试通过。没有安装或升级依赖。`git diff --check` 通过；组合分支与第二阶段的 crates、frontend、tests、scripts、manifest 和 lockfile 差异为空，因此复跑覆盖相同产品代码。

再次分别请求 `7312` 与 `7322` 后端，固定 `jinglun-040 / 100 / 2 / 0.45`，删除第二阶段新增 detail 后，两份 map 深相等；canonical SHA-256 为 `758258255450520f846224ebfd2655ec50e5ad5c5e5710fadc7da1a5323a7136`。新增 detail 为 257,064 字节，shade 范围 113–140，中性值 128，符号候选 146 个。原始模拟与 Phase 1 的更早比对沿用已提交验收证据，本轮没有重跑最初未改动的上游服务。

Chrome 调试连接因已有实例占用不可用，改用现有 Chrome 的新标签查看 `7320`。本轮人工检查了另一种子“经纶·山河初定”的 500 地块全图、169% 与 286% 放大、实际符号与编号、河网和阴影开关；选区与画面位置保持。截图为 [natural-map-review-detail.png](natural-map-review-detail.png)。原 Agent 的全套 Chromium 回归记录已阅读但没有全部重新执行；Safari、Firefox、低端设备和长期内存压力仍未覆盖，不宣称体验验收完成。

### 交付边界

这次包含 Rust 后端表现字段导出，不能说“后端未改”。固定样本原地图数据不变，256×160 源采样仍限制高倍细节。两阶段未压缩响应合计增加约 0.5 MB。阴影和平滑是显示方法，资源潜力不是矿山、天然通行条件不是道路。

原预览服务 `7310/7312`、`7320/7322` 为另一任务已有进程，本轮保留；没有新增部署或修改 secrets。地图公开组合分支保留原三个提交；本轮正文和审查记录只属于本地产品协作文档，未挑入地图上游 PR。创建前已重新核对远端 base/head 与重复 PR，使用上述精确分支，没有使用整个 Develop。复核记录初存独立工作树 `/Users/wangyucheng/Projects/jinglun-worktrees/natural-map-review`，避免覆盖另一任务同期写入的 D020 交接。
