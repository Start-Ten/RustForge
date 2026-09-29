# 贡献指南

1. Fork → `feature/<scope>-<desc>` 分支（从 `develop` 切出）。
2. 提交遵循 Conventional Commits（`feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert(scope): subject`）。
3. 本地过 hooks：`git config core.hooksPath .githooks`（pre-commit: fmt+clippy+test；commit-msg: 校验；pre-push: 全量测试）。
4. PR 模板见 `.github/PULL_REQUEST_TEMPLATE.md`，Issue 模板见 `.github/ISSUE_TEMPLATE/`。
5. 新公开 API 必须先更新 `docs/step2.5-interface-freeze.md`（变更提案）。
6. 性能敏感改动附 `cargo bench`/示例计时对照。

## 代码规范
- rustfmt 默认 + `use_small_heuristics="Max"`；clippy `-D warnings`。
- unsafe 必须带 `// SAFETY:` 说明；公开项必须有 doc 注释。
- 每 STEP 一个或多个原子提交，禁止混合不相关 STEP。
