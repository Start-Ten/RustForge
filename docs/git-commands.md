# Git 命令清单（可直接复制执行）

## 初始化与 hooks

```bash
git init -b main
git config core.hooksPath .githooks
git add -A && git commit -m "chore: initial workspace scaffold"
```

## 分支模型（trunk-based）

```bash
git checkout -b develop main                  # 开发集成分支
git checkout -b feature/rhi-software develop  # 功能分支
git checkout develop && git merge --no-ff feature/rhi-software
git branch -d feature/rhi-software
git checkout -b release/0.1.0 develop         # 发布分支
git checkout -b hotfix/0.1.1 main             # 热修复
```

## 常用操作

```bash
git status / git diff / git diff --staged
git add <path> / git add -p
git commit --amend --no-edit
git log --oneline --graph --decorate --all
git rebase develop && git rebase --continue / --abort
git tag -a v0.1.0 -m "MVP" && git push origin v0.1.0
git revert <sha>
git stash push -m wip / git stash pop
git cherry-pick <sha>
```

## 标签策略

```bash
git tag -a v0.1.0-mvp  -m "MVP:   STEP 3-11"
git tag -a v0.2.0-alpha -m "Alpha: STEP 12"
git tag -a v0.5.0-beta  -m "Beta:  STEP 13"
git tag -a v1.0.0       -m "1.0:   STEP 14 全平台分发"
```

## Git LFS（首次）

```bash
git lfs install
git lfs track "*.png" "*.glb" "*.wav"   # 完整清单见 .gitattributes
git add .gitattributes
```

## 子模块（可选外部 SDK）

```bash
git submodule add <url> external/<name>
git submodule update --init --recursive
```

## 钩子安装（新克隆后）

```bash
git config core.hooksPath .githooks   # pre-commit / commit-msg / pre-push
```

## 提交规范（Conventional Commits）

```
feat(rf-rhi): add software rasterizer
fix(rf-ecs): archetype move keeps change ticks
docs(readme): add quickstart
perf(rf-math): sse-friendly mat4 layout
test(rf-asset): png importer roundtrip
ci: add aarch64 cross-check
```
