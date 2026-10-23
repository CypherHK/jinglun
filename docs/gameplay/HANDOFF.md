# jinglun 本地接手交接

交接日期：2026-10-23，新加坡时间。本文件优先于同目录 `archive/` 中的历史草案和旧交接。

## 当前停止点

**治理玩法域暂停，尚未开工。** 用户最新截图明确要求：

> v2 草案收到，谢谢。地区对比 PR 照常推进。治理玩法域（v2 的“下一实施单元”）请先不要开工：本地已确认新的切片目标（内陆第二产业中心，玩家从 2–3 个内陆候选中选址），与 v2 第 1 节的目标和“固定管理 10 块”不一致；地块口径也还在讨论。对齐后我会发确认版规则。v2 中账本守恒、托管、拒绝原因、次月生效和水源包络这些约束方向不受影响。

因此，不能把 v2 的三国配置、固定管理 10 块、第 24 月评价指标、具体资金和阈值，或架构草图视为当前实施授权。用户还未发来新的完整规则。先完成地区对比 PR，等待规则对齐后再做治理代码。原先提出的 24 月首局、预告压力、企业自主投资是历史方向；遇到与新切片冲突的内容，以新确认版为准。地块数量和管理口径现在未定，不再默认沿用“100 块中固定经营 10 块”。

## 仓库、分支与提交

| 用途 | 位置或提交 |
| --- | --- |
| 用户 fork | https://github.com/CypherHK/jinglun |
| 上游 | https://github.com/CircuitMurderer/jinglun |
| 已有 fork 评审 PR | https://github.com/CypherHK/jinglun/pull/1 |
| 地区对比功能分支 | `gameplay/0.6.1-candidate-comparison` |
| 功能提交 | `05a5fd3bbbbabab98d4aaa02a14daf5bbae232da` |
| 上游 `main`，本轮已 fetch | `c2dc9f0d9ab404214952264c9313970750c423a5` |
| fork PR 目标分支 | `gameplay/0.6.1-review-base`，固定在 `c2dc9f0` |
| fork `main` | `fb92387f308810239b36174cb27b5778e572bb6b`，没有推进 |
| 后续工作分支 | `Develop`，大小写精确如此；从地区对比功能提交建立，随后只加入本交接和证据 |
| 应用 / 生成器版本 | `0.6.1-gameplay` / `0.6.0` |

本次交接提交不含治理实现，不改变地区对比评审分支。完整原功能 diff 在 [`candidate-comparison.patch.gz`](candidate-comparison.patch.gz)（gzip 压缩的原始 diff），10 个文件、274 行新增、4 行删除。可以用下面命令复核，避免把 `Develop` 的交接文档误算进功能差异：

```sh
git diff c2dc9f0d9ab404214952264c9313970750c423a5..05a5fd3bbbbabab98d4aaa02a14daf5bbae232da
git diff c2dc9f0d9ab404214952264c9313970750c423a5..05a5fd3bbbbabab98d4aaa02a14daf5bbae232da -- crates Cargo.toml Cargo.lock
```

第二条输出为空。地区对比没有修改 Rust 后端、生成接口、地图生成器或依赖版本。

## PR 处理结果与下一步

作者 CircuitMurderer 已于 2026-10-23 15:31 新加坡时间批准 fork PR，留言 “No change in backend, right? looks good.”：

