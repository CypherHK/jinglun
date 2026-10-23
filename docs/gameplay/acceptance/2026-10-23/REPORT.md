# 本轮地区对比验收

2026-10-23，Cloud `/workspace/jinglun-gameplay`，精确提交 `05a5fd3bbbbabab98d4aaa02a14daf5bbae232da`，相对上游基线 `c2dc9f0d9ab404214952264c9313970750c423a5`。本文件记录新运行结果；`../../archive/previous-cloud-handoff.md` 中的历史通过项没有当作本次运行证据，也没有沿用不存在的历史浏览器脚本。本次 `verify_current.py` 从当前源码新写。

## 本轮结果

- `npm run check` 成功，日志 types.log。
- `npm run test:web` 18通过、0失败，日志 frontend-tests.log。
- `cargo test --workspace --locked --offline`：server 1、world 21通过、0失败，doc-tests 0，日志 rust-tests.log。
- `npm run build` 成功，日志 build.log；release Rust server build成功，日志 server-build.log。
- Python Playwright + 系统 Chromium `154.0.8037.57`，生产 Vite preview + 真实 release Rust HTTP API，最终24项检查全部通过。精确检查名和值见 browser-results.json；无未捕获pageerror。唯一console error为脚本有意返回的HTTP503。
- 桌面1600×1000与390×844；检查2至4候选顺序/去重/第五项限制、共享来源只列一次与全部成员、全年/单月水量换算、混合地块/纯海洋、静态字段不随月份改变、月均温、图层/播放/主题/抽屉/选择状态保留、查看候选、原生键盘滚动、sticky首列、移除焦点、失败保留、可见“重新尝试”成功清空、全部清空焦点。
- 移动端浅色和深色截图等待html实际data-theme一致及400ms绘制；记录CSS背景变量并核对图像hash不同，实际查看两图确认主题。关键截图 desktop-light.png、mobile-light.png、mobile-dark.png、failed-generation-preserved.png。
- 精确源码行及规划上限、潜力、地质估计与实际产出边界见 FIELD-DEFINITIONS.md。

未发现本范围内产品缺陷，无需产品源码修复。没有实现、测试或验证治理系统、企业决策、经济平衡或存档系统；这些工作暂停。没有验证公网部署和其他浏览器。性能仅记录单次样本，不是分布性基准。

## 真实地图样本

仓库用 `map-sample-100.json.gz` 保存完整纯 WorldMap，用 `map-response-100.json.gz` 保存 API envelope；gzip 解压后的原始文件名分别是 `map-sample-100.json` 和 `map-response-100.json`。下述字节数和 SHA-256 均指解压后的内容。

- seed `经纶·综合治理·01`，cell_count100，relaxations2，land_ratio0.45，generator_version0.6.0。
- 完整map 6,500,066字节，100地块、1,893来源。
- map SHA-256 `a85bbbdfcaf0553ed2e48f12d0c1cc82c4c7a98f876d6a01796ce26f16202421`。
- 最终API response 6,500,113字节，本次后端generation_ms311.675885，请求总耗时约0.4823s。与历史handoff的201.3ms不混写。
- 所有浏览器水量核对都基于该真实完整map，未用精简mock地图。只有故障路径专门返回503，其后的重试恢复真实后端。

## 观察脚本失败与修正

保留失败日志以免把观察问题误报为产品缺陷。

1. attempt1-observer-error.log/json：对零水量格式化的期望值失败，后续检查未运行。attempt2-observer-negative-zero.log进一步打印显示，Python Playwright传递0.0给page.evaluate时形成-0，Intl输出“-0”，产品显示正常0。最终脚本把期望值的零规范成整数0；没有改产品。
2. attempt3-observer-overlay.log/json：前20项通过后，脚本点击被错误遮罩覆盖的表单“生成这片世界”超时。产品已经提供可见“重新尝试”。最终脚本走该真实按钮，通过成功清空与clear focus；未使用force click、隐藏遮罩或DOM触发。
3. 前次移动端两主题截图没有等待绘制，不能作为双主题证据。最终等待真实theme状态、CSS背景与绘制后重拍，并人工查看。最终文件为新证据。
4. attempt3-overlay.png 是保留的失败现场截图（原failure.png），不代表最终结果。

## 本地复跑

依赖需由本地已有环境提供；脚本不会安装或升级依赖。先启动当前feature的真实Rust服务（默认3001）并运行生产前端（默认4173，有/api代理）。

```sh
python verify_current.py --output ./acceptance-output --base-url http://127.0.0.1:4173 --api-url http://127.0.0.1:3001 --chromium /path/to/chromium
```

也支持 ACCEPTANCE_OUTPUT、ACCEPTANCE_BASE_URL、ACCEPTANCE_API_URL、ACCEPTANCE_CHROMIUM。脚本会生成新map与结果；重跑建议使用新output目录保留旧证据。

本轮缓存依赖仅使用临时软链指向 `/workspace/jinglun/node_modules`。Rust使用固定1.96.1工具链，CARGO_HOME=/workspace/jinglun/.cache/cargo、RUSTUP_HOME=/workspace/.rustup、CARGO_TARGET_DIR=/workspace/jinglun/target，全部offline/locked。验收执行阶段没有安装升级依赖、修改产品源码或提交、推送代码。随后根代理只将本交接和证据提交到 Develop。后端相关路径对基线diff为空，feature改动只涉及前端、版本和前端测试。清理证据见 cleanup.json。
