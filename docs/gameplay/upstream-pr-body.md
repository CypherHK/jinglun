## Why

地块档案只能逐块查看，选址时难以对照建设、农业与水资源条件。此变更在档案侧栏加入候选地比较。

## What changed

- 按加入顺序比较 2—4 个地区，支持查看、移除和清空。切换月份、图层、主题与选区时保留候选；生成成功后清空，失败时保留。
- 按月或全年比较气候和水量，共享水源按 ID 去重。资源潜力、地质估计和规划上限沿用现有含义。
- 应用版本为 `0.6.1-gameplay`，支持窄屏横向表格、固定指标列与键盘滚动。

## Scope

本次交付地区选址比较。国家、企业投资、治理月结和玩法存读档留待后续开发。

## Blast Radius

变更涉及档案侧栏和生成成功后的候选重置。Rust 后端、地图生成器、HTTP 接口与依赖版本均无变更。

## Verification

- 本轮 Cloud 独立复验通过：`npm run check`、18 项前端测试、22 项 Rust 基线测试和生产构建。
- 真实 Rust 后端与 Chromium 154 的 24 项浏览器检查通过，覆盖桌面、390px 深浅主题、键盘滚动、共享水源、失败保留和可见重试成功后的清空。没有未捕获页面异常；唯一 503 为主动注入的失败场景。
- [本轮报告、脚本、日志和截图](https://github.com/CypherHK/jinglun/tree/Develop/docs/gameplay/acceptance/2026-10-23) 与旧环境记录分开保存。未安装或升级依赖。

@CircuitMurderer 地区对比已在 [fork 评审 PR #1](https://github.com/CypherHK/jinglun/pull/1) 获得你的批准。这是同一功能提交的正式上游 PR，目标为 `main`。后续 gameplay 开发将进入贡献者 fork 的 `Develop` 分支。
