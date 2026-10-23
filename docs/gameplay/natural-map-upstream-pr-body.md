## Why

原地图的海岸跟随治理地块边界，减少地块数时，自然轮廓也会变粗。本 PR 增加可选的「自然地形」，用已有环境采样绘制海岸、地表过渡和高程阴影，同时保留治理区选择与指标比较。

## What changed

- Rust 增量导出带版本的自然海岸、浅海、类别路径与地貌表现数据。阴影只读原高程，不改变海拔、水文、资源或建设条件。
- 自然模式增加平滑地表、独立阴影开关、按缩放出现的山地和森林符号、真实地块编号。主图与小地图共用几何和缓存纹理。
- 保留经典模式、专题图层、相机与选区。旧响应缺少表现字段时回退；Canvas 不可用时保留 SVG。自然河网使用 SVG，经典河网保留原 GPU 生命周期。

## Scope

这是两阶段的组合交付，提交按「提案、第一阶段、第二阶段」排列。Fork 内审见 [Phase 1 #2](https://github.com/CypherHK/jinglun/pull/2) 和 [Phase 2 #3](https://github.com/CypherHK/jinglun/pull/3)。

默认仍为经典模式。原采样保持 256×160，平滑不增加模拟精度；阴影使用 24 倍显示夸张。没有新增 WASM、3D、运行依赖、国家、城市或设施模型。

## Blast Radius

本 PR 同时修改 Rust 表现数据导出和前端绘制。固定样本中，两阶段新增字段以外的地图内容不变；100 地块未压缩响应由约 6.11 MB 增至 6.61 MB。高倍缩放仍受源分辨率限制。

## Verification

- 独立复跑通过 30 项 Rust 测试、28 项前端测试、TypeScript 检查和生产构建；组合分支与第二阶段的代码及锁文件一致。
- 独立对比 `jinglun-040 / 100 地块 / 2 次松弛 / 45% 陆域` 的两阶段响应，除新增 detail 和非确定性耗时外，原始地图逐字段相同。
- 原实现的 Chromium 验证涵盖 100、500、2,000 地块、深浅主题、窄屏、缩放、开关和旧数据降级。本轮另在 Chrome 人工检查全图、放大、河网与阴影开关。未验证 Safari、Firefox 或低端设备性能。

方法、完整截图与可复跑脚本见 [第一阶段记录](https://github.com/CypherHK/jinglun/blob/ad573def8500072485ac82d0b5aaece5e2d2b118/docs/verification/2026-10-23-natural-map.md) 和 [第二阶段记录](https://github.com/CypherHK/jinglun/blob/ad573def8500072485ac82d0b5aaece5e2d2b118/docs/verification/2026-10-23-natural-map-phase2.md)。

经典模式，同一世界：

![经典地块](https://raw.githubusercontent.com/CypherHK/jinglun/ad573def8500072485ac82d0b5aaece5e2d2b118/docs/verification/natural-map-phase1/classic-100.png)

自然模式，第二阶段放大细节：

![自然地形细节](https://raw.githubusercontent.com/CypherHK/jinglun/ad573def8500072485ac82d0b5aaece5e2d2b118/docs/verification/natural-map-phase2/phase2-detail.png)
