# 计划 11-03 总结

## 已完成

- 创建 `.planning/skills/learning-flow/REGRESSION-GUARDRAILS.md`，把 Phase 10 的风险点映射到自动化测试、人工 smoke 和命令。
- 覆盖选错项标红、非 A 正确答案、冷启动 Study bounce、Today fallback、AI 非阻塞、错词/报告持久化、侧边栏路由、排行榜/图片投票、release packaging/lifecycle。
- 明确当前环境中 Flutter focused tests 可能超时，超时必须记录为 `BLOCKED`，不能算作通过。

## 变更文件

- `.planning/skills/learning-flow/REGRESSION-GUARDRAILS.md`

## 验证

- Guardrail matrix 对每个风险都给出自动化 surface 和 manual surface。
- 命令集区分可靠 gate、可行时运行的 Flutter focused tests、以及 release-device 必跑 smoke。

## 备注

- 该文档承接 Phase 10 verifier 的 `human_needed` 结论，并保留 release-device 验证要求。
