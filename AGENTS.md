# jinglun 开发协作规则

用户于 2026-10-24 确认：以 fork 的 Develop 为唯一开发集成主线，主目录固定跟随它；worktree 只用于临时任务，不再使用时及时删除。本文件保存长期执行规则，当前位置和实现进度只写 `docs/gameplay/HANDOFF.md`。

## 仓库与事实来源

- `origin` 固定为 `https://github.com/CypherHK/jinglun.git`，开发集成分支为大写 `Develop`，也是 fork 的默认分支。
- `upstream` 固定为 `https://github.com/CircuitMurderer/jinglun.git`。上游 PR 必须以作者的小写 `develop` 为 base；每次核实精确分支名，不自动改投 `main`。
- 本仓库本地 Git 约定：`remote.pushDefault=origin`、`push.default=simple`、`pull.ff=only`，远端默认引用 `origin/HEAD` 指向 `origin/Develop`。只设置本仓库，不改全局配置；分叉时显式处理，不让普通 pull 自动创建合并。
- 本地主目录 `/Users/wangyucheng/Projects/jinglun` 固定检出 `Develop`，作为日常查看、运行、验收入口。交付后应与 `origin/Develop` 指向同一提交。
- `/Users/wangyucheng/Projects/jinglun-design/design/` 由用户和 Claude 维护设计，Codex 只读。实现授权以用户实际指令为准，设计文档本身不构成授权。
- `jinglun-design/reference/` 是固定版本参考副本，不作为当前开发入口，不自动 fetch、pull、切分支或写入。
- Git 当前事实用实际分支、完整提交、工作区和远端核验；工程进度读 `docs/gameplay/HANDOFF.md`，不能从另一个 worktree 的同名文件推断最新状态。

## 开工与集成

1. 检查适用指令、工作区、分支、remotes、worktree 和 HANDOFF；保留所有未提交工作。任务开始前核实最新 `origin/Develop`。
2. 新任务从该集成基线建立任务分支，使用 `gameplay/<任务>`、`feat/<任务>`、`fix/<任务>` 或 `chore/<任务>`。独立代码任务在 `/Users/wangyucheng/Projects/jinglun-worktrees/<任务>` 工作；简单工程记录可以在干净的主目录更新。
3. 并行写入使用独立 worktree 或明确不重叠的文件。只有一个主代理负责集成与推送，子代理只提交自己的任务分支，不各自推 Develop。
4. 集成前核实远端是否推进，先在任务分支处理新增基线、冲突和必要验证，再快进本地 Develop。发现本地 Develop 独有提交时先保存并检查用途，不直接 reset、覆盖或把无关任务混合合并。
5. 只以非强制推送更新 fork 的 Develop；本地主目录随后对齐已交付提交。工程检查、玩法验收和人工体验分别记录；开发快照进入 Develop 不等于已验收或可发布。
6. 活跃任务不因另一任务归位或清理被切分支、覆盖文件或删除目录。任务交接须给出路径、分支、完整提交、验证结果及未完成项。

## 上游同步与版本

- 引入上游更新是独立同步任务，在 `chore/sync-upstream-<版本或日期>` 等分支上检查兼容性后集成，不通过一次普通 pull 顺手升级地图、规则或存档基线。
- 固定剧本的地图生成器、输入哈希和已确认初值按授权保持。上游应用版本与我们的 gameplay 版本允许不同，不能只改版本号表示已同步源码。
- 应用版本使用 `X.X.X-gameplay`；地图生成器、剧本、规则和存档版本分别管理。精确身份使用完整 Git commit；不把分支名当作固定版本。
- 不自动创建或合并 PR、部署、安装或升级依赖；另有用户明确授权时按该授权执行。

## Worktree 与分支收尾

- Worktree 仅保留给正在执行的任务或仍在使用的预览；等待设计确认的已提交快照可以保留分支，不必长期占用工作目录。
- 任务完成、改动已集成且记录齐备后，及时用 `git worktree remove <路径>` 删除不用的 worktree，不留到下一轮积累。PR 仍开放不自动构成保留目录的理由，实际预览或修改仍在使用时才保留。
- 删除前检查已跟踪改动、未跟踪文件、忽略目录中的独有输入或证据，以及进程是否正在使用该目录。独有内容先提交、归档或移交；不以 `--force` 或 `git clean` 跳过检查。
- 使用过 cherry-pick 的分支核对补丁内容是否已集成，不能只依赖 `git branch --merged`。已集成临时本地分支可以退役；独有提交或原型保存在有明确用途的 `archive/<用途>-<日期>` 分支。
- 保留仍在开发、提供预览或承载开放 PR 的相关分支；不顺带删除远端分支。清理范围、归档位置及仍保留的目录写入 HANDOFF。

## 记录职责

- `AGENTS.md` 是本仓库的长期协作规则；`docs/gameplay/HANDOFF.md` 是唯一当前工程状态入口。不要新增平行进度文件或项目地图。
- 设计决策只由用户与 Claude 讨论后写入 design/；Codex 在 HANDOFF 记录输入哈希、实施结果、校准提案、缺口及待设计处理的问题。
- 完成任务时提交耐久产物并记录日期、位置、提交及验证结果。未通过验收的内容明确标记，不以工程测试或历史 Cloud 结果代替本轮玩法及体验验收。