- [作者批准与原文](https://github.com/CypherHK/jinglun/pull/1#pullrequestreview-5625236683)
- [本轮已回复，确认后端未改](https://github.com/CypherHK/jinglun/pull/1#issuecomment-6360041471)

Cloud 网络已恢复，实际网络策略为 `unrestricted / enforced`。CLI 的 `gh api user` 返回 `CypherHK`，对 fork 有写权限。连接器 REST 创建上游 PR 返回 `403 Resource not accessible by integration`；CLI `gh pr create` 的 GraphQL `createPullRequest` 也返回同样的集成权限错误。因此**没有创建新的上游 PR**。创建前后均查询上游全部状态中同一 head，没有发现重复 PR。此问题不能再描述成网络不可达，也不能把“能读 GitHub”理解成“能创建上游 PR”。

本地接手先核对自己的 CLI 认证及最新上游，再查重复 PR。用同一功能分支正式向上游 `main` 提交，正文保留 `@CircuitMurderer` 和 fork PR #1 的关系。不要为了这一步把 `Develop` 的交接文档一起提交上游。

准备好的正文见 [`upstream-pr-body.md`](upstream-pr-body.md)。在确认没有重复 PR 后：

```sh
gh pr create --repo CircuitMurderer/jinglun \
  --head CypherHK:gameplay/0.6.1-candidate-comparison \
  --base main \
  --title 'feat(gameplay): add candidate region comparison' \
  --body-file docs/gameplay/upstream-pr-body.md
```

创建成功后，核实 head/base 与状态，把新 PR 附加到本地 Codex 任务，并在新旧 PR 中互链。用户已授权该提交和必要的作者回复。不要自动合并、关闭旧评审 PR 或部署。如果仍然权限不足，记录实际错误并说明需用户修复本地 GitHub 认证/授权；不要反复把同一请求称为网络问题。

## 实现状态

接手时及本次交接时，治理相关类型、代码、测试：**没有**。检索范围为功能提交的 `crates/`、`frontend/`、`tests/` 及完整文件树。现有 `crates/world/src/lakes.rs::step` 属于水文计算，不能当作治理月结。

地区对比已实现：按加入顺序保留 2—4 个候选、按月/全年比较、共享水源 ID 去重、生成成功清空、失败保留，以及键盘和窄屏支持。相关源码和测试都在 `05a5fd3`：

- `frontend/src/comparison/Comparison.tsx`、`model.ts`、`comparison.css`、`index.ts`
- `frontend/src/ui/App.tsx`、`Inspector.tsx`、`frontend/src/i18n/zh-CN.ts`
- `tests/comparison.test.mjs`
- `package.json`、`package-lock.json` 只调整应用版本

治理原文完整保存在 [`archive/minimal-governance-v2.md`](archive/minimal-governance-v2.md)，未改写规则、状态或数值。它是已被最新指示暂停的提案，未变成代码。两份架构草图与旧交接同在 `archive/`，仅用于回溯。旧交接中的网络限制、路径及“下一实施单元”授权已过时。

## 验证证据

旧 Cloud 环境曾通过类型检查、18 项前端测试、22 项 Rust 基线测试、生产构建及真实后端 Chromium 验收。这些是历史结果。

本轮独立运行通过类型检查、18 项前端测试、22 项 Rust 基线测试、生产构建，以及真实后端与 Chromium 154 的 24 项浏览器检查。浏览器场景、截图、观察脚本修正和未覆盖项记录在 [`acceptance/2026-10-23/REPORT.md`](acceptance/2026-10-23/REPORT.md)。只以该报告及对应日志表述本轮结果，不把历史测试累计成新覆盖率。完整地图和 API 响应以 gzip 原样压缩保存；深浅主题、失败现场截图与可复跑脚本也已携带。样本只是技术诊断样本，不是获准的治理剧本。

## 保留的工程约束

- 作者负责地图设计和生成，我们负责 gameplay。应用版本采用 `X.X.X-gameplay`；地图生成器、规则、剧本和存档版本分别管理。
- 账本守恒、补贴托管、全部拒绝原因、次月生效和共享水源包络方向继续有效，但等待新的玩法规则后再落实现有草案。
- 100 地块完整地图的本轮诊断样本为 6,500,066 字节，原 API envelope 约 6.5 MB。不能因地块数少就假设载荷很小。
- 现有生成接口上限为 4096 字节。未来存档/玩法路由如需大请求，应单独设计，不能误改为全局大上限。
- 存档必须保存完整地图；不能依赖新版生成器重新生成。`WorldMap` 含 `&'static str` 且仅实现 `Serialize`，不要直接假设能派生 `Deserialize`。历史方案建议 owned JSON 保存原图、派生私有玩法投影，尚未实现。
- 地图季节预览与游戏月份必须分开。现有月份切换不推进治理时间。
- 水源 ID 是环境出口采样 ID，不能当作数组下标。淡水规划包络需要乘 `environment.scale.water_unit_m3` 才是立方米；地下水基流已包含，不能重复相加。
- 建设候选面积不是已占用土地、已建设施或实际产出；天然关联水源不代表水权、管线或实际供水。具体字段定义见本轮验收报告。
- 不自动安装或升级依赖，不自动合并 PR，不部署。没有治理回放、企业投资测试或玩法存读档测试可以继承为“通过”。

## 本地环境与 Pstack

用户现已要求转回本地开发，取代此前“所有项目工作在 Cloud”的环境要求。Cloud 仍仅完成本轮验收、文档和推送，没有在用户 Mac 上执行项目操作。

先检查本地 `AGENTS.md`、Git remotes、分支、commit、工作区和最新上游；保留用户未提交的工作。不要假设本地与 Cloud 共用 `/workspace`。取得 fork 的 `Develop` 后，先读本文件。

Pstack 仍为主导套件，所有 subagent 必须使用 **GPT-6.1 Sol (`gpt-6.1-sol`)**。这条用户明确选择优先于历史 HARNESS 中 `inherit-parent` 的默认规则。并行写作使用独立 worktree 或互不重叠的文件范围。

旧线程记录的本地原安装位置如下，**本轮没有验证这些路径在当前 Mac 上仍存在**：

- `/Users/wangyucheng/agent-skills/cursor-plugins/pstack`
- `/Users/wangyucheng/agent-skills/pstack-entrypoints/HARNESS.md`
- `~/.cursor/rules/pstack-models.mdc`

先读取实际本地映射及适用技能。若路径失效，寻找已有安装；无法取得时明确报告，不能声称已读。Cloud 原完整 281 文件导入目录已丢失，本轮只从旧线程恢复了相关文本，包括 `CLOUD-HARNESS.md`、Feature、How、Architect、Arena、验证和 PR 流程；没有完整恢复整套技能。`cursor-team-kit/deslop` 未取得，也没有执行。不要为迁移自动安装套件或运行会安装依赖的 Pstack 脚本。

Cloud 使用已有缓存完成构建，没有安装或升级依赖。本地需检测自己的 Node、Rust、依赖和浏览器状态，不要照抄 Cloud 绝对路径。通常检查命令为 `npm run check`、`npm run test:web`、`npm run build` 与 `cargo test --workspace --locked --offline`；离线缓存缺失应报告，不自动联网安装。

## 可回读的上下文

- 原架构与开发线程：`threadId=01a16705-3ca5-74bb-824b-8d99d88cd0bc`，`hostId=durable`，标题“梳理 jinglun 架构与进度”。
- 本轮 Cloud 网络与接手线程：`threadId=01a167a4-11b5-76c7-91b1-b2279efc70b9`，`hostId=durable`。

原文件从旧线程的未截断命令输出恢复；v2 另外应用了旧线程最后一次“国家比较→地区比较”的文字修正。没有原文件的独立 hash 可比，不声称完成原磁盘字节校验。
